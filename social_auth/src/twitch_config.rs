use crate::error::Error;
use reqwest::{header, ClientBuilder, StatusCode, Url};
use rocket::{
    response::Redirect,
    serde::{Deserialize, DeserializeOwned, Serialize},
    State,
};
use std::borrow::Borrow;
use tokio::{fs, sync::Mutex};

const AUTHORIZE_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
// const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";
const SEARCH_CATEGORIES_URL: &str = "https://api.twitch.tv/helix/search/categories";
const GET_USER_URL: &str = "https://api.twitch.tv/helix/users";
const CHANNEL_URL: &str = "https://api.twitch.tv/helix/channels";
const COMMERICAL_URL: &str = "https://api.twitch.tv/helix/channels/commercial";

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

enum TwitchRequestMethod {
    Get,
    Patch,
    Post,
}

// TODO: rename this after fully moving to new error
#[derive(Debug, Serialize, Deserialize)]
pub struct TwitchErrorJson {
    pub message: String,
    pub status: u16,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitchAdJson {
    length: u64,
    message: String,
    retry_after: u64,
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
        Url::parse_with_params(
            AUTHORIZE_URL,
            [
                ("client_id", self.client_id.as_str()),
                ("redirect_uri", self.redirect_uri.as_str()),
                (
                    "scope",
                    "channel:manage:broadcast user:read:email channel:edit:commercial",
                ),
                ("response_type", "code"),
                ("force_verify", "true"),
            ],
        )
        .expect("Unable to parse twitch authorize url")
        .to_string()
    }

    // the return type of this function is kinda ugly but there's no real alternative for if theres no body in the http response
    async fn twitch_request<I, B, R, K, V>(
        &self,
        url: &str,
        method: TwitchRequestMethod,
        query: I,
        body: Option<B>,
    ) -> Result<Option<R>, Error>
    where
        I: IntoIterator,
        B: Serialize,
        R: DeserializeOwned,
        K: AsRef<str>,
        V: AsRef<str>,
        <I as IntoIterator>::Item: Borrow<(K, V)>,
    {
        if let Some(auth_info) = &*self.auth_info.lock().await {
            let url = Url::parse_with_params(url, query)?;

            let mut headers = header::HeaderMap::new();
            let auth_header_value = format!("Bearer {}", auth_info.access_token);
            headers.insert(
                "Authorization",
                header::HeaderValue::from_str(&auth_header_value)?,
            );
            headers.insert("Client-Id", header::HeaderValue::from_str(&self.client_id)?);

            let client = ClientBuilder::new().default_headers(headers).build()?;
            let response = match method {
                TwitchRequestMethod::Get => client.get(url).send().await?,
                TwitchRequestMethod::Patch if body.is_some() => {
                    client.patch(url).json(&body).send().await?
                }
                TwitchRequestMethod::Patch => client.patch(url).send().await?,
                TwitchRequestMethod::Post if body.is_some() => {
                    client.post(url).json(&body).send().await?
                }
                TwitchRequestMethod::Post => client.post(url).send().await?,
            };

            if !response.status().is_success() {
                let twitch_error = response.json::<TwitchErrorJson>().await?;
                return Err(twitch_error.into());
            }
            // TODO: what to do when not body?
            if let Some(content_length) = response.content_length() {
                if content_length > 0 {
                    return Ok(Some(response.json::<R>().await?));
                }
            }

            return Ok(None);
        }
        Err(Error::new_bad_request(
            "no twitch auth info available".to_string(),
        ))
    }

    /// calls the twitch search endpoint and takes the first result as the id
    pub async fn get_game_id_from_string(&self, game_name: &str) -> Result<String, Error> {
        #[derive(Debug, Deserialize)]
        struct InnerResponse {
            id: String,
        }

        #[derive(Debug, Deserialize)]
        struct Response {
            data: Vec<InnerResponse>,
        }

        let res: Option<Response> = self
            .twitch_request(
                SEARCH_CATEGORIES_URL,
                TwitchRequestMethod::Get,
                [("query", game_name)],
                None::<()>,
            )
            .await?;

        if let Some(mut res) = res {
            if res.data.is_empty() {
                return Ok("".to_string());
            }

            return Ok(res.data.remove(0).id);
        }

        Err(Error::new_internal_server_error(
            "body was none".to_string(),
        ))
    }

    pub async fn get_channel_id_from_string(&self, channel_name: &str) -> Result<String, Error> {
        #[derive(Debug, Deserialize)]
        struct InnerResponse {
            id: String,
        }

        #[derive(Debug, Deserialize)]
        struct Response {
            data: Vec<InnerResponse>,
        }

        let res: Option<Response> = self
            .twitch_request(
                GET_USER_URL,
                TwitchRequestMethod::Get,
                [("login", channel_name)],
                None::<()>,
            )
            .await?;

        if let Some(mut res) = res {
            if res.data.is_empty() {
                return Err(Error::new_bad_request(
                    format!("channel with name {} doesn't exist", channel_name),
                ));
            }

            return Ok(res.data.remove(0).id);
        }

        Err(Error::new_internal_server_error(
            "body was none".to_string(),
        ))
    }

    pub async fn update_channel(
        &self,
        channel_id: &str,
        game_id: &str,
        title: &str,
    ) -> Result<(), Error> {
        #[derive(Debug, Serialize)]
        struct UpdateChannelBody<'a> {
            game_id: &'a str,
            title: &'a str,
        }

        let update_channel_body = UpdateChannelBody { game_id, title };

        let _res: Option<()> = self
            .twitch_request(
                CHANNEL_URL,
                TwitchRequestMethod::Patch,
                [("broadcaster_id", channel_id)],
                Some(update_channel_body),
            )
            .await?;

        Ok(())
    }

    pub async fn run_commercial(
        &self,
        channel_id: String,
        length: u16,
    ) -> Result<TwitchAdJson, Error> {

        #[derive(Serialize)]
        struct StartCommericalBody {
            broadcaster_id: String,
            length: u16,
        }

        #[derive(Debug, Deserialize)]
        struct Response {
            data: Vec<TwitchAdJson>,
        }

        let start_commerical_body = StartCommericalBody {
            broadcaster_id: channel_id,
            length,
        };

        let res: Option<Response> = self
            .twitch_request(
                COMMERICAL_URL,
                TwitchRequestMethod::Post,
                Vec::new() as Vec<(&str, &str)>,
                Some(start_commerical_body),
            )
            .await?;

        if let Some(mut res) = res {
            return Ok(res.data.remove(1));
        }

        Err(Error::new_internal_server_error(
            "body was none".to_string(),
        ))
    }
}

#[get("/authorize/callback?<code>")]
async fn authorize_callback(twitch: &State<Twitch>, code: &str) -> Result<Redirect, Error> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!(
        "{}?client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        TOKEN_URL, twitch.client_id, twitch.client_secret, code, twitch.redirect_uri
    ))
        .send()
        .await?;
    if !StatusCode::is_success(&res.status()) {
        let twitch_err: TwitchErrorJson = res.json().await?;
        return Err(twitch_err.into());
    }

    let auth_info: TwitchAuthInfo = res.json().await?;

    fs::write("twitch_auth.json", serde_json::to_vec(&auth_info)?).await?;
    let mut auth_info_mutex = twitch.auth_info.lock().await;
    *auth_info_mutex = Some(auth_info);
    Ok(Redirect::to("/"))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitch", |rocket| async {
        rocket.mount("/twitch", routes![authorize_callback])
    })
}
