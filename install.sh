#!/bin/sh
# Install murex: fetch the prebuilt binary for this platform from the latest
# GitHub release (no toolchain needed), or build from source when a release
# asset is unavailable and cargo is. Also links the skill for Codex when run
# from a checkout.
#
# Usage:
#   ./install.sh                                  # from a checkout
#   curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh

set -eu

REPO_URL="https://github.com/janek-moon/murex"
BIN_DIR="${MUREX_BIN_DIR:-$HOME/.local/bin}"

have() {
    command -v "$1" >/dev/null 2>&1
}

release_target() {
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64)          echo aarch64-apple-darwin ;;
        Darwin-x86_64)         echo x86_64-apple-darwin ;;
        Linux-x86_64)          echo x86_64-unknown-linux-musl ;;
        Linux-aarch64|Linux-arm64) echo aarch64-unknown-linux-musl ;;
        *)                     return 1 ;;
    esac
}

install_from_release() {
    have curl || return 1
    target=$(release_target) || return 1
    echo "==> Downloading murex ($target) from the latest release"
    mkdir -p "$BIN_DIR"
    curl -fsSL "$REPO_URL/releases/latest/download/murex-$target.tar.gz" \
        | tar -xz -C "$BIN_DIR" murex
    chmod +x "$BIN_DIR/murex"
    echo "==> Installed $BIN_DIR/murex"
    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) echo "note: $BIN_DIR is not on PATH - add it to your shell profile" >&2 ;;
    esac
}

install_from_source() {
    have cargo || return 1
    [ -f "$(dirname "$0")/Cargo.toml" ] || return 1
    echo "==> Building murex from source"
    cargo install --path "$(dirname "$0")"
}

if install_from_release; then
    :
elif install_from_source; then
    :
else
    echo "Could not install murex: no release asset for this platform and no" >&2
    echo "cargo + checkout to build from. Install Rust (https://rustup.rs) and" >&2
    echo "run ./install.sh from a clone of $REPO_URL." >&2
    exit 1
fi

# Codex discovers skills from ~/.codex/skills/<name>/SKILL.md - the same
# format Claude Code reads from this repo's skills/ directory. Only possible
# from a checkout; the Claude Code plugin ships the skills by itself.
SKILLS_DIR=$(dirname "$0")/skills
if [ -d "$SKILLS_DIR" ] && [ -d "$HOME/.codex" ]; then
    mkdir -p "$HOME/.codex/skills"
    for skill in "$SKILLS_DIR"/*/; do
        name=$(basename "$skill")
        ln -sfn "$(cd "$skill" && pwd)" "$HOME/.codex/skills/$name"
        echo "==> Linked skill into ~/.codex/skills/$name"
    done
fi

echo
echo "Done. In Claude Code: /murex:spiral, /murex:audit. By hand:"
echo "  murex --root <target-repo> status"
