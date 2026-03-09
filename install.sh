#!/usr/bin/env bash
# Install archon from GitHub releases or build from source.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/auser/archon/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/auser/archon/main/install.sh | bash -s -- --version v0.2.0
#   ./install.sh --from-source

set -euo pipefail

REPO="auser/archon"
INSTALL_DIR="${ARCHON_INSTALL_DIR:-$HOME/.local/bin}"
VERSION=""
FROM_SOURCE=false

usage() {
    echo "Usage: install.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --version <tag>    Install a specific version (e.g. v0.2.0)"
    echo "  --from-source      Build from source instead of downloading a binary"
    echo "  --install-dir <p>  Install directory (default: ~/.local/bin)"
    echo "  --help             Show this help"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --from-source) FROM_SOURCE=true; shift ;;
        --install-dir) INSTALL_DIR="$2"; shift 2 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1"; usage; exit 1 ;;
    esac
done

info()  { echo "  $(tput bold)>$(tput sgr0) $*"; }
ok()    { echo "  $(tput setaf 2)✓$(tput sgr0) $*"; }
error() { echo "  $(tput setaf 1)✗$(tput sgr0) $*" >&2; }

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)      error "Unsupported OS: $os"; exit 1 ;;
    esac

    case "$arch" in
        x86_64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)       error "Unsupported architecture: $arch"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

install_from_source() {
    if ! command -v cargo &>/dev/null; then
        error "cargo not found. Install Rust: https://rustup.rs"
        exit 1
    fi

    local src_dir
    if [[ -f "Cargo.toml" ]] && grep -q 'name = "archon"' Cargo.toml 2>/dev/null; then
        src_dir="."
    else
        info "Cloning $REPO..."
        src_dir="$(mktemp -d)"
        trap 'rm -rf "$src_dir"' EXIT
        git clone --depth 1 ${VERSION:+--branch "$VERSION"} "https://github.com/$REPO.git" "$src_dir"
    fi

    info "Building from source..."
    cargo build --release --manifest-path "$src_dir/Cargo.toml"
    mkdir -p "$INSTALL_DIR"
    cp "$src_dir/target/release/archon" "$INSTALL_DIR/archon"
    chmod +x "$INSTALL_DIR/archon"
    ok "Built and installed to $INSTALL_DIR/archon"
    return
}

install_from_release() {
    local platform="$1"

    if [[ -z "$VERSION" ]]; then
        info "Fetching latest release..."
        VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')"
        if [[ -z "$VERSION" ]]; then
            error "Could not determine latest version. Try --from-source or --version <tag>"
            exit 1
        fi
    fi

    local asset="archon-${VERSION}-${platform}.tar.gz"
    local url="https://github.com/$REPO/releases/download/${VERSION}/${asset}"

    info "Downloading archon $VERSION for $platform..."
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    if ! curl -fsSL "$url" -o "$tmp/$asset"; then
        error "Download failed. Binary may not be available for your platform."
        info "Falling back to source build..."
        install_from_source
        return
    fi

    tar -xzf "$tmp/$asset" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    cp "$tmp/archon" "$INSTALL_DIR/archon"
    chmod +x "$INSTALL_DIR/archon"
    ok "Installed archon $VERSION to $INSTALL_DIR/archon"
}

main() {
    echo ""
    echo "  archon installer"
    echo ""

    if $FROM_SOURCE; then
        install_from_source
    else
        local platform
        platform="$(detect_platform)"
        install_from_release "$platform"
    fi

    # Check if install dir is in PATH.
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        info "Add $INSTALL_DIR to your PATH:"
        echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    fi

    echo ""
    ok "Run 'archon --help' to get started"
    echo ""
}

main
