use std::net::SocketAddr;

use crate::config::{Key, PeerConfig};

#[derive(Debug, Clone)]
pub struct Peer {
    pub name: String,
    pub public_key: Key,
    pub preshared_key: Option<Key>,
    pub endpoint: Option<SocketAddr>,
    pub persistent_keepalive_seconds: Option<u16>,
}

impl Peer {
    pub fn from_config(config: PeerConfig) -> Self {
        Self {
            name: config.name,
            public_key: config.public_key,
            preshared_key: config.preshared_key,
            endpoint: None,
            persistent_keepalive_seconds: config.persistent_keepalive_seconds,
        }
    }
}
