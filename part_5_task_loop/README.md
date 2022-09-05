# lykin tutorial

## Part 5: Task Loop and Post Fetching

### Introduction

In the last installment we added support to our key-value database for dealing with Scuttlebutt posts and wrote code to create and filter streams of Scuttlebutt messages. Since our peers may have authored tens of thousands of messages, it's useful to create a way of fetching and filtering message streams as a background process. Today we'll do just that; writing a task loop that we can be invoked from our web application route handlers and used to execute potentially long-running processes.

### Outline

Here's what we'll tackle in this fifth part of the series:

 - Create an asynchronous task loop
 - Create a message passing channel and spawn the task loop
 - Write sbot-related task functions
 - Fetch root posts on subscription

### Libraries

The following libraries are introduced in this part:

 - [`async-std`](https://crates.io/crates/async-std)
 
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

Reviewing the code above: first an unbounded, asynchronous channel is created and split into transmitting (`tx`) and receiving (`rx`) ends, after which the transmitting channel is cloned. The task loop is then spawned and takes with it the receiving end of the channel. As we did previously with the `db` instance, the transmitting half of the channel is added to the managed state of the Rocket application; this will allow us to transmit tasks to the task loop from our web route handlers. And finaly, a shutdown handler is attached to the Rocket application in order to send a cancellation task to the task loop before the program ends. This ensures that the task loop closes cleanly.

### Write Sbot-Related Task Functions

Now it's time to write the functions that will be executed when the `FetchAllPosts` and `FetchLatestName` tasks are invoked. These functions will be responsible for retrieving data from the sbot and updating the database with the latest values:

`src/task_loop.rs`

```rust
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
```

### Fetch Root Posts on Subscription

### Conclusion

## Funding
