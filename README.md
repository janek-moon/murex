# ouroboros-spiral

A Boehm spiral-model cycle controller, packaged as an [Ouroboros](https://github.com/Q00/ouroboros)
UserLevel plugin.

## Why

Ouroboros already drives execution and iteration. Its `evolve` loop is
**quality-driven**: it regenerates until an evaluation gate passes. The spiral
model is **risk-driven**: each cycle exists to retire the largest risk, and a
commitment review between cycles decides whether the next one earns its cost.

That risk quadrant is the one thing Ouroboros does not have, so it is the only
thing this plugin adds. It holds the register, ranks by exposure, picks what
each cycle must de-risk, and gates the commitment review. It never executes
work - `ooo spiral cycle` emits a spike brief you hand to `ooo auto` or
`ooo run`.

## Install

```bash
ouroboros plugin add /path/to/ouroboros-spiral --plugin spiral
```

Requires Python 3.10+. No third-party dependencies.

## Use

```bash
ooo spiral start "ship realtime collaborative editing"
ooo spiral risk add "CRDT memory may exceed the 2GB box" --probability 0.6 --impact 0.9
ooo spiral cycle                     # -> spike brief for the top-exposure risk
# ... execute the brief through ooo auto / ooo run ...
ooo spiral commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
ooo spiral status                    # radius + remaining exposure
```

`skills/spiral/SKILL.md` is the agent-facing surface: it is what teaches Claude
Code, Codex, and the other runtimes when to reach for a spiral and how to drive
one. The full walkthrough lives there.

## Layout

| Path                       | Role                                              |
|----------------------------|---------------------------------------------------|
| `ouroboros.plugin.json`    | Plugin manifest (schema 0.1)                      |
| `ouroboros_spiral/`        | Controller logic and the CLI entrypoint           |
| `skills/spiral/SKILL.md`   | In-agent surface for Claude Code / Codex          |
| `test_spiral.py`           | Self-check: `python3 test_spiral.py`              |

State is written to `.ouroboros/spiral.json` in the target repository.

## Scope

Risk scoring is human judgement, entered through `risk add`. The plugin does
not call a model to guess probabilities - it is the deterministic bookkeeping
and the gate, so that whatever the agent claims about a risk stays auditable
against the evidence recorded when the risk was closed.

MIT.
