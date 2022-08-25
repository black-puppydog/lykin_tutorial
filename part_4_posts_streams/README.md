# lykin tutorial

## Part 4: Posts and Message Streams

### Introduction

In the last installment we completed the subscribe / unsubscribe flow of our application while learning some new Scuttlebutt RPC methods (`is_following()`, `follow()`, `unfollow()` and `get_name()`) and creating a simple key-value database. Today we'll extend our database by adding the ability to store Scuttlebutt post-type messages. We'll also learn how to create a stream of all Scuttlebutt messages authored by a peer and how to filter those messages by type. The work we do in this installment will pave the way for populating our key-value database with posts made by the peers we subscribe to. Let's get into it!

### Outline

Here's what we'll tackle in this fourth part of the series:

 - Create a post data structure
 - Create a stream of Scuttlebutt messages
 - Filter post-type Scuttlebutt messages
 - Initialise a database tree for posts
 - Add a post to the database
 - Add a post batch to the database

### Create a Post Data Structure

We'll begin by creating a `Post` struct to store data about each Scuttlebutt post we want to render in our application. The fields of our struct will diverge from the fields we expect in a Scuttlebutt post-type message. Open `src/db.rs` and add the following code (I've included code comments to further define each field):

```rust
// The text and metadata of a Scuttlebutt root post.
#[derive(Debug, Deserialize, Serialize)]
pub struct Post {
    // The key of the post-type message, also known as a message reference.
    pub key: String,
    // The text of the post (may be formatted as markdown).
    pub text: String,
    // The date the post was published (e.g. 17 May 2021).
    pub date: String,
    // The sequence number of the post-type message.
    pub sequence: u64,
    // The read state of the post; true if read, false if unread.
    pub read: bool,
    // The timestamp representing the date the post was published.
    pub timestamp: i64,
    // The subject of the post, represented as the first 53 characters of
    // the post text.
    pub subject: Option<String>,
}
```

Note that the fields of our `Post` struct diverge from the fields of a Scuttlebutt message. Here's a post-type message from the Scuttlebutt Protocol Guide for comparison:

```json
{
  "previous": "%XphMUkWQtomKjXQvFGfsGYpt69sgEY7Y4Vou9cEuJho=.sha256",
  "author": "@FCX/tsDLpubCPKKfIrw4gc+SQkHcaD17s7GI6i/ziWY=.ed25519",
  "sequence": 2,
  "timestamp": 1514517078157,
  "hash": "sha256",
  "content": {
    "type": "post",
    "text": "Second post!"
  }
}
```

The struct we've implemented ignores the `hash` and `previous` fields while adding others that are necessary for our application (for example, the `read` and `subject` fields).

Now we can implement a `new()` method for our `Post` struct:

```rust
impl Post {
    // Create a new instance of the Post struct. A default value of `false` is
    // set for `read`.
    pub fn new(
        key: String,
        text: String,
        date: String,
        sequence: u64,
        timestamp: i64,
        subject: Option<String>,
    ) -> Post {
        Post {
            key,
            text,
            date,
            sequence,
            timestamp,
            subject,
            read: false,
        }
    }
}
```

### Create a Stream of Scuttlebutt Messages

We're going to step away from database concerns for a moment to focus on obtaining Scuttlebutt messages and filtering them. In this section we'll demonstrate the `create_history_stream()` RPC method, one of the foundational methods in Scuttlebutt development, which takes a Scuttlebutt peer ID and returns a stream of messages authored by that peer.

Let's write a `get_message_stream()` function that will take the key of the Scuttlebutt peer we're interested in, along with a sequence number (this will come in handy later). In the function we'll initialise a connection to the sbot, define the arguments for the RPC call and then make the `create_history_stream()` call - returning the stream to the caller. Calling this function with a `sequence_number` of 0 would return a stream of every message ever authored by the given public key.

`src/sbot.rs`

```rust
// Return a stream of messages authored by the given public key.
//
// This returns all messages regardless of type.
pub async fn get_message_stream(
    public_key: &str,
    sequence_number: u64,
) -> impl futures::Stream<Item = Result<SsbMessageKVT, GolgiError>> {
    let mut sbot = init_sbot().await.unwrap();

    let history_stream_args = CreateHistoryStream::new(public_key.to_string())
				// Define the shape of the returned messages: defining `keys_values`
				// as `(true, true)` will result in messages being returned as KVTs. KVT
				// stands for Key Value Timestamp. The Key is the message ID and the Value
				// contains the actual data of the message (including fields such as
				// `author`, `previous`, `hash` etc.).
        .keys_values(true, true)
				// Define the starting point of the message stream. In other words,
				// only return messages starting after the given sequence number.
        .after_seq(sequence_number);

    sbot.create_history_stream(history_stream_args)
        .await
        .unwrap()
}
```

### Filter Post-Type Scuttlebutt Messages

Now that we have the ability to obtain all messages authored by a specific peer, we need a way to filter those messages and extract only the root posts (our application isn't concerned with replies to posts). The function we'll write to perform this task might appear intimidating but we're simply iterating over a stream of messages, filtering for post-type messages with a `root` field and extracting the data we need for our application. A vector of `Post` is returned, using the struct we defined at the beginning of this installment of the tutorial.

`src/sbot.rs`

```rust
// Filter a stream of messages and return a vector of root posts.
pub async fn get_root_posts(
    history_stream: impl futures::Stream<Item = Result<SsbMessageKVT, GolgiError>>,
) -> (u64, Vec<Post>) {
    let mut latest_sequence = 0;
    let mut posts = Vec::new();

    futures::pin_mut!(history_stream);

    while let Some(res) = history_stream.next().await {
        match res {
            Ok(msg) => {
								// Filter by content type to only select post-type messages.
                if msg.value.is_message_type(SsbMessageContentType::Post) {
                    let content = msg.value.content.to_owned();
                    if let Value::Object(content_map) = content {
												// If the content JSON object contains a key-value pair
												// with a key of `root` this indicates the message
												// is a reply to another message. The value of the `root`
												// key is the message ID of the message being replied to.
												// In our case, since we only want root posts, we ignore
												// any message with a `root` field.
                        if !content_map.contains_key("root") {
                            latest_sequence = msg.value.sequence;

                            let text = content_map.get_key_value("text").unwrap().1.to_string();
                            let timestamp = msg.value.timestamp.round() as i64 / 1000;
                            let datetime = NaiveDateTime::from_timestamp(timestamp, 0);
                            let date = datetime.format("%d %b %Y").to_string();
														// Copy the beginning of the post text to serve as the
														// subject (for display in the UI).
                            let subject = text.get(0..52).map(|s| s.to_string());

                            let post = Post::new(
                                msg.key.to_owned(),
                                text,
                                date,
                                msg.value.sequence,
                                timestamp,
                                subject,
                            );

                            posts.push(post)
                        }
                    }
                }
            }
            Err(err) => {
                // Print the `GolgiError` of this element to `stderr`.
                warn!("err: {:?}", err);
            }
        }
    }

    (latest_sequence, posts)
}
```

### Initialise a Database Tree for Posts

You may recall creating a database tree for peers in the previous tutorial installment. Now we can add a tree to store posts, first by updating our `Database` struct and then by opening the tree on our database instance.

`src/db.rs`

```rust
// An instance of the key-value database and relevant trees.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Database {
    // The sled database instance.
    db: Db,
    // A database tree containing Peer struct instances for all the peers
    // we are subscribed to.
    peer_tree: Tree,
    // A database tree containing Post struct instances for all of the posts
    // we have downloaded from the peer to whom we subscribe.
    pub post_tree: Tree,
}

impl Database {
    // Initialise the database by opening the database file, loading the
    // peers and posts trees and returning an instantiated Database struct.
    pub fn init(path: &Path) -> Self {
        // Open the database at the given path.
        // The database will be created if it does not yet exist.
        // This code will panic if an IO error is encountered.
        info!("Initialising sled database");
        let db = sled::open(path).expect("Failed to open database");
        debug!("Opening 'peers' database tree");
        let peer_tree = db
            .open_tree("peers")
            .expect("Failed to open 'peers' database tree");
        debug!("Opening 'posts' database tree");
        let post_tree = db
            .open_tree("posts")
            .expect("Failed to open 'posts' database tree");

        Database {
            db,
            peer_tree,
            post_tree,
        }
    }

```

Don't forget to delete the previous instance of the database on file before attempting to compile and execute this code (database migrations are out of the scope of this tutorial, hence the heavy-handed approach). If you're on Linux it'll likely be at `~/.config/lykin/database`.

### Add a Post to the Database

The last things we'll tackle in this installment are the methods required to add posts to the database. The process is very similar to the one we employed for adding peers to the database.

`src/db.rs`

```rust
impl Database {
    // ...

    // Add a post to the database by inserting an instance of the Post struct
    // into the post tree. The key of the entry is formed by concatenating
    // the public key of the peer who authored the post and the key of the
    // post itself, separated by an underscore. The Post is serialized as
    // bincode before the database entry is inserted.
    //
    // This method can also be used to update an existing database entry.
    pub fn add_post(&self, public_key: &str, post: Post) -> Result<Option<IVec>> {
        let post_key = format!("{}_{}", public_key, post.key);
        debug!("Serializing post data for {} to bincode", &post_key);
        let post_bytes = bincode::serialize(&post).unwrap();

        debug!("Inserting post {} into 'posts' database tree", &post_key);
        self.post_tree.insert(post_key.as_bytes(), post_bytes)
    }
}
```

Notice the `post_key` variable in the code above: the concatenation of the public key of the peer who authored the post and the post key (aka. message ID or message reference). Here's an example:

`@HEqy940T6uB+T+d9Jaa58aNfRzLx9eRWqkZljBmnkmk=.ed25519_%AbEupzW67huP6LUNO2CAhkK2RNCeUsmbPAP7rgCi3HY=.sha256`

Using this approach will allow us to retrieve all the posts by a given public key from our posts database tree by using the public key as the prefix with which to filter entries.

### Add a Post Batch to the Database

On most occasions we'll find ourselves in a situation where we wish to add multiple posts by a single author to the database. The sled database we're using has an `apply_batch()` method to apply atomic updates. Let's write an `add_post_batch` for our `Database` implementation:

`src/db.rs`

```rust
impl Database {
    // ...

    // Add a batch of posts to the database by inserting a vector of instances
    // of the Post struct into the post tree. The key of each entry is formed
    // by concatenating the public key of the peer who authored the post and
    // the key of the post itself, separated by an underscore. Each Post is
    // serialized as bincode before the database entry is inserted.
    pub fn add_post_batch(&self, public_key: &str, posts: Vec<Post>) -> Result<()> {
        let mut post_batch = Batch::default();

        for post in posts {
            let post_key = format!("{}_{}", public_key, post.key);
            debug!("Serializing post data for {} to bincode", &post_key);
            let post_bytes = bincode::serialize(&post).unwrap();

            debug!("Inserting post {} into 'posts' database tree", &post_key);
            post_batch.insert(post_key.as_bytes(), post_bytes)
        }

        debug!("Applying batch insertion into 'posts' database tree");
        self.post_tree.apply_batch(post_batch)
    }
}
```

### Conclusion

In this installment we learned how to request a stream of Scuttlebutt messages from an sbot and how to filter those messages by type. We added a `Post` data structure, a database tree to contain posts and methods to add posts to the database. All of this work was done in preparation for the next step in the development of our application: fetching the posts of each peer we subscribe to and storing them in the database whenever a subscription event occurs.

In the next tutorial installment we'll write an asynchronous task loop to run background processes. We'll put the task loop into action by invoking a post-fetching and filtering task from our subscription route handler, powered by the code we wrote today. 

## Funding

This work has been funded by a Scuttlebutt Community Grant.
