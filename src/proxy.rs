use crate::config;
use hyper::{
    body::to_bytes,
    client::HttpConnector,
    header::{HeaderValue, CONNECTION, UPGRADE},
    Body, Client, Request, Response, Server, Uri,
    service::{make_service_fn, service_fn},
};
use std::convert::Infallible;
use std::net::SocketAddr;

async fn proxy(
    req: Request<Body>,
    client: Client<HttpConnector>,
    target: &str,
) -> Result<Response<Body>, hyper::Error> {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body).await?;
    
    // Rewrite the URI to target port 5173 on localhost
    let uri = format!("{}{}", target, parts.uri.path_and_query().map(|x| x.as_str()).unwrap_or(""))
        .parse::<Uri>()
        .unwrap();

    let mut proxied_request = Request::from_parts(parts, Body::from(body_bytes));
    *proxied_request.uri_mut() = uri;

    // Forward the request to the target server
    client.request(proxied_request).await
}

pub async fn run(settings: &config::Settings) -> Result<(), Box<dyn std::error::Error>> {

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.listen_port));
    let client = Client::new();

    // Create the server and define the service
    use std::sync::Arc;
    let settings_clone = Arc::new(settings.clone());
    let make_svc = make_service_fn(move |_conn| {
        let client = client.clone();
        let settings = settings_clone.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                let client = client.clone();
                let settings = settings.clone();
                async move {
                    // Check for WebSocket upgrade request
                    if req.headers().get(UPGRADE).map(|v| v.as_bytes()) == Some(b"websocket") {
                        let (mut parts, body) = req.into_parts();
                        let uri = format!("ws://localhost:5173{}", parts.uri.path_and_query().map(|x| x.as_str()).unwrap_or(""))
                            .parse::<Uri>()
                            .unwrap();
                        
                        parts.uri = uri;
                        let upgraded_request = Request::from_parts(parts, body);
                        
                        return client.request(upgraded_request).await;
                    }
                    
                    proxy(req, client, &settings.webapp_url.clone()).await
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!(
        "Proxy server running at http://localhost:{} forwarding to {}",
        settings.listen_port, settings.webapp_url
    );

    // Run the server
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}