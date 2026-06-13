# DDC VPN Linux Client

Desktop VPN client for `../svc-vpn`, built with Tauri, React, and a Rust backend.

## Current State

This repo contains the Linux client shell and backend command surface:

- Profile CRUD persisted as TOML in the user's local app data directory.
- Tauri commands for `list_profiles`, `save_profile`, `delete_profile`, `connect`, `disconnect`, `status`, and `recent_logs`.
- React UI for profile editing, connection state, connect/disconnect, and recent logs.
- Linux tunnel lifecycle implementation that renders a WireGuard config and calls `wg-quick up` / `wg-quick down`.
- WSL-aware startup checks for `/dev/net/tun` so missing kernel/TUN support is reported clearly.

The server in `../svc-vpn` is still marked as an MVP scaffold. Do not use this as a production VPN until the server protocol implementation is complete, interoperable with standard WireGuard clients, and externally reviewed.

## Runtime Requirements

Install WireGuard userspace tools:

```sh
sudo apt install wireguard-tools
```

The app needs permission to create a tunnel interface and routes. The simplest development path is to start the Tauri app with sudo:

```sh
sudo -E npm run tauri dev
```

By default the backend runs `sudo -n wg-quick ...` when it is not already root. That means password prompts are intentionally avoided inside the app. To use a desktop prompt helper instead, set:

```sh
DDC_VPN_PRIVILEGE_HELPER=pkexec npm run tauri dev
```

## WSL

This client can run inside WSL 2 when the distribution has `/dev/net/tun` and WireGuard kernel support. If `/dev/net/tun` is missing, update WSL and use a WSL 2 distribution with TUN support before connecting.

Because WSL networking is NATed through Windows, a tunnel created inside WSL primarily affects Linux processes in that distribution. It is not a replacement for a native Windows system-wide VPN client.

## Development

```sh
npm install
npm run build
npm run tauri dev
```

On Linux, Tauri's native WebKit/DBus development packages are required for Rust checks and local app runs.
