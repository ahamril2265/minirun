#!/usr/bin/env bash
# Download and verify the Alpine minirootfs, then extract it as minirun's
# base image.
#
# Usage: scripts/get-alpine.sh [DEST]      (default: ~/containers/alpine)
# Env:   ALPINE_VERSION (default 3.24.4), ALPINE_ARCH (default: uname -m)
#
# Extraction uses sudo because every file in the tarball is owned by uid 0
# (and /etc/shadow by gid 42 "shadow"). tar run as a normal user cannot
# chown, so it would silently make everything owned by you, and root inside
# the container would see a rootfs it doesn't own.
set -euo pipefail

VERSION="${ALPINE_VERSION:-3.24.4}"
ARCH="${ALPINE_ARCH:-$(uname -m)}"
DEST="${1:-$HOME/containers/alpine}"

BRANCH="v${VERSION%.*}"   # 3.24.4 -> v3.24
FILE="alpine-minirootfs-${VERSION}-${ARCH}.tar.gz"
URL="https://dl-cdn.alpinelinux.org/alpine/${BRANCH}/releases/${ARCH}/${FILE}"

# Idempotent: never touch an existing rootfs. Later steps rely on it being
# an unmodified read-only image layer.
if [ -d "$DEST" ] && [ -n "$(ls -A "$DEST")" ]; then
    echo "[get-alpine] $DEST already exists, nothing to do"
    exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "[get-alpine] downloading $URL"
curl -fsSL -o "$TMP/$FILE" "$URL"
curl -fsSL -o "$TMP/$FILE.sha256" "$URL.sha256"

# The .sha256 file is "<hash>  <filename>", so check from inside $TMP.
(cd "$TMP" && sha256sum -c "$FILE.sha256")

sudo mkdir -p "$DEST"
# --numeric-owner: use the uid/gid numbers from the tarball, not whatever
# the host's /etc/passwd maps those names to.
sudo tar -xzf "$TMP/$FILE" -C "$DEST" --numeric-owner
echo "[get-alpine] extracted Alpine $VERSION ($ARCH) to $DEST"
