use async_std::{channel::Receiver, task};
use log::info;

pub enum Task {
    Cancel,
    FetchAllPosts(String),
    FetchLatestName(String),
}

/// Spawn an asynchronous loop which receives tasks over an unbounded channel
/// and invokes task functions accordingly.
pub async fn spawn(rx: Receiver<Task>) {
    task::spawn(async move {
        while let Ok(task) = rx.recv().await {
            match task {
                // Fetch all messages authored by the given peer, filter
                // the root posts and insert them into the posts tree of the
                // database.
                Task::FetchAllPosts(peer_id) => {
                    info!("Fetching all posts for peer: {}", peer_id);
                }
                // Fetch the latest name for the given peer and update the
                // peer entry in the peers tree of the database.
                Task::FetchLatestName(peer_id) => {
                    info!("Fetching latest name for peer: {}", peer_id);
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
