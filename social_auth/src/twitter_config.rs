use rocket::{
    response::Redirect,
    serde::{json::Json, Deserialize, Serialize},
    State,
};

use tokio::sync::Mutex;

use egg_mode::{auth, KeyPair};

use tokio::fs;

struct Twitter {
    callback_url: String,
    request_token: Mutex<Option<KeyPair>>,
    con_token: KeyPair,
}

impl Twitter {
    pub fn new(
        api_key: String,
        api_secret: String,
        callback_url: String,
    ) -> Twitter {
        Twitter {
            callback_url,
            request_token: Mutex::new(None),
            con_token: egg_mode::KeyPair::new(api_key, api_secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TwitterError {
    status: u16,
    message: String,
    error: Option<String>,
}

#[derive(Debug, Responder)]
struct TwitterErrorResponse {
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
    let req_token = egg_mode::auth::request_token(&twitter.con_token, &twitter.callback_url).await?;
    let redirect_url = auth::authorize_url(&req_token);
    let mut token = twitter.request_token.lock().await;
    *token = Some(req_token);
    Ok(Redirect::to(redirect_url))
}

#[get("/authorize/callback?<oauth_token>&<oauth_verifier>")]
async fn authorize_callback(
    twitter: &State<Twitter>,
    oauth_token: &str,
    oauth_verifier: &str,
) -> Result<Redirect, TwitterErrorResponse> {
    let request_token = twitter.request_token.lock().await;
    if let Some(ref request_token) = *request_token {
        let (token, _, _) =
            egg_mode::auth::access_token(twitter.con_token.clone(), &request_token, oauth_verifier)
                .await?;
        fs::write("twitter_auth.json", serde_json::to_vec(&token)?).await?;
    }
    Ok(Redirect::to("/"))
}

pub fn stage(api_key: String, api_secret: String, callback_url: String) -> rocket::fairing::AdHoc {
    let twitter = Twitter::new(api_key, api_secret, callback_url);

    rocket::fairing::AdHoc::on_ignite("twitter", |rocket| async {
        rocket
            .mount("/twitter", routes![authorize, authorize_callback])
            .manage(twitter)
    })
}
