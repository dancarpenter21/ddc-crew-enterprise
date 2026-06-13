# DDC VPN Windows Client

Desktop VPN client for `../svc-vpn`, built with Tauri, React, and a Rust backend.

## Current State

This repo now contains the Windows client shell and backend command surface:

- Profile CRUD persisted as TOML in the user's local app data directory.
- Tauri commands for `list_profiles`, `save_profile`, `delete_profile`, `connect`, `disconnect`, `status`, and `recent_logs`.
- React UI for profile editing, connection state, connect/disconnect, and recent logs.
- Tunnel lifecycle abstraction with Windows-specific hooks isolated in `src-tauri/src/tunnel.rs`.

The server in `../svc-vpn` still needs its WireGuard compatibility completed before this can be used as a production VPN. Until then, the client can manage profiles and exercise the lifecycle, but the real packet tunnel is not complete.

## Development

```sh
npm install
npm run build
npm run tauri dev
```

On Linux, Tauri's native WebKit/DBus development packages are required for Rust checks. On Windows, the Wintun implementation should be completed in `src-tauri/src/tunnel.rs` and run with administrator privileges so the app can create adapters, assign addresses, and install routes.
