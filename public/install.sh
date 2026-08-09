#!/bin/sh
# Achronyme installer - https://achrony.me
#
# Usage:
#   curl -fsSL https://achrony.me/install.sh | sh
#   curl -fsSL https://achrony.me/install.sh | sh -s -- --version 0.1.1
#
# Installs `ach` to ~/.local/bin and the Linux AOT runtime to ~/.local/lib.

set -e

REPO="achronyme/achronyme"
ACHRONYME_INSTALL_BIN_DIR="${ACHRONYME_BIN_DIR:-$HOME/.local/bin}"
ACHRONYME_INSTALL_LIB_DIR="${ACHRONYME_LIB_DIR:-$HOME/.local/lib}"
VERSION=""

# --- Parse arguments ---

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            if [ "$#" -lt 2 ]; then
                echo "error: --version requires a value"
                exit 1
            fi
            VERSION="$2"
            shift 2
            ;;
        --help)
            echo "Usage: install.sh [--version VERSION]"
            echo ""
            echo "Options:"
            echo "  --version VERSION   Install a specific version (e.g. 0.1.1)"
            echo "                      Default: latest release"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# --- Detect platform ---

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  PLATFORM="linux" ;;
    Darwin) PLATFORM="macos" ;;
    *)
        echo "error: unsupported operating system: $OS"
        echo "Achronyme supports Linux and macOS. For Windows, download from:"
        echo "  https://github.com/$REPO/releases"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)  ARCH_SUFFIX="x86_64" ;;
    aarch64|arm64) ARCH_SUFFIX="aarch64" ;;
    *)
        echo "error: unsupported architecture: $ARCH"
        exit 1
        ;;
esac

ARTIFACT="achronyme-${PLATFORM}-${ARCH_SUFFIX}"

# --- Resolve version ---

if [ -z "$VERSION" ]; then
    TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)
    if [ -z "$TAG" ]; then
        echo "error: could not determine latest release"
        exit 1
    fi
else
    TAG="v$VERSION"
fi

RELEASE_URL="https://github.com/$REPO/releases/download/$TAG"

# --- Download ---

echo "Installing Achronyme $TAG ($PLATFORM $ARCH_SUFFIX)..."

ACHRONYME_TMP_ROOT=$(mktemp -d)
trap 'rm -rf "$ACHRONYME_TMP_ROOT"' EXIT HUP INT TERM

download_file() {
    DOWNLOAD_URL="$1"
    DOWNLOAD_DESTINATION="$2"
    HTTP_CODE=$(curl -fsSL -w '%{http_code}' -o "$DOWNLOAD_DESTINATION" \
        "$DOWNLOAD_URL" 2>/dev/null) || true

    if [ "$HTTP_CODE" != "200" ]; then
        echo "error: failed to download $DOWNLOAD_URL (HTTP $HTTP_CODE)"
        echo ""
        echo "Available releases: https://github.com/$REPO/releases"
        exit 1
    fi
}

# --- Install ---

mkdir -p "$ACHRONYME_INSTALL_BIN_DIR"

if [ "$OS" = "Linux" ]; then
    ARCHIVE="$ARTIFACT.tar.gz"
    CHECKSUM="$ARCHIVE.sha256"

    download_file "$RELEASE_URL/$ARCHIVE" "$ACHRONYME_TMP_ROOT/$ARCHIVE"
    download_file "$RELEASE_URL/$CHECKSUM" "$ACHRONYME_TMP_ROOT/$CHECKSUM"

    (
        cd "$ACHRONYME_TMP_ROOT"
        sha256sum --check "$CHECKSUM"
    )
    tar -C "$ACHRONYME_TMP_ROOT" -xzf "$ACHRONYME_TMP_ROOT/$ARCHIVE"

    BUNDLE_ROOT="$ACHRONYME_TMP_ROOT/$ARTIFACT"
    if [ ! -x "$BUNDLE_ROOT/bin/ach" ] || \
       [ ! -f "$BUNDLE_ROOT/lib/libakron_aot_runtime.a" ]; then
        echo "error: release bundle is missing the binary or AOT runtime"
        exit 1
    fi

    mkdir -p "$ACHRONYME_INSTALL_LIB_DIR"
    cp "$BUNDLE_ROOT/bin/ach" "$ACHRONYME_INSTALL_BIN_DIR/ach"
    cp "$BUNDLE_ROOT/lib/libakron_aot_runtime.a" \
        "$ACHRONYME_INSTALL_LIB_DIR/libakron_aot_runtime.a"
    chmod +x "$ACHRONYME_INSTALL_BIN_DIR/ach"
    chmod 644 "$ACHRONYME_INSTALL_LIB_DIR/libakron_aot_runtime.a"
else
    download_file "$RELEASE_URL/$ARTIFACT" "$ACHRONYME_TMP_ROOT/$ARTIFACT"
    mv "$ACHRONYME_TMP_ROOT/$ARTIFACT" "$ACHRONYME_INSTALL_BIN_DIR/ach"
    chmod +x "$ACHRONYME_INSTALL_BIN_DIR/ach"
fi

# --- Verify PATH ---

path_configured() {
    echo "$PATH" | tr ':' '\n' | grep -Fqx "$ACHRONYME_INSTALL_BIN_DIR"
}

ACH_VERSION=$("$ACHRONYME_INSTALL_BIN_DIR/ach" --version 2>/dev/null || echo "unknown")

echo ""
echo "  Achronyme installed successfully!"
echo ""
echo "  Binary:  $ACHRONYME_INSTALL_BIN_DIR/ach"
if [ "$OS" = "Linux" ]; then
    echo "  Runtime: $ACHRONYME_INSTALL_LIB_DIR/libakron_aot_runtime.a"
fi
echo "  Version: $ACH_VERSION"
echo ""

if ! path_configured; then
    echo "  $ACHRONYME_INSTALL_BIN_DIR is not in your PATH. Add it with:"
    echo "    export PATH=\"$ACHRONYME_INSTALL_BIN_DIR:\$PATH\""
    echo ""
fi

echo "  Get started:"
echo "    ach init hello --template vm"
echo "    cd hello && ach run"
echo ""
echo "  Guide: https://achrony.me/docs/getting-started/hello-world"
echo ""
