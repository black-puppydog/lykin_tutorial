# lykin tutorial

## Part 7: Latest Posts and Names

### Introduction

In the last tutorial installment we updated the user interface of our application and added the ability to display a list of peer subscriptions. Today we'll turn our attention to staying up-to-date with the latest posts authored by the peers we subscribe to. In doing so, we'll add the ability to keep track of the latest sequence number for each peer we follow - as well as syncing only the latest posts for each peer.

This installment will be a short one, since much of the groundwork has already been done in previous installments.

### Outline

 - Update the database to store the latest sequence number
 - Update the sequence number when fetching posts
 - Add a task to fetch the latest posts
 - Add a route handler to invoke the `FetchLatestPosts` task
 - Update the navigation template

### Update the Database to Store the Latest Sequence Number

The main objective of this tutorial installment is to be able to request only the latest messages for each peer we subscribe to from the sbot. In order to do so, we need to know the sequence number of the most recently published message already in our key-value store. With that information, we can say to the sbot: "please give me all messages for peer X with sequence number greater than Y".

We're going to add a `latest_sequence` field to the `Peer` struct in our database code, as well as a method for updating that value:

`src/db.rs`

```rust
/// Scuttlebutt peer data.
#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub public_key: String,
    pub name: String,
    pub latest_sequence: u64,
}

impl Peer {
    pub fn new(public_key: &str) -> Peer {
        Peer {
            public_key: public_key.to_string(),
            name: "".to_string(),
            // Set the value of latest_sequence to 0.
            latest_sequence: 0,
        }
    }

    // ...

    // Modify the latest_sequence field of an instance of the Peer struct,
    // leaving the other values unchanged.
    pub fn set_latest_sequence(self, latest_sequence: u64) -> Peer {
        Self {
            latest_sequence,
            ..self
        }
    }
}
```

### Update the Sequence Number When Fetching Posts

Now that we have a way to store and update the latest sequence number for each peer in our database, we need to update our post-fetching function in the task loop accordingly.

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
```

### Add a Task to Fetch the Latest Posts

We already have a `FetchAllPosts` variant of the `Task` enum in our task loop. Let's add a `FetchLatestPosts` variant, along with a match statement and sbot-related function call: 

`src/task_loop.rs`

```rust
pub enum Task {
    Cancel,
    FetchAllPosts(String),
    FetchLatestPosts(String),
    FetchLatestName(String),
}

// Spawn an asynchronous loop which receives tasks over an unbounded channel
// and invokes task functions accordingly.
pub async fn spawn(db: Database, rx: Receiver<Task>) {
    task::spawn(async move {
        while let Ok(task) = rx.recv().await {
            match task {
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
                // ...
            }
        }
    }
}
```

You'll notice that the same function (`fetch_posts_and_update_db()`) is called by both the `FetchAllPosts` and `FetchLatestPosts` tasks; the difference is the value passed in for the third parameter: `after_sequence`. When fetching all posts we pass in a value of 0, while the value of `peer.latest_sequence` is passed when fetching only the latest posts. This relatively simple addition to our code has provided a very efficient means of syncing the latest posts from our local go-sbot instance to the key-value database.

### Add a Route Handler to Invoke the FetchLatestPosts Task

Now we can begin exposing a means for the user to invoke the `FetchLatestPosts` task. This will be done by clicking an icon on the navigation bar of the web interface. Once clicked, a GET request will be sent to `/posts/download_latest`. Let's write the route handler to accept the request and invoke the task for each peer we're subscribed to.

`src/routes.rs`

```rust
#[get("/posts/download_latest")]
pub async fn download_latest_posts(db: &State<Database>, tx: &State<Sender<Task>>) -> Redirect {
    // Iterate through the list of peers in the key-value database.
    // These are all the peers we're subscribed to via lykin.
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
```

You'll notice in the code above that we also invoke the `FetchLatestName` task for each peer. This ensures that our application stays up-to-date with the ways our peers have chosen to name themselves.

Now we need to mount the `download_latest_posts` route to our Rocket application:

`src/main.rs`

```rust
#[launch]
async fn rocket() -> _ {
    // ...

    rocket::build()
        .manage(db)
        .manage(tx)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                home,
                subscribe_form,
                unsubscribe_form,
                // Here we add the route we just wrote.
                download_latest_posts
            ],
        )
        .mount("/", FileServer::from(relative!("static")))
        .attach(AdHoc::on_shutdown("cancel task loop", |_| {
            Box::pin(async move {
                tx_clone.send(Task::Cancel).await.unwrap();
            })
        }))
}
```

### Update the Navigation Template

We need to remove the `disabled` and `icon` classes from the 'Download latest posts' anchor element and add an `href` tag. Once this change has been made, clicking on the download icon will fetch the latest posts for all the peers we're subscribed to.

`templates/topbar.html.tera`

```html
<div class="nav">
  <div class="flex-container">
    <a href="/posts/download_latest" title="Download latest posts">
      <img src="/icons/download.png">
    </a>
    <!-- ... -->
  </div>
</div>
```

### Conclusion

That marks the conclusion of a relatively short installment in which we added the ability to keep our key-value database up-to-date with the latest posts and name assignments published by the peers we subscribe to. We updated the database to be able to track the latest sequence number for each peer and added a task to fetch all posts with a sequence number greater than that which is stored. We then added a route handler to invoke the task for each peer and wired it up to the download icon in the navigation bar of our UI.

In the next installment we'll write more route handlers and update our templates in order to show a list of posts each peer has made. We'll also add the ability to display the content of each post. We are on the cusp of realising the fruits of our labour!

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.

