use crate::config;
use crate::firewall;

use futures_util::TryStreamExt;
use ntex::http;
use ntex::web;

const EXCLUDED_HEADERS: &[&str] = &[
    "host",
    "content-length", // Let ntex handle this automatically
    "connection",
];

async fn forward(
    req: web::HttpRequest,
    body: ntex::util::Bytes,
    client: web::types::State<http::Client>,
    forward_url: web::types::State<url::Url>,
) -> Result<web::HttpResponse, web::Error> {

    // Build the target URL
    let mut new_url = forward_url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    firewall::inspect(&req, &new_url).await;

    // Create forwarded request
    let mut forwarded_req = client.request_from(new_url.as_str(), req.head());

    // Copy important headers (optional)
    for (key, value) in req.headers().iter() {
        if !EXCLUDED_HEADERS.contains(&key.as_str()) {
            forwarded_req = forwarded_req.set_header(key.clone(), value.clone());
        }
    }

    // Send request and get response
    let res = forwarded_req
        .send_body(body)
        .await
        .map_err(web::Error::from)?;

    // Build response with all headers and streaming body
    let mut client_resp = web::HttpResponse::build(res.status());

    // Copy response headers
    for (key, value) in res.headers().iter() {
        client_resp.header(key.clone(), value.clone());
    }

    Ok(client_resp.streaming(res.into_stream()))
}

pub async fn run(settings: &config::Settings) -> std::io::Result<()> {
    println!(
        "Proxy server running at http://localhost:{} fowarding to {}",
        settings.listen_port, settings.backend_url
    );

    let forward_url = settings.backend_url.to_owned();
    let forward_url = url::Url::parse(&forward_url)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;

    let server = web::server(move || {
        web::App::new()
            .state(http::Client::new())
            .state(forward_url.clone())
            .wrap(web::middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .bind(("0.0.0.0", settings.listen_port))?;

    let server = if settings.threading.workers > 0 {
        server.workers(settings.threading.workers.into())
    } else {
        server
    };

    server
        .run()
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
}
