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

use tokio_rustls::server::TlsStream;
use rustls::ServerConnection;
use crate::waf::TlsInspector;

use tokio_rustls::TlsAcceptor;
use rustls::{ServerConfig, Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;

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

    // Load TLS configuration if enabled
    let tls_acceptor = if settings.tls_enabled {
        // Load certificates
        let cert_file = File::open(&settings.tls_cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = File::open(&settings.tls_key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)?;
        let key = PrivateKey(keys.remove(0));

        // Create TLS config
        let mut tls_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        
        // Enable inspecting client hello
        tls_config.max_fragment_size = None;
        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        
        Some(TlsAcceptor::from(Arc::new(tls_config)))
    } else {
        None
    };

    let settings_clone = Arc::new(settings.clone());
    let make_svc = make_service_fn(move |conn| {
        let client = client.clone();
        let settings = settings_clone.clone();
        let tls_acceptor = tls_acceptor.clone();
        
        // Get client IP from connection
        let client_addr = conn.remote_addr();

        async move {
            Ok::<_, Infallible>(service_fn(move |mut req| {
                let client = client.clone();
                let settings = settings.clone();
                async move {
                    // Generate a unique request ID
                    let rid = uuid::Uuid::new_v4().to_string();

                    // Add TLS fingerprint if available
                    if let Some(tls_info) = req.extensions().get::<TlsStream>() {
                        let (_, session) = tls_info.get_ref();
                        if let Some(version) = session.protocol_version() {
                            req.headers_mut().insert(
                                "x-tls-version", 
                                format!("{:?}", version).parse().unwrap()
                            );
                        }
                        if let Some(cipher) = session.negotiated_cipher_suite() {
                            req.headers_mut().insert(
                                "x-tls-cipher", 
                                cipher.suite().to_string().parse().unwrap()
                            );
                        }
                    }

                    if req.headers().get(UPGRADE).map(|h| h.to_str().unwrap_or("")).unwrap_or("").to_lowercase() == "websocket" {
                        return websocket::handle_websocket(req, &settings.webapp_url).await;
                    }

                    let client_ip = req
                        .extensions()
                        .get::<SocketAddr>()
                        .cloned()
                        .unwrap_or(client_addr);

                    proxy(&rid, req, client, &settings.webapp_url, client_ip).await
                }
            }))
        }
    });

    let server = if let Some(acceptor) = tls_acceptor {
        // Create TLS listener
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        println!("WAF proxy started successfully with TLS on {}", addr);

        // Wrap connections with TLS
        let incoming = async_stream::stream! {
            loop {
                let (socket, _) = listener.accept().await?;
                let acceptor = acceptor.clone();
                yield acceptor.accept(socket).await;
            }
        };

        Server::builder(hyper::server::accept::from_stream(incoming))
            .serve(make_svc)
    } else {
        println!("WAF proxy started successfully without TLS on {}", addr);
        Server::bind(&addr).serve(make_svc)
    };

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}