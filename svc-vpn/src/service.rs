use std::sync::Arc;

use anyhow::Context;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::{
    config::Config,
    device::TunDevice,
    net::UdpTransport,
    peer::Peer,
    protocol::{WireGuardEngine, parse_ipv4_destination},
    routing::{AllowedIps, Route},
};

pub struct Service {
    config: Config,
}

impl Service {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        info!(
            listen = %self.config.server.listen,
            interface = %self.config.server.interface,
            "starting svc-vpn"
        );

        let tun = Arc::new(TunDevice::open(&self.config.server.interface)?);
        let udp = Arc::new(UdpTransport::bind(self.config.server.listen).await?);

        let engine = Arc::new(Mutex::new(WireGuardEngine::from_config(&self.config)?));

        let mut routes = Vec::new();
        let peers = self
            .config
            .peers
            .into_iter()
            .enumerate()
            .map(|(peer_index, peer_config)| {
                for cidr in &peer_config.allowed_ips {
                    routes.push(Route {
                        peer_index,
                        cidr: *cidr,
                    });
                }
                Peer::from_config(peer_config)
            })
            .collect::<Vec<_>>();

        let routing = Arc::new(AllowedIps::new(routes));
        let peers = Arc::new(Mutex::new(peers));

        let udp_task = tokio::spawn(read_udp_loop(
            Arc::clone(&udp),
            Arc::clone(&tun),
            Arc::clone(&engine),
        ));
        let tun_task = tokio::spawn(read_tun_loop(
            Arc::clone(&tun),
            Arc::clone(&udp),
            Arc::clone(&peers),
            Arc::clone(&routing),
            Arc::clone(&engine),
        ));

        tokio::select! {
            result = udp_task => result.context("UDP task join failure")??,
            result = tun_task => result.context("TUN task join failure")??,
            _ = shutdown_signal() => info!("shutdown signal received"),
        }

        Ok(())
    }
}

async fn read_udp_loop(
    udp: Arc<UdpTransport>,
    tun: Arc<TunDevice>,
    engine: Arc<Mutex<WireGuardEngine>>,
) -> anyhow::Result<()> {
    let mut buffer = vec![0u8; 65535];
    loop {
        let (len, from) = udp.recv_from(&mut buffer).await?;
        debug!(bytes = len, %from, "received UDP packet");
        let events = engine.lock().await.handle_udp(&buffer[..len], from)?;
        for outbound in events.udp {
            udp.send_to(&outbound.bytes, outbound.destination).await?;
        }
        for packet in events.tun {
            tun.write_packet(&packet.bytes).await?;
        }
    }
}

async fn read_tun_loop(
    tun: Arc<TunDevice>,
    udp: Arc<UdpTransport>,
    peers: Arc<Mutex<Vec<Peer>>>,
    routing: Arc<AllowedIps>,
    engine: Arc<Mutex<WireGuardEngine>>,
) -> anyhow::Result<()> {
    let mut buffer = vec![0u8; 65535];
    loop {
        let len = tun.read_packet(&mut buffer).await?;
        let destination = parse_ipv4_destination(&buffer[..len])?;
        let Some(peer_index) = routing.lookup(destination) else {
            warn!(%destination, "dropping packet with no matching allowed IP");
            continue;
        };

        match engine.lock().await.handle_tun(peer_index, &buffer[..len]) {
            Ok(events) => {
                for outbound in events.udp {
                    udp.send_to(&outbound.bytes, outbound.destination).await?;
                }
            }
            Err(error) => {
                let peers = peers.lock().await;
                let peer_name = peers
                    .get(peer_index)
                    .map(|peer| peer.name.as_str())
                    .unwrap_or("<unknown>");
                warn!(peer_index, peer_name, %error, "dropping outbound tunnel packet");
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
