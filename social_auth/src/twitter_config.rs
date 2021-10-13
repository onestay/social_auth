use rocket::{
    response::Redirect,
    serde::{json::Json, Deserialize, Serialize},
    State,
};

use tokio::sync::Mutex;
use egg_mode::{KeyPair, Token, auth};

use tokio::fs;

pub struct Twitter {
    callback_url: String,
    request_token: Mutex<Option<KeyPair>>,
    con_token: KeyPair,
    pub auth_token: Mutex<Option<Token>>
}

impl Twitter {
    pub async fn new(
        api_key: String,
        api_secret: String,
        callback_url: String,
    ) -> Twitter {
        let token = match fs::read_to_string("twitter_auth.json").await {
            Ok(token) => {
                Some(serde_json::from_str::<Token>(&token).expect("invalid twitter_auth.json"))
            }
            Err(_) => None
        };

        Twitter {
            callback_url,
            request_token: Mutex::new(None),
            con_token: egg_mode::KeyPair::new(api_key, api_secret),
            auth_token: Mutex::new(token),
        }
    }

    pub async fn get_authorize_url(&self) -> Result<String, TwitterErrorResponse> {
        let req_token = egg_mode::auth::request_token(&self.con_token, &self.callback_url).await?;
        let redirect_url = auth::authorize_url(&req_token);
        let mut token = self.request_token.lock().await;
        *token = Some(req_token);

        Ok(redirect_url)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterError {
    status: u16,
    message: String,
    error: Option<String>,
}

#[derive(Debug, Responder)]
pub struct TwitterErrorResponse {
    inner: Json<TwitterError>,
}

impl From<egg_mode::error::Error> for TwitterErrorResponse {
    fn from(err: egg_mode::error::Error) -> Self {
        TwitterErrorResponse {
            inner: Json(TwitterError {
                status: 500,
                message: String::from("internal server error"),
                error: Some(err.to_string()),
            }),
        }
    }
}

impl From<std::io::Error> for TwitterErrorResponse {
    fn from(err: std::io::Error) -> Self {
        TwitterErrorResponse {
            inner: Json(TwitterError {
                status: 500,
                message: String::from("internal server error"),
                error: Some(err.to_string()),
            }),
        }
    }
}

impl From<serde_json::Error> for TwitterErrorResponse {
    fn from(err: serde_json::Error) -> Self {
        TwitterErrorResponse {
            inner: Json(TwitterError {
                status: 500,
                message: String::from("internal server error"),
                error: Some(err.to_string()),
            }),
        }
    }
}

#[get("/authorize")]
async fn authorize(twitter: &State<Twitter>) -> Result<Redirect, TwitterErrorResponse> {
    let redirect_url = twitter.get_authorize_url().await?;
    Ok(Redirect::to(redirect_url))
}

#[get("/authorize/callback?<oauth_token>&<oauth_verifier>")]
// we need to allow unused variables here since oauth_token is not needed by us but provided by the twitter api callback, sadly rocket doesn't let us prefix it with _ either
#[allow(unused_variables)] async fn authorize_callback(
    oauth_token: &str,
    twitter: &State<Twitter>,
    oauth_verifier: &str,
) -> Result<Redirect, TwitterErrorResponse> {
    let request_token = twitter.request_token.lock().await;
    if let Some(ref request_token) = *request_token {
        let (token, _, _) =
            egg_mode::auth::access_token(twitter.con_token.clone(), request_token, oauth_verifier)
                .await?;
        fs::write("twitter_auth.json", serde_json::to_vec(&token)?).await?;
        let mut saved_auth_token = twitter.auth_token.lock().await;
        *saved_auth_token = Some(token);
    }
    Ok(Redirect::to("/"))
}



pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("twitter", |rocket| async {
        rocket
            .mount("/twitter", routes![authorize, authorize_callback])
    })
}