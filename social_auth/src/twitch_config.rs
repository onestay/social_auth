use rocket::{
    response::Redirect,
    serde::{json::Json, Deserialize, Serialize},
    State,
};

use reqwest::StatusCode;
use tokio::fs;

const AUTHORIZE_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";

struct Twitch {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl Twitch {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Twitch {
        Twitch {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TwitchResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    scope: Vec<String>,
    token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TwitchError {
    status: u16,
    message: String,
    error: Option<String>,
}

#[get("/authorize")]
fn authorize(twitch: &State<Twitch>) -> Redirect {
    Redirect::to(format!("{}?client_id={}&redirect_uri={}&response_type=code&scope=user:read:email&force_verify=true", AUTHORIZE_URL, twitch.client_id, twitch.redirect_uri))
}

#[get("/authorize/callback?<code>")]
async fn authorize_callback(
    twitch: &State<Twitch>,
    code: &str,
) -> Result<Redirect, TwitchErrorResponse> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!(
        "{}?client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        TOKEN_URL, twitch.client_id, twitch.client_secret, code, twitch.redirect_uri
    ))
        .send()
        .await?;
    if !StatusCode::is_success(&res.status()) {
        let twitch_err: TwitchError = res.json().await?;
        return Err(twitch_err.into());
    }
    fs::write("twitch_auth.json", res.bytes().await?).await?;
    Ok(Redirect::to("/"))
}

#[derive(Debug, Responder)]
struct TwitchErrorResponse {
    inner: Json<TwitchError>,
}

impl From<reqwest::Error> for TwitchErrorResponse {
    fn from(err: reqwest::Error) -> Self {
        let twitch_error = TwitchError {
            status: err
                .status()
                .unwrap_or_else(|| StatusCode::INTERNAL_SERVER_ERROR)
                .as_u16(),
            message: err.to_string(),
            error: Some(String::from("internal server error")),
        };
        TwitchErrorResponse {
            inner: Json(twitch_error),
        }
    }
}

impl From<TwitchError> for TwitchErrorResponse {
    fn from(twitch_error: TwitchError) -> Self {
        TwitchErrorResponse {
            inner: Json(twitch_error),
        }
    }
}

impl From<std::io::Error> for TwitchErrorResponse {
    fn from(err: std::io::Error) -> Self {
        let twitch_error = TwitchError {
            status: 500,
            error: Some(String::from("internal server error")),
            message: err.to_string(),
        };

        TwitchErrorResponse {
            inner: Json(twitch_error),
        }
    }
}

pub fn stage<'a>(client_id: String, client_secret: String, redirect_uri: String) -> rocket::fairing::AdHoc {
    let twitch = Twitch::new(
        client_id,
        client_secret,
        redirect_uri,
    );

    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket
            .mount("/twitch", routes![authorize, authorize_callback])
            .manage(twitch)
    })
}
