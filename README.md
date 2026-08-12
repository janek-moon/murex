# murex

English | [한국어](README.ko.md)

A Boehm spiral-model conductor for coding agents. Installed as a Claude Code
or Codex plugin, it turns the agent into the driver of a risk-driven loop:
register what is unknown, spike the largest risk each cycle through
[Ouroboros](https://github.com/Q00/ouroboros) (`ooo auto`), gate on a
commitment review, repeat until the exposure is drained. Rust, single binary.

Named for the spiral-shelled sea snail that Tyrian purple was extracted from -
you had to break the shell to get the dye, which is the bargain this tool makes
explicit: you spend a cycle to learn what you cannot learn without spending it.

## How it works

```
you (agent) ── murex start / risk add        register + score the unknowns
     │
     ├─ murex cycle ──────────────▶ spike brief for the top-exposure risk
     ├─ ooo auto "<instruction>" ─▶ Ouroboros executes the spike
     ├─ murex commit ─────────────▶ continue | pivot | stop, cost, evidence
     │
     └─ repeat until remaining_exposure reaches 0 or a commit says stop
```

Three parts, three jobs. `murex` is the deterministic bookkeeping - the risk
register, exposure ranking, and the commitment gate - and never executes work.
Ouroboros is the execution engine each spike runs through. The agent conducts:
it interviews the human into a scored register, hands briefs to the engine,
verifies the evidence, and closes the gate.

Where Ouroboros's `evolve` loop is **quality-driven** (regenerate until an
evaluation gate passes), this loop is **risk-driven**: each cycle exists to
retire the largest risk, and the review between cycles decides whether the
next one earns its cost.

## Install

```bash
./install.sh
```

Installs Ouroboros (`uv tool install ouroboros-ai`) if it is missing, builds
and installs the `murex` binary, registers it with Ouroboros, and links the
skill into `~/.codex/skills` when Codex is present. Needs `cargo`, and `uv`
only when Ouroboros has to be installed; it fails with a pointer rather than
guessing if either is absent.

**Claude Code** - install as a plugin, which ships the skill:

```
/plugin marketplace add janek-moon/murex
/plugin install murex@murex
```

The binary itself still comes from `install.sh` (or
`cargo install --git https://github.com/janek-moon/murex`).

**Codex** - `install.sh` links `skills/murex` into `~/.codex/skills/murex`;
Codex reads the same SKILL.md format Claude Code does.

The same steps by hand:

```bash
uv tool install ouroboros-ai        # only if `ooo` is not already there
cargo install --path .              # puts `murex` on PATH
ouroboros plugin discover .         # inspect the manifest, writes nothing
ouroboros plugin install .          # optional: adds the `ooo murex` surface
```

## Use

The skill (`skills/murex/SKILL.md`) teaches the agent the full loop; by hand
it looks like:

```bash
murex start "ship realtime collaborative editing"
murex risk add "CRDT memory may exceed the 2GB box" --probability 0.6 --impact 0.9
murex cycle                          # -> spike brief for the top-exposure risk
ooo auto "<the brief's instruction>" # Ouroboros executes the spike
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
murex status                         # radius + remaining exposure
```

All commands take `--root <repo>` (default `.`). Registered with Ouroboros,
the same commands are also available as `ooo murex <cmd>`.

## Layout

| Path                      | Role                                          |
|---------------------------|-----------------------------------------------|
| `.claude-plugin/`         | Claude Code plugin + marketplace manifests    |
| `skills/murex/SKILL.md`   | The agent-facing surface (Claude Code, Codex) |
| `src/lib.rs`              | Controller logic - register, ranking, gate    |
| `src/main.rs`             | CLI entrypoint; argv in, JSON out             |
| `ouroboros.plugin.json`   | Ouroboros UserLevel plugin manifest           |
| `install.sh`              | Host-aware installer                          |
| `tests/spiral.rs`         | Self-check: `cargo test`                      |

The Ouroboros manifest keeps the filename `ouroboros.plugin.json` because that
plugin contract dictates it. State is written to `.murex/spiral.json` in the
target repository, as plain JSON, so the register stays readable and diffable
in review.

## Scope

Risk scoring is human judgement, entered through `risk add`. The plugin does
not call a model to guess probabilities - it is the deterministic bookkeeping
and the gate, so that whatever the agent claims about a risk stays auditable
against the evidence recorded when the risk was closed.
