use log::{info, warn};
use rocket::{
    form::Form,
    get, post,
    request::FlashMessage,
    response::{Flash, Redirect},
    uri, FromForm,
};
use rocket_dyn_templates::{context, Template};

use crate::{sbot, utils};

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
pub async fn subscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        if let Ok(whoami) = sbot::whoami().await {
            match sbot::is_following(&whoami, &peer.public_key).await {
                Ok(status) if status.as_str() == "false" => {
                    info!("Not currently following peer {}", &peer.public_key);
                }
                Ok(status) if status.as_str() == "true" => {
                    info!(
                        "Already following peer {}. No further action taken",
                        &peer.public_key
                    )
                }
                _ => (),
            }
        } else {
            warn!("Received an error during `whoami` RPC call. Please ensure the go-sbot is running and try again")
        }
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
        if let Ok(whoami) = sbot::whoami().await {
            match sbot::is_following(&whoami, &peer.public_key).await {
                Ok(status) if status.as_str() == "true" => {
                    info!("Currently following peer {}", &peer.public_key);
                }
                Ok(status) if status.as_str() == "false" => {
                    info!(
                        "Not currently following peer {}. No further action taken",
                        &peer.public_key
                    );
                }
                _ => (),
            }
        } else {
            warn!("Received an error during `whoami` RPC call. Please ensure the go-sbot is running and try again")
        }
    }

    Ok(Redirect::to(uri!(home)))
}
