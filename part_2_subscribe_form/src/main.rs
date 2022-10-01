#![doc = include_str!("../README.md")]

mod routes;
mod sbot;
mod utils;

use rocket::{launch, routes};
use rocket_dyn_templates::Template;

use crate::routes::*;

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
}
