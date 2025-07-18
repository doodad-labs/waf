use crate::config;
use hyper::{
    body::to_bytes,
    client::HttpConnector,
    header::{CONNECTION, UPGRADE},
    upgrade::Upgraded,
    Body, Client, Request, Response, Server, Uri,
    service::{make_service_fn, service_fn},
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{io::copy_bidirectional, net::TcpStream};

async fn proxy(
    req: Request<Body>,
    client: Client<HttpConnector>,
    target: &str,
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
    *proxied_request.uri_mut() = uri;

    client.request(proxied_request).await
}

async fn handle_websocket(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let uri_path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
    let ws_addr = format!("127.0.0.1:5173"); // Your upstream WS server

    let response = Response::builder()
        .status(101)
        .header(CONNECTION, "upgrade")
        .header(UPGRADE, "websocket")
        .body(Body::empty())
        .unwrap();

    // Upgrade the incoming connection
    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                if let Ok(mut server) = TcpStream::connect(ws_addr).await {
                    if let Ok(mut upgraded_client) = upgraded_downcast(upgraded).await {
                        let _ = copy_bidirectional(&mut upgraded_client, &mut server).await;
                    }
                }
            }
            Err(e) => eprintln!("Upgrade error: {}", e),
        }
    });

    Ok(response)
}

async fn upgraded_downcast(upgraded: Upgraded) -> Result<tokio::io::DuplexStream, std::io::Error> {
    // Convert `Upgraded` into something that supports AsyncRead/AsyncWrite
    use tokio::io::duplex;

    let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);
    let (duplex_a, duplex_b) = duplex(1024);
    let (mut duplex_a_rd, mut duplex_a_wr) = tokio::io::split(duplex_a);

    // Copy from upgraded -> duplex
    tokio::spawn(async move {
        let _ = tokio::io::copy(&mut client_rd, &mut duplex_a_wr).await;
    });

    // Copy from duplex -> upgraded
    tokio::spawn(async move {
        let _ = tokio::io::copy(&mut duplex_a_rd, &mut client_wr).await;
    });

    Ok(duplex_b)
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
                    if let Some(upgrade_val) = req.headers().get(UPGRADE) {
                        if upgrade_val == "websocket" {
                            return handle_websocket(req).await;
                        }
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

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}
