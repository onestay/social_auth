use std::collections::HashMap;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::response::content;
use tokio::fs;

#[get("/auth?<service>")]
async fn get_twitch_info(_api_key: ApiKey<'_>, service: &str) -> Result<content::Json<Vec<u8>>, ApiErrorResponse> {
    let bytes = fs::read(format!("{}_auth.json", service)).await;
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(e)  if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ApiErrorResponse::file_not_found(service));
        }
        Err(e) => {
            return Err(e.into())
        }
    };

    Ok(content::Json(bytes))
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckAvailResponse {
    services: HashMap<String, bool>
}

#[get("/avail")]
async fn check_avail(_api_key: ApiKey<'_>) -> Json<CheckAvailResponse> {
    let mut response = CheckAvailResponse {
        services: HashMap::new()
    };
    let services = vec!["twitch".to_string(), "twitter".to_string()];

    for service in services.into_iter() {
        let file = fs::File::open(format!("{}_auth.json", service)).await;
        if file.is_ok() {
            response.services.insert(service, true);
        } else {
            response.services.insert(service, false);
        }
    }
    Json(response)
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket.mount("/api/v1", routes![get_twitch_info, check_avail])
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiError {
    status: u16,
    message: String,
    error: Option<String>,
}

#[derive(Debug, Responder)]
struct ApiErrorResponse {
    inner: Json<ApiError>,
}

impl ApiErrorResponse {
    fn file_not_found(service: &str) -> Self {
        ApiErrorResponse { inner: Json(ApiError {
            status: 401,
            message: String::from("bad request"),
            error: Some(format!("no {} info available", service)),
        }) }
    }
}

impl From<std::io::Error> for ApiErrorResponse {
    fn from(err: std::io::Error) -> Self {
        ApiErrorResponse {
            inner: Json(ApiError {
                status: 500,
                message: String::from("internal server error"),
                error: Some(err.to_string()),
            }),
        }
    }
}
struct ApiKey<'r>(&'r str);

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

impl<'r> ApiKey<'r> {
    fn is_valid(key: &str) -> bool {
        key == "abc"
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey<'r> {
    type Error = ApiKeyError;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("Authorization") {
            None => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            Some(key) if ApiKey::is_valid(key) => Outcome::Success(ApiKey(key)),
            Some(_) => Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid)),
        }
    }
}
