use crate::config;
use hyper::{
    body::to_bytes,
    client::HttpConnector,
    header::{CONNECTION, UPGRADE},
    Body, Client, Request, Response, Server, Uri,
    service::{make_service_fn, service_fn},
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::io::{AsyncWriteExt, AsyncReadExt}; // Import AsyncReadExt
use hyper::upgrade;
use tokio::sync::Mutex;
use futures::{SinkExt, StreamExt};

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

async fn handle_websocket(req: Request<Body>, ws_target: &str) -> Result<Response<Body>, hyper::Error> {
    if !req.headers().contains_key(UPGRADE) {
        return Ok(Response::new(Body::from("Expected Upgrade header")));
    }

    let ws_target_with_path = format!("{}{}", ws_target, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
    if ws_target_with_path.is_empty() {
        return Ok(Response::new(Body::from("WebSocket target URL is empty")));
    }

    if !ws_target_with_path.starts_with("http://") && !ws_target_with_path.starts_with("https://") {
        return Ok(Response::new(Body::from("WebSocket target URL must start with http:// or https://")));
    }

    // Convert the WebSocket target URL to a valid format
    let ws_target_with_path = ws_target_with_path.replace("http://", "ws://").replace("https://", "wss://");

    println!("Handling WebSocket upgrade request to {}", ws_target_with_path);

    let ws_target = ws_target_with_path.to_string();
    tokio::spawn(async move {
        match upgrade::on(req).await {
            Ok(upgraded) => {
                if let Err(e) = tunnel(upgraded, ws_target).await {
                    eprintln!("websocket error: {}", e);
                }
            }
            Err(e) => eprintln!("upgrade error: {}", e),
        }
    });

    let mut response = Response::new(Body::empty());
    *response.status_mut() = hyper::StatusCode::SWITCHING_PROTOCOLS;
    response.headers_mut().insert(UPGRADE, hyper::header::HeaderValue::from_static("websocket"));
    response.headers_mut().insert(CONNECTION, hyper::header::HeaderValue::from_static("Upgrade"));
    Ok(response)
}

async fn tunnel(upgraded: upgrade::Upgraded, ws_target: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Tunneling WebSocket connection to {}", ws_target);

    let url = url::Url::parse(&ws_target)?;
    let url_string = url.to_string();

    println!("Initiating handshake with WebSocket URL: {}", url_string);
    let (ws_stream, _) = connect_async(url_string).await?;
    println!("WebSocket handshake has been successfully completed");

    let (sink, mut stream) = ws_stream.split();
    let sink = Arc::new(Mutex::new(sink));
    let upgraded = Arc::new(Mutex::new(upgraded));

    // Clone the Arc for the spawned task
    let sink_clone = Arc::clone(&sink);
    let upgraded_clone = Arc::clone(&upgraded);

    tokio::spawn(async move {
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    match msg {
                        Message::Text(text) => {
                            if let Err(e) = upgraded_clone.lock().await.write_all(text.as_bytes()).await {
                                eprintln!("Error writing to upgraded stream: {}", e);
                                break;
                            }
                        }
                        Message::Binary(data) => {
                            if let Err(e) = upgraded_clone.lock().await.write_all(&data).await {
                                eprintln!("Error writing to upgraded stream: {}", e);
                                break;
                            }
                        }
                        Message::Close(_) => {
                            println!("Received close message from WebSocket server");
                            break;
                        }
                        Message::Ping(ping_data) => {
                            if let Err(e) = sink_clone.lock().await.send(Message::Pong(ping_data)).await {
                                eprintln!("Error sending pong: {}", e);
                                break;
                            }
                        }
                        Message::Pong(_) => {
                            // Handle Pong if needed
                        }
                        Message::Frame(_) => {
                            // Handle Frame if needed
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    let mut buffer = [0u8; 4096];
    loop {
        match upgraded.lock().await.read(&mut buffer).await {
            Ok(0) => {
                println!("Upgraded stream closed");
                break;
            }
            Ok(n) => {
                if let Err(e) = sink.lock().await.send(Message::Binary(buffer[..n].to_vec().into())).await {
                    eprintln!("Error sending binary message: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error reading from upgraded stream: {}", e);
                break;
            }
        }
    }

    Ok(())
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
                        return handle_websocket(req, &settings.webapp_url).await;
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