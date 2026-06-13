use std::{path::PathBuf, process::Stdio};

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

#[cfg(target_os = "linux")]
async fn platform_connect(profile: &VpnProfile) -> anyhow::Result<String> {
    ensure_linux_tunnel_support().await?;
    ensure_command("wg-quick").await?;
    let tunnel_name = tunnel_name(profile);
    let config_path = wireguard_config_path(&tunnel_name)?;
    std::fs::write(&config_path, render_wireguard_config(profile))
        .with_context(|| format!("write tunnel config {}", config_path.display()))?;
    restrict_config_permissions(&config_path)?;

    let status = privileged_command("wg-quick")
        .arg("up")
        .arg(&config_path)
        .status()
        .await
        .context("start wg-quick; install wireguard-tools and run the client with privileges")?;
    if !status.success() {
        bail!("wg-quick up failed for {tunnel_name}");
    }

    Ok(tunnel_name)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
async fn platform_disconnect(tunnel_name: &str) -> anyhow::Result<()> {
    let config_path = wireguard_config_path(tunnel_name)?;
    let status = privileged_command("wg-quick")
        .arg("down")
        .arg(&config_path)
        .status()
        .await
        .context("stop wg-quick tunnel")?;
    if !status.success() {
        bail!("wg-quick down failed for {tunnel_name}");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn restrict_config_permissions(path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .with_context(|| format!("read permissions for {}", path.display()))?
        .permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)
        .with_context(|| format!("restrict permissions for {}", path.display()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn ensure_linux_tunnel_support() -> anyhow::Result<()> {
    if !std::path::Path::new("/dev/net/tun").exists() {
        if is_wsl() {
            bail!(
                "/dev/net/tun is missing. In WSL, enable WSL 2 with a kernel that includes TUN/WireGuard support, then start the client from that distribution with sudo."
            );
        }
        bail!("/dev/net/tun is missing; load the tun module and run the client with CAP_NET_ADMIN or sudo");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn ensure_command(command: &str) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .with_context(|| format!("check for {command}"))?;
    if !status.success() {
        bail!("{command} is required; install wireguard-tools");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn privileged_command(program: &str) -> tokio::process::Command {
    let helper = std::env::var("DDC_VPN_PRIVILEGE_HELPER").unwrap_or_else(|_| "sudo".to_string());
    if is_effective_root() {
        tokio::process::Command::new(program)
    } else if helper == "sudo" {
        let mut command = tokio::process::Command::new("sudo");
        command.arg("-n").arg(program);
        command
    } else {
        let mut command = tokio::process::Command::new(helper);
        command.arg(program);
        command
    }
}

#[cfg(target_os = "linux")]
fn is_effective_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|value| value.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
async fn platform_connect(profile: &VpnProfile) -> anyhow::Result<String> {
    if profile.endpoint.is_empty() {
        bail!("endpoint is required");
    }
    Ok(tunnel_name(profile))
}

#[cfg(not(target_os = "linux"))]
async fn platform_disconnect(_tunnel_name: &str) -> anyhow::Result<()> {
    Ok(())
}

fn tunnel_name(profile: &VpnProfile) -> String {
    let suffix = profile.id.chars().take(8).collect::<String>();
    let mut name = format!("ddc-vpn-{suffix}");
    name.retain(|ch| ch.is_ascii_alphanumeric() || ch == '-');
    name
}
