mod db;
mod routes;
mod sbot;
mod task_loop;
mod utils;

use async_std::channel;
use log::info;
use rocket::{
    fairing::AdHoc,
    fs::{relative, FileServer},
    launch, routes,
};
use rocket_dyn_templates::Template;
use xdg::BaseDirectories;

use crate::{db::Database, routes::*, task_loop::Task};

#[launch]
async fn rocket() -> _ {
    // Create the key-value database.
    let xdg_dirs = BaseDirectories::with_prefix("lykin").unwrap();
    let db_path = xdg_dirs
        .place_config_file("database")
        .expect("cannot create database directory");
    let db = Database::init(&db_path);
    let db_clone = db.clone();

    // Create a message passing channel.
    let (tx, rx) = channel::unbounded();
    let tx_clone = tx.clone();

    // Spawn the task loop, passing in the receiver half of the channel.
    info!("Spawning task loop");
    task_loop::spawn(db_clone, rx).await;

    rocket::build()
        .manage(db)
        .manage(tx)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                home,
                subscribe_form,
                unsubscribe_form,
                download_latest_posts,
                post,
                posts
            ],
        )
        .mount("/", FileServer::from(relative!("static")))
        .attach(AdHoc::on_shutdown("cancel task loop", |_| {
            Box::pin(async move {
                tx_clone.send(Task::Cancel).await.unwrap();
            })
        }))
}
