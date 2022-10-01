#![doc = include_str!("../README.md")]

mod db;
mod routes;
mod sbot;
mod utils;

use rocket::{launch, routes};
use rocket_dyn_templates::Template;
use xdg::BaseDirectories;

use crate::{db::Database, routes::*};

#[launch]
async fn rocket() -> _ {
    // Create the key-value database.
    let xdg_dirs = BaseDirectories::with_prefix("lykin").unwrap();
    let db_path = xdg_dirs
        .place_config_file("database")
        .expect("cannot create database directory");
    let db = Database::init(&db_path);

    rocket::build()
        .manage(db)
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
}
