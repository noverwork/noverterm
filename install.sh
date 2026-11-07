#!/bin/sh
# Install the latest Noverterm release on macOS (Apple Silicon).
# Usage: curl -fsSL https://raw.githubusercontent.com/noverwork/noverterm/main/install.sh | sh
set -eu

APP="noverterm.app"
URL="https://github.com/noverwork/noverterm/releases/latest/download/noverterm.app.tar.gz"
DEST="/Applications"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This installer supports macOS only. Download other platforms from https://github.com/noverwork/noverterm/releases" >&2
  exit 1
fi

if [ "$(uname -m)" != "arm64" ]; then
  echo "Noverterm currently ships Apple Silicon (arm64) builds only." >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading Noverterm..."
curl -fsSL "$URL" | tar -xz -C "$TMP"

if [ ! -d "$TMP/$APP" ]; then
  echo "Download did not contain $APP" >&2
  exit 1
fi

SUDO=""
if [ ! -w "$DEST" ]; then
  SUDO="sudo"
fi

$SUDO rm -rf "$DEST/$APP"
$SUDO mv "$TMP/$APP" "$DEST/"
# Builds are ad-hoc signed, not notarized; make sure Gatekeeper does not quarantine them.
$SUDO xattr -dr com.apple.quarantine "$DEST/$APP" 2>/dev/null || true

echo "Installed to $DEST/$APP"
