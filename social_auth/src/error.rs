use rocket::serde::{Serialize, Deserialize, json::Json};

use url::ParseError;
use std::sync::PoisonError;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    status: u16,
    error: String,
    message: String
}

#[derive(Responder)]
pub enum Error {
    #[response(status=400)]
    BadRequest(Json<ErrorResponse>),
    #[response(status=500)]
    InternalServerError(Json<ErrorResponse>),
    #[response(status=403)]
    Unauthorized(Json<ErrorResponse>)
}

impl Error {
    pub fn new_bad_request(message: String) -> Self {
        Self::BadRequest(Json(Self::new_error_response(400, message)))
    }

    pub fn new_internal_server_error(message: String) -> Self {
        Self::InternalServerError(Json(Self::new_error_response(500, message)))
    }

    pub fn new_unauthorized(message: String) -> Self {
        Self::Unauthorized(Json(Self::new_error_response(403, message)))
    }

    fn new_error_response(status: u16, message: String) -> ErrorResponse {
        let error = match status {
            400 => "bad Request",
            500 => "internal server error",
            403 => "unauthorized",
            _ => "unknown"
        }.to_string();

        ErrorResponse {
            status,
            error,
            message
        }
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}