---
name: audit
description: "Use when you want to check whether a running or finished spiral stayed risk-driven or became an incremental build wearing spiral clothing. Audits a spiral against the risk-driven invariants the binary cannot enforce: scores that actually rank, gate evidence that actually retires risks, exposure that actually drains, a stop that gets considered. Verdicts cite the ledger."
---

# murex - Spiral Audit

## Description

You audit a spiral run for the discipline the binary cannot enforce. murex
already guarantees the mechanics: every cycle targets the top-exposure open
risk, a pending cycle must close through a gate, risks are never deleted.
What it cannot check is the human side - whether the numbers, evidence, and
decisions fed into it were honest. That is this skill.

The failure mode it hunts: **an incremental build wearing spiral clothing.**
Cycles that ship features and call them spikes, gates rubber-stamped
`continue`, a register scored once at the start and never touched again. The
spiral model is risk-driven - what each cycle does is chosen by the largest
remaining unknown, not by "what to build next". A spiral that stops being
risk-driven is just a slower waterfall, and this audit says so.

Use it mid-spiral as a health check, or after `stop`/drain as a post-mortem.

## Prerequisites

A ledger: `.murex/spiral.json` in the target repo. Read the file directly -
it is plain JSON and the audit needs every field - and cross-check the
derived numbers with the binary:

```bash
murex --root . status       # radius, remaining_exposure, history
murex --root . risk list    # open (ranked) and closed risks
```

## The checks

Read the whole ledger first, then walk the table. Every verdict must cite
ledger fields - risk ids, cycle numbers, exposures, evidence strings. A
finding without a citation is not a finding.

| # | Check | Red flag in the ledger |
|---|-------|------------------------|
| 1 | Scores rank | `probability`/`impact` clustered on one value across risks - the exposure ranking carries no information, so "top risk" was chosen by accident |
| 2 | Evidence retires | risks closed `resolved` whose `evidence` has no number, artifact, or path in it; gate `outcome` empty |
| 3 | Exposure drains | open exposure flat or rising across two closed cycles while `cumulative_cost` grows - spikes are not producing evidence, or features are being built under a spike's name |
| 4 | Cost is real | closed cycles with `cost: 0` - the radius is fiction and the commitment review compared against nothing |
| 5 | Unknowns land in the register | several cycles in, yet no risk has `cycle_opened > 0` - implementation discovered nothing? More likely discoveries bypassed the ledger |
| 6 | Accepted means decided | `accepted` risks with empty `evidence` - shipped past knowingly, but the reason is not auditable |
| 7 | Stop gets considered | remaining exposure still high after repeated cycles or a pivot, every gate says `continue` - "do not build" is a legitimate outcome nobody weighed |
| 8 | The spiral earns its cost | every open exposure marginal - no real unknowns left; the correct move is to leave the spiral and just build |

For check 3, reconstruct the exposure timeline from the risks themselves:
a risk counts toward cycle N's open exposure when `cycle_opened <= N` and
(`cycle_closed` is null or `> N`).

## Verdict

One line per check - `PASS` or `FINDING`, citation attached:

```
1 scores-rank      FINDING  6 of 7 risks scored 0.5/0.5; only R3 differs
2 evidence-retires PASS     R1 "380MB RSS", R4 "auth handshake trace"
3 exposure-drains  FINDING  cycles 3-4: exposure 1.12 -> 1.12, cost +2.5
...
```

Then a stance, not a summary: the single most damaging finding, and the
smallest fix for it. Map fixes to the existing surface only - re-score
(`murex risk add` was honest, the numbers were not), record the missing
evidence (`murex risk close --evidence`), register the bypassed unknowns
(`murex risk add`), take the gate seriously next `murex commit`, or leave
the spiral (`murex stop --reason`).

## Boundaries

- **Never edit `.murex/spiral.json` by hand.** The audit reads the ledger;
  every correction goes through the binary or the human.
- The audit judges the run, not the objective. Whether the goal is worth
  pursuing belongs to the commitment review; whether the review was honest
  belongs here.
