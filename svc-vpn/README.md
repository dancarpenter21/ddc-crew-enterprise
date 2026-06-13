# svc-vpn

`svc-vpn` is a Rust userspace VPN server scaffold targeting a Linux,
WireGuard-compatible service. The current implementation provides the service
shape, configuration, TUN/UDP plumbing, peer routing, and protocol test
scaffolding.

Important: the cryptographic WireGuard handshake and transport encryption are
not complete yet. Do not expose this service as a production VPN until the
protocol engine is completed, interoperable with standard WireGuard clients,
and externally reviewed.

## Run

```sh
cargo run -- --config examples/config.toml
```

The process needs Linux TUN access and typically these capabilities:

```sh
sudo setcap cap_net_admin,cap_net_raw+ep target/release/svc-vpn
```

## Configuration

See `examples/config.toml`.

Keys may be 32-byte hex strings or standard base64-encoded 32-byte values.

## Networking

The service opens `/dev/net/tun`, binds the configured UDP listen address, and
routes plaintext tunnel packets to peers using longest-prefix match over
`allowed_ips`.

Interface address assignment, forwarding, and NAT are intentionally left to the
host for now. A typical Linux gateway also needs IP forwarding and firewall/NAT
rules for the tunnel CIDR.
