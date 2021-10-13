use crate::error::Error;
use reqwest::{StatusCode, Url};
use rocket::{
    response::Redirect,
    serde::{json::Json, Deserialize, DeserializeOwned, Serialize},
    State,
};
use serde_json::to_string;
use std::{borrow::Borrow, sync::Mutex};
use tokio::fs;

const AUTHORIZE_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
// const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";
const SEARCH_CATEGORIES_URL: &str = "https://api.twitch.tv/helix/search/categories";

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitchAuthInfo {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    scope: Vec<String>,
    token_type: String,
}

pub struct Twitch {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    pub auth_info: Mutex<Option<TwitchAuthInfo>>,
}

impl Twitch {
    pub async fn new(client_id: String, client_secret: String, redirect_uri: String) -> Twitch {
        let auth_info = match fs::read_to_string("twitch_auth.json").await {
            Ok(auth_info) => Some(
                serde_json::from_str::<TwitchAuthInfo>(&auth_info)
                    .expect("invalid twitch_auth.json"),
            ),
            Err(_) => None,
        };

        Twitch {
            client_id,
            client_secret,
            redirect_uri,
            auth_info: Mutex::new(auth_info),
        }
    }

    pub fn get_authorize_url(&self) -> String {
        format!("{}?client_id={}&redirect_uri={}&response_type=code&scope=user:read:email&force_verify=true", AUTHORIZE_URL, self.client_id, self.redirect_uri)
    }

    async fn twitch_request<I, B, R, K, V>(
        &self,
        url: &str,
        query: I,
        body: Option<B>,
    ) -> Result<R, Error>
    where
        I: IntoIterator,
        B: Serialize,
        R: DeserializeOwned,
        K: AsRef<str>,
        V: AsRef<str>,
        <I as IntoIterator>::Item: Borrow<(K, V)>,
    {
        if let Some(auth_info) = &*self.auth_info.lock()? {
            let url = Url::parse_with_params(url, query)?;

            let client = reqwest::Client::new();
            let res = client
                .post(url)
                .bearer_auth(&auth_info.access_token)
                .header("Client-Id", &self.client_id)
                .send()
                .await?
                .json::<R>().await?;

            return Ok(res);
        }
        Err(Error::new_bad_request("no twitch auth info available".to_string()))
    }

    /// calls the twitch search endpoint and takes the first result as the id
    pub async fn get_game_id_from_string(&self, game_name: &str) -> Result<String, Error> {
        
        #[derive(Debug, Deserialize)]
        struct InnerResponse {
            id: String,
        }
        
        #[derive(Debug, Deserialize)]
        struct Response {
            data: Vec<InnerResponse>
        }

        let mut res: Response = self.twitch_request(SEARCH_CATEGORIES_URL, [("query", game_name)], None::<()>).await?;

        if res.data.is_empty() {
            return Ok("".to_string());
        }

        Ok(res.data.remove(0).id)
    }

    pub fn game_channel_id_from_string(channel_name: &str) -> Result<u64, TwitchErrorResponse> {
        todo!()
    }

    pub fn update_channel(
        channel_id: u64,
        game_id: u64,
        title: &str,
    ) -> Result<(), TwitchErrorResponse> {
        todo!()
    }

    pub fn run_commercial(channel_id: u64) -> Result<(), TwitchErrorResponse> {
        todo!()
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
pub struct TwitchErrorResponse {
    inner: Json<TwitchError>,
}

impl From<reqwest::Error> for TwitchErrorResponse {
    fn from(err: reqwest::Error) -> Self {
        let twitch_error = TwitchError {
            status: err
                .status()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
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

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket.mount("/twitch", routes![authorize_callback])
    })
}
