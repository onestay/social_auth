use crate::templates;

use rocket::State;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::response::{content, status};
use tokio::fs;

use crate::twitter_config::Twitter;

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
    pub twitter: bool,
    pub twitch: bool
}

#[get("/avail")]
async fn check_avail(_api_key: ApiKey<'_>) -> Json<CheckAvailResponse> {
    Json(CheckAvailResponse{
        twitter: templates::is_twitter_avail(),
        twitch: templates::is_twitch_avail()
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

impl From<egg_mode::error::Error> for ApiErrorResponse {
    fn from(err: egg_mode::error::Error) -> Self {
        ApiErrorResponse {
            inner: Json(ApiError {
                status: 500,
                message: String::from("internal server error"),
                error: Some(err.to_string()),
            }),
        }
    }
}


#[derive(Deserialize)]
struct PostTweetRequest<'r> {
    body: &'r str
}


#[post("/tweet", data = "<tweet_body>")]
async fn post_tweet(_api_key: ApiKey<'_>, tweet_body: Json<PostTweetRequest<'_>>, twitter: &State<Twitter>) -> Result<status::Custom<()>, ApiErrorResponse> {
    if let Some(token) = &*twitter.auth_token.lock().await {
        egg_mode::tweet::DraftTweet::new(tweet_body.body.to_string()).send(token).await?;
        return Ok(status::Custom(Status::NoContent, ()));
    }

    Err(ApiErrorResponse::file_not_found("twitter"))
}

#[derive(Deserialize)]
struct TwitchUpdateRequest<'r> {
    game: &'r str,
    title: &'r str,
    channel_id: u32
}

#[post("/twitch_update", data = "<twitch_data>")]
async fn twitch_update(_api_key: ApiKey<'_>, twitch_data: Json<TwitchUpdateRequest<'_>>) {
    
}

struct ApiKey<'r>(&'r str);

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey<'r> {
    type Error = ApiKeyError;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let api_key = req.rocket().state::<crate::templates::Sessions>().expect("Sessions not managed").api_key();
        match req.headers().get_one("Authorization") {
            None => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            Some(key) if key == api_key => Outcome::Success(ApiKey(key)),
            Some(_) => Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid)),
        }
    }
}


pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket.mount("/api/v1", routes![get_twitch_info, check_avail, post_tweet])
    })
}
