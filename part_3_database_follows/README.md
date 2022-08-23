# lykin tutorial

## Part 3: Database and Follows 

### Introduction

Having learned how to make follow-graph queries in part two, this tutorial installment will demonstrate how to follow and unfollow Scuttlebutt peers and how to query the name of a peer. In addition to the Scuttlebutt-related code, we will create a key-value database to store a list of peers to whom we are subscribed. The subscription logic of our application will be largely complete by the end of this installment.

### Outline

Here's what we'll tackle in this third part of the series:

 - Setup a key-value database
 - Create a peer data structure
 - Add and remove peers from the database
 - Follow and unfollow a peer
 - Get the name of a peer
 - Extract follow / unfollow logic
 - Pass database instance to route handlers
 - Complete the subscribe / unsubscribe flow

### Libraries

The following libraries are introduced in this part:

 - [`bincode`](https://crates.io/crates/bincode)
 - [`serde`](https://crates.io/crates/serde)
 - [`sled`](https://crates.io/crates/sled)
 - [`xdg`](https://crates.io/crates/xdg)

### Setup a Key-Value Database

We're going to use [sled](https://sled.rs/) in order to store the data used by our application; namely, peer and post-related data. Sled is a transactional embedded database written in pure Rust. We'll create a separate module for the database code to continue our trend of keeping code separate and organised. Let's begin by creating a `struct` to store our database instance and peer tree. We will also implement an initialisation method for the database:

`src/db.rs`

```rust
use sled::{Db, Tree};

#[derive(Clone)]
pub struct Database {
    // The sled database instance.
    db: Db,
    // A database tree for all the peers we are subscribed to.
    peer_tree: Tree,
}

impl Database {
    // Initialise the database by opening the database file, loading the
    // peers tree and returning an instantiated Database struct.
    pub fn init(path: &Path) -> Self {
        // Open the database at the given path.
        // The database will be created if it does not yet exist.
        let db = sled::open(path).expect("Failed to open database");
        // Open a database tree with the name "peers".
        let peer_tree = db
            .open_tree("peers")
            .expect("Failed to open 'peers' database tree");

        Database { db, peer_tree }
    }
}
```

The initialisation method requires a `Path` in order to open / create the database. We can use the [xdg](https://crates.io/crates/xdg) crate to generate a path using the XDG Base Directory specification. Open `src/main.rs` and add the following code:

```rust
mod db;

use xdg::BaseDirectories.

use crate::{db::Database, routes::*};

#[launch]
async fn rocket() -> _ {
    // Define "lykin" as the prefix for the base directories.
    let xdg_dirs = BaseDirectories::with_prefix("lykin").unwrap();
    // Generate a configuration file path named "database".
    // On Linux, the path will be `~/.config/lykin/database`.
    let db_path = xdg_dirs
        .place_config_file("database")
        .expect("cannot create database directory");

    // Create the key-value database.
    let db = Database::init(&db_path);

    rocket::build()
        // Add the database instance to the Managed State of our Rocket
        // application. This allows us to access the database from inside
        // of our route handlers.
        .manage(db)
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
}
```

### Create a Peer Data Structure

Now that we've initialised our database and have a place to store peer data, we can define the shape of that data by creating a `Peer` struct. For now we'll simply be storing the public key and name of each peer. Add this code to what we already have in `src/db.rs`:

```rust
use serde::{Deserialize, Serialize};

// Scuttlebutt peer data.
#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub public_key: String,
    pub name: String,
}
```

In addition to the datastructure itself, we'll implement a couple of methods to be able to create and modify instances of the `struct`.

`src/db.rs`

```rust
impl Peer {
    // Create a new instance of the Peer struct using the given public
    // key. A default value is set for name.
    pub fn new(public_key: &str) -> Peer {
        Peer {
            public_key: public_key.to_string(),
            name: "".to_string(),
        }
    }

    // Modify the name field of an instance of the Peer struct, leaving
    // the other values unchanged.
    pub fn set_name(self, name: &str) -> Peer {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}
```

### Add and Remove Peers from Database

Let's extend the implementation of `Database` to include methods for adding and removing peers:

`src/db.rs`


```rust
use sled::{IVec, Result};

impl Database {
    pub fn init(path: &Path) -> Self {
        // ...
    }

    // Add a peer to the database by inserting the public key into the peer
    // tree.
    pub fn add_peer(&self, peer: Peer) -> Result<Option<IVec>> {
        // Serialise peer data as bincode. 
        let peer_bytes = bincode::serialize(&peer).unwrap();

        // Insert the serialised peer data into the 'peers' database tree,
        // using the public key of the peer as the key for the database entry.
        self.peer_tree.insert(&peer.public_key, peer_bytes)
    }

    // Remove a peer from the database, as represented by the given public
    // key.
    pub fn remove_peer(&self, public_key: &str) -> Result<()> {
        self.peer_tree.remove(&public_key).map(|_| ())
    }
}
```

You'll notice in the above code snippet that we're serialising the peer data as bincode before inserting it. The sled database we're using expects values in the form of a byte vector; bincode thus provides a neat way of storing complex datastructures (such as our `Peer` `struct`).

That's enough database code for the moment. Now we can return to our Scuttlebutt-related code and complete the peer subscription flows.

### Follow / Unfollow a Peer

Let's open the `src/sbot.rs` module and write the functions we need to be able to follow and unfollow Scuttlebutt peers. Each function will simply take the public key of the peer whose relationship we wish to change:

```rust
pub async fn follow_peer(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.follow(public_key).await.map_err(|e| e.to_string())
}

pub async fn unfollow_peer(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.unfollow(public_key).await.map_err(|e| e.to_string())
}
```

The `Ok(_)` variant of the returned `Result` type will contain the message reference of the published follow / unfollow message. Once again, we are transforming any possible error to a `String` for easier handling and reporting in the caller function.

At this point we have the capability to check whether we follow a peer, to add and remove peer data to our key-value store, and to follow and unfollow a peer. Casting our minds back to the `subscribe` and `unsubscribe` route handlers of our webserver, we can now add calls to `follow_peer()` and `unfollow_peer()`:

`src/routes.rs`

```rust
// Update this match block in `subscribe_form`
match sbot::is_following(&whoami, remote_peer).await {
    Ok(status) if status.as_str() == "false" => {
        // If we are not following the peer, call the `follow_peer` method.
        match sbot::follow_peer(remote_peer).await {
            Ok(_) => info!("Followed peer {}", &remote_peer),
            Err(e) => warn!("Failed to follow peer {}: {}", &remote_peer, e),
        }
    }
    Ok(status) if status.as_str() == "true" => {
        info!(
            "Already following peer {}. No further action taken",
            &remote_peer
        )
    }
    _ => (),
}

// Update this match block in `unsubscribe_form`
match sbot::is_following(&whoami, remote_peer).await {
    Ok(status) if status.as_str() == "true" => {
        // If we are following the peer, call the `unfollow_peer` method.
        info!("Unfollowing peer {}", &remote_peer);
        match sbot::unfollow_peer(remote_peer).await {
            Ok(_) => {
                info!("Unfollowed peer {}", &remote_peer);
            }
            Err(e) => warn!("Failed to unfollow peer {}: {}", &remote_peer, e),
        }
    }
    _ => (),
}
```

Excellent. We're now able to initiate follow and unfollow actions via the web interface of our application. Checking the state of our relationship with the peer helps to prevent publishing unnecessary follow / unfollow messages. There is no need to publish an additional follow message if we already follow a peer.

We're almost ready to start adding and removing peers to our key-value store each time a `subscribe` or `unsubscribe` form action is submitted. Before we can do that we need to be able to query the name of a peer.

### Get Peer Name

Querying the name of a Scuttlebutt peer is just as simple as following or unfollowing:

`src/sbot.rs`

```rust
pub async fn get_name(public_key: &str) -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.get_name(public_key).await.map_err(|e| e.to_string())
}
```

As usual, we initialise a connection with the sbot and then make our method call. This method will either return the name of a peer or the public key of the peer. The public key is returned if the sbot does not have a name stored in its indexes; this can happen if the peer is out of range of our follow graph, for example. You've probably seen this behaviour in your favourite Scuttlebutt client...sometimes it takes a while to receive an `about` message containing an assigned name for a peer.

### Extract Follow / Unfollow Logic

Now let's go back to our subscribe and unsubscribe route handlers and separate some of the Scuttlebutt control flow out into the `sbot` module. Separating concerns like this will help to bring greater clarity to the handler functions.

Add the following two function to `src/sbot.rs`. You'll notice that we're using a `Result` return type for each function. This will allow us to match on the outcome in our route handlers and report back to the UI. The logging makes the functions look very busy but the sbot actions tell the story. 

`src/sbot.rs`

```rust
pub async fn follow_if_not_following(remote_peer: &str) -> Result<(), String> {
    if let Ok(whoami) = whoami().await {
        match is_following(&whoami, remote_peer).await {
            Ok(status) if status.as_str() == "false" => {
                match follow_peer(remote_peer).await {
                    Ok(_) => {
                        info!("Followed peer {}", &remote_peer);
                    
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = warn!("Failed to follow peer {}: {}", &remote_peer, e);
                        warn!("{}", err_msg);

                        Err(err_msg)
                    }
                }
            }
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

pub async fn unfollow_if_following(remote_peer: &str) {
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

                        Err(e)
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

        Err(e)
    }
}
```

Now we can remove the follow / unfollow logic from our route handlers and call `sbot::follow_if_not_following()` and `sbot::unfollow_if_following()` instead:

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
        match sbot::follow_if_not_following(&peer.public_key).await {
            Ok(_) => (),
            Err(e) => {
                warn!("{}", e);
                return Err(Flash::error(Redirect::to(uri!(home)), e));
            }
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
        match sbot::unfollow_if_following(&peer.public_key).await {
            Ok(_) => (),
            Err(e) => {
                warn!("{}", e);
                return Err(Flash::error(Redirect::to(uri!(home)), e));
            }
        }
    }

    Ok(Redirect::to(uri!(home)))
}
```

### Pass Database Instance to Route Handlers

We are about to add some database interactions to the code in our `/subscribe` and `/unsubscribe` route handlers. If you recall, we added an instance of our database to the managed state of our Rocket application at the beginning of this tutorial; we instantiated the database and called `rocket::build.manage(db)` in `src/main.rs`. By doing so we gained the ability to access the database from our route handlers. The final requirement is that we add the `db` as a parameter in the function signature of each handler (`db: &State<Database>`):

`src/routes.rs`

```rust
use rocket::State;

use crate::db::Database;

#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(db: &State<Database>, peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    // ...
}

#[post("/unsubscribe", data = "<peer>")]
pub async fn unsubscribe_form(db: &State<Database>, peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
    // ...
}
```

### Complete the Subscribe / Unsubscribe Flow

We now have all the pieces we need to complete the subscribe and unsubscribe actions for our web application. Before modifying the code, here's a simple outline of what each handler will do (assuming the "happy path" occurs and no errors are generated):

```text
/subscribe

 -> validate public key of peer
 -> get name of peer
 -> follow peer if not following
 -> add peer (public key and name) to database

/unsubscribe

 -> validate public key of peer
 -> unfollow peer if following
 -> remove peer from database
```

Let's add the `get_name()`, `add_peer()` and `remove_peer()` logic.

`src/routes.rs`

```rust
#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(db: &State<Database>, peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
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
pub async fn unsubscribe_form(db: &State<Database>, peer: Form<PeerForm>) -> Result<Redirect, Flash<Redirect>> {
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
```

At this point it's a good idea to run the code and experiment with subscribing and unsubscribing to peers. Remember to set the `RUST_LOG` environment variable so you can view the output as you interact with the application:

`RUST_LOG=info cargo run`

### Conclusion

In this installment we added an important pillar of our application: the key-value database. We added code to instantiate the database and created a `Peer` datastructure to store data about each peer we subscribe to. We also added methods for adding and removing peers from the database. By leveraging Rocket's managed state, we exposed our instantiated database to the code in our route handlers.

In addition to all of the database-related work, we added Scuttlebutt code to follow, unfollow and retrieve the name for a peer. Such actions are fundamental to any social Scuttlebutt application you may want to write.

Finally, we put all the pieces together and completed the workflow for our subscription and unsubscription routes. Well done for making it this far!

In the next installment we'll deal primarily with Scuttlebutt messages - learning how to get all the messages authored by a peer, as well as how to filter down to post-type messages and add them to our key-value database.

## Funding

This work has been funded by a Scuttlebutt Community Grant.
