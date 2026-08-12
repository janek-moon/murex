---
name: spiral
description: "Run risk-driven spiral-model development: register risks, spike the largest one per cycle in an isolated subagent, gate on a commitment review, repeat until exposure is drained"
---

# murex - Spiral-Model Conductor

## Description

You are the conductor of a Boehm spiral. Three parts, three jobs:

- **`murex`** (this plugin's binary) - the deterministic bookkeeping: risk
  register, top-risk selection, the commitment gate. It never executes work.
- **An executor** - a fresh subagent of your host: clean context in,
  evidence out.
- **You** - interview the human into a scored risk register, hand each cycle's
  brief to the executor, verify the evidence it brings back, close the gate,
  repeat.

Use this when the work has real unknowns: an unproven integration, a
performance target nobody has measured, a vendor API that may not do what the
docs claim. If the requirements are already clear, just build it - a spiral
with no risks is a slower waterfall.

Where a quality loop iterates until an evaluation gate passes, this loop
iterates until the **risks** are retired - and a commitment review between
cycles decides whether the next one is worth its cost.

## Prerequisites

`murex` on PATH. If missing, fetch the prebuilt binary for this platform
(falls back to a cargo build when no release asset fits):

```bash
command -v murex || curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh
```

## The loop

```
start -> risk add (interview) -> [ cycle -> spike -> commit ]* -> drained
```

### 1. Open the spiral

```bash
murex --root . start "ship realtime collaborative editing" \
  --constraint "must stay on the existing Postgres box" \
  --alternative "CRDT" --alternative "OT with a central server"
```

### 2. Register what is not known

Interview the human for the unknowns, then score each: `probability` (0..1)
that it bites, `impact` (0..1) of the damage if it does. Exposure is the
product; only the ranking matters, so score consistently rather than precisely.

```bash
murex --root . risk add "CRDT memory footprint may exceed the 2GB box limit" \
  --probability 0.6 --impact 0.9 \
  --mitigation "prototype with a 10k-op document, measure RSS"
murex --root . risk add "Vendor websocket SDK may not support our auth scheme" \
  --probability 0.4 --impact 0.7
```

### 3. Open a cycle

```bash
murex --root . cycle
```

Returns the spike brief for the highest-exposure open risk: an `instruction`
that names exactly one risk and forbids broadening scope.

### 4. Execute the spike

Dispatch a fresh subagent with the brief's `instruction` and nothing else - a
clean context that builds the smallest prototype and reports its evidence
back, while your own context stays on the register.

When the spike returns, **verify the evidence yourself** - read what it
built, run its checks - before touching the gate. The gate records what you
verified, not what the executor claims.

### 5. Commitment review

```bash
murex --root . commit --decision continue --cost 1.5 \
  --resolve R1 --evidence "10k-op doc held at 380MB RSS; headroom is fine" \
  --outcome "CRDT approach viable"
```

`--decision` is the gate:

| Decision   | Meaning                                                        |
|------------|----------------------------------------------------------------|
| `continue` | Risk retired or reduced; the next cycle is worth its cost.      |
| `pivot`    | Evidence killed the current approach; switch to an alternative. |
| `stop`     | The objective is not worth the remaining exposure. Spiral ends. |

Pass `--resolve` only when the evidence genuinely retires the risk. An
inconclusive spike leaves the risk open, and it will be picked again - which
is the correct signal that it needs another cycle.

### 6. Repeat until drained

Go back to step 3. Check convergence with:

```bash
murex --root . status
```

Reports `radius` (cycles completed, cumulative cost) and `remaining_exposure`.
Falling exposure against rising cost is convergence; the spiral is done when
`remaining_exposure` reaches 0 or a commit decides `stop`. Flat exposure
across two cycles means the spikes are not producing evidence - reframe the
risk with the human before spending another cycle.

## Notes

- State lives in `.murex/spiral.json` in the target repo; commit it to share
  the register. Risks are never deleted, only `resolved` or `accepted`
  (`murex risk close <id> --status accepted` for risks the human decides to
  live with), so what you knowingly shipped past stays auditable.
