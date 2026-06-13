use std::{collections::HashSet, fs, net::SocketAddr, path::Path};

use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use ipnet::IpNet;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub private_key: Key,
    pub listen: SocketAddr,
    pub interface: String,
    pub tunnel: IpNet,
    #[serde(default = "default_mtu")]
    pub mtu: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerConfig {
    pub name: String,
    pub public_key: Key,
    #[serde(default)]
    pub preshared_key: Option<Key>,
    #[serde(default)]
    pub allowed_ips: Vec<IpNet>,
    #[serde(default)]
    pub persistent_keepalive_seconds: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key([u8; 32]);

impl Key {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_key(&raw).map_err(serde::de::Error::custom)
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path.as_ref())?;
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server.interface.is_empty() || self.server.interface.len() > 15 {
            bail!("server.interface must be 1..=15 bytes for Linux IFNAMSIZ");
        }
        if self.server.mtu < 576 || self.server.mtu > 9000 {
            bail!("server.mtu must be between 576 and 9000");
        }
        if self.peers.is_empty() {
            bail!("at least one peer is required");
        }

        let mut names = HashSet::new();
        let mut public_keys = HashSet::new();
        for peer in &self.peers {
            if peer.name.trim().is_empty() {
                bail!("peer name cannot be empty");
            }
            if !names.insert(peer.name.clone()) {
                bail!("duplicate peer name {}", peer.name);
            }
            if !public_keys.insert(peer.public_key.clone()) {
                bail!("duplicate public key for peer {}", peer.name);
            }
            if peer.allowed_ips.is_empty() {
                bail!("peer {} must have at least one allowed IP", peer.name);
            }
        }
        Ok(())
    }
}

fn parse_key(raw: &str) -> anyhow::Result<Key> {
    let bytes = if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(raw).context("invalid hex key")?
    } else {
        STANDARD.decode(raw).context("invalid base64 key")?
    };

    if bytes.len() != 32 {
        bail!("key must decode to exactly 32 bytes");
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(Key(key))
}

fn default_mtu() -> u16 {
    1420
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_peer_names() {
        let config = toml::from_str::<Config>(
            r#"
            [server]
            private_key = "0000000000000000000000000000000000000000000000000000000000000001"
            listen = "0.0.0.0:51820"
            interface = "svcwg0"
            tunnel = "10.44.0.1/24"

            [[peers]]
            name = "alice"
            public_key = "0000000000000000000000000000000000000000000000000000000000000002"
            allowed_ips = ["10.44.0.2/32"]

            [[peers]]
            name = "alice"
            public_key = "0000000000000000000000000000000000000000000000000000000000000003"
            allowed_ips = ["10.44.0.3/32"]
            "#,
        )
        .unwrap();

        assert!(
            config
                .validate()
                .unwrap_err()
                .to_string()
                .contains("duplicate peer name")
        );
    }
}
