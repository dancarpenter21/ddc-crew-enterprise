use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use svc_vpn::{config::Config, service::Service};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(author, version, about = "Userspace Rust VPN server")]
struct Args {
    /// Path to the svc-vpn TOML configuration.
    #[arg(short, long, env = "SVC_VPN_CONFIG", default_value = "svc-vpn.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)
        .with_context(|| format!("failed to load config {}", args.config.display()))?;

    Service::new(config).run().await
}
