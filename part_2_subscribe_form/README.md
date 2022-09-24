# lykin tutorial

## Part 2: Subscription Form and Key Validation

### Introduction

In the first part of the tutorial series we created a basic web server and wrote our first Scuttlebutt-related code. This tutorial installment will add an HTML form and route handler(s) to allow peer subscriptions through the web interface. We will learn how to validate public keys submitted via the form and how to check whether or not we follow the peer represented by a submitted key. These additions will pave the way for following and unfollowing peers.

There's a lot of ground to cover today. Let's dive into it.

### Outline

Here's what we'll tackle in this second part of the series:

 - Split code into modules
 - Add peer subscription form and routes
 - Validate a public key
 - Add flash messages
 - Check if we are following a peer

### Libraries

The following libraries are introduced in this part:

 - [`log`](https://crates.io/crates/log)
 - [`rocket_dyn_templates`](https://crates.io/crates/rocket_dyn_templates)

### Split Code into Modules

A simple task to begin with: let's create an `sbot` module and a `routes` module and reorganise our code from the first part of the tutorial.

`src/routes.rs`

```rust
use rocket::get;

use crate::sbot;

#[get("/")]
pub async fn home() -> String {
    match sbot::whoami().await {
        Ok(id) => id,
        Err(e) => format!("whoami call failed: {}", e),
    }
}
```

`src/sbot.rs`

```rust
use std::env;

use golgi::{sbot::Keystore, Sbot};

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
```

`src/main.rs`

```rust
mod routes;
mod sbot;

use rocket::{launch, routes};

use crate::routes::*;

#[launch]
async fn rocket() -> _ {
    rocket::build().mount("/", routes![home])
}
```

### Add Peer Subscription Form and Routes

Now that we've taken care of some housekeeping, we can begin adding new functionality. We need a way to accept a public key; this will allow us to subscribe and unsubscribe to the posts of a particular peer. We'll use the [Tera templating engine](https://tera.netlify.app/) to create HTML templates for our application. Tera is inspired by the [Jinja2 template language](https://jinja.palletsprojects.com/en/3.0.x/) and is supported by [Rocket](https://rocket.rs/).

The Tera functionality we require is bundled in the `rocket_dyn_templates` crate. We can add that to our manifest:

`Cargo.toml`

`rocket_dyn_templates = { version = "0.1.0-rc.1", features = ["tera"] }`

We will modify the Rocket launch code in `src/main.rs` to attach a template fairing. Fairings are Rocket's approach to structured middleware:

```rust
use rocket_dyn_templates::Template;

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
}
```

Let's create a base template and add a form for submitting a Scuttlebutt public key. First we need to make a `templates` directory in the root of our lykin project:

`mkdir templates`

Open a template file for editing (notice the `.tera` suffix):

`templates/base.html.tera`

For now we'll write some HTML boilerplate code and a form to accept a public key. We'll use the same form for subscription and unsubscription events. Also notice the `{{ whoami }}` syntax which allows us to render a variable from the template context (defined in the route handler):

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>lykin</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
  </head>
  <body>
    <h1>lykin</h1>
    <p>{{ whoami }}</p>
    <form action="/subscribe" method="post">
      <label for="public_key">Public Key</label>
      <input type="text" id="public_key" name="public_key" maxlength=53>
      <input type="submit" value="Subscribe">
      <input type="submit" value="Unsubscribe" formaction="/unsubscribe">
    </form>
  </body>
</html>
```

Our `home` request handler needs to be updated to serve this HTML template:

`src/routes.rs`

```rust
use rocket_dyn_templates::{context, Template};

#[get("/")]
pub async fn home() -> Template {
    let whoami = match sbot::whoami().await {
        Ok(id) => id,
        Err(e) => format!("whoami call failed: {}", e),
    };

    Template::render("base", context! { whoami: whoami })
}
```

With the form in place, we can write our POST request handlers in our Rocket web server. These request handlers will be responsible for processing the submitted public key and triggering the follow / unfollow calls to the sbot. For now we'll simply use the `log` library to confirm that the handler(s) have been called before redirecting to the home route.

First add the `log` dependency to `Cargo.toml`:

`log = "0.4"`

Then add the subscription route handlers to the existing code in `src/routes.rs`:

```rust
use log::info;
use rocket::{form::Form, get, post, response::Redirect, uri, FromForm}

#[derive(FromForm)]
pub struct PeerForm {
    pub public_key: String,
}


#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(peer: Form<PeerForm>) -> Redirect {
    info!("Subscribing to peer {}", &peer.public_key);

    Redirect::to(uri!(home))
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(peer: Form<PeerForm>) -> Redirect {
    info!("Unsubscribing to peer {}", &peer.public_key);

    Redirect::to(uri!(home))
}
```

Finally, we need to register these two new routes in our Rocket launch code:

`src/main.rs`

```rust
#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
}
```

Run the project with the appropriate log level and ensure that everything is working correctly. You can test this by pasting a public key into the form input and clicking the Subscribe and Unsubscribe buttons.

`RUST_LOG=lykin=info cargo run`

### Validate a Public Key

We can now write some code to validate the input from our subscription form and ensure the data represents a valid Ed25519 Scuttlebutt key. We'll create a utilities module to house this function:

`src/utils.rs`

```rust
pub fn validate_public_key(public_key: &str) -> Result<(), String> {
    // Ensure the ID starts with the correct sigil link.
    if !public_key.starts_with('@') {
        return Err("expected '@' sigil as first character".to_string());
    }

    // Find the dot index denoting the start of the algorithm definition tag.
    let dot_index = match public_key.rfind('.') {
        Some(index) => index,
        None => return Err("no dot index was found".to_string()),
    };

    // Check the hashing algorithm (must end with ".ed25519").
    if !&public_key.ends_with(".ed25519") {
        return Err("hashing algorithm must be ed25519".to_string());
    }

    // Obtain the base64 portion (substring) of the public key.
    let base64_str = &public_key[1..dot_index];

    // Ensure the length of the base64 encoded ed25519 public key is correct.
    if base64_str.len() != 44 {
        return Err("base64 data length is incorrect".to_string());
    }

    Ok(())
}
```

Now the validation function can be called from our subscribe / unsubscribe route handlers, allowing us to ensure the provided public key is valid before using it to make further RPC calls to the sbot:

`src/routes.rs`

```rust
use crate::utils;

#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(peer: Form<PeerForm>) -> Redirect {
    info!("Subscribing to peer {}", &peer.public_key);
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        warn!("Public key {} is invalid: {}", &peer.public_key, e);
    }

    Redirect::to(uri!(home))
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(peer: Form<PeerForm>) -> Redirect {
		info!("Unsubscribing to peer {}", &peer.public_key);
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        warn!("Public key {} is invalid: {}", &peer.public_key, e);
    }

    Redirect::to(uri!(home))
}
```

### Add Flash Messages

Our log messages are helpful to us during development and production runs but the user of our applications is missing out on valuable information; they will have no idea whether or not the public keys they submit for subscription are valid. Let's add flash message support so we have a means of reporting back to the user via the UI.

Rocket makes this addition very simple, having [built-in support for flash message cookies](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Flash.html) in both the response and request handlers. We will have to update our `src/routes.rs` file as follows:

```rust
use rocket::{request::FlashMessage, response::Flash};

#[get("/")]
// Note the addition of the `flash` parameter.
pub async fn home(flash: Option<FlashMessage<'_>>) -> Template {
    // ...
    
    // The `flash` parameter value is added to the template context data.
    Template::render("base", context! { whoami: whoami, flash: flash })
}

#[post("/subscribe", data = "<peer>")]
// We return a `Result` type instead of a simple `Redirect`.
pub async fn subscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    info!("Subscribing to peer {}", &peer.public_key);
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("Public key {} is invalid: {}", &peer.public_key, e);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    }

    Ok(Redirect::to(uri!(home)))
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    info!("Unsubscribing to peer {}", &peer.public_key);
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("Public key {} is invalid: {}", &peer.public_key, e);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    }

    Ok(Redirect::to(uri!(home)))
}
```

From the code changes we've made above we can see that a successful key validation will simply result in a redirect to the home page, while an error during key validation will result in a redirect with the addition of a flash message cookie. Now we need to update our HTML template to show any error flash messages which might be set.

`templates/base.html.tera`

Add the following code below the `</form>` tag:

```html
{% if flash and flash.kind == "error" %}
<p style="color: red;">[ {{ flash.message }} ]</p>
{% endif %}
```

Now, if a submitted public key is invalid, a red error message will be displayed below the form - informing the application user of the error.

### Check Peer Follow Status

OK, that's a lot of web application shenanigans but I know you're really here for the Scuttlebutt goodness. Let's close-out this installment by writing a function to check whether or not the peer represented by our local go-sbot instance follows another peer; in simpler words: do we follow a peer account or not?

In order to do this using the `golgi` RPC library, we have to construct a `RelationshipQuery` `struct` and call the `friends_is_following()` method. Let's add a convenience function to initialise the sbot, construct the query and call `friends_is_following()` RPC method:

`src/sbot.rs`

```rust
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
```

When calling `is_following()`, we are asking: "does the peer represented by `public_key_a` follow the peer represented by `public_key_b`?" The returned value may be `Ok("true")`, `Ok("false")` or an error. Let's add these queries to our subscribe and unsubscribe route handlers:

`src/routes.rs`

```rust
#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    if let Err(e) = utils::validate_public_key(&peer.public_key) {
        let validation_err_msg = format!("Public key {} is invalid: {}", &peer.public_key, e);
        warn!("{}", validation_err_msg);
        return Err(Flash::error(Redirect::to(uri!(home)), validation_err_msg));
    } else {
        info!("Public key {} is valid", &peer.public_key);
        // Retrieve the value of the local public key by calling `whoami`.
        if let Ok(whoami) = sbot::whoami().await {
            // Do we follow the peer represented by the submitted public key?
            match sbot::is_following(&whoami, &peer.public_key).await {
                Ok(status) if status.as_str() == "false" => {
                    info!("Not currently following peer {}", &peer.public_key);
                    // This is where we will initiate a follow in the next
                    // installment of the tutorial series.
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
```

The code above is quite verbose due to the fact that we are matching on multiple possibilities. We could just as easily ignore the "already following" case in the subscription handler and the "not following" case in the unsubscription handler. The real star of the show is the sbot method: `sbot::is_following(peer_a, peer_b)`.

### Conclusion

Today we did a lot of work to make our project a more complete web application. We improved the organisation of our codebase by splitting it into modules, added an HTML form and handlers to enable peer subscription events, learned how to validate public keys and query follow status, and added flash message support to be able to report errors via the UI.

If you're confused by any of the code samples above, remember that you can see the complete code for this installment in the git repo.

In the next installment we'll add a key-value store and learn how to follow and unfollow Scuttlebutt peers.

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.
