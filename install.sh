#!/bin/sh
# Install murex, and the Ouroboros runtime it plugs into if that is missing.
#
# murex is a plugin. On its own it is a CLI that keeps a risk register, but the
# `ooo murex <cmd>` invocation only exists once Ouroboros can see it. So this
# installs the host first when it is absent, then the binary, then registers
# one with the other.

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

if have ouroboros; then
    echo "==> Ouroboros already installed"
else
    if ! have uv; then
        echo "Ouroboros is not installed, and uv is needed to install it." >&2
        echo "Install uv from https://docs.astral.sh/uv/ and re-run this script." >&2
        exit 1
    fi
    echo "==> Installing Ouroboros (ouroboros-ai)"
    uv tool install ouroboros-ai
    # uv puts tool binaries in ~/.local/bin, which is not on every PATH.
    if ! have ouroboros; then
        echo "Installed ouroboros-ai, but 'ouroboros' is still not on PATH." >&2
        echo "uv installs tools into ~/.local/bin - add it to PATH and re-run." >&2
        exit 1
    fi
fi

echo "==> Building and installing murex"
cargo install --path "$REPO"

echo "==> Registering murex as an Ouroboros plugin"
# `install` rather than `add`: this repository ships a single manifest, and
# `add --plugin <name>` expects a plugin catalog, which it does not have.
ouroboros plugin install "$REPO"

# Codex discovers skills from ~/.codex/skills/<name>/SKILL.md - the same
# format Claude Code reads from this repo's skills/ directory, so one link
# serves both hosts.
if [ -d "$HOME/.codex" ]; then
    mkdir -p "$HOME/.codex/skills"
    ln -sfn "$REPO/skills/murex" "$HOME/.codex/skills/murex"
    echo "==> Linked skill into ~/.codex/skills/murex"
fi

echo
echo "Done. Start a spiral with:"
echo "  ooo murex start \"<objective>\""
echo "or drive the binary directly:"
echo "  murex --root <target-repo> status"
