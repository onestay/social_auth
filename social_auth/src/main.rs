mod twitch_config;
mod api;
mod twitter_config;
#[macro_use]
extern crate rocket;
use rocket::fs::FileServer;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", FileServer::from("public"))
        .attach(twitch_config::stage())
        .attach(api::stage())
        .attach(twitter_config::stage())
}
