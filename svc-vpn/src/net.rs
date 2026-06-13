use std::net::SocketAddr;

use tokio::net::UdpSocket;

#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> anyhow::Result<Self> {
        Ok(Self {
            socket: UdpSocket::bind(addr).await?,
        })
    }

    pub async fn recv_from(&self, buffer: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buffer).await
    }

    pub async fn send_to(&self, packet: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.socket.send_to(packet, addr).await
    }
}
