# lykin tutorial

## Part 5: Task Loop and Post Fetching

### Introduction

In the last installment we added support to our key-value database for dealing with Scuttlebutt posts and wrote code to create and filter streams of Scuttlebutt messages. Since our peers may have authored tens of thousands of messages, it's useful to create a way of fetching and filtering message streams as a background process. Today we'll do just that; writing a task loop that can be invoked from our web application route handlers and used to execute potentially long-running processes.

### Outline

Here's what we'll tackle in this fifth part of the series:

 - Create an asynchronous task loop
 - Create a message passing channel and spawn the task loop
 - Write sbot-related task functions
 - Pass database instance into task loop
 - Fetch root posts on subscription

### Create an Asynchronous Task Loop

Let's start by defining a task type that enumerates the various tasks we might want to carry out. We'll create a separate module for our task loop:

`src/task_loop.rs`

```rust
pub enum Task {
    Cancel,
    FetchAllPosts(String),
    FetchLatestName(String),
}
```

The `Task` enum is simple enough: we can fetch all the posts by a given peer (the `String` value is the public key of the peer we're interested in), fetch the latest name assigned to a peer or cancel the task loop.

We're going to use a message passing approach in order to trigger tasks inside the loop. Let's write the basic loop code now, adding it below the `Task` we just defined, while also adding the necessary crate imports:

```rust
use async_std::{channel::Receiver, task};
use log::info;

// Spawn an asynchronous loop which receives tasks over an unbounded channel
// and invokes task functions accordingly.
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
```

The loop spawning function is fairly simple: it takes the receiver half of a channel and expects messages of type `Task` to be delivered; it matches on the `Task` variant each time a message is received on the channel and acts accordingly. Writing an async loop like this means that we can call functions without blocking the execution of the rest of our program. This is a particularly useful in route handlers where we want to be able to trigger a task and then immediately respond to the request in order to keep the UI snappy and responsive.

### Create Message Passing Channel and Spawn the Task Loop

Let's return to the root of our application to create the message passing channel, spawn the task loop and add the channel transmitter to managed state:

`src/main.rs`

```rust
mod task_loop;

use async_std::channel;
use log::info;
use rocket::fairing::AdHoc;

use crate::task_loop::Task;

#[launch] async fn rocket() -> _ {
    // ...

		// Create the key-value database.
		// ...

    // Create a message passing channel.
    let (tx, rx) = channel::unbounded();
    let tx_clone = tx.clone();

    // Spawn the task loop, passing in the receiver half of the channel.
    info!("Spawning task loop");
    task_loop::spawn(rx).await;

		rocket::build()
				.manage(db)
				// Add the transmitter half of the channel to the managed state
				// of the Rocket application.
				.manage(tx)
				// ...
				// Send a task loop cancellation message when the application
				// is shutting down.
        .attach(AdHoc::on_shutdown("cancel task loop", |_| {
            Box::pin(async move {
                tx_clone.send(Task::Cancel).await.unwrap();
            })
        }))
}
```

Reviewing the code above: first an unbounded, asynchronous channel is created and split into transmitting (`tx`) and receiving (`rx`) ends, after which the transmitting channel is cloned. The task loop is then spawned and takes with it the receiving end of the channel. As we did previously with the `db` instance, the transmitting half of the channel is added to the managed state of the Rocket application; this will allow us to transmit tasks to the task loop from our web route handlers. Finally, a shutdown handler is attached to the Rocket application in order to send a cancellation task to the task loop before the program ends. This ensures that the task loop closes cleanly.

### Write Sbot-Related Task Functions

Before we can write the sbot-related task functions, we first need to add a method to our database code to allow the retrieval of data for a specific peer. Since we serialized the peer data as bincode before inserting it into the database, we need to deserialize the value after fetching it.

`src/db.rs`

```rust
impl Database {
    // pub fn add_peer(&self, peer: Peer) -> Result<Option<IVec>> {
        // ...
    // }

    // Get a single peer from the peer tree, defined by the given public key.
    // The byte value for the matching entry, if found, is deserialized from
    // bincode into an instance of the Peer struct.
    pub fn get_peer(&self, public_key: &str) -> Result<Option<Peer>> {
        debug!(
            "Retrieving peer data for {} from 'peers' database tree",
            &public_key
        );
        let peer = self
            .peer_tree
            .get(public_key.as_bytes())
            .unwrap()
            .map(|peer| {
                debug!("Deserializing peer data for {} from bincode", &public_key);
                bincode::deserialize(&peer).unwrap()
            });

        Ok(peer)
    }

    // ...
}
```

Now it's time to write the functions that will be executed when the `FetchAllPosts` and `FetchLatestName` tasks are invoked. These functions will be responsible for retrieving data from the sbot and updating the database with the latest values. We can keep our task loop neat and readable by separating this logic into functions:

`src/task_loop.rs`

```rust
use log::warn;

use crate::{Database, sbot};

// Retrieve a set of posts from the local sbot instance and add them to the
// posts tree of the database.
//
// A stream of messages is first requested for the peer represented by the
// given public key (ID), starting after the given sequence number. The root
// posts are filtered from the set of messages and added to the database as a
// batch. Finally, the value of the latest sequence for the peer is updated
// and saved to the existing database entry.
async fn fetch_posts_and_update_db(db: &Database, peer_id: String, after_sequence: u64) {
    let peer_msgs = sbot::get_message_stream(&peer_id, after_sequence).await;
    let (_latest_sequence, root_posts) = sbot::get_root_posts(peer_msgs).await;

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
}

// Request the name of the peer represented by the given public key (ID)
// and update the existing entry in the database.
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
```

These function calls can now be added to our task matching code in the task loop. Note that we also need to add the database instance as a parameter in the function isgnature:

`src/task_loop.rs`

```rust
pub async fn spawn(db: Database, rx: Receiver<Task>) {
    task::spawn(async move {
        while let Ok(task) = rx.recv().await {
            match task {
                Task::FetchAllPosts(peer_id) => {
                    info!("Fetching all posts for peer: {}", peer_id);
                    fetch_posts_and_update_db(&db, peer_id, 0).await;
                }
                Task::FetchLatestName(peer_id) => {
                    info!("Fetching latest name for peer: {}", peer_id);
                    fetch_name_and_update_db(&db, peer_id).await;
                }
                Task::Cancel => {
                    info!("Exiting task loop...");
                    break;
                }
            }
        }
    });
}
```

### Pass Database Instance Into Task Loop

As it currently stands, our code will fail to compile because `task_loop::spawn()` expects a database instance which has not yet been provided. We need to revisit the code in the root of our application to clone the database and pass it into the task loop:

`src/main.rs`

```rust
#[launch]
async fn rocket() -> _ {
    // ...
    let db = Database::init(&db_path);
    // Clone the database instance.
    let db_clone = db.clone();

    // Create a message passing channel.
    let (tx, rx) = channel::unbounded();
    let tx_clone = tx.clone();

    // Spawn the task loop.
    info!("Spawning task loop");
    // Pass the clone database instance and the rx channel into the task loop.
    task_loop::spawn(db_clone, rx).await;

    // ...
}
```

### Fetch Root Posts on Subscription

Great, the task loop is primed and ready for action. We are very close to being able to initiate tasks from the route handler(s) of our web application. Earlier in this installment of the tutorial we created a message passing channel in `src.main.rs` and added the transmission end of the channel to the managed state of our Rocket instance. We need to add the transmitter as a parameter of the `subscribe_form` function before we can invoke tasks:

`src/routes.rs`

```rust
use async_std::channel::Sender;

use crate::task_loop::Task;

#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(
    db: &State<Database>,
    tx: &State<Sender<Task>>,
    peer: Form<PeerForm>,
) -> Result<Redirect, Flash<Redirect>> {
    info!("Subscribing to peer {}", &peer.public_key);
    // ...
}
```

Now, when a subscription event occurs (ie. the subscribe form is submitted with a peer ID), we can trigger a task to fetch all the root posts for that peer and add them to the key-value database. Note that I've omitted most of the code we've already written from the sample below. The most important three lines are those beginning with `if let Err(e) = tx.send...`.

```rust
#[post("/subscribe", data = "<peer>")]
pub async fn subscribe_form(
    db: &State<Database>,
    tx: &State<Sender<Task>>,
    peer: Form<PeerForm>,
) -> Result<Redirect, Flash<Redirect>> {
    // ... {
        match sbot::follow_if_not_following(&peer.public_key).await {
            Ok(_) => {
                if db.add_peer(peer_info).is_ok() {
                    // ...

                    // Fetch all root posts authored by the peer we're subscribing
                    // to. Posts will be added to the key-value database.
                    if let Err(e) = tx.send(Task::FetchAllPosts(peer_id)).await {
                        warn!("Task loop error: {}", e)
                    }
                } else {
                    // ...
                }
            }
            Err(e) => {
                // ...
            }
        }
    }

    Ok(Redirect::to(uri!(home)))
}
```

### Conclusion

In this installment we wrote an asynchronous task loop and `Task` type to be able to execute background processes in our application. We created task variants and functions for two primary operations: 1. fetching all the root posts for a peer and adding them to the key-value database, and 2. fetching the latest name assigned to a peer. We created a message passing channel, passed the receiving end to the task loop and the transmitting end to the managed state of our web application, and invoked the fetch-all task from our subscription route handler.

The `Task` type and loop we wrote today can be easily extended by adding more variants. It's a part of the code we will return to in a future installment.

In the next tutorial installment we'll focus on updating the web interface. We'll add more templates to create a modular layout, write some CSS and populate a list of peers from the data in our key-value store. Soon the application will begin to take shape!

## Funding

This work has been funded by a Scuttlebutt Community Grant.
