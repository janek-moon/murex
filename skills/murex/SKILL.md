---
name: murex
description: "Run risk-driven spiral-model cycles: register risks, de-risk the largest one per cycle, gate on a commitment review"
---

# ooo murex - Risk-Driven Spiral Cycles

## Description

`ooo evolve` iterates until a quality gate passes. `ooo murex` iterates until
the **risks** are retired. Each cycle exists to produce evidence about exactly
one risk - the highest-exposure open one - and every cycle ends in a
commitment review that decides whether the next one is worth its cost.

Use this skill when the work has real unknowns: an unproven integration, a
performance target nobody has measured, a vendor API that may not do what the
docs claim. If the requirements are already clear, use `ooo auto` instead -
a spiral with no risks is just a slower waterfall.

This plugin does not execute work. `ooo murex cycle` emits a **spike brief**;
hand that brief to `ooo auto` or `ooo run`, then report back with
`ooo murex commit`.

## Flow

```
start(objective)
  -> risk add (one entry per unknown, scored probability x impact)
  -> cycle          # picks the top-exposure risk, emits the spike brief
  -> [ooo auto/run] # you execute the brief through the normal runtime
  -> commit         # decision + cost; resolves the risk if evidence held
  -> cycle ...      # repeat; radius grows with cost, exposure should fall
```

The spiral converges when `remaining_exposure` reaches 0, and terminates early
whenever a commitment review returns `stop`.

## Usage

### 1. Open the spiral

```bash
ooo murex start "ship realtime collaborative editing" \
  --constraint "must stay on the existing Postgres box" \
  --alternative "CRDT" --alternative "OT with a central server"
```

### 2. Register what you do not know

Score each risk with `probability` (0..1) that it bites, and `impact` (0..1)
of the damage if it does. Exposure is their product; only the ranking matters,
so score consistently rather than precisely.

```bash
ooo murex risk add "CRDT memory footprint may exceed the 2GB box limit" \
  --probability 0.6 --impact 0.9 \
  --mitigation "prototype with a 10k-op document, measure RSS"
ooo murex risk add "Vendor websocket SDK may not support our auth scheme" \
  --probability 0.4 --impact 0.7
ooo murex risk list
```

### 3. Open a cycle and read the brief

```bash
ooo murex cycle
```

Returns the brief for the top risk. Execute it through the normal runtime -
build the smallest prototype that produces evidence, and nothing more.

### 4. Commitment review

```bash
ooo murex commit --decision continue --cost 1.5 \
  --resolve R1 --evidence "10k-op doc held at 380MB RSS; headroom is fine" \
  --outcome "CRDT approach viable"
```

`--decision` is the gate:

| Decision   | Meaning                                                        |
|------------|----------------------------------------------------------------|
| `continue` | Risk retired or reduced; the next cycle is worth its cost.      |
| `pivot`    | Evidence killed the current approach; switch to an alternative. |
| `stop`     | The objective is not worth the remaining exposure. Spiral ends. |

Omit `--resolve` when the spike was inconclusive - the risk stays open and
will be picked again, which is the correct signal that it needs another cycle.

### 5. Check the radius

```bash
ooo murex status
```

Reports cycles completed, `cumulative_cost` (the spiral's radius), and
`remaining_exposure`. Falling exposure against rising cost is convergence;
flat exposure across two cycles means the spikes are not producing evidence -
reframe the risk before spending another cycle.

## Notes

- State lives in `.murex/spiral.json`; commit it to share the register.
- Risks are never deleted, only `resolved` or `accepted`, so the history of
  what you knowingly shipped past stays auditable.
- `accepted` is a real option: use it for risks you have decided to live with
  rather than spend a cycle on.
