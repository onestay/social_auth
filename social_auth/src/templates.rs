use crate::twitch_config::Twitch;
use crate::twitter_config::Twitter;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rocket::http::Cookie;
use rocket::{
    form::Form,
    http::CookieJar,
    request::{self, FromRequest, Request},
    response::Redirect,
    serde::Serialize,
    State,
};
use rocket_dyn_templates::Template;
use std::path::Path;
use tokio::sync::Mutex;

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("templates", |rocket| async {
        rocket
            .mount("/", routes![index, index_no_login, login, login_post])
            .attach(Template::fairing())
    })
}

fn is_twitch_avail() -> bool {
    Path::new("twitch_auth.json").exists()
}

fn is_twitter_avail() -> bool {
    Path::new("twitter_auth.json").exists()
}

#[derive(Debug, Serialize)]
pub struct IndexContext<'a> {
    creator: String,
    twitch_url: String,
    twitter_url: String,
    twitter_avail: bool,
    twitch_avail: bool,
    api_key: &'a str
}

pub struct Sessions {
    sessions: Mutex<Vec<String>>,
    password: String,
    api_key: String,
}

impl Sessions {
    pub fn new(password: String) -> Self {

        Sessions {
            sessions: Mutex::new(vec![]),
            password,
            api_key: gen_random_string(30)
        }
    }
}

pub struct Authenticated;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> request::Outcome<Authenticated, Self::Error> {
        if let Some(session_token) = request.cookies().get("session") {
            let session_token = session_token.value();
            let sessions = &request
                .rocket()
                .state::<Sessions>()
                .expect("sessions state is not being managed")
                .sessions
                .lock()
                .await;
            if sessions.iter().any(|e| e == session_token) {
                return request::Outcome::Success(Authenticated {});
            }
        };
        request::Outcome::Forward(())
    }
}

#[get("/")]
async fn index(
    twitch: &State<Twitch>,
    twitter: &State<Twitter>,
    sessions: &State<Sessions>,
    _authenticated: Authenticated,
) -> Template {
    let context = IndexContext {
        creator: "onestay".to_string(),
        twitch_url: twitch.get_authorize_url(),
        twitter_url: twitter
            .get_authorize_url()
            .await
            .expect("can't get twitter url"),
        twitch_avail: is_twitch_avail(),
        twitter_avail: is_twitter_avail(),
        api_key: &sessions.api_key
    };
    Template::render("index", context)
}

#[get("/", rank = 2)]
fn index_no_login() -> Redirect {
    Redirect::to("/login")
}

#[get("/login")]
async fn login() -> Template {
    Template::render("login", ())
}

#[derive(FromForm)]
struct LoginForm<'r> {
    password: &'r str,
}

#[post("/login", data = "<login_form>")]
async fn login_post(
    login_form: Form<LoginForm<'_>>,
    sessions: &State<Sessions>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    if login_form.password == sessions.password {
        let session_token: String = gen_random_string(30);

        cookies.add(Cookie::new("session", session_token.clone()));
        sessions.sessions.lock().await.push(session_token);
        return Redirect::to("/");
    }

    Redirect::to("/login")
}

fn gen_random_string(n: usize) -> String {
    thread_rng()
    .sample_iter(&Alphanumeric)
    .take(n)
    .map(char::from)
    .collect()
}