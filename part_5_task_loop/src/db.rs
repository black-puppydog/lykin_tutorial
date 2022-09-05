use std::path::Path;

use log::{debug, info};
use serde::{Deserialize, Serialize};
use sled::{Batch, Db, IVec, Result, Tree};

/// Scuttlebutt peer data.
#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub public_key: String,
    pub name: String,
}

impl Peer {
    /// Create a new instance of the Peer struct using the given public
    /// key. A default value is set for name.
    pub fn new(public_key: &str) -> Peer {
        Peer {
            public_key: public_key.to_string(),
            name: "".to_string(),
        }
    }

    /// Modify the name field of an instance of the Peer struct, leaving
    /// the other values unchanged.
    pub fn set_name(self, name: &str) -> Peer {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

/// The text and metadata of a Scuttlebutt root post.
#[derive(Debug, Deserialize, Serialize)]
pub struct Post {
    /// The key of the post-type message, also known as a message reference.
    pub key: String,
    /// The text of the post (may be formatted as markdown).
    pub text: String,
    /// The date the post was published (e.g. 17 May 2021).
    pub date: String,
    /// The sequence number of the post-type message.
    pub sequence: u64,
    /// The read state of the post; true if read, false if unread.
    pub read: bool,
    /// The timestamp representing the date the post was published.
    pub timestamp: i64,
    /// The subject of the post, represented as the first 53 characters of
    /// the post text.
    pub subject: Option<String>,
}

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

/// An instance of the key-value database and relevant trees.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Database {
    /// The sled database instance.
    db: Db,
    /// A database tree containing Peer struct instances for all the peers
    /// we are subscribed to.
    peer_tree: Tree,
    /// A database tree containing Post struct instances for all of the posts
    /// we have downloaded from the peer to whom we subscribe.
    pub post_tree: Tree,
}

impl Database {
    /// Initialise the database by opening the database file, loading the
    /// peers tree and returning an instantiated Database struct.
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

    /// Add a peer to the database by inserting the public key into the peer
    /// tree.
    pub fn add_peer(&self, peer: Peer) -> Result<Option<IVec>> {
        debug!("Serializing peer data for {} to bincode", &peer.public_key);
        let peer_bytes = bincode::serialize(&peer).unwrap();

        debug!(
            "Inserting peer {} into 'peers' database tree",
            &peer.public_key
        );
        self.peer_tree.insert(&peer.public_key, peer_bytes)
    }

    /// Get a single peer from the peer tree, defined by the given public key.
    /// The byte value for the matching entry, if found, is deserialized from
    /// bincode into an instance of the Peer struct.
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

    /// Remove a peer from the database, as represented by the given public
    /// key.
    pub fn remove_peer(&self, public_key: &str) -> Result<()> {
        debug!("Removing peer {} from 'peers' database tree", &public_key);
        self.peer_tree.remove(&public_key).map(|_| ())
    }

    /// Add a post to the database by inserting an instance of the Post struct
    /// into the post tree.
    pub fn add_post(&self, public_key: &str, post: Post) -> Result<Option<IVec>> {
        let post_key = format!("{}_{}", public_key, post.key);
        debug!("Serializing post data for {} to bincode", &post_key);
        let post_bytes = bincode::serialize(&post).unwrap();

        debug!("Inserting post {} into 'posts' database tree", &post_key);
        self.post_tree.insert(post_key.as_bytes(), post_bytes)
    }

    /// Add a batch of posts to the database by inserting a vector of instances
    /// of the Post struct into the post tree.
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
