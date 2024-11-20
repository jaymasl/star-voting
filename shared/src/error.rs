use serde::{Serialize, Deserialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    #[error("Invalid input provided")]
    InvalidInput,
    #[error("Resource not found")]
    NotFound,
    #[error("Operation not authorized")]
    Unauthorized,
    #[error("Resource conflict")]
    Conflict,
    #[error("Internal system error")]
    SystemError,
    #[error("Validation failed")]
    ValidationFailed,
    #[error("Rate limit exceeded")]
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(details) = &self.details {
            write!(f, "{}: {} ({})", self.code, self.message, details)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(code: ErrorCode, message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details.into()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;