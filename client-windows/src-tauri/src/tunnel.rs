use anyhow::{bail, Context};

use crate::profile::VpnProfile;

#[derive(Debug, Default)]
pub struct TunnelManager {
    active_profile_id: Option<String>,
    active_tunnel_name: Option<String>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(&mut self, profile: &VpnProfile) -> anyhow::Result<()> {
        if self.active_profile_id.as_deref() == Some(profile.id.as_str()) {
            return Ok(());
        }
        if self.active_profile_id.is_some() {
            self.disconnect().await?;
        }
        let tunnel_name = platform_connect(profile).await?;
        self.active_profile_id = Some(profile.id.clone());
        self.active_tunnel_name = Some(tunnel_name);
        Ok(())
    }

    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(tunnel_name) = self.active_tunnel_name.take() {
            platform_disconnect(&tunnel_name).await?;
            self.active_profile_id = None;
        }
        Ok(())
    }
}

#[cfg(windows)]
async fn platform_connect(profile: &VpnProfile) -> anyhow::Result<String> {
    use std::{fs, path::PathBuf};

    let tunnel_name = tunnel_name(profile);
    let config_path = wireguard_config_path(&tunnel_name)?;
    fs::write(&config_path, render_wireguard_config(profile))
        .with_context(|| format!("write tunnel config {}", config_path.display()))?;

    let status = tokio::process::Command::new("wireguard.exe")
        .arg("/installtunnelservice")
        .arg(&config_path)
        .status()
        .await
        .context("start wireguard.exe; install WireGuard for Windows or bundle it with the app")?;
    if !status.success() {
        bail!("WireGuard tunnel install failed for {tunnel_name}");
    }

    Ok(tunnel_name)
}

#[cfg(windows)]
fn render_wireguard_config(profile: &VpnProfile) -> String {
    let mut config = String::new();
    config.push_str("[Interface]\n");
    config.push_str(&format!("PrivateKey = {}\n", profile.private_key.trim()));
    config.push_str(&format!("Address = {}\n", profile.tunnel_address));
    config.push_str(&format!("MTU = {}\n", profile.mtu));
    if !profile.dns_servers.is_empty() {
        config.push_str(&format!("DNS = {}\n", profile.dns_servers.join(", ")));
    }
    config.push_str("\n[Peer]\n");
    config.push_str(&format!(
        "PublicKey = {}\n",
        profile.server_public_key.trim()
    ));
    if let Some(psk) = &profile.preshared_key {
        config.push_str(&format!("PresharedKey = {}\n", psk.trim()));
    }
    config.push_str(&format!("Endpoint = {}\n", profile.endpoint));
    config.push_str(&format!(
        "AllowedIPs = {}\n",
        profile.allowed_ips.join(", ")
    ));
    if profile.persistent_keepalive_seconds > 0 {
        config.push_str(&format!(
            "PersistentKeepalive = {}\n",
            profile.persistent_keepalive_seconds
        ));
    }
    config
}

#[cfg(windows)]
fn wireguard_config_path(tunnel_name: &str) -> anyhow::Result<PathBuf> {
    let mut path = dirs::data_local_dir().context("cannot resolve local app data directory")?;
    path.push("DDC");
    path.push("VPN");
    path.push("tunnels");
    std::fs::create_dir_all(&path)
        .with_context(|| format!("create tunnel config dir {}", path.display()))?;
    path.push(format!("{tunnel_name}.conf"));
    Ok(path)
}

#[cfg(windows)]
async fn platform_disconnect(tunnel_name: &str) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("wireguard.exe")
        .arg("/uninstalltunnelservice")
        .arg(tunnel_name)
        .status()
        .await
        .context("stop wireguard.exe tunnel service")?;
    if !status.success() {
        bail!("WireGuard tunnel uninstall failed for {tunnel_name}");
    }
    Ok(())
}

#[cfg(not(windows))]
async fn platform_connect(profile: &VpnProfile) -> anyhow::Result<String> {
    if profile.endpoint.is_empty() {
        bail!("endpoint is required");
    }
    Ok(tunnel_name(profile))
}

#[cfg(not(windows))]
async fn platform_disconnect(_tunnel_name: &str) -> anyhow::Result<()> {
    Ok(())
}

fn tunnel_name(profile: &VpnProfile) -> String {
    let suffix = profile.id.chars().take(8).collect::<String>();
    let mut name = format!("ddc-vpn-{suffix}");
    name.retain(|ch| ch.is_ascii_alphanumeric() || ch == '-');
    name
}
