use crate::twitch_config::Twitch;
use crate::twitter_config::Twitter;
use rocket::{serde::Serialize, State};
use rocket_dyn_templates::Template;
use std::path::Path;

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("templates", |rocket| async {
        rocket
            .mount("/", routes![index])
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
pub struct IndexContext {
    creator: String,
    twitch_url: String,
    twitter_url: String,
    twitter_avail: bool,
    twitch_avail: bool,
}

#[get("/")]
async fn index(twitch: &State<Twitch>, twitter: &State<Twitter>) -> Template {
    let context = IndexContext {
        creator: "onestay".to_string(),
        twitch_url: twitch.get_authorize_url(),
        twitter_url: twitter
            .get_authorize_url()
            .await
            .expect("can't get twitter url"),
        twitch_avail: is_twitch_avail(),
        twitter_avail: is_twitter_avail(),
    };
    Template::render("index", context)
}
