# DDC VPN Android

Android client for the VPN service profile format used by `../client-windows`
and `../client-linux`.

The app stores profiles locally, requests Android VPN consent, and runs an
embedded WireGuard userspace tunnel through `com.wireguard.android:tunnel`.

## Prerequisites

- JDK 21
- Android SDK platform 36
- Android build tools 36.0.0

Run the setup script on Linux to install the Android command-line tools and SDK
packages:

```sh
./setup.sh
```

The script installs packages under `$ANDROID_HOME` when set, otherwise under
`$HOME/Android/Sdk`.

## Build

Use Android Studio or an Android SDK/JDK environment:

```sh
./gradlew testDebugUnitTest assembleDebug
```

In restricted environments where Gradle cannot write to `$HOME/.gradle`, keep
the Gradle cache inside this checkout:

```sh
GRADLE_USER_HOME=.gradle-tmp ./gradlew testDebugUnitTest assembleDebug
```

The debug APK is written under `app/build/outputs/apk/debug/`.

## Project Layout

- `app/src/main/java/com/ddc/vpn`: application, profile storage, UI, and tunnel
  integration code
- `app/src/main/java/com/ddc/vpn/tunnel`: WireGuard config rendering and tunnel
  lifecycle code
- `app/src/test/java/com/ddc/vpn`: JVM unit tests for profile validation and
  WireGuard config rendering
- `setup.sh`: Android SDK command-line setup helper for Linux

## Profile Fields

- Private key
- Server public key
- Optional preshared key
- Endpoint, for example `vpn.example.com:51820`
- Tunnel address, for example `10.44.0.2/32`
- Allowed IPs, DNS servers, MTU, and persistent keepalive

`../svc-vpn` currently documents its WireGuard protocol implementation as
incomplete, so this client should be treated as a development/test client until
the service is production-ready.
