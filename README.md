# murex

English | [한국어](README.ko.md)

![murex — spiral-model conductor](assets/banner.png)

A Boehm spiral-model conductor for coding agents. Installed as a Claude Code
or Codex plugin, it drives a risk-driven loop: register what is unknown,
spike the largest risk each cycle through
[Ouroboros](https://github.com/Q00/ouroboros) (`ooo auto`), gate on a
commitment review, repeat until the exposure is drained. Rust, single binary.

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

`murex` is the deterministic bookkeeping - risk register, exposure ranking
(probability × impact), the commitment gate - and never executes work.
Ouroboros is the execution engine each spike runs through. Where Ouroboros's
`evolve` loop iterates until a quality gate passes, this loop iterates until
the risks are retired. State lives in `.murex/spiral.json` in the target
repository, as plain JSON.

## Install

```bash
./install.sh   # installs Ouroboros if missing, builds murex, registers both hosts
```

Claude Code:

```
/plugin marketplace add janek-moon/murex
/plugin install murex@murex
```

Codex reads the same skill from `~/.codex/skills/spiral`, linked by
`install.sh`.

## Use

The skill (`skills/spiral/SKILL.md`, invoked as `/murex:spiral`) teaches the
agent the full loop; by hand:

```bash
murex start "ship realtime collaborative editing"
murex risk add "CRDT memory may exceed the 2GB box" --probability 0.6 --impact 0.9
murex cycle                          # -> spike brief for the top-exposure risk
ooo auto "<the brief's instruction>" # Ouroboros executes the spike
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
murex status                         # radius + remaining exposure
```

All commands take `--root <repo>` (default `.`). Registered with Ouroboros
(`ouroboros plugin install .`), the same commands are also available as
`ooo murex <cmd>`.
