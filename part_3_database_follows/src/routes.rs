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
    sbot, utils,
};

#[derive(FromForm)]
pub struct PeerForm {
    pub public_key: String,
}

#[get("/")]
pub async fn home(flash: Option<FlashMessage<'_>>) -> Template {
    let whoami = match sbot::whoami().await {
        Ok(id) => id,
        Err(e) => format!("Error making `whoami` RPC call: {}. Please ensure the local go-sbot is running and refresh.", e),
    };

    Template::render("base", context! { whoami: whoami, flash: flash })
}

#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(
    db: &State<Database>,
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

/*
#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        sbot::follow_if_not_following(&peer.public_key).await;
    }

    Ok(Redirect::to(uri!(home)))
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        sbot::unfollow_if_following(&peer.public_key).await;
    }

    Ok(Redirect::to(uri!(home)))
}
*/
