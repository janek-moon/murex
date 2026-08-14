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

### Two modes

Unclear requirements: run the spiral above to de-risk before committing.
Clear requirements: run the ratchet below to build bottom-up, verifying each
layer before the next. A drained spiral hands off to a ratchet.

```
you (agent) ── murex ratchet start / add     decompose into verifiable components
     │
     ├─ murex ratchet next ───────▶ build brief for the lowest buildable component
     ├─ build (fresh subagent) ───▶ smallest implementation, evidence back
     ├─ murex ratchet verify ─────▶ evidence, cost; locks the component
     │
     └─ repeat until every component is verified
```

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
curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh
```

Fetches the prebuilt binary for your platform from the latest release - no
toolchain needed. From a checkout, `./install.sh` does the same and also
links the skill for Codex; it falls back to `cargo install --path .` when no
release asset fits the platform.

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

When `remaining_exposure` reaches 0, `status` prints a handoff line: switch
to the ratchet (`skills/ratchet/SKILL.md`, invoked as `/murex:ratchet`) to
build the de-risked feature bottom-up, verifying each layer before the next:

```bash
murex ratchet start "ship CSV export" --requirement "a user downloads a valid CSV of their data"
murex ratchet add "CSV row encoder" --requirement "encodes one record to RFC-4180"
murex ratchet next                   # -> build brief for the lowest buildable component
# build it in a fresh subagent
murex ratchet verify C1 --evidence "cargo test csv_encoder green" --cost 1.0
murex ratchet status                 # verified/total + the current frontier
```

A third skill (`skills/audit/SKILL.md`, invoked as `/murex:audit`) reviews a
running ledger for the discipline the binary cannot enforce - scores that
actually rank, gate evidence that actually retires risks, exposure that
actually drains - and flags an incremental build wearing spiral clothing.
Verdicts cite risk ids and cycle numbers from `.murex/spiral.json`.
