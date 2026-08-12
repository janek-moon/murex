# murex

English | [한국어](README.ko.md)

![murex — spiral-model conductor](assets/banner.png)

A Boehm spiral-model conductor for coding agents. Installed as a Claude Code
or Codex plugin, it drives a risk-driven loop: register what is unknown,
spike the largest risk each cycle, gate on a commitment review, repeat until
the exposure is drained. Rust, single binary.

## How it works

```
you (agent) ── murex start / risk add        register + score the unknowns
     │
     ├─ murex cycle ──────────────▶ spike brief for the top-exposure risk
     ├─ spike (fresh subagent) ───▶ smallest prototype, evidence back
     ├─ murex commit ─────────────▶ continue | pivot | stop, cost, evidence
     │
     └─ repeat until remaining_exposure reaches 0 or a commit says stop
```

`murex` is the deterministic bookkeeping - risk register, exposure ranking
(probability × impact), the commitment gate - and never executes work. The
spike runs in a fresh subagent of the conducting agent: the same isolation an
external engine would give, without one. Where a quality loop iterates until
a gate passes, this loop iterates until the risks are retired. State lives in
`.murex/spiral.json` in the target repository, as plain JSON.

## Why a spiral, not agile

Agile's short sprints and small increments come from an era when writing
code was slow and expensive. AI agents have made building nearly free -
what is expensive now is building quickly on top of a wrong assumption. An
agent is fast and confident, and will happily polish a doomed approach until
the tests pass. So murex counts progress not in how much got built but in
which risks were retired: spending only matters when it reduces uncertainty,
and stopping is a legitimate outcome.

## Install

```bash
./install.sh   # builds murex and links the skill for Claude Code / Codex
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
# spike the brief in a fresh subagent
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
murex status                         # radius + remaining exposure
```

All commands take `--root <repo>` (default `.`).
