# murex

A Boehm spiral-model cycle controller, packaged as an [Ouroboros](https://github.com/Q00/ouroboros)
UserLevel plugin. Rust, single binary.

Named for the spiral-shelled sea snail that Tyrian purple was extracted from -
you had to break the shell to get the dye, which is the bargain this tool makes
explicit: you spend a cycle to learn what you cannot learn without spending it.

## Why

Ouroboros already drives execution and iteration. Its `evolve` loop is
**quality-driven**: it regenerates until an evaluation gate passes. The spiral
model is **risk-driven**: each cycle exists to retire the largest risk, and a
commitment review between cycles decides whether the next one earns its cost.

That risk quadrant is the one thing Ouroboros does not have, so it is the only
thing this plugin adds. It holds the register, ranks by exposure, picks what
each cycle must de-risk, and gates the commitment review. It never executes
work - `ooo murex cycle` emits a spike brief you hand to `ooo auto` or
`ooo run`.

## Build and install

```bash
./install.sh
```

That installs Ouroboros first if it is missing — murex is a plugin, so the
`ooo murex` invocation does not exist without a host — then builds the binary
and registers one with the other. It needs `cargo`, and `uv` only when
Ouroboros has to be installed; it fails with a pointer rather than guessing if
either is absent.

The same thing by hand:

```bash
uv tool install ouroboros-ai        # only if `ouroboros` is not already there
cargo install --path .              # puts `murex` on PATH
ouroboros plugin discover .         # inspect the manifest, writes nothing
ouroboros plugin install .
```

The manifest's entrypoint is the bare binary name, so the plugin resolves it
from PATH. The binary also runs standalone, with identical argv and JSON output:

```bash
murex --root <target-repo> status
```

## Use

```bash
ooo murex start "ship realtime collaborative editing"
ooo murex risk add "CRDT memory may exceed the 2GB box" --probability 0.6 --impact 0.9
ooo murex cycle                     # -> spike brief for the top-exposure risk
# ... execute the brief through ooo auto / ooo run ...
ooo murex commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
ooo murex status                    # radius + remaining exposure
```

`skills/murex/SKILL.md` is the agent-facing surface: it is what teaches Claude
Code, Codex, and the other runtimes when to reach for a spiral and how to drive
one. The full walkthrough lives there.

## Layout

| Path                     | Role                                          |
|--------------------------|-----------------------------------------------|
| `ouroboros.plugin.json`  | Plugin manifest (schema 0.1)                  |
| `src/lib.rs`             | Controller logic - register, ranking, gate    |
| `src/main.rs`            | CLI entrypoint; argv in, JSON out             |
| `skills/murex/SKILL.md`  | In-agent surface for Claude Code / Codex      |
| `tests/spiral.rs`        | Self-check: `cargo test`                      |

The manifest keeps the filename `ouroboros.plugin.json` because the plugin
contract dictates it. State is written to `.murex/spiral.json` in the target
repository, as plain JSON, so the register stays readable and diffable in review.

## Scope

Risk scoring is human judgement, entered through `risk add`. The plugin does
not call a model to guess probabilities - it is the deterministic bookkeeping
and the gate, so that whatever the agent claims about a risk stays auditable
against the evidence recorded when the risk was closed.

MIT.
