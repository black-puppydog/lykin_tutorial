use std::env;

use async_std::stream::StreamExt;
use chrono::NaiveDateTime;
use golgi::{
    api::{friends::RelationshipQuery, history_stream::CreateHistoryStream},
    messages::{SsbMessageContentType, SsbMessageKVT},
    sbot::Keystore,
    GolgiError, Sbot,
};
use log::{info, warn};
use serde_json::value::Value;

use crate::db::Post;

/// Initialise a connection to a Scuttlebutt server.
pub async fn init_sbot() -> Result<Sbot, String> {
    let go_sbot_port = env::var("GO_SBOT_PORT").unwrap_or_else(|_| "8021".to_string());

    let keystore = Keystore::GoSbot;
    let ip_port = Some(format!("127.0.0.1:{}", go_sbot_port));
    let net_id = None;

    Sbot::init(keystore, ip_port, net_id)
        .await
        .map_err(|e| e.to_string())
}

/// Return the public key of the local sbot instance.
pub async fn whoami() -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    sbot.whoami().await.map_err(|e| e.to_string())
}

/// Check follow status.
///
/// Is peer A (`public_key_a`) following peer B (`public_key_b`)?
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

/// Follow a peer.
pub async fn follow_peer(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    sbot.follow(public_key).await.map_err(|e| e.to_string())
}

/// Unfollow a peer.
pub async fn unfollow_peer(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    sbot.unfollow(public_key).await.map_err(|e| e.to_string())
}

/// Check the follow status of a remote peer and follow them if not already
/// following.
pub async fn follow_if_not_following(remote_peer: &str) -> Result<(), String> {
    if let Ok(whoami) = whoami().await {
        match is_following(&whoami, remote_peer).await {
            Ok(status) if status.as_str() == "false" => match follow_peer(remote_peer).await {
                Ok(_) => {
                    info!("Followed peer {}", &remote_peer);
                    Ok(())
                }
                Err(e) => {
                    let err_msg = format!("Failed to follow peer {}: {}", &remote_peer, e);
                    warn!("{}", err_msg);
                    Err(err_msg)
                }
            },
            Ok(status) if status.as_str() == "true" => {
                info!(
                    "Already following peer {}. No further action taken",
                    &remote_peer
                );
                Ok(())
            }
            _ => Err(
                "Failed to determine follow status: received unrecognised response from local sbot"
                    .to_string(),
            ),
        }
    } else {
        let err_msg = String::from("Received an error during `whoami` RPC call. Please ensure the go-sbot is running and try again");
        warn!("{}", err_msg);
        Err(err_msg)
    }
}

/// Check the follow status of a remote peer and unfollow them if already
/// following.
pub async fn unfollow_if_following(remote_peer: &str) -> Result<(), String> {
    if let Ok(whoami) = whoami().await {
        match is_following(&whoami, remote_peer).await {
            Ok(status) if status.as_str() == "true" => {
                info!("Unfollowing peer {}", &remote_peer);
                match unfollow_peer(remote_peer).await {
                    Ok(_) => {
                        info!("Unfollowed peer {}", &remote_peer);
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to unfollow peer {}: {}", &remote_peer, e);
                        warn!("{}", err_msg);
                        Err(err_msg)
                    }
                }
            }
            _ => Err(
                "Failed to determine follow status: received unrecognised response from local sbot"
                    .to_string(),
            ),
        }
    } else {
        let err_msg = String::from("Received an error during `whoami` RPC call. Please ensure the go-sbot is running and try again");
        warn!("{}", err_msg);
        Err(err_msg)
    }
}

/// Return a stream of messages authored by the given public key.
///
/// This returns all messages regardless of type.
#[allow(dead_code)]
pub async fn get_message_stream(
    public_key: &str,
    sequence_number: u64,
) -> impl futures::Stream<Item = Result<SsbMessageKVT, GolgiError>> {
    let mut sbot = init_sbot().await.unwrap();

    let history_stream_args = CreateHistoryStream::new(public_key.to_string())
        .keys_values(true, true)
        .after_seq(sequence_number);

    sbot.create_history_stream(history_stream_args)
        .await
        .unwrap()
}

/// Return the name (self-identifier) for the peer associated with the given
/// public key.
///
/// The public key of the peer will be returned if a name is not found.
pub async fn get_name(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;

    sbot.get_name(public_key).await.map_err(|e| e.to_string())
}

/// Filter a stream of messages and return a vector of root posts.
///
/// Each returned vector element includes the key of the post, the content
/// text, the date the post was published, the sequence number of the post
/// and whether it is read or unread.
#[allow(dead_code)]
pub async fn get_root_posts(
    history_stream: impl futures::Stream<Item = Result<SsbMessageKVT, GolgiError>>,
) -> (u64, Vec<Post>) {
    let mut latest_sequence = 0;
    let mut posts = Vec::new();

    futures::pin_mut!(history_stream);

    while let Some(res) = history_stream.next().await {
        match res {
            Ok(msg) => {
                if msg.value.is_message_type(SsbMessageContentType::Post) {
                    let content = msg.value.content.to_owned();
                    if let Value::Object(content_map) = content {
                        if !content_map.contains_key("root") {
                            latest_sequence = msg.value.sequence;

                            let text = match content_map.get_key_value("text") {
                                Some(value) => value.1.to_string(),
                                None => String::from(""),
                            };
                            let timestamp = msg.value.timestamp.round() as i64 / 1000;
                            let datetime = NaiveDateTime::from_timestamp(timestamp, 0);
                            let date = datetime.format("%d %b %Y").to_string();
                            let subject = text.get(0..52).map(|s| s.to_string());

                            let post = Post::new(
                                msg.key.to_owned(),
                                text,
                                date,
                                msg.value.sequence,
                                timestamp,
                                subject,
                            );

                            posts.push(post)
                        }
                    }
                }
            }
            Err(err) => {
                // Print the `GolgiError` of this element to `stderr`.
                warn!("err: {:?}", err);
            }
        }
    }

    (latest_sequence, posts)
}
