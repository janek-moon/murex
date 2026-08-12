#!/bin/sh
# Install murex: build the binary and link the skill where the hosts find it.

set -eu

REPO=$(cd "$(dirname "$0")" && pwd)

have() {
    command -v "$1" >/dev/null 2>&1
}

if ! have cargo; then
    echo "murex is a Rust binary and needs cargo to build it." >&2
    echo "Install Rust from https://rustup.rs and re-run this script." >&2
    exit 1
fi

echo "==> Building and installing murex"
cargo install --path "$REPO"

# Codex discovers skills from ~/.codex/skills/<name>/SKILL.md - the same
# format Claude Code reads from this repo's skills/ directory, so one link
# serves both hosts.
if [ -d "$HOME/.codex" ]; then
    mkdir -p "$HOME/.codex/skills"
    ln -sfn "$REPO/skills/spiral" "$HOME/.codex/skills/spiral"
    echo "==> Linked skill into ~/.codex/skills/spiral"
fi

echo
echo "Done. In Claude Code: /murex:spiral. By hand:"
echo "  murex --root <target-repo> status"
