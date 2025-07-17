use futures::try_join;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Uri};
use log::warn;
use std::convert::Infallible;

use crate::config;

async fn block_sql(uri: &String) -> Result<(), &'static str> {
    if uri.contains("SELECT") || uri.contains("DROP") {
        warn!("Blocked potential SQL Injection {}", uri);
        Err("XSS Detected")
    } else {
        Ok(())
    }
}

async fn block_xss(uri: &String) -> Result<(), &'static str> {
    if uri.contains("<script>") {
        warn!("Blocked potential xss attack {}", uri);
        Err("XSS Detected")
    } else {
        Ok(())
    }
}

async fn proxy_request(
    client: &Client<hyper::client::HttpConnector>,
    req: Request<Body>,
    backend_url: String,
) -> Result<Response<Body>, hyper::Error> {
    // Modify the request's URI to point to your backend application (localhost:8000)
    let uri_string = format!("{}{}", backend_url, req.uri());
    let uri: Uri = uri_string.parse().unwrap();

    let proxied_request = Request::builder()
        .method(req.method())
        .uri(uri)
        .body(req.into_body())
        .unwrap();

    client.request(proxied_request).await
}

// Handle the request
async fn handle_request(
    client: Client<hyper::client::HttpConnector>,
    req: Request<Body>,
    backend_url: String,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri().to_string();
    println!("Received request: {:?}", req);

    let filter_result = try_join!(block_sql(&uri), block_xss(&uri));

    match filter_result {
        Ok(_) => match proxy_request(&client, req, backend_url).await {
            Ok(res) => Ok(res),
            Err(err) => {
                warn!("Error proxying request: {}", err);
                Ok(Response::builder()
                    .status(500)
                    .body(Body::from("Internal Server Error"))
                    .unwrap())
            }
        },
        Err(_) => {
            //Block request
            Ok(Response::builder()
                .status(403)
                .body(Body::from("Request blocked by WAF"))
                .unwrap())
        }
    }
}

pub async fn run(settings: &config::Settings) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let make_svc = make_service_fn(move |_conn| {
        let client = client.clone();
        let settings = settings.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_request(client.clone(), req, settings.backend_url.clone()))) }
    });

    // Set up the WAF server to listen on port 3000
    let addr = ([127, 0, 0, 1], settings.listen_port).into();
    let server = Server::bind(&addr).serve(make_svc);

    println!("WAF running on http://127.0.0.1:{}", settings.listen_port);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}