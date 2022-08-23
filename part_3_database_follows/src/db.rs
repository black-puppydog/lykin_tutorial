use std::path::Path;

use log::{debug, info};
use serde::{Deserialize, Serialize};
use sled::{Db, IVec, Result, Tree};

/// Scuttlebutt peer data.
#[derive(Debug, Deserialize, Serialize)]
pub struct Peer {
    pub public_key: String,
    pub name: String,
}

impl Peer {
    /// Create a new instance of the Peer struct using the given public
    /// key. Default values are set for latest_sequence and name.
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

/// An instance of the key-value database and relevant trees.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Database {
    /// The sled database instance.
    db: Db,
    /// A database tree containing Peer struct instances for all the peers
    /// we are subscribed to.
    peer_tree: Tree,
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

        Database { db, peer_tree }
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

    /// Remove a peer from the database, as represented by the given public
    /// key.
    pub fn remove_peer(&self, public_key: &str) -> Result<()> {
        debug!("Removing peer {} from 'peers' database tree", &public_key);
        self.peer_tree.remove(&public_key).map(|_| ())
    }
}
