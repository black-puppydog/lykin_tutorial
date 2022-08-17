use std::env;

use golgi::{api::friends::RelationshipQuery, sbot::Keystore, Sbot};

pub async fn init_sbot() -> Result<Sbot, String> {
    let go_sbot_port = env::var("GO_SBOT_PORT").unwrap_or_else(|_| "8021".to_string());

    let keystore = Keystore::GoSbot;
    let ip_port = Some(format!("127.0.0.1:{}", go_sbot_port));
    let net_id = None;

    Sbot::init(keystore, ip_port, net_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn whoami() -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    sbot.whoami().await.map_err(|e| e.to_string())
}

pub async fn is_following(public_key_a: &str, public_key_b: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    let query = RelationshipQuery {
        source: public_key_a.to_string(),
        dest: public_key_b.to_string(),
    };

    sbot.friends_is_following(query)
        .await
        .map_err(|e| e.to_string())
}
