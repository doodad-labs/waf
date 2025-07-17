use ntex::web;

pub async fn inspect(req: &web::HttpRequest, new_url: &url::Url) {
    println!("{}: {}", req.method(), new_url);
}