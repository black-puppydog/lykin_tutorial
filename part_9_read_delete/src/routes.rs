use async_std::channel::Sender;
use log::{info, warn};
use rocket::{
    form::Form,
    get, post,
    request::FlashMessage,
    response::{Flash, Redirect},
    uri, FromForm, State,
};
use rocket_dyn_templates::{context, Template};

use crate::{
    db::{Database, Peer},
    sbot,
    task_loop::Task,
    utils,
};

#[derive(FromForm)]
pub struct PeerForm {
    pub public_key: String,
}

#[get("/")]
pub async fn home(db: &State<Database>, flash: Option<FlashMessage<'_>>) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        let unread_count = db.get_unread_post_count(&peer.public_key);
        peers_unread.push((peer, unread_count.to_string()));
    }

    Template::render("base", context! { peers: &peers_unread, flash: flash })
}

#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(
    db: &State<Database>,
    tx: &State<Sender<Task>>,
    peer: Form<PeerForm>,
) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        // Retrieve the name of the peer to which we are subscribing.
        let peer_name = match sbot::get_name(&peer.public_key).await {
            Ok(name) => name,
            Err(e) => {
                warn!("Failed to fetch name for peer {}: {}", &peer.public_key, e);
                // Return an empty string if an error occurs.
                String::from("")
            }
        };
        let peer_info = Peer::new(&peer.public_key).set_name(&peer_name);

        match sbot::follow_if_not_following(&peer.public_key).await {
            Ok(_) => {
                // Add the peer to the database.
                if db.add_peer(peer_info).is_ok() {
                    info!("Added {} to 'peers' database tree", &peer.public_key);
                    let peer_id = peer.public_key.to_string();

                    // Fetch all root posts authored by the peer we're subscribing
                    // to. Posts will be added to the key-value database.
                    if let Err(e) = tx.send(Task::FetchAllPosts(peer_id)).await {
                        warn!("Task loop error: {}", e)
                    }
                } else {
                    let err_msg = format!(
                        "Failed to add peer {} to 'peers' database tree",
                        &peer.public_key
                    );
                    warn!("{}", err_msg);
                    return Err(Flash::error(Redirect::to(uri!(home)), err_msg));
                }
            }
            Err(e) => {
                warn!("{}", e);
                return Err(Flash::error(Redirect::to(uri!(home)), e));
            }
        }
    }

    Ok(Redirect::to(uri!(home)))
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(
    db: &State<Database>,
    peer: Form<PeerForm>,
) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        match sbot::unfollow_if_following(&peer.public_key).await {
            Ok(_) => {
                // Remove the peer from the database.
                if db.remove_peer(&peer.public_key).is_ok() {
                    info!(
                        "Removed peer {} from 'peers' database tree",
                        &peer.public_key
                    );
                } else {
                    warn!(
                        "Failed to remove peer {} from 'peers' database tree",
                        &peer.public_key
                    );
                }
            }
            Err(e) => {
                warn!("{}", e);
                return Err(Flash::error(Redirect::to(uri!(home)), e));
            }
        }
    }

    Ok(Redirect::to(uri!(home)))
}

#[get("/posts/download_latest")]
pub async fn download_latest_posts(db: &State<Database>, tx: &State<Sender<Task>>) -> Redirect {
    for peer in db.get_peers() {
        // Fetch the latest root posts authored by each peer we're
        // subscribed to. Posts will be added to the key-value database.
        if let Err(e) = tx
            .send(Task::FetchLatestPosts(peer.public_key.clone()))
            .await
        {
            warn!("Task loop error: {}", e)
        }

        // Fetch the latest name for each peer we're subscribed to and update
        // the database.
        if let Err(e) = tx.send(Task::FetchLatestName(peer.public_key)).await {
            warn!("Task loop error: {}", e)
        }
    }

    Redirect::to(uri!(home))
}

#[get("/posts/<public_key>")]
pub async fn posts(db: &State<Database>, public_key: &str) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        let unread_count = db.get_unread_post_count(&peer.public_key);
        peers_unread.push((peer, unread_count.to_string()));
    }

    let posts = db.get_posts(public_key).unwrap();

    // Define context data to be rendered in the template.
    let context = context! {
        selected_peer: &public_key,
        peers: &peers_unread,
        posts: &posts
    };

    Template::render("base", context)
}

#[get("/posts/<public_key>/<msg_id>")]
pub async fn post(db: &State<Database>, public_key: &str, msg_id: &str) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        let unread_count = db.get_unread_post_count(&peer.public_key);
        peers_unread.push((peer, unread_count.to_string()));
    }

    let posts = db.get_posts(public_key).unwrap();
    let post = db.get_post(public_key, msg_id).unwrap();

    let context = context! {
        peers: &peers_unread,
        selected_peer: &public_key,
        selected_post: &msg_id,
        posts: &posts,
        post: &post,
        post_is_selected: &true
    };

    Template::render("base", context)
}

#[get("/posts/<public_key>/<msg_id>/read")]
pub async fn mark_post_read(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    // Retrieve the post from the database using the public key and msg_id
    // from the URL.
    if let Ok(Some(mut post)) = db.get_post(public_key, msg_id) {
        // Mark the post as read.
        post.read = true;
        // Reinsert the modified post into the database.
        db.add_post(public_key, post).unwrap();
    } else {
        warn!(
            "Failed to find post {} authored by {} in 'posts' database tree",
            msg_id, public_key
        )
    }

    Redirect::to(uri!(post(public_key, msg_id)))
}

#[get("/posts/<public_key>/<msg_id>/unread")]
pub async fn mark_post_unread(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    if let Ok(Some(mut post)) = db.get_post(public_key, msg_id) {
        post.read = false;
        db.add_post(public_key, post).unwrap();
    } else {
        warn!(
            "Failed to find post {} authored by {} in 'posts' database tree",
            msg_id, public_key
        )
    }

    Redirect::to(uri!(post(public_key, msg_id)))
}

#[get("/posts/<public_key>/<msg_id>/delete")]
pub async fn delete_post(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    // Delete the post from the database.
    match db.remove_post(public_key, msg_id) {
        Ok(_) => info!(
            "Removed post {} by {} from 'posts' database tree",
            msg_id, public_key
        ),
        Err(e) => warn!(
            "Failed to remove post {} by {} from 'posts' database tree: {}",
            msg_id, public_key, e
        ),
    }

    Redirect::to(uri!(posts(public_key)))
}
