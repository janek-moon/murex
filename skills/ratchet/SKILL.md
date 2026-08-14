---
name: ratchet
description: "Use when requirements are clear and you want to build a feature the disciplined way - bottom-up, verifying each layer before the next. Decompose into components with verifiable requirements, build the lowest one whose dependencies are all verified (in a fresh subagent), verify it with evidence, and ratchet upward until complete. The binary refuses to build on unverified ground. For work with real unknowns or risk, use spiral instead."
---

# murex - Ratchet: bottom-up verified construction

## Description

You conduct a ratchet: a feature with clear requirements, built from the bottom
up, one verified layer at a time. Three parts, three jobs (mirror of the spiral):

- **`murex ratchet`** - deterministic bookkeeping: the component register,
  the buildable frontier, the verification gate. It never executes work.
- **An executor** - a fresh subagent: one component's requirement in, evidence out.
- **You** - decompose the feature into components with verifiable requirements,
  hand each build to the executor, verify its evidence yourself, close the gate.

Use this when the requirements are clear. The reason it is more than "just build
it bottom-up": the binary **refuses to hand you a component until every
dependency is verified**, and **refuses to record a verification without
evidence** - so you cannot build on unverified ground, and a level, once
verified, is locked. For work with real unknowns, use `/murex:spiral` instead;
a drained spiral hands off here.

## Prerequisites

`murex` on PATH, new enough to have the ratchet. Feature-detect and (re)install
if missing or stale:

```bash
murex ratchet --help >/dev/null 2>&1 || curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh
```

## The loop

```
start -> add* (decompose) -> [ next -> build (subagent) -> verify ]* -> complete
```

### 1. Open the ratchet

```bash
murex --root . ratchet start "ship CSV export" --requirement "a user downloads a valid CSV of their data"
```

### 2. Decompose into verifiable components (leaves first)

Register the lowest parts first; a component may only depend on ids that already
exist. Each `--requirement` must be checkable.

```bash
murex --root . ratchet add "CSV row encoder" --requirement "encodes one record to an RFC-4180 line, quotes embedded commas/quotes"
murex --root . ratchet add "CSV writer" --requirement "streams all rows with a header" --depends-on C1
murex --root . ratchet add "export endpoint" --requirement "GET /export returns text/csv for the current user" --depends-on C2
```

### 3. Take the next buildable component

```bash
murex --root . ratchet next
```

Returns the build brief for the lowest buildable component - its requirement and
the evidence/interfaces of its already-verified dependencies.

### 4. Build it in a fresh subagent

Dispatch a fresh subagent with the brief and nothing else: it builds the
smallest implementation that satisfies the requirement and reports its evidence
(tests run, output shown). Your context stays on the register.

### 5. Verify, then gate

When it returns, **verify the evidence yourself** - run the checks - before
touching the gate.

```bash
murex --root . ratchet verify C1 --evidence "cargo test csv_encoder green; RFC-4180 sample matches byte-for-byte" --cost 1.0
```

If the build did not meet its requirement, `murex ratchet rework C1 --note "<what failed>"` returns it to the frontier for another attempt.

### 6. Repeat until complete

Go back to step 3. Check progress with `murex --root . ratchet status`
(verified/total, the current frontier). The ratchet is `complete` when every
component is verified; confirm the whole against the `start` requirement.

## Notes

- State lives in `.murex/ratchet.json`; commit it to share the build ledger.
  It is independent of a spiral's `.murex/spiral.json` - a repo can run either.
- A verified component is final - the ratchet does not slip. Decompose so that
  "verified" is a claim you can stand behind.
- `murex ratchet list` groups components by state; `murex ratchet stop --reason "..."` abandons the ratchet.
