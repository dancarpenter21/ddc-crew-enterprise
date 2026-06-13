use std::str::FromStr;

use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnProfile {
    pub id: String,
    pub name: String,
    pub private_key: String,
    pub server_public_key: String,
    #[serde(default)]
    pub preshared_key: Option<String>,
    pub endpoint: String,
    pub tunnel_address: String,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
    #[serde(default)]
    pub dns_servers: Vec<String>,
    pub mtu: u16,
    pub persistent_keepalive_seconds: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnStatus {
    pub state: ConnectionState,
    pub active_profile_id: Option<String>,
    pub active_profile_name: Option<String>,
    pub endpoint: Option<String>,
    pub tunnel_address: Option<String>,
    pub last_error: Option<String>,
}

impl Default for VpnStatus {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            active_profile_id: None,
            active_profile_name: None,
            endpoint: None,
            tunnel_address: None,
            last_error: None,
        }
    }
}

impl VpnStatus {
    pub fn connecting(profile: &VpnProfile) -> Self {
        Self::from_profile(ConnectionState::Connecting, profile, None)
    }

    pub fn connected(profile: &VpnProfile) -> Self {
        Self::from_profile(ConnectionState::Connected, profile, None)
    }

    pub fn failed(profile: &VpnProfile, error: String) -> Self {
        Self::from_profile(ConnectionState::Failed, profile, Some(error))
    }

    fn from_profile(
        state: ConnectionState,
        profile: &VpnProfile,
        last_error: Option<String>,
    ) -> Self {
        Self {
            state,
            active_profile_id: Some(profile.id.clone()),
            active_profile_name: Some(profile.name.clone()),
            endpoint: Some(profile.endpoint.clone()),
            tunnel_address: Some(profile.tunnel_address.clone()),
            last_error,
        }
    }
}

impl VpnProfile {
    pub fn normalize(mut self) -> anyhow::Result<Self> {
        if self.id.trim().is_empty() {
            self.id = Uuid::new_v4().to_string();
        }
        self.name = self.name.trim().to_string();
        self.endpoint = self.endpoint.trim().to_string();
        self.tunnel_address = self.tunnel_address.trim().to_string();
        self.allowed_ips = self
            .allowed_ips
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        self.dns_servers = self
            .dns_servers
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        self.preshared_key = self
            .preshared_key
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            bail!("profile name is required");
        }
        parse_key(&self.private_key).context("invalid private key")?;
        parse_key(&self.server_public_key).context("invalid server public key")?;
        if let Some(key) = &self.preshared_key {
            parse_key(key).context("invalid preshared key")?;
        }
        validate_endpoint(&self.endpoint)?;
        IpNet::from_str(&self.tunnel_address).context("invalid tunnel address")?;
        if self.allowed_ips.is_empty() {
            bail!("at least one allowed IP is required");
        }
        for allowed_ip in &self.allowed_ips {
            IpNet::from_str(allowed_ip)
                .with_context(|| format!("invalid allowed IP {allowed_ip}"))?;
        }
        for dns_server in &self.dns_servers {
            dns_server
                .parse::<std::net::IpAddr>()
                .with_context(|| format!("invalid DNS server {dns_server}"))?;
        }
        if !(576..=9000).contains(&self.mtu) {
            bail!("MTU must be between 576 and 9000");
        }
        Ok(())
    }
}

pub fn parse_key(raw: &str) -> anyhow::Result<[u8; 32]> {
    let raw = raw.trim();
    let bytes = if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(raw).context("invalid hex key")?
    } else {
        STANDARD.decode(raw).context("invalid base64 key")?
    };

    if bytes.len() != 32 {
        bail!("key must decode to exactly 32 bytes");
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn validate_endpoint(endpoint: &str) -> anyhow::Result<()> {
    let (host, port) = endpoint
        .rsplit_once(':')
        .context("endpoint must include a port")?;
    if host.trim().is_empty() {
        bail!("endpoint host is required");
    }
    let port = port
        .parse::<u16>()
        .context("endpoint port must be a valid TCP/UDP port")?;
    if port == 0 {
        bail!("endpoint port must be non-zero");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> VpnProfile {
        VpnProfile {
            id: String::new(),
            name: "test".to_string(),
            private_key: "0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(),
            server_public_key: "0000000000000000000000000000000000000000000000000000000000000002"
                .to_string(),
            preshared_key: None,
            endpoint: "127.0.0.1:51820".to_string(),
            tunnel_address: "10.44.0.2/32".to_string(),
            allowed_ips: vec!["10.44.0.0/24".to_string()],
            dns_servers: vec!["1.1.1.1".to_string()],
            mtu: 1420,
            persistent_keepalive_seconds: 25,
        }
    }

    #[test]
    fn validates_profile_and_assigns_id() {
        assert!(!profile().normalize().unwrap().id.is_empty());
    }

    #[test]
    fn rejects_invalid_key() {
        let mut profile = profile();
        profile.private_key = "not-a-key".to_string();
        assert!(profile.normalize().is_err());
    }

    #[test]
    fn accepts_dns_endpoint() {
        let mut profile = profile();
        profile.endpoint = "vpn.example.com:51820".to_string();
        assert!(profile.normalize().is_ok());
    }
}
