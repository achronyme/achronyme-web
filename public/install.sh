#!/bin/sh
# Achronyme installer — https://achrony.me
#
# Usage:
#   curl -fsSL https://achrony.me/install.sh | sh
#   curl -fsSL https://achrony.me/install.sh | sh -s -- --version 0.1.0-beta.7
#
# Installs the `ach` binary to ~/.achronyme/bin and adds it to PATH.

set -e

REPO="achronyme/achronyme"
INSTALL_DIR="$HOME/.achronyme/bin"
VERSION=""

# --- Parse arguments ---

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --help)
            echo "Usage: install.sh [--version VERSION]"
            echo ""
            echo "Options:"
            echo "  --version VERSION   Install a specific version (e.g. 0.1.0-beta.7)"
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

URL="https://github.com/$REPO/releases/download/$TAG/$ARTIFACT"

# --- Download ---

echo "Installing Achronyme $TAG ($PLATFORM $ARCH_SUFFIX)..."

TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

HTTP_CODE=$(curl -fsSL -w '%{http_code}' -o "$TMPFILE" "$URL" 2>/dev/null) || true

if [ "$HTTP_CODE" != "200" ]; then
    echo "error: failed to download $URL (HTTP $HTTP_CODE)"
    echo ""
    echo "Available releases: https://github.com/$REPO/releases"
    exit 1
fi

# --- Install ---

mkdir -p "$INSTALL_DIR"
mv "$TMPFILE" "$INSTALL_DIR/ach"
chmod +x "$INSTALL_DIR/ach"

# --- Update PATH ---

SHELL_NAME="$(basename "$SHELL")"
EXPORT_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""

path_configured() {
    echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"
}

add_to_rc() {
    RC_FILE="$1"
    if [ -f "$RC_FILE" ] && grep -qF "$INSTALL_DIR" "$RC_FILE" 2>/dev/null; then
        return  # already configured
    fi
    echo "" >> "$RC_FILE"
    echo "# Achronyme" >> "$RC_FILE"
    echo "$EXPORT_LINE" >> "$RC_FILE"
}

if ! path_configured; then
    case "$SHELL_NAME" in
        zsh)  add_to_rc "$HOME/.zshrc" ;;
        bash)
            if [ -f "$HOME/.bash_profile" ]; then
                add_to_rc "$HOME/.bash_profile"
            else
                add_to_rc "$HOME/.bashrc"
            fi
            ;;
        fish)
            FISH_CONFIG="$HOME/.config/fish/config.fish"
            if ! grep -qF "$INSTALL_DIR" "$FISH_CONFIG" 2>/dev/null; then
                mkdir -p "$(dirname "$FISH_CONFIG")"
                echo "" >> "$FISH_CONFIG"
                echo "# Achronyme" >> "$FISH_CONFIG"
                echo "set -gx PATH $INSTALL_DIR \$PATH" >> "$FISH_CONFIG"
            fi
            ;;
        *)  add_to_rc "$HOME/.profile" ;;
    esac
fi

# --- Verify ---

ACH_VERSION=$("$INSTALL_DIR/ach" --version 2>/dev/null || echo "unknown")

echo ""
echo "  Achronyme installed successfully!"
echo ""
echo "  Binary:  $INSTALL_DIR/ach"
echo "  Version: $ACH_VERSION"
echo ""

if ! path_configured; then
    echo "  Restart your shell or run:"
    echo "    $EXPORT_LINE"
    echo ""
fi

echo "  Get started:"
echo "    ach run examples/hello.ach"
echo ""
