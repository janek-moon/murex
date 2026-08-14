# Design: the `ratchet` mode, spiral repairs, and recognition

Date: 2026-08-14

## Motivation

murex today is a Boehm-spiral conductor: it only earns its keep when the work
has real unknowns. Its own `spiral` skill says so and disengages otherwise —
*"If the requirements are already clear, just build it - a spiral with no risks
is a slower waterfall."* That leaves the common case unserved: a feature whose
requirements are already clear, which still deserves discipline in **how** it is
built.

This change adds that discipline as a second mode, `ratchet`, and — because the
plugin's descriptions describe mechanism rather than trigger — rewrites the skill
descriptions so the right mode fires in the right situation. It also repairs two
real gaps in the existing spiral that the code review surfaced.

Scope (agreed):

1. **`ratchet` mode** — bottom-up, verification-gated construction, as a new
   binary command group plus a new skill.
2. **Recognition rewrite** — `spiral`, `audit`, and `ratchet` descriptions gain
   explicit "Use when…" triggers and cross-lane pointers; plugin/marketplace
   descriptions reflect both modes.
3. **Spiral gap 1** — make `pivot` real and stop `--alternative` being dead data.
4. **Spiral gap 2** — hand a drained spiral off to `ratchet` (de-risk → build).

Out of scope (deferred, see end).

---

## 1. The `ratchet` mode

### Concept

The mirror of the spiral. Where the spiral ranks by **exposure** and gates on a
**commitment review**, the ratchet ranks by **dependency depth** (build the
lowest parts first) and gates on **verification**. Its defining property, and
the reason it lives in the binary rather than a paragraph of advice: **you
cannot build on unverified ground, and a verification with no evidence is not a
verification.** A verified level locks and never slips back — hence "ratchet".

The loop, mirroring the spiral's `cycle → spike → commit`:

```
ratchet start "<feature>" [--requirement "<acceptance>"]
ratchet add "<component>" --requirement "<spec>" [--depends-on C1 …]   (decompose)
┌─ ratchet next ─────────────▶ build brief for the lowest buildable component
│  build (fresh subagent) ───▶ smallest impl satisfying the requirement + evidence
│  ratchet verify <id> --evidence "…"      (gate; rework if it failed)
└─ repeat until every component is verified → complete
ratchet status
```

Each build runs in a **fresh subagent** — clean context in, verified evidence
out — briefed with the component's requirement and the evidence/interfaces of
its already-verified dependencies. The conducting agent verifies that evidence
itself before closing the gate, exactly as the spiral verifies a spike.

### State

New file `.murex/ratchet.json`, independent of `.murex/spiral.json`. A repo may
run a spiral, a ratchet, or both; the two data models never mix. New Rust module
`src/ratchet.rs` mirrors `src/lib.rs`, reusing its plumbing conventions
(`round4`, `now`, `SpiralError`/`Result`, load/save, the argv-in/JSON-out
contract).

```
Component {
  id: String,            // "C1", "C2", … monotonic; never reused
  description: String,
  requirement: String,   // verifiable acceptance for THIS component
  depends_on: Vec<String>,
  status: String,        // "unbuilt" | "building" | "verified"
  evidence: String,      // set on verify
  step_built: Option<u32>,
  step_verified: Option<u32>,
}

Step {                   // history entry, mirror of spiral's Cycle
  n: u32,
  opened_at: String,
  component: String,     // id this step targets
  result: Option<String>,// None = pending; "verified" | "rework"
  cost: f64,
  note: String,          // rework reason, or verify outcome
  closed_at: Option<String>,
}

Ratchet {
  objective: String,
  requirement: String,   // overall acceptance (from `start --requirement`)
  created_at: String,
  status: String,        // "active" | "complete" | "stopped"
  step: u32,
  cumulative_cost: f64,
  components: Vec<Component>,
  steps: Vec<Step>,
  stopped_reason: Option<String>,
}
```

### Commands (`murex ratchet <action>`)

The existing spiral commands stay top-level (`murex start`, `murex cycle`, …) for
backward compatibility; the ratchet is a nested group.

| Command | Effect |
|---------|--------|
| `ratchet start "<feature>" [--requirement "<acc>"]` | Open a ratchet. Errors if `.murex/ratchet.json` exists. |
| `ratchet add "<desc>" --requirement "<spec>" [--depends-on C1 …]` | Register a component. Every `--depends-on` id **must already exist** (forward references rejected). |
| `ratchet next` | Open a step against the lowest buildable component; emit its build brief. |
| `ratchet verify <id> --evidence "<proof>" [--cost <n>]` | Gate (pass). Requires non-empty evidence. |
| `ratchet rework <id> [--note "<why>"] [--cost <n>]` | Build failed verification; return the component to `unbuilt` so it is picked again. |
| `ratchet list` | Components grouped: verified / building / buildable frontier / blocked. |
| `ratchet status` | Progress (verified/total), frontier, history, completion. |
| `ratchet stop [--reason "<why>"]` | Abandon the ratchet. |

`verify` and `rework` take the id of the component under the currently-open step
and error if it does not match (they close the pending step, so the id is a
guard against verifying the wrong thing).

### Enforced invariants (the reason it is a binary)

1. **No building on unverified ground.** `next` only ever returns a component
   whose every `depends_on` is `verified`.
2. **No evidence, no verification.** `verify` rejects empty `--evidence`.
3. **One step at a time.** `next` errors while a step is pending (mirror of the
   spiral's pending-cycle guard).
4. **Acyclic by construction.** `depends_on` must reference already-registered
   ids, so a cycle cannot be formed and no cycle-detection pass is needed.
   Unknown dep ids are rejected at `add`.
5. **Immutable ledger.** Components are never deleted; ids are monotonic;
   build/verify/rework history is preserved.
6. **A ratchet does not slip.** A `verified` component is final; there is no
   `reopen` in v1 (see deferred).
7. **Completion.** When every component is `verified`, status becomes `complete`.

### Selection and the always-non-empty frontier

`depth(c) = 0` if `depends_on` is empty, else `1 + max(depth(dep))`. Buildable =
`unbuilt` with all deps `verified`. `next` picks the buildable component of
minimum depth, ties broken by id order (mirror of the spiral's "highest
exposure, ties by id").

At `next` time no component is `building` (the prior step was closed by
`verify`/`rework`; `rework` returns its target to `unbuilt`). So every component
is `unbuilt` or `verified`, and the minimum-depth `unbuilt` component's deps —
being strictly lower depth — are all `verified`. Therefore whenever any
`unbuilt` component remains, the buildable frontier is non-empty: there is no
"stuck" state to error on. If no `unbuilt` component remains, the ratchet is
`complete`.

`next` with **no components registered at all** errors (`add` one first, mirror
of the spiral's "no open risks"); `complete` applies only once at least one
component exists and all are `verified`.

Because `rework` only ever targets the currently-building component, a verified
component's dependencies stay verified — no cascade, no slip.

### Build brief (from `next`)

```json
{
  "step": 3,
  "objective": "<feature>",
  "build": "<component description>",
  "component_id": "C3",
  "requirement": "<verifiable spec for C3>",
  "depends_on": [{"id": "C1", "description": "…", "evidence": "…"}],
  "instruction": "Step 3 builds exactly one component: <desc>. It must satisfy: <requirement>. Build the smallest implementation that satisfies it, then verify against that requirement. You may rely on these already-verified components: C1 (<evidence>). Do not broaden scope — other components get their own steps."
}
```

---

## 2. Recognition rewrite

Auto-invocation keys off the SKILL.md frontmatter `description`. The current
descriptions describe mechanism ("spike", "commitment review", "exposure
drained") and carry no trigger signal. All three descriptions are rewritten to:
"Use when …" + concrete situations and developer phrasings + a pointer to the
sibling mode so the model routes to the correct lane.

- **spiral** — "Use when a coding task has real unknowns — an unproven
  integration, an unvalidated performance target, a vendor API that may not
  behave as its docs claim — and you want to de-risk before committing. …
  **For clear, well-specified requirements, use `ratchet` instead.**"
- **ratchet** — "Use when requirements are clear and you want to build a feature
  the disciplined way — bottom-up, verifying each layer before building the
  next. … **For work with real unknowns or risk, use `spiral` instead.**"
- **audit** — keep the substance; prepend a trigger cue ("Use when you want to
  check whether a running or finished spiral stayed risk-driven or became an
  incremental build wearing spiral clothing").

`.claude-plugin/plugin.json` and `marketplace.json` descriptions are updated to
name both modes (risk-driven spiral **and** verification-gated ratchet). Skill
bodies are unchanged except where sections below require it.

---

## 3. Spiral gap 1 — make `pivot` real, revive `alternatives`

Confirmed in code: `Spiral.alternatives` is written by `start` (`lib.rs:203`) and
**read nowhere** — `cycle`, `commit`, `status`, `list`, and `brief` all ignore
it. `pivot` is in `DECISIONS` but `commit` treats it identically to `continue`
(only `stop` branches). The `spiral` skill documents `pivot` as "switch to an
alternative" — documented, unimplemented.

Changes (all backward-compatible via `#[serde(default)]` so existing
`spiral.json` files still load):

- `Spiral` gains `approach: Option<String>` — the currently adopted alternative
  (`None` = the original approach).
- `Cycle` gains `adopted: Option<String>` — the alternative a pivot switched to.
- `commit --decision pivot --adopt "<alternative>"` records the switch: sets
  `state.approach`, sets the cycle's `adopted`, and appends the value to
  `alternatives` if it is new (so an alternative discovered mid-spiral is
  captured without a separate command). `--adopt` is accepted only with
  `--decision pivot`.
- `status` output includes `alternatives` and the current `approach`.
- The cycle `brief` (from `cycle`) includes `alternatives` and `approach`, so a
  spike knows which approach it is de-risking.

No separate `alternative add` command — "adopt if new" covers discovery.

## 4. Spiral gap 2 — hand off to the ratchet

When a spiral drains (`remaining_exposure == 0`) while still `active`, the
requirements are as clear as they will get — the ratchet's entry condition.
Low-cost seam:

- `commit` and `status` include a `handoff` field when `remaining_exposure == 0`
  and status is `active`: e.g. `"requirements de-risked — run `murex ratchet
  start \"<objective>\"` to build it out"`.
- The `spiral` skill gains a short closing note pointing at `ratchet` once
  exposure is drained; the `ratchet` skill notes it can follow a drained spiral.

This makes the plugin a two-stage pipeline — de-risk with the spiral, build with
the ratchet — rather than two disconnected tools.

---

## 5. Release: the binary and the plugin move in lockstep

Two artifacts ship from this repo, versioned and distributed **independently**:

- the **binary** `murex` — per-platform GitHub Release assets, fetched by
  `install.sh`; version in `Cargo.toml`. `.github/workflows/release.yml` triggers
  on a `v*` tag push and publishes the assets.
- the **plugin / skills** — the Claude Code marketplace reads the repo directly
  (so a merge to `main` updates it), and `install.sh` symlinks `skills/*` for
  Codex; version in `plugin.json`.

Updating the plugin does **not** update the binary. This change touches both: the
new `ratchet` skill invokes `murex ratchet …`, and the `spiral` skill's `--adopt`
and `handoff` need the new binary. A new skill against an old binary breaks.
Requirements:

- Both versions bump to **0.5.0** (`Cargo.toml` and `plugin.json`) and release
  together.
- **Stale-binary guard.** Today the skill prerequisite installs only when `murex`
  is *missing* (`command -v murex || …`), so an existing 0.4.0 binary is never
  upgraded and `murex ratchet` would fail. The skills instead **feature-detect**
  the capability they need and reinstall a stale binary:
  - ratchet skill: `murex ratchet --help >/dev/null 2>&1 || curl -fsSL …/install.sh | sh`
  - spiral skill: guard `commit --decision pivot --adopt` the same way (detect the
    `--adopt` flag; reinstall if absent).
- Add `murex --version` — wire clap's `#[command(version)]` from
  `CARGO_PKG_VERSION` — for human visibility and any future version check.
- **Ordering.** Publish the 0.5.0 binary (push the `v0.5.0` tag) **before or with**
  the plugin update, so `murex ratchet` resolves the moment the new skill reaches
  users. Since both come from the same commit: merge to `main`, then tag that
  commit `v0.5.0` and push the tag.

---

## Files touched

New:
- `src/ratchet.rs` — ratchet engine.
- `tests/ratchet.rs` — ratchet integration tests.
- `skills/ratchet/SKILL.md` — the ratchet skill; prerequisite feature-detects
  `murex ratchet` and reinstalls a stale/missing binary.

Changed:
- `src/lib.rs` — gap 1 (approach/adopted, pivot handling, surface alternatives),
  gap 2 (handoff field).
- `src/main.rs` — `Ratchet` subcommand group; `--adopt` on `Commit`; wire
  `#[command(version)]` for `murex --version`.
- `tests/spiral.rs` — pivot/`--adopt` + handoff coverage; old-file load.
- `skills/spiral/SKILL.md` — description rewrite; pivot/`--adopt` doc; handoff
  note; prerequisite feature-detects the `--adopt` capability.
- `skills/audit/SKILL.md` — description trigger cue.
- `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json` — version 0.5.0
  (plugin.json) and two-mode descriptions.
- `Cargo.toml` — version 0.5.0.
- `README.md`, `README.ko.md` — document the ratchet and the handoff.
- `install.sh` — closing message lists `/murex:ratchet`. (Skill linking already
  loops over `skills/*/`, so the new skill is linked automatically.)

## Testing

Mirror `tests/spiral.rs` conventions (integration tests over the library API
with `tempfile`).

`tests/ratchet.rs`:
- `start` writes state; a second `start` errors.
- `add` rejects an unknown dep id; forward references are impossible (dep must
  pre-exist) → cycles cannot form.
- `next` picks the minimum-depth leaf; ties break by id.
- `next` errors while a step is pending.
- `next` never returns a component with an unverified dep (build-on-unverified
  ground is unreachable).
- `verify` rejects empty evidence; on pass, locks the component and advances the
  frontier.
- `rework` returns the component to `unbuilt`; `next` picks it again.
- Verifying the last component sets `complete`; a `verified` component stays
  verified.

`tests/spiral.rs` (additions):
- `commit --decision pivot --adopt X` sets `approach`, records `adopted`, appends
  X to `alternatives` when new; surfaced in `status`.
- a `spiral.json` without `approach`/`adopted` still deserializes (serde default).
- `handoff` appears once `remaining_exposure` hits 0 while active.

## Deferred (not in this change)

- **`ratchet-audit` skill** — the mirror of the spiral audit (catch vacuous
  evidence, rubber-stamped verifications). The binary already hard-enforces the
  core discipline, so it is a fast-follow, not a v1 requirement.
- **`ratchet --constraint`** — the component `requirement` carries acceptance for
  v1.
- **Reopening a verified component** — a ratchet does not slip by design; add a
  `reopen` later only if a real need appears.
- **Spiral gap 3 (exposure timeline in `status`) and gap 4 (`risk rescore`)** —
  considered and declined for this change.
