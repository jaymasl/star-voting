use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};
use rocket::http::Header;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let origin = req.headers().get_one("Origin").unwrap_or("http://localhost:8080");

        if origin.starts_with("http://localhost") {
            res.set_header(Header::new("Access-Control-Allow-Origin", origin));
            res.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS, DELETE"));
            res.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type, Authorization, X-CSRF-Token"));
            res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
            res.set_header(Header::new("Access-Control-Max-Age", "86400"));
        }
    }
}