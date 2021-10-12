mod twitch_config;
mod api;
mod twitter_config;
mod templates;
#[macro_use]
extern crate rocket;
use rocket::fs::FileServer;
use std::env;


#[launch]
fn rocket() -> _ {
    let config = Config::from_env();
    let twitch = twitch_config::Twitch::new(config.twitch_client_id, config.twitch_client_secret, config.twitch_redirect_uri);
    let twitter = twitter_config::Twitter::new(config.twitter_api_key, config.twitter_api_secret, config.twitter_callback_url);

    rocket::build()
        .manage(twitch)
        .manage(twitter)
        .mount("/", FileServer::from("public/"))
        .attach(templates::stage())
        .attach(twitch_config::stage())
        .attach(api::stage())
        .attach(twitter_config::stage())
}

struct Config {
    twitch_client_id: String,
    twitch_client_secret: String,
    twitch_redirect_uri: String,
    twitter_api_key: String,
    twitter_api_secret: String,
    twitter_callback_url: String
}

impl Config {
    fn from_env() -> Self {
        Config {
            twitch_client_id: env::var("TWITCH_CLIENT_ID").expect("did not find a TWITCH_CLIENT_ID"),
            twitch_client_secret: env::var("TWITCH_CLIENT_SECRET").expect("did not find a TWITCH_CLIENT_SECRET"),
            twitch_redirect_uri: env::var("TWITCH_REDIRECT_URI").unwrap_or_else(|_| String::from("http://localhost:8000/twitch/authorize/callback")),
            twitter_api_key: env::var("TWITTER_API_KEY").expect("did not find a TWITTER_API_KEY"),
            twitter_api_secret: env::var("TWITTER_API_SECRET").expect("did not find a TWITTER_API_SECRET"),
            twitter_callback_url: env::var("TWITTER_CALLBACK_URL").unwrap_or_else(|_| String::from("http://127.0.0.1:8000/twitter/authorize/callback")),

        }
    }
}