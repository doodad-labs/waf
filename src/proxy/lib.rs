use crate::config;
use hyper::{
    body::to_bytes,
    client::HttpConnector,
    header::{UPGRADE},
    Body, Client, Request, Response, Server, Uri,
    service::{make_service_fn, service_fn},
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};


use super::websocket;

async fn proxy(
    rid: &str,
    req: Request<Body>,
    client: Client<HttpConnector>,
    target: &str,
    client_ip: SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body).await?;

    let uri = format!(
        "{}{}",
        target,
        parts.uri.path_and_query().map(|x| x.as_str()).unwrap_or("")
    )
    .parse::<Uri>()
    .unwrap();

    let mut proxied_request = Request::from_parts(parts, Body::from(body_bytes));
    *proxied_request.uri_mut() = uri.clone();

    // Set the headers for the proxied request
    {
        // Add the x-forwarded-for header
        proxied_request.headers_mut().insert(
            "x-forwarded-for",
            client_ip.ip().to_string().parse().unwrap(),
        );

        // Add the x-forwarded-proto header
        let scheme = if uri.scheme_str() == Some("https") {
            "https"
        } else {
            "http"
        };

        proxied_request.headers_mut().insert(
            "x-forwarded-proto",
            scheme.parse().unwrap(),
        );

        // Add the x-forwarded-host header
        proxied_request.headers_mut().insert(
            "x-forwarded-host",
            uri.host().unwrap_or("").parse().unwrap(),
        );

        // Add the x-forwarded-port header
        if let Some(port) = uri.port_u16() {
            proxied_request.headers_mut().insert(
                "x-forwarded-port",
                port.to_string().parse().unwrap(),
            );
        }

        // Add the x-request-id header
        proxied_request.headers_mut().insert(
            "x-request-id",
            rid.to_string().parse().unwrap(),
        );
    }

    client.request(proxied_request).await
}


pub async fn run(settings: &config::Settings) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], settings.listen_port));
    let client = Client::new();

    let settings_clone = Arc::new(settings.clone());
    let make_svc = make_service_fn(move |_conn| {
        let client = client.clone();
        let settings = settings_clone.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let client = client.clone();
                let settings = settings.clone();
                async move {

                    // Generate a unique request ID
                    let rid = uuid::Uuid::new_v4().to_string();

                    if req.headers().get(UPGRADE).map(|h| h.to_str().unwrap_or("")).unwrap_or("").to_lowercase() == "websocket" {
                        return websocket::handle_websocket(req, &settings.webapp_url).await;
                    }

                    let client_ip = req
                        .extensions()
                        .get::<SocketAddr>()
                        .cloned()
                        .unwrap_or(SocketAddr::new([0, 0, 0, 0].into(), 0));

                    proxy(&rid, req, client, &settings.webapp_url, client_ip).await
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("WAF proxy started successfully.");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())

}