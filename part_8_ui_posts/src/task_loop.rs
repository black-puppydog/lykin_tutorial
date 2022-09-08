use async_std::{channel::Receiver, task};
use log::{info, warn};

use crate::{sbot, Database};

async fn fetch_posts_and_update_db(db: &Database, peer_id: String, after_sequence: u64) {
    let peer_msgs = sbot::get_message_stream(&peer_id, after_sequence).await;
    let (latest_sequence, root_posts) = sbot::get_root_posts(peer_msgs).await;

    match db.add_post_batch(&peer_id, root_posts) {
        Ok(_) => {
            info!(
                "Inserted batch of posts into database post tree for peer: {}",
                &peer_id
            )
        }
        Err(e) => warn!(
            "Failed to insert batch of posts into database post tree for peer: {}: {}",
            &peer_id, e
        ),
    }

    // Update the value of the latest sequence number for
    // the peer (this is stored in the database).
    if let Ok(Some(peer)) = db.get_peer(&peer_id) {
        db.add_peer(peer.set_latest_sequence(latest_sequence))
            .unwrap();
    }
}

/// Request the name of the peer represented by the given public key (ID)
/// and update the existing entry in the database.
async fn fetch_name_and_update_db(db: &Database, peer_id: String) {
    match sbot::get_name(&peer_id).await {
        Ok(name) => {
            if let Ok(Some(peer)) = db.get_peer(&peer_id) {
                let updated_peer = peer.set_name(&name);
                match db.add_peer(updated_peer) {
                    Ok(_) => info!("Updated name for peer: {}", &peer_id),
                    Err(e) => {
                        warn!("Failed to update name for peer: {}: {}", &peer_id, e)
                    }
                }
            }
        }
        Err(e) => warn!("Failed to fetch name for {}: {}", &peer_id, e),
    }
}

pub enum Task {
    Cancel,
    FetchAllPosts(String),
    FetchLatestPosts(String),
    FetchLatestName(String),
}

/// Spawn an asynchronous loop which receives tasks over an unbounded channel
/// and invokes task functions accordingly.
pub async fn spawn(db: Database, rx: Receiver<Task>) {
    task::spawn(async move {
        while let Ok(task) = rx.recv().await {
            match task {
                // Fetch all messages authored by the given peer, filter
                // the root posts and insert them into the posts tree of the
                // database.
                Task::FetchAllPosts(peer_id) => {
                    info!("Fetching all posts for peer: {}", peer_id);
                    fetch_posts_and_update_db(&db, peer_id, 0).await;
                }
                // Fetch only the latest messages authored by the given peer,
                // ie. messages with sequence numbers greater than those
                // which are already stored in the database.
                //
                // Retrieve the root posts from those messages and insert them
                // into the posts tree of the database.
                Task::FetchLatestPosts(peer_id) => {
                    if let Ok(Some(peer)) = db.get_peer(&peer_id) {
                        info!("Fetching latest posts for peer: {}", peer_id);
                        fetch_posts_and_update_db(&db, peer_id, peer.latest_sequence).await;
                    }
                }
                // Fetch the latest name for the given peer and update the
                // peer entry in the peers tree of the database.
                Task::FetchLatestName(peer_id) => {
                    info!("Fetching latest name for peer: {}", peer_id);
                    fetch_name_and_update_db(&db, peer_id).await;
                }
                // Break out of the task loop.
                Task::Cancel => {
                    info!("Exiting task loop...");
                    break;
                }
            }
        }
    });
}
