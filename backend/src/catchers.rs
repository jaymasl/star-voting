use rocket::{Request, catch, serde::json::Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorMessage {
    error: String,
    status: u16,
}

#[catch(403)]
pub fn forbidden(req: &Request) -> Json<ErrorMessage> {
    let error_msg = match req.uri().path().segments().next() {
        Some("ballot") => "You have already cast a ballot for this vote.",
        _ => "Access forbidden. You may have already voted or the vote has ended."
    };
    
    Json(ErrorMessage {
        error: error_msg.into(),
        status: 403
    })
}

#[catch(429)]
pub fn too_many_requests(_req: &Request) -> Json<ErrorMessage> {
    Json(ErrorMessage {
        error: "Rate limit exceeded. Please wait before trying again.".into(),
        status: 429
    })
}

#[catch(400)]
pub fn bad_request(_req: &Request) -> Json<ErrorMessage> {
    Json(ErrorMessage {
        error: "Invalid request parameters.".into(),
        status: 400
    })
}

#[catch(500)]
pub fn internal_error(_req: &Request) -> Json<ErrorMessage> {
    Json(ErrorMessage {
        error: "An internal server error occurred.".into(),
        status: 500
    })
}

#[catch(404)]
pub fn not_found(_req: &Request) -> Json<ErrorMessage> {
    Json(ErrorMessage {
        error: "The requested resource was not found.".into(),
        status: 404
    })
}