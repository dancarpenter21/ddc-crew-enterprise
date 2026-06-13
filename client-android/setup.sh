#!/usr/bin/env bash
set -euo pipefail

SDK_ROOT="${ANDROID_HOME:-$HOME/Android/Sdk}"
CMDLINE_TOOLS_VERSION="14742923"
CMDLINE_TOOLS_ZIP="commandlinetools-linux-${CMDLINE_TOOLS_VERSION}_latest.zip"
CMDLINE_TOOLS_URL="https://dl.google.com/android/repository/${CMDLINE_TOOLS_ZIP}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$SDK_ROOT/cmdline-tools"

if [ ! -x "$SDK_ROOT/cmdline-tools/latest/bin/sdkmanager" ]; then
  echo "Downloading Android command-line tools..."
  curl -L --fail "$CMDLINE_TOOLS_URL" -o "$TMP_DIR/$CMDLINE_TOOLS_ZIP"

  echo "Installing Android command-line tools to $SDK_ROOT..."
  unzip -q "$TMP_DIR/$CMDLINE_TOOLS_ZIP" -d "$TMP_DIR"
  rm -rf "$SDK_ROOT/cmdline-tools/latest"
  mkdir -p "$SDK_ROOT/cmdline-tools/latest"
  mv "$TMP_DIR/cmdline-tools/"* "$SDK_ROOT/cmdline-tools/latest/"
else
  echo "Android command-line tools already installed at $SDK_ROOT."
fi

export ANDROID_HOME="$SDK_ROOT"
export ANDROID_SDK_ROOT="$SDK_ROOT"
export PATH="$SDK_ROOT/cmdline-tools/latest/bin:$SDK_ROOT/platform-tools:$PATH"

run_interactive() {
  if [ -t 0 ]; then
    "$@"
  elif [ -r /dev/tty ]; then
    "$@" </dev/tty
  else
    echo "Error: $1 requires user input, but no terminal is available." >&2
    echo "Run this setup script from an interactive shell." >&2
    exit 1
  fi
}

echo "Installing Android SDK packages..."
run_interactive sdkmanager --install \
  "platform-tools" \
  "platforms;android-36" \
  "build-tools;36.0.0"

echo "Reviewing Android SDK licenses..."
run_interactive sdkmanager --licenses

echo
echo "Android SDK setup complete."
echo "Add this to your shell profile if it is not already present:"
echo "export ANDROID_HOME=\"$SDK_ROOT\""
echo "export ANDROID_SDK_ROOT=\"\$ANDROID_HOME\""
echo "export PATH=\"\$ANDROID_HOME/cmdline-tools/latest/bin:\$ANDROID_HOME/platform-tools:\$PATH\""
