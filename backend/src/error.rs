use rocket::http::Status;
use rocket::response::Responder;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum ApiError {
    #[error("Vote not found")]
    NotFound,
    #[error("Invalid vote ID")]
    InvalidId,
    #[error("Invalid ballot")]
    InvalidBallot,
    #[error("Vote has ended")]
    VoteEnded,
    #[error("Vote is ongoing")]
    VoteOngoing,
    #[error("Internal error: {0}")]
    Internal(String),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ApiError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let status = match self {
            ApiError::NotFound => Status::NotFound,
            ApiError::InvalidId => Status::BadRequest,
            ApiError::InvalidBallot => Status::BadRequest,
            ApiError::VoteEnded => Status::Forbidden,
            ApiError::VoteOngoing => Status::Forbidden,
            ApiError::Internal(_) => Status::InternalServerError,
        };

        rocket::Response::build_from(self.to_string().respond_to(req)?)
            .status(status)
            .ok()
    }
}