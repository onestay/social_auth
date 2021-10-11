use crate::twitch_config::TwitchResponse;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::response::content;
use tokio::fs;

#[get("/twitch")]
async fn get_twitch_info(key: ApiKey<'_>) -> Result<content::Json<Vec<u8>>, ApiErrorResponse> {
    let bytes = fs::read("twitch_auth.json").await?;

    Ok(content::Json(bytes))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket.mount("/api/v1", routes![get_twitch_info])
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
