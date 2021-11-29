use crate::error::Error;
use crate::twitch_config::TwitchAdJson;
use crate::{templates, twitch_config::Twitch};

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::{content, status};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
use tokio::fs;

use crate::twitter_config::Twitter;

#[get("/auth?<service>")]
async fn get_twitch_info(
    _api_key: ApiKey<'_>,
    service: &str,
) -> Result<content::Json<Vec<u8>>, Error> {
    let bytes = fs::read(format!("{}_auth.json", service)).await;
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::new_bad_request(
                format!("no {} auth info available", service),
            ));
        }
        Err(e) => return Err(e.into()),
    };

    Ok(content::Json(bytes))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericApiResponse<T: Serialize> {
    data: T,
}

#[get("/twitch/login_to_id?<login>")]
async fn twitch_game_to_id(
    _api_key: ApiKey<'_>,
    twitch: &State<Twitch>,
    login: &str,
) -> Result<Json<GenericApiResponse<String>>, Error> {
    let res = twitch.get_channel_id_from_string(login).await?;

    Ok(Json(GenericApiResponse { data: res }))
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckAvailResponse {
    pub twitter: bool,
    pub twitch: bool,
}

#[get("/avail")]
async fn check_avail(_api_key: ApiKey<'_>) -> Json<CheckAvailResponse> {
    Json(CheckAvailResponse {
        twitter: templates::is_twitter_avail(),
        twitch: templates::is_twitch_avail(),
    })
}

#[catch(400)]
fn bad_request(_req: &Request) -> Error {
    Error::new_bad_request("malformed or unauthorized request".to_string())
}

#[catch(404)]
fn not_found(_req: &Request) -> Error {
    Error::new_not_found("the requested resource does not exist".to_string())
}

#[derive(Deserialize)]
struct PostTweetRequest<'r> {
    body: &'r str,
}

#[post("/tweet", data = "<tweet_body>")]
async fn post_tweet(
    _api_key: ApiKey<'_>,
    tweet_body: Json<PostTweetRequest<'_>>,
    twitter: &State<Twitter>,
) -> Result<status::Custom<()>, Error> {
    if let Some(ref token) = *twitter.auth_token.lock().await {
        egg_mode::tweet::DraftTweet::new(tweet_body.body.to_string())
            .send(token)
            .await?;
        return Ok(status::Custom(Status::NoContent, ()));
    }

    Err(Error::new_auth_not_avail("twitter"))
}

#[derive(Deserialize)]
struct TwitchUpdateRequest<'r> {
    game: &'r str,
    title: &'r str,
    login: &'r str,
}

#[post("/twitch/update", data = "<twitch_data>")]
async fn twitch_update(_api_key: ApiKey<'_>, twitch_data: Json<TwitchUpdateRequest<'_>>, twitch: &State<Twitch>) -> Result<status::Custom<()>, Error> {
    let channel_id = twitch.get_channel_id_from_string(twitch_data.login).await?;
    let game_id = twitch.get_game_id_from_string(twitch_data.game).await?;
    twitch.update_channel(&channel_id, &game_id, twitch_data.title).await?;

    Ok(status::Custom(Status::NoContent, ()))
}

#[post("/twitch/commercial?<login>&<length>")]
async fn twitch_commercial(_api_key: ApiKey<'_>, twitch: &State<Twitch>, login: &str, length: u16) -> Result<Json<TwitchAdJson>, Error> {
    let channel_id = twitch.get_channel_id_from_string(login).await?;
    let res = twitch.run_commercial(channel_id, length).await?;

    Ok(Json(res))
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
        let api_key = req
            .rocket()
            .state::<crate::templates::Sessions>()
            .expect("Sessions not managed")
            .api_key();
        match req.headers().get_one("Authorization") {
            None => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            Some(key) if key == api_key => Outcome::Success(ApiKey(key)),
            Some(_) => Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid)),
        }
    }
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket
            .mount(
                "/api/v1",
                routes![get_twitch_info, check_avail, post_tweet, twitch_game_to_id, twitch_update, twitch_commercial],
            )
            .register("/api/v1", catchers![bad_request, not_found])
    })
}
