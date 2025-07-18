use crate::config;

use hyper::{
    body::to_bytes,
    client::HttpConnector,
    header::UPGRADE,
    Body, Client, Request, Response, Server, Uri,
    service::{make_service_fn, service_fn},
};

use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use super::websocket;

use tokio_rustls::server::TlsStream;
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
    tls_version: Option<String>,
    tls_cipher: Option<String>,
) -> Result<Response<Body>, hyper::Error> {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body).await?;

    println!("TLS Version: {:?}", tls_version);
    println!("TLS Cipher: {:?}", tls_cipher);

    let uri = format!(
        "{}{}",
        target,
        parts.uri.path_and_query().map(|x| x.as_str()).unwrap_or("")
    )
    .parse::<Uri>()
    .unwrap();

    let mut proxied_request = Request::from_parts(parts, Body::from(body_bytes.clone()));
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

        // Send the request and get the response
    let mut response = client.request(proxied_request).await?;

    // Inject JavaScript into the response body if it's HTML
    if let Some(content_type) = response.headers().get("content-type") {
        if content_type.to_str().unwrap_or("").contains("text/html") {
            let response_body = to_bytes(response.body_mut()).await?;
            let js_injection = format!(
                "<script>console.log('Request ID: {}');</script>",
                rid
            );
            let body_string = String::from_utf8_lossy(&response_body);
            let modified_body = body_string.replace("</body>", &(js_injection + "</body>"));
            
            // Update the response body
            *response.body_mut() = Body::from(modified_body.clone());
            
            // Update the content-length header to match the new body size
            response.headers_mut().insert(
                "content-length",
                modified_body.len().to_string().parse().unwrap(),
            );
        }
    }

    Ok(response)
}

pub async fn run(settings: &config::Settings) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], settings.listen_port));
    let client = Client::new();
    
    // Uncomment this when you add TLS fields to Settings
    let tls_acceptor = if settings.tls.tls_enabled {
        // Load certificates
        let cert_path = settings.tls.tls_cert_path.as_ref().ok_or("TLS cert path is not set")?;
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_path = settings.tls.tls_key_path.as_ref().ok_or("TLS key path is not set")?;
        let key_file = File::open(key_path)?;
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
    let make_svc = make_service_fn(move |conn: &hyper::server::conn::AddrStream| {
        let client = client.clone();
        let settings = settings_clone.clone();
        
        // Get client IP from connection
        let client_addr = conn.remote_addr();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let client = client.clone();
                let settings = settings.clone();
                async move {
                    // Generate a unique request ID
                    let rid = uuid::Uuid::new_v4().to_string();

                    // Extract TLS fingerprint if available
                    let (tls_version_raw, tls_cipher_raw) = if settings.tls.tls_enabled {
                        if let Some(tls_info) = req.extensions().get::<TlsStream<tokio::net::TcpStream>>() {
                            let (_, session) = tls_info.get_ref();
                            let version = session.protocol_version().map(|v| format!("{:?}", v));
                            let cipher = session.negotiated_cipher_suite().map(|c| format!("{:?}", c.suite()));
                            (version, cipher)
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    let tls_version = tls_version_raw.map(|v| v.to_string());
                    let tls_cipher = tls_cipher_raw.map(|c| c.to_string());

                    if req.headers().get(UPGRADE).map(|h| h.to_str().unwrap_or("")).unwrap_or("").to_lowercase() == "websocket" {
                        return websocket::handle_websocket(req, &settings.webapp_url).await;
                    }

                    let client_ip = req
                        .extensions()
                        .get::<SocketAddr>()
                        .cloned()
                        .unwrap_or(client_addr);

                    proxy(&rid, req, client, &settings.webapp_url, client_ip, tls_version, tls_cipher).await
                }
            }))
        }
    });

    let server = if let Some(_acceptor) = tls_acceptor {
        // TLS implementation would go here when Settings has TLS fields
        println!("WAF proxy started successfully with TLS on {}", addr);
        Server::bind(&addr).serve(make_svc)
    } else {
        println!("WAF proxy started successfully without TLS on {}", addr);
        Server::bind(&addr).serve(make_svc)
    };

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}