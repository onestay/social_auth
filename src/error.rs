use rocket::serde::{Serialize, Deserialize, json::Json};

use url::ParseError;
use std::sync::PoisonError;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    status: u16,
    error: String,
    message: String
}

#[derive(Debug, Responder)]
#[allow(clippy::enum_variant_names)] 
pub enum Error {
    #[response(status=400)]
    BadRequest(Json<ErrorResponse>),
    #[response(status=500)]
    InternalServerError(Json<ErrorResponse>),
    #[response(status=404)]
    NotFound(Json<ErrorResponse>)
}

impl Error {
    pub fn new_bad_request(message: String) -> Self {
        Self::BadRequest(Json(Self::new_error_response(400, message)))
    }

    pub fn new_internal_server_error(message: String) -> Self {
        Self::InternalServerError(Json(Self::new_error_response(500, message)))
    }

    pub fn new_not_found(message: String) -> Self {
        Self::NotFound(Json(Self::new_error_response(404, message)))
    }

    pub fn new_auth_not_avail(service: &str) -> Self {
        Self::BadRequest(Json(Self::new_error_response(403, format!("no {} auth info available", service))))
    }

    fn new_error_response(status: u16, message: String) -> ErrorResponse {
        let error = match status {
            400 => "bad Request",
            500 => "internal server error",
            403 => "unauthorized",
            404 => "not found",
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

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}

impl From<crate::twitch_config::TwitchErrorJson> for Error {
    fn from(err: crate::twitch_config::TwitchErrorJson) -> Self {
        Self::new_internal_server_error(format!(
            "twitch responded with {} ({}) message: {}",
            err.status, err.error, err.message
        ))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}

impl From<egg_mode::error::Error> for Error {
    fn from(err: egg_mode::error::Error) -> Self {
        Self::new_internal_server_error(err.to_string())
    }
}