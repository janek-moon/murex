#!/bin/sh
# Install murex: build the binary, link the skill where the hosts find it,
# and set up the optional Ouroboros integration when the tooling is present.
#
# Ouroboros is not required. The default executor for a spike is a fresh
# subagent of the conducting agent; `ooo auto` is the optional engine for a
# different runtime, a detached run, or an independent evaluation gate.

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

# Optional Ouroboros integration: the `ooo murex <cmd>` surface, and
# `ooo auto` as an alternative spike engine. Best-effort - skipping it
# leaves the default subagent flow fully functional.
if have ouroboros; then
    echo "==> Registering murex with Ouroboros"
    # `install` rather than `add`: this repository ships a single manifest,
    # and `add --plugin <name>` expects a plugin catalog it does not have.
    ouroboros plugin install "$REPO"
elif have uv; then
    echo "==> Installing Ouroboros (optional integration)"
    uv tool install ouroboros-ai
    # uv puts tool binaries in ~/.local/bin, which is not on every PATH.
    if have ouroboros; then
        ouroboros plugin install "$REPO"
    else
        echo "Installed ouroboros-ai, but 'ouroboros' is not on PATH (~/.local/bin);" >&2
        echo "skipping registration - add it to PATH and re-run to complete it." >&2
    fi
else
    echo "==> Skipping Ouroboros integration (optional) - install uv and re-run to add it"
fi

echo
echo "Done. In Claude Code: /murex:spiral. By hand:"
echo "  murex --root <target-repo> status"
