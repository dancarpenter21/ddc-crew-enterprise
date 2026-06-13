# ddc-crew-enterprise

Enterprise VPN client and service projects for the DDC crew family controls stack.

## Sub-projects

| Project | Description | README |
| --- | --- | --- |
| `svc-vpn` | Rust userspace VPN server scaffold targeting a Linux, WireGuard-compatible service. It includes configuration loading, TUN/UDP plumbing, peer routing, and protocol test scaffolding. The WireGuard cryptographic handshake and transport encryption are not production-complete yet. | [`svc-vpn/README.md`](svc-vpn/README.md) |
| `client-linux` | Linux desktop VPN client built with Tauri, React, TypeScript, and Rust. It manages profiles, renders WireGuard configs, and drives `wg-quick` on Linux. | [`client-linux/README.md`](client-linux/README.md) |
| `client-windows` | Windows desktop VPN client built with Tauri, React, TypeScript, and Rust. It shares the profile and command surface with the Linux client, with Windows tunnel hooks isolated in the Tauri backend. | [`client-windows/README.md`](client-windows/README.md) |
| `client-android` | Android VPN client using Kotlin, Jetpack Compose, and the WireGuard Android tunnel library. It stores profiles locally and runs an embedded userspace tunnel after Android VPN consent. | [`client-android/README.md`](client-android/README.md) |

## WSL Build Environment

These instructions assume Ubuntu on WSL 2. Use WSL 2, not WSL 1, because the VPN service and Linux client need Linux networking features such as `/dev/net/tun`.

### 1. Install system packages

```sh
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  file \
  git \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libxdo-dev \
  pkg-config \
  unzip \
  wget \
  wireguard-tools
```

Package purposes:

- `build-essential`, `pkg-config`, and `libssl-dev`: native build tooling for Rust crates.
- `curl`, `wget`, `unzip`, and `file`: setup and packaging helpers used by Rust, Tauri, and Android tooling.
- `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, and `libxdo-dev`: Linux native dependencies required by Tauri desktop builds.
- `wireguard-tools`: `wg` and `wg-quick` for the Linux VPN client runtime.

### 2. Install Rust

`svc-vpn` uses Rust edition 2024, so install a current stable Rust toolchain.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustup default stable
rustup update
```

Verify:

```sh
rustc --version
cargo --version
```

### 3. Install Node.js and npm

The Tauri clients use Vite, React, and TypeScript. Install a current LTS Node.js release. One common WSL path is NodeSource:

```sh
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt install -y nodejs
```

Verify:

```sh
node --version
npm --version
```

### 4. Install JDK 21

The Android project uses the Kotlin JVM toolchain set to Java 21.

```sh
sudo apt install -y openjdk-21-jdk
```

Verify:

```sh
java -version
```

### 5. Install Android SDK packages

The Android project includes a Linux setup helper that installs Android command-line tools, platform 36, platform tools, and build tools 36.0.0.

```sh
cd client-android
./setup.sh
```

The script installs under `$ANDROID_HOME` when set, otherwise under `$HOME/Android/Sdk`. Add these exports to your shell profile if they are not already present:

```sh
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
```

Open a new shell or source your shell profile before building Android.

## Build Everything

Run these commands from the repository root after completing the WSL setup.

### VPN service

```sh
cd svc-vpn
cargo test
cargo build
```

For a release binary:

```sh
cargo build --release
```

Running the service requires Linux TUN access and usually network capabilities. See [`svc-vpn/README.md`](svc-vpn/README.md).

### Linux desktop client

```sh
cd client-linux
npm install
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

Running or connecting the Linux client requires `wireguard-tools`, `/dev/net/tun`, and permission to create interfaces and routes. See [`client-linux/README.md`](client-linux/README.md).

### Windows desktop client

```sh
cd client-windows
npm install
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

The Windows client can be type-checked and Rust-tested in WSL, but the configured Tauri bundles are Windows installer formats (`msi` and `nsis`). Produce those installer artifacts from Windows with the Windows Rust toolchain, Node.js, npm, and the required Tauri Windows packaging tools.

### Android client

```sh
cd client-android
./gradlew testDebugUnitTest assembleDebug
```

If Gradle cannot write to `$HOME/.gradle` in a restricted environment, keep its cache inside the checkout:

```sh
GRADLE_USER_HOME=.gradle-tmp ./gradlew testDebugUnitTest assembleDebug
```

The debug APK is written under `client-android/app/build/outputs/apk/debug/`.

## Notes

- The VPN service is not production-ready until the WireGuard protocol engine is completed, interoperable with standard clients, and externally reviewed.
- WSL VPN tunnels affect processes inside the WSL distribution. They do not replace a native Windows system-wide VPN client.
- Prefer the sub-project READMEs for runtime details, current limitations, and project-specific commands.
