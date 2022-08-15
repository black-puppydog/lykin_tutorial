use std::env;

use golgi::{sbot::Keystore, Sbot};
use rocket::{get, launch, routes};

async fn init_sbot() -> Result<Sbot, String> {
    let go_sbot_port = env::var("GO_SBOT_PORT").unwrap_or_else(|_| "8021".to_string());

    let keystore = Keystore::GoSbot;
    let ip_port = Some(format!("127.0.0.1:{}", go_sbot_port));
    let net_id = None;

    Sbot::init(keystore, ip_port, net_id)
        .await
        .map_err(|e| e.to_string())
}

async fn whoami() -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.whoami().await.map_err(|e| e.to_string())
}

#[get("/")]
async fn home() -> String {
    match whoami().await {
        Ok(id) => id,
        Err(e) => format!("whoami call failed: {}", e),
    }
}

#[launch]
async fn rocket() -> _ {
    rocket::build().mount("/", routes![home])
}
