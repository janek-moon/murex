# Ratchet Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a verification-gated, bottom-up build mode (`ratchet`) to murex, repair the spiral's dead `pivot`/`alternatives`, hand a drained spiral off to the ratchet, and rewrite the skill descriptions so the right mode auto-fires.

**Architecture:** `ratchet` is the mirror of the existing spiral engine — a new self-contained Rust module (`src/ratchet.rs`, module `murex::ratchet`) with its own state file `.murex/ratchet.json`, reusing the spiral's plumbing (`round4`, `now`, `SpiralError`/`Result`, load/save, argv-in/JSON-out). Where the spiral ranks by exposure and gates on a commitment review, the ratchet ranks by dependency depth and gates on evidence-backed verification. The spiral's own commands stay top-level for backward compatibility; ratchet is a nested `murex ratchet <action>` group.

**Tech Stack:** Rust 2021, clap (derive), serde/serde_json, chrono; integration tests over the library API with `tempfile`. Skills are Markdown with YAML frontmatter; manifests are JSON.

## Global Constraints

- **Versions:** bump `Cargo.toml` and `.claude-plugin/plugin.json` to **0.5.0** together.
- **No new dependencies.** Mirror the spiral; reuse existing crates only.
- **Backward compatibility:** existing `.murex/spiral.json` files MUST still load — every new struct field is `#[serde(default, skip_serializing_if = "Option::is_none")]` (or `#[serde(default)]`). Existing top-level spiral commands (`murex start`, `murex cycle`, …) MUST keep working unchanged.
- **JSON-only protocol:** every command path prints JSON on stdout and nothing else; failures print `{"error": ...}` and exit 1 (see `src/main.rs:1-9`).
- **Component ids:** `C1`, `C2`, … monotonic, never reused (mirror of risk `R<n>`, `src/lib.rs:232-234`).
- **Ratchet states:** component `status` ∈ {`unbuilt`, `building`, `verified`}; ratchet `status` ∈ {`active`, `complete`, `stopped`}.
- **Test convention:** integration tests in `tests/<name>.rs` call the library API directly (`use murex::ratchet as rt;`), assert errors via a local `expect_err(result, needle)` helper, and read JSON via `serde_json::Value::pointer`. Mirror `tests/spiral.rs` exactly.
- **Plan convention:** every code step gives the *complete* test (the contract). For implementation bodies that are a mechanical mirror of the spiral, the step says exactly which `src/lib.rs` lines to mirror; novel logic (frontier/depth, gate, invariants) is given as full code. Reading `src/lib.rs` is expected — it is the reference implementation.

---

## File Structure

New:
- `src/ratchet.rs` — the ratchet engine (module `murex::ratchet`). One responsibility: the ratchet register, frontier selection, and the verification gate. Never executes work.
- `tests/ratchet.rs` — ratchet integration tests.
- `skills/ratchet/SKILL.md` — the ratchet skill (teaches the loop; prerequisite feature-detects the binary).

Modified:
- `src/lib.rs` — expose shared helpers as `pub(crate)`; add `pub mod ratchet;`; spiral gap 1 (`approach`/`adopted` fields, `pivot --adopt`, surface `alternatives`); gap 2 (`handoff`).
- `src/main.rs` — `Ratchet` subcommand group; `--adopt` on `Commit`; `#[command(version)]`.
- `tests/spiral.rs` — coverage for `--adopt`, old-file load, `handoff`.
- `skills/spiral/SKILL.md`, `skills/audit/SKILL.md` — description rewrites; spiral gets pivot/handoff docs + stale-binary guard.
- `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json` — 0.5.0 + two-mode descriptions.
- `Cargo.toml` — 0.5.0.
- `README.md`, `README.ko.md` — document the ratchet + handoff.
- `install.sh` — closing message lists `/murex:ratchet`.

**Library API locked here (used across tasks — keep signatures identical):**

```rust
// src/ratchet.rs — module murex::ratchet
pub const STATE_PATH: &str = ".murex/ratchet.json";

pub struct Component { pub id: String, pub description: String, pub requirement: String,
    pub depends_on: Vec<String>, pub status: String, pub evidence: String,
    pub step_built: Option<u32>, pub step_verified: Option<u32> }
pub struct Step { pub n: u32, pub opened_at: String, pub component: String,
    pub result: Option<String>, pub cost: f64, pub note: String, pub closed_at: Option<String> }
pub struct Ratchet { pub objective: String, pub requirement: String, pub created_at: String,
    pub status: String, pub step: u32, pub cumulative_cost: f64,
    pub components: Vec<Component>, pub steps: Vec<Step>, pub stopped_reason: Option<String> }

pub fn start(root: &Path, objective: &str, requirement: &str) -> Result<Value>;
pub fn add_component(root: &Path, description: &str, requirement: &str, depends_on: Vec<String>) -> Result<Value>;
pub fn open_step(root: &Path) -> Result<Value>;          // CLI: `murex ratchet next`
pub fn verify(root: &Path, id: &str, evidence: &str, cost: f64) -> Result<Value>;
pub fn rework(root: &Path, id: &str, note: &str, cost: f64) -> Result<Value>;
pub fn list(root: &Path) -> Result<Value>;
pub fn status(root: &Path) -> Result<Value>;
pub fn stop(root: &Path, reason: &str) -> Result<Value>;
pub fn load(root: &Path) -> Result<Ratchet>;
pub fn buildable_frontier(state: &Ratchet) -> Vec<&Component>;  // ranked: depth asc, id asc
```

---

## Task 1: Ratchet state, `start`, `add_component`

**Files:**
- Create: `src/ratchet.rs`
- Modify: `src/lib.rs` (expose shared helpers; register module)
- Test: `tests/ratchet.rs`

**Interfaces:**
- Consumes: from `src/lib.rs` — `Result`, `SpiralError` (already `pub`), plus the newly `pub(crate)` helpers `err`, `round4`, `now`. (`check_unit` and `id_order` are spiral-specific and are NOT reused; the ratchet defines its own `id_order` in Task 2.)
- Produces: `ratchet::{STATE_PATH, Component, Step, Ratchet, start, add_component, load}`.

- [ ] **Step 1: Expose shared helpers in `src/lib.rs`**

Change these three private items to `pub(crate)` so the ratchet module can reuse them (do NOT duplicate them). All three are used by the module across Tasks 1–4 (`round4` in `verify`, `now` throughout, `err` throughout):
- `fn err` (`src/lib.rs:39`) → `pub(crate) fn err`
- `fn round4` (`src/lib.rs:43`) → `pub(crate) fn round4`
- `fn now` (`src/lib.rs:47`) → `pub(crate) fn now`

Then add the module declaration near the top of `src/lib.rs` (after the `use` block, around line 18):

```rust
pub mod ratchet;
```

- [ ] **Step 2: Write the failing test** (`tests/ratchet.rs`)

```rust
//! Self-check for the ratchet controller. Run: cargo test

use std::path::Path;

use murex::ratchet as rt;
use tempfile::TempDir;

fn expect_err<T>(result: rt::Result<T>, needle: &str) {
    match result {
        Ok(_) => panic!("expected error containing {needle:?}"),
        Err(error) => assert!(
            error.to_string().contains(needle),
            "expected {needle:?} in {error}"
        ),
    }
}

#[test]
fn start_then_add_registers_components() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();

    // No ratchet yet.
    expect_err(rt::status(root), "no ratchet here");

    rt::start(root, "ship CSV export", "a user downloads a valid CSV").expect("start");
    // One ratchet per file.
    expect_err(rt::start(root, "again", ""), "already exists");
    assert!(root.join(".murex/ratchet.json").exists());

    // Empty description / requirement are rejected.
    expect_err(rt::add_component(root, "", "spec", vec![]), "must not be empty");
    expect_err(rt::add_component(root, "parser", "", vec![]), "requirement");

    // A leaf component.
    let c1 = rt::add_component(root, "CSV row encoder", "encodes a row to RFC-4180", vec![])
        .expect("C1");
    assert_eq!(c1.pointer("/component/id").unwrap(), "C1");
    assert_eq!(c1.pointer("/component/status").unwrap(), "unbuilt");

    // A dependent component — dep must already exist.
    expect_err(
        rt::add_component(root, "writer", "streams rows", vec!["C9".into()]),
        "unknown component",
    );
    let c2 = rt::add_component(root, "CSV writer", "writes all rows", vec!["C1".into()])
        .expect("C2");
    assert_eq!(c2.pointer("/component/id").unwrap(), "C2");
    assert_eq!(
        c2.pointer("/component/depends_on/0").unwrap(), "C1"
    );

    // Ids are monotonic and never reused.
    let state = rt::load(root).expect("load");
    assert_eq!(state.components.len(), 2);
    assert_eq!(state.objective, "ship CSV export");
    assert_eq!(state.requirement, "a user downloads a valid CSV");
    assert_eq!(state.status, "active");
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --test ratchet start_then_add_registers_components`
Expected: compile error / FAIL (module or functions not defined).

- [ ] **Step 4: Implement `src/ratchet.rs` — header, types, load/save, `start`, `add_component`**

Mirror the spiral's structure in `src/lib.rs`:
- Module header + `use` block: mirror `src/lib.rs:12-20` but `use crate::{err, now, round4, Result, SpiralError};` and `use serde::{Deserialize, Serialize}; use serde_json::{json, Value}; use std::fs; use std::path::{Path, PathBuf};`.
- `pub const STATE_PATH: &str = ".murex/ratchet.json";`
- Structs `Component`, `Step`, `Ratchet` exactly as in the locked API block above, each `#[derive(Debug, Clone, Serialize, Deserialize)]`.
- `impl Component` with `fn to_value(&self) -> Value` mirroring `Risk::to_value` (`src/lib.rs:74-78`), adding derived fields: `value["buildable"]` (bool — see Task 2 helper) is NOT added here to avoid needing state; add only `value` as-is for now. (Task 2 adds a state-aware view.)
- `state_file`, `load`, `save` — mirror `src/lib.rs:112-136`, swapping `STATE_PATH` and the "no ratchet here - run `murex ratchet start \"<objective>\"` first" / "corrupt state" messages.
- `fn component_index(state, id) -> Result<usize>` — mirror `risk_index` (`src/lib.rs:164-170`), error `"unknown component {id:?}"`.
- `pub fn start`:

```rust
pub fn start(root: &Path, objective: &str, requirement: &str) -> Result<Value> {
    if objective.trim().is_empty() {
        return err("objective must not be empty");
    }
    let path = state_file(root);
    if path.exists() {
        return err(format!(
            "ratchet already exists at {} - see `murex ratchet status`",
            path.display()
        ));
    }
    let state = Ratchet {
        objective: objective.to_string(),
        requirement: requirement.to_string(),
        created_at: now(),
        status: "active".to_string(),
        step: 0,
        cumulative_cost: 0.0,
        components: Vec::new(),
        steps: Vec::new(),
        stopped_reason: None,
    };
    save(root, &state)?;
    Ok(json!({
        "objective": objective,
        "status": "active",
        "state_file": path.display().to_string(),
        "next": "murex ratchet add \"<component>\" --requirement \"<spec>\"",
    }))
}
```

- `pub fn add_component`:

```rust
pub fn add_component(
    root: &Path,
    description: &str,
    requirement: &str,
    depends_on: Vec<String>,
) -> Result<Value> {
    if description.trim().is_empty() {
        return err("component description must not be empty");
    }
    if requirement.trim().is_empty() {
        return err("component requirement must not be empty");
    }
    let mut state = load(root)?;
    // Deps must already exist: forward references are impossible, so no cycle
    // can ever form and no cycle-detection pass is needed.
    for dep in &depends_on {
        if !state.components.iter().any(|c| &c.id == dep) {
            return err(format!("unknown component {dep:?} in --depends-on"));
        }
    }
    let component = Component {
        id: format!("C{}", state.components.len() + 1),
        description: description.to_string(),
        requirement: requirement.to_string(),
        depends_on,
        status: "unbuilt".to_string(),
        evidence: String::new(),
        step_built: None,
        step_verified: None,
    };
    let value = component.to_value();
    state.components.push(component);
    save(root, &state)?;
    Ok(json!({ "component": value }))
}

pub fn load(root: &Path) -> Result<Ratchet> { /* mirror src/lib.rs:116-125 */ }
```

Also implement `status` as a STUB that at least errors when no file exists, so the test's first assertion (`"no ratchet here"`) passes; the full `status` lands in Task 4. Minimal stub:

```rust
pub fn status(root: &Path) -> Result<Value> {
    let state = load(root)?;                 // errors "no ratchet here" when absent
    Ok(json!({ "objective": state.objective, "ratchet_status": state.status }))
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --test ratchet start_then_add_registers_components`
Expected: PASS. Also run `cargo build` to confirm the crate (incl. `src/main.rs`) still compiles.

- [ ] **Step 6: Commit**

```bash
git add src/ratchet.rs src/lib.rs tests/ratchet.rs
git commit -m "Add the ratchet register: start and add_component"
```

---

## Task 2: Ratchet `open_step` — frontier selection + build brief

**Files:**
- Modify: `src/ratchet.rs`
- Test: `tests/ratchet.rs`

**Interfaces:**
- Consumes: `ratchet::{Component, Ratchet, load, save, component_index}`.
- Produces: `ratchet::{open_step, buildable_frontier}`; internal `depth(&Ratchet, &Component) -> u32`, `pending_index(&Ratchet) -> Option<usize>`.

- [ ] **Step 1: Write the failing test** (append to `tests/ratchet.rs`)

```rust
#[test]
fn next_walks_the_frontier_bottom_up() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    rt::start(root, "feature", "acceptance").expect("start");

    // No components yet: next must refuse.
    expect_err(rt::open_step(root), "no components");

    // C1 leaf; C2 depends on C1; C3 depends on C2. Depth 0,1,2.
    rt::add_component(root, "leaf", "spec1", vec![]).expect("C1");
    rt::add_component(root, "mid", "spec2", vec!["C1".into()]).expect("C2");
    rt::add_component(root, "top", "spec3", vec!["C2".into()]).expect("C3");

    // The only buildable component is the leaf C1.
    let state = rt::load(root).unwrap();
    let frontier: Vec<String> = rt::buildable_frontier(&state)
        .iter().map(|c| c.id.clone()).collect();
    assert_eq!(frontier, ["C1"]);

    // `next` opens a step against C1 and briefs it.
    let opened = rt::open_step(root).expect("next");
    assert_eq!(opened.pointer("/step").unwrap(), 1);
    assert_eq!(opened.pointer("/brief/component_id").unwrap(), "C1");
    assert_eq!(opened.pointer("/brief/requirement").unwrap(), "spec1");
    // C1 is now building.
    assert_eq!(rt::load(root).unwrap().components[0].status, "building");

    // One step at a time.
    expect_err(rt::open_step(root), "still open");
}

#[test]
fn frontier_orders_by_depth_then_id() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    rt::start(root, "f", "a").expect("start");
    // Eleven leaves: tie-break must be C1 before C10 (numeric id order).
    for i in 1..=11 {
        rt::add_component(root, &format!("leaf {i}"), "spec", vec![]).expect("add");
    }
    let state = rt::load(root).unwrap();
    let ids: Vec<String> = rt::buildable_frontier(&state).iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids[..3], ["C1", "C2", "C3"]);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ratchet next_walks_the_frontier_bottom_up frontier_orders_by_depth_then_id`
Expected: FAIL (functions not defined).

- [ ] **Step 3: Implement selection + `open_step` in `src/ratchet.rs`**

```rust
/// Component ids are `C<n>`; compare the number so C10 sorts after C2.
fn id_order(id: &str) -> u32 {
    id.trim_start_matches('C').parse().unwrap_or(u32::MAX)
}

fn is_verified(state: &Ratchet, id: &str) -> bool {
    state.components.iter().any(|c| c.id == id && c.status == "verified")
}

/// Longest path to a leaf: 0 for no deps, else 1 + max(dep depth).
/// The DAG is acyclic by construction (deps pre-exist), so this terminates.
fn depth(state: &Ratchet, comp: &Component) -> u32 {
    comp.depends_on
        .iter()
        .filter_map(|d| state.components.iter().find(|c| &c.id == d))
        .map(|d| 1 + depth(state, d))
        .max()
        .unwrap_or(0)
}

/// Unbuilt components whose every dependency is verified, lowest depth first,
/// ties by id. The reason `next` can never hand back unverified ground.
pub fn buildable_frontier(state: &Ratchet) -> Vec<&Component> {
    let mut frontier: Vec<&Component> = state
        .components
        .iter()
        .filter(|c| c.status == "unbuilt" && c.depends_on.iter().all(|d| is_verified(state, d)))
        .collect();
    frontier.sort_by(|a, b| {
        depth(state, a)
            .cmp(&depth(state, b))
            .then_with(|| id_order(&a.id).cmp(&id_order(&b.id)))
    });
    frontier
}

fn pending_index(state: &Ratchet) -> Option<usize> {
    state.steps.iter().rposition(|s| s.result.is_none())
}

fn build_brief(state: &Ratchet, comp: &Component, step_n: u32) -> Value {
    let deps: Vec<Value> = comp
        .depends_on
        .iter()
        .filter_map(|d| state.components.iter().find(|c| &c.id == d))
        .map(|d| json!({ "id": d.id, "description": d.description, "evidence": d.evidence }))
        .collect();
    json!({
        "step": step_n,
        "objective": state.objective,
        "build": comp.description,
        "component_id": comp.id,
        "requirement": comp.requirement,
        "depends_on": deps,
        "instruction": format!(
            "Step {step_n} builds exactly one component: {} - it must satisfy: {}. \
             Build the smallest implementation that satisfies that requirement, then \
             verify against it. You may rely on the already-verified components listed \
             in depends_on. Do not broaden scope - other components get their own steps.",
            comp.description, comp.requirement,
        ),
    })
}

pub fn open_step(root: &Path) -> Result<Value> {
    let mut state = load(root)?;
    if state.status != "active" {
        return err(format!("ratchet is {} - no further steps", state.status));
    }
    if let Some(i) = pending_index(&state) {
        return err(format!(
            "step {} is still open - close it with `murex ratchet verify <id> --evidence` \
             or `murex ratchet rework <id>`",
            state.steps[i].n
        ));
    }
    if state.components.is_empty() {
        return err("no components - add one with `murex ratchet add`");
    }
    // Every component is unbuilt or verified here (no step is pending), so the
    // minimum-depth unbuilt component's deps are all verified: the frontier is
    // non-empty whenever any component is unbuilt.
    let top_id = match buildable_frontier(&state).first() {
        Some(c) => c.id.clone(),
        None => return err("all components verified - ratchet is complete"),
    };
    let idx = component_index(&state, &top_id)?;
    state.step += 1;
    let n = state.step;
    state.steps.push(Step {
        n,
        opened_at: now(),
        component: top_id.clone(),
        result: None,
        cost: 0.0,
        note: String::new(),
        closed_at: None,
    });
    state.components[idx].status = "building".to_string();
    state.components[idx].step_built = Some(n);
    let brief = build_brief(&state, &state.components[idx], n);
    save(root, &state)?;
    Ok(json!({
        "step": n,
        "brief": brief,
        "next": [
            "hand the brief to a fresh subagent and collect its evidence",
            format!("then `murex ratchet verify {top_id} --evidence \"<proof>\"`"),
        ],
    }))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ratchet`
Expected: PASS (all ratchet tests so far). Run `cargo build`.

- [ ] **Step 5: Commit**

```bash
git add src/ratchet.rs tests/ratchet.rs
git commit -m "Add ratchet next: bottom-up frontier selection and the build brief"
```

---

## Task 3: Ratchet `verify` + `rework` + completion (the gate)

**Files:**
- Modify: `src/ratchet.rs`
- Test: `tests/ratchet.rs`

**Interfaces:**
- Consumes: `ratchet::{load, save, component_index, pending_index}`.
- Produces: `ratchet::{verify, rework}`; internal `close_step(...)` helper optional.

- [ ] **Step 1: Write the failing test** (append to `tests/ratchet.rs`)

```rust
#[test]
fn verify_gates_on_evidence_and_ratchets_up() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    rt::start(root, "feature", "acceptance").expect("start");
    rt::add_component(root, "leaf", "spec1", vec![]).expect("C1");
    rt::add_component(root, "top", "spec2", vec!["C1".into()]).expect("C2");

    // Can't verify with no open step.
    expect_err(rt::verify(root, "C1", "proof", 1.0), "no open step");

    rt::open_step(root).expect("open C1");
    // Evidence is mandatory: a verification with no proof is not a verification.
    expect_err(rt::verify(root, "C1", "   ", 1.0), "evidence");
    // The id must match the open step's component.
    expect_err(rt::verify(root, "C2", "proof", 1.0), "open step targets");

    let v = rt::verify(root, "C1", "unit tests green, RFC-4180 sample matches", 1.0).expect("verify");
    assert_eq!(v.pointer("/result").unwrap(), "verified");
    assert_eq!(v.pointer("/ratchet_status").unwrap(), "active");
    let state = rt::load(root).unwrap();
    assert_eq!(state.components[0].status, "verified");
    assert_eq!(state.cumulative_cost, 1.0);

    // C2 is now buildable; build and verify it → ratchet completes.
    let opened = rt::open_step(root).expect("open C2");
    assert_eq!(opened.pointer("/brief/component_id").unwrap(), "C2");
    let done = rt::verify(root, "C2", "integration test writes 1000 rows", 2.0).expect("verify C2");
    assert_eq!(done.pointer("/ratchet_status").unwrap(), "complete");
    assert_eq!(rt::load(root).unwrap().status, "complete");

    // A completed ratchet takes no more steps; verified stays verified.
    expect_err(rt::open_step(root), "complete");
}

#[test]
fn rework_returns_a_component_to_the_frontier() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    rt::start(root, "feature", "acceptance").expect("start");
    rt::add_component(root, "leaf", "spec1", vec![]).expect("C1");

    rt::open_step(root).expect("open C1");
    let r = rt::rework(root, "C1", "encoder mangles embedded quotes", 0.5).expect("rework");
    assert_eq!(r.pointer("/result").unwrap(), "rework");
    // Back to unbuilt, and the failed attempt still cost radius.
    let state = rt::load(root).unwrap();
    assert_eq!(state.components[0].status, "unbuilt");
    assert_eq!(state.cumulative_cost, 0.5);
    // It is picked again.
    let ids: Vec<String> = rt::buildable_frontier(&state).iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids, ["C1"]);
    rt::open_step(root).expect("re-open C1");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ratchet verify_gates_on_evidence_and_ratchets_up rework_returns_a_component_to_the_frontier`
Expected: FAIL (functions not defined).

- [ ] **Step 3: Implement `verify` and `rework`**

```rust
/// Shared close: assert an open step exists and that it targets `id`.
fn take_pending<'a>(state: &'a Ratchet, id: &str) -> Result<usize> {
    let Some(i) = pending_index(state) else {
        return err("no open step - start one with `murex ratchet next`");
    };
    if state.steps[i].component != id {
        return err(format!(
            "open step targets {}, not {id:?}",
            state.steps[i].component
        ));
    }
    Ok(i)
}

pub fn verify(root: &Path, id: &str, evidence: &str, cost: f64) -> Result<Value> {
    if cost < 0.0 {
        return err(format!("cost must not be negative, got {cost}"));
    }
    if evidence.trim().is_empty() {
        return err("evidence must not be empty - a verification with no proof is not a verification");
    }
    let mut state = load(root)?;
    let si = take_pending(&state, id)?;
    let ci = component_index(&state, id)?;
    let n = state.steps[si].n;
    state.components[ci].status = "verified".to_string();
    state.components[ci].evidence = evidence.to_string();
    state.components[ci].step_verified = Some(n);
    let step = &mut state.steps[si];
    step.result = Some("verified".to_string());
    step.cost = cost;
    step.note = evidence.to_string();
    step.closed_at = Some(now());
    state.cumulative_cost = round4(state.cumulative_cost + cost);
    let verified = state.components.iter().filter(|c| c.status == "verified").count();
    let total = state.components.len();
    if verified == total {
        state.status = "complete".to_string();
    }
    save(root, &state)?;
    Ok(json!({
        "step": n,
        "component": id,
        "result": "verified",
        "cumulative_cost": state.cumulative_cost,
        "ratchet_status": state.status,
        "verified": verified,
        "total": total,
    }))
}

pub fn rework(root: &Path, id: &str, note: &str, cost: f64) -> Result<Value> {
    if cost < 0.0 {
        return err(format!("cost must not be negative, got {cost}"));
    }
    let mut state = load(root)?;
    let si = take_pending(&state, id)?;
    let ci = component_index(&state, id)?;
    let n = state.steps[si].n;
    // A ratchet does not slip a *verified* level; a failed build just returns
    // to the frontier to be picked again.
    state.components[ci].status = "unbuilt".to_string();
    state.components[ci].step_built = None;
    let step = &mut state.steps[si];
    step.result = Some("rework".to_string());
    step.cost = cost;
    step.note = note.to_string();
    step.closed_at = Some(now());
    state.cumulative_cost = round4(state.cumulative_cost + cost);
    save(root, &state)?;
    Ok(json!({
        "step": n,
        "component": id,
        "result": "rework",
        "cumulative_cost": state.cumulative_cost,
        "note": note,
    }))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ratchet`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ratchet.rs tests/ratchet.rs
git commit -m "Add the ratchet gate: verify (evidence-required) and rework"
```

---

## Task 4: Ratchet `list`, `status`, `stop`

**Files:**
- Modify: `src/ratchet.rs` (replace the Task 1 `status` stub)
- Test: `tests/ratchet.rs`

**Interfaces:**
- Consumes: `ratchet::{load, save, buildable_frontier, depth, pending_index}`.
- Produces: `ratchet::{list, status, stop}` (final forms).

- [ ] **Step 1: Write the failing test** (append to `tests/ratchet.rs`)

```rust
#[test]
fn status_reports_progress_and_frontier() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    rt::start(root, "feature", "acceptance").expect("start");
    rt::add_component(root, "leaf", "spec1", vec![]).expect("C1");
    rt::add_component(root, "top", "spec2", vec!["C1".into()]).expect("C2");

    let s = rt::status(root).expect("status");
    assert_eq!(s.pointer("/ratchet_status").unwrap(), "active");
    assert_eq!(s.pointer("/progress/verified").unwrap(), 0);
    assert_eq!(s.pointer("/progress/total").unwrap(), 2);
    // Only the leaf is on the frontier.
    assert_eq!(s.pointer("/frontier/0").unwrap(), "C1");

    rt::open_step(root).expect("open");
    rt::verify(root, "C1", "green", 1.0).expect("verify");
    let s2 = rt::status(root).expect("status");
    assert_eq!(s2.pointer("/progress/verified").unwrap(), 1);
    assert_eq!(s2.pointer("/frontier/0").unwrap(), "C2"); // frontier advanced

    // list groups by state.
    let l = rt::list(root).expect("list");
    assert_eq!(l.pointer("/verified/0/id").unwrap(), "C1");
    assert_eq!(l.pointer("/frontier/0/id").unwrap(), "C2");

    // stop ends it.
    rt::stop(root, "descoped").expect("stop");
    assert_eq!(rt::load(root).unwrap().status, "stopped");
    assert_eq!(rt::status(root).unwrap().pointer("/ratchet_status").unwrap(), "stopped");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test ratchet status_reports_progress_and_frontier`
Expected: FAIL (`list`/`stop` undefined; `status` stub returns wrong shape).

- [ ] **Step 3: Replace the `status` stub and add `list`, `stop`**

```rust
pub fn status(root: &Path) -> Result<Value> {
    let state = load(root)?;
    let verified = state.components.iter().filter(|c| c.status == "verified").count();
    let total = state.components.len();
    let unbuilt = state.components.iter().filter(|c| c.status == "unbuilt").count();
    let frontier: Vec<String> =
        buildable_frontier(&state).iter().map(|c| c.id.clone()).collect();
    let components: Vec<Value> = state
        .components
        .iter()
        .map(|c| json!({
            "id": c.id, "description": c.description, "status": c.status,
            "depth": depth(&state, c),
            "depends_on": c.depends_on,
        }))
        .collect();
    let history: Vec<Value> = state
        .steps
        .iter()
        .map(|s| json!({ "n": s.n, "component": s.component, "result": s.result, "cost": s.cost }))
        .collect();
    Ok(json!({
        "objective": state.objective,
        "requirement": state.requirement,
        "ratchet_status": state.status,
        "step": state.step,
        "radius": {
            "steps_completed": state.steps.iter().filter(|s| s.result.is_some()).count(),
            "cumulative_cost": state.cumulative_cost,
        },
        "progress": { "verified": verified, "total": total, "unbuilt": unbuilt },
        "frontier": frontier,
        "components": components,
        "pending_step": pending_index(&state).map(|i| state.steps[i].n),
        "history": history,
    }))
}

pub fn list(root: &Path) -> Result<Value> {
    let state = load(root)?;
    let frontier_ids: Vec<String> =
        buildable_frontier(&state).iter().map(|c| c.id.clone()).collect();
    let pick = |pred: &dyn Fn(&Component) -> bool| -> Vec<Value> {
        state.components.iter().filter(|c| pred(c))
            .map(|c| json!({ "id": c.id, "description": c.description,
                             "status": c.status, "depth": depth(&state, c) }))
            .collect()
    };
    Ok(json!({
        "verified": pick(&|c| c.status == "verified"),
        "building": pick(&|c| c.status == "building"),
        "frontier": pick(&|c| frontier_ids.contains(&c.id)),
        "blocked":  pick(&|c| c.status == "unbuilt" && !frontier_ids.contains(&c.id)),
    }))
}

pub fn stop(root: &Path, reason: &str) -> Result<Value> {
    let mut state = load(root)?;
    state.status = "stopped".to_string();
    state.stopped_reason = Some(reason.to_string());
    save(root, &state)?;
    Ok(json!({ "ratchet_status": "stopped", "reason": reason }))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --test ratchet`
Expected: PASS (all ratchet tests). Run `cargo build`.

- [ ] **Step 5: Commit**

```bash
git add src/ratchet.rs tests/ratchet.rs
git commit -m "Add ratchet observability: list, status, stop"
```

---

## Task 5: CLI wiring — `murex ratchet <action>` + `murex --version`

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: everything from `murex::ratchet`.
- Produces: the `murex ratchet …` CLI surface; `murex --version`.

- [ ] **Step 1: Add `version` to the top-level command**

In `src/main.rs`, change the `#[command(...)]` on `struct Cli` (`src/main.rs:19`):

```rust
#[command(name = "murex", version, about = "Risk-driven spiral-model cycles.")]
```

(`version` with no value pulls `CARGO_PKG_VERSION`.)

- [ ] **Step 2: Add the `Ratchet` subcommand group**

Add to `enum Command` (`src/main.rs:28-68`):

```rust
    /// Bottom-up, verification-gated build for clear requirements.
    Ratchet {
        #[command(subcommand)]
        action: RatchetAction,
    },
```

Add a new enum near `RiskAction`:

```rust
#[derive(Subcommand)]
enum RatchetAction {
    /// Open a ratchet for a feature with clear requirements.
    Start {
        objective: String,
        #[arg(long, default_value = "")]
        requirement: String,
    },
    /// Register a component and its dependencies (which must already exist).
    Add {
        description: String,
        #[arg(long)]
        requirement: String,
        #[arg(long = "depends-on")]
        depends_on: Vec<String>,
    },
    /// Emit the build brief for the lowest buildable component.
    Next,
    /// Gate: mark a component verified (evidence required).
    Verify {
        id: String,
        #[arg(long)]
        evidence: String,
        #[arg(long, default_value_t = 0.0)]
        cost: f64,
    },
    /// A build failed verification; return it to the frontier.
    Rework {
        id: String,
        #[arg(long, default_value = "")]
        note: String,
        #[arg(long, default_value_t = 0.0)]
        cost: f64,
    },
    /// Components grouped by state.
    List,
    /// Progress, frontier, and history.
    Status,
    /// Abandon the ratchet.
    Stop {
        #[arg(long, default_value = "")]
        reason: String,
    },
}
```

- [ ] **Step 3: Dispatch it**

In `fn dispatch` (`src/main.rs:91-124`), add an arm mirroring the spiral arms:

```rust
        Command::Ratchet { action } => match action {
            RatchetAction::Start { objective, requirement } =>
                spiral::ratchet::start(root, objective, requirement),
            RatchetAction::Add { description, requirement, depends_on } =>
                spiral::ratchet::add_component(root, description, requirement, depends_on.clone()),
            RatchetAction::Next => spiral::ratchet::open_step(root),
            RatchetAction::Verify { id, evidence, cost } =>
                spiral::ratchet::verify(root, id, evidence, *cost),
            RatchetAction::Rework { id, note, cost } =>
                spiral::ratchet::rework(root, id, note, *cost),
            RatchetAction::List => spiral::ratchet::list(root),
            RatchetAction::Status => spiral::ratchet::status(root),
            RatchetAction::Stop { reason } => spiral::ratchet::stop(root, reason),
        },
```

(`use murex as spiral;` already aliases the crate at `src/main.rs:16`.)

- [ ] **Step 4: Build and smoke-test the CLI end-to-end**

```bash
cargo build
T=$(mktemp -d)
./target/debug/murex --version
./target/debug/murex --root "$T" ratchet start "demo" --requirement "it works"
./target/debug/murex --root "$T" ratchet add "leaf" --requirement "spec1"
./target/debug/murex --root "$T" ratchet add "top" --requirement "spec2" --depends-on C1
./target/debug/murex --root "$T" ratchet next
./target/debug/murex --root "$T" ratchet verify C1 --evidence "green" --cost 1
./target/debug/murex --root "$T" ratchet status
rm -rf "$T"
```
Expected: `--version` prints `murex 0.5.0` (after Task 9 version bump; `0.4.0` until then — acceptable here). Each ratchet command prints JSON; `next` briefs C1; `status` shows frontier `["C2"]` after verifying C1.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "Wire the ratchet CLI group and murex --version"
```

---

## Task 6: Spiral gap 1 — make `pivot` real, surface `alternatives`

**Files:**
- Modify: `src/lib.rs`, `src/main.rs`
- Test: `tests/spiral.rs`

**Interfaces:**
- Consumes: existing `spiral::{start, add_risk, open_cycle, commit, status, load}`.
- Produces: `commit(root, decision, cost, outcome, resolve, evidence, adopt: &str)` — **note the new trailing `adopt` parameter**; `Spiral.approach: Option<String>`, `Cycle.adopted: Option<String>`.

- [ ] **Step 1: Write the failing test** (append to `tests/spiral.rs`)

```rust
#[test]
fn pivot_adopts_an_alternative_and_surfaces_it() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    sp::start(root, "editing", vec![], vec!["CRDT".into(), "OT".into()]).expect("start");
    sp::add_risk(root, "CRDT memory", 0.6, 0.9, "").expect("R1");
    sp::open_cycle(root, vec![]).expect("cycle 1");

    // Adopting an alternative is only valid on a pivot.
    expect_err(
        sp::commit(root, "continue", 1.0, "", vec![], "", "OT"),
        "--adopt requires --decision pivot",
    );

    // Pivot to OT: recorded on the cycle and as the spiral's current approach.
    sp::commit(root, "pivot", 1.0, "CRDT too heavy", vec![], "", "OT").expect("pivot");
    let status = sp::status(root).expect("status");
    assert_eq!(status.pointer("/approach").unwrap(), "OT");
    let alts = status.pointer("/alternatives").unwrap().as_array().unwrap();
    assert!(alts.iter().any(|a| a == "OT"));

    // Pivoting to a newly-discovered approach appends it to alternatives.
    sp::open_cycle(root, vec![]).expect("cycle 2");
    sp::commit(root, "pivot", 1.0, "", vec![], "", "server-authoritative").expect("pivot 2");
    let alts2 = sp::load(root).unwrap().alternatives;
    assert!(alts2.iter().any(|a| a == "server-authoritative"));
    // The brief carries the current approach so a spike knows its context.
    let opened = sp::open_cycle(root, vec![]).expect("cycle 3");
    assert_eq!(opened.pointer("/brief/approach").unwrap(), "server-authoritative");
}

#[test]
fn old_spiral_state_without_approach_still_loads() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".murex")).unwrap();
    // A 0.4.0 ledger: no `approach`, cycles have no `adopted`.
    std::fs::write(root.join(".murex/spiral.json"), r#"{
      "objective":"legacy","created_at":"2026-01-01T00:00:00Z","status":"active",
      "cycle":0,"cumulative_cost":0.0,"constraints":[],"alternatives":[],
      "risks":[],"cycles":[]
    }"#).unwrap();
    let state = sp::load(root).expect("legacy loads");
    assert_eq!(state.objective, "legacy");
    assert!(state.approach.is_none());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test spiral pivot_adopts_an_alternative_and_surfaces_it old_spiral_state_without_approach_still_loads`
Expected: FAIL (compile error: `commit` takes 6 args, `approach` field missing).

- [ ] **Step 3: Add fields (backward-compatible)**

In `src/lib.rs`, add to `struct Spiral` (after `stopped_reason`, `src/lib.rs:108-109`):

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approach: Option<String>,
```

Add to `struct Cycle` (near `closed_at`, `src/lib.rs:91-94`):

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted: Option<String>,
```

Initialize `approach: None` in `start` (`src/lib.rs:196-207`) and `adopted: None` wherever a `Cycle` is constructed (`open_cycle`, `src/lib.rs:321-336`).

- [ ] **Step 4: Thread `adopt` through `commit`**

Change the signature (`src/lib.rs:356-363`) to add a trailing `adopt: &str`:

```rust
pub fn commit(root: &Path, decision: &str, cost: f64, outcome: &str,
    resolve: Vec<String>, evidence: &str, adopt: &str) -> Result<Value> {
```

After the decision/cost validation and before loading state, guard:

```rust
    if !adopt.is_empty() && decision != "pivot" {
        return err("--adopt requires --decision pivot");
    }
```

After `entry.closed_at = Some(now());` and before the `if decision == "stop"` block (`src/lib.rs:386-396`), record the adoption:

```rust
    if decision == "pivot" && !adopt.is_empty() {
        state.cycles[index].adopted = Some(adopt.to_string());
        state.approach = Some(adopt.to_string());
        if !state.alternatives.iter().any(|a| a == adopt) {
            state.alternatives.push(adopt.to_string());
        }
    }
```

(`index` is the pending cycle index resolved earlier at `src/lib.rs:371-374`; place this after `entry` mutations finish so the borrow of `state.cycles[index]` via `entry` is dropped — reborrow `state.cycles[index]` fresh as shown.)

- [ ] **Step 5: Surface `alternatives`/`approach` in `status` and `brief`**

In `status` (the final `Ok(json!({...}))`, `src/lib.rs:442-455`), add:

```rust
        "alternatives": state.alternatives,
        "approach": state.approach,
```

In `brief` (`src/lib.rs:277-295`), add to the JSON object:

```rust
        "alternatives": state.alternatives,
        "approach": state.approach,
```

- [ ] **Step 6: Update the `commit` call site in `src/main.rs`**

Add `--adopt` to the `Commit` variant (`src/main.rs:48-60`):

```rust
        #[arg(long, default_value = "")]
        adopt: String,
```

Destructure and pass it (`src/main.rs:114-120`):

```rust
        Command::Commit { decision, cost, outcome, resolve, evidence, adopt } =>
            spiral::commit(root, decision, *cost, outcome, resolve.clone(), evidence, adopt),
```

- [ ] **Step 7: Fix the existing `commit` test calls**

`tests/spiral.rs` already calls `sp::commit(...)` with 6 args in several places (`src/../tests/spiral.rs:65-92,105,118`). Add a trailing `""` to each existing call so they compile. (Do NOT change their behavior.)

- [ ] **Step 8: Run to verify all spiral tests pass**

Run: `cargo test --test spiral`
Expected: PASS (old + new).

- [ ] **Step 9: Commit**

```bash
git add src/lib.rs src/main.rs tests/spiral.rs
git commit -m "Make spiral pivot real: --adopt records the approach, surface alternatives"
```

---

## Task 7: Spiral gap 2 — hand a drained spiral off to the ratchet

**Files:**
- Modify: `src/lib.rs`
- Test: `tests/spiral.rs`

**Interfaces:**
- Consumes: `total_exposure` (`src/lib.rs:154-158`), `commit`, `status`.
- Produces: a `handoff` field on `commit` and `status` output when exposure is drained.

- [ ] **Step 1: Write the failing test** (append to `tests/spiral.rs`)

```rust
#[test]
fn drained_spiral_points_at_the_ratchet() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    sp::start(root, "ship export", vec![], vec![]).expect("start");
    sp::add_risk(root, "only risk", 0.5, 0.5, "").expect("R1");
    sp::open_cycle(root, vec![]).expect("cycle");
    let done = sp::commit(root, "continue", 1.0, "", vec!["R1".into()], "ok", "").expect("commit");
    // Exposure is drained → requirements are clear → point at the ratchet.
    assert_eq!(number(&done, "/remaining_exposure"), 0.0);
    let handoff = done.pointer("/handoff").unwrap().as_str().unwrap();
    assert!(handoff.contains("murex ratchet start"));
    // status echoes it while the spiral is still active and drained.
    let s = sp::status(root).expect("status");
    assert!(s.pointer("/handoff").unwrap().as_str().unwrap().contains("ratchet"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test spiral drained_spiral_points_at_the_ratchet`
Expected: FAIL (no `handoff` field).

- [ ] **Step 3: Implement the handoff helper and wire it in**

Add near `total_exposure` in `src/lib.rs`:

```rust
/// When exposure is drained and the spiral is still active, the remaining
/// unknowns are gone: the natural next step is to build, which is the ratchet.
fn handoff(state: &Spiral) -> Option<String> {
    if state.status == "active" && total_exposure(state) == 0.0 {
        Some(format!(
            "requirements de-risked - run `murex ratchet start \"{}\"` to build it out",
            state.objective
        ))
    } else {
        None
    }
}
```

In `commit`'s result object (`src/lib.rs:398-405`), add:

```rust
        "handoff": handoff(&state),
```

In `status`'s result object (`src/lib.rs:442-455`), add:

```rust
        "handoff": handoff(&state),
```

(`serde_json` renders `None` as `null`; the test only reads `handoff` when it expects a string, so `null` otherwise is fine.)

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test`
Expected: PASS (whole suite — spiral + ratchet).

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs tests/spiral.rs
git commit -m "Hand a drained spiral off to the ratchet"
```

---

## Task 8: The `ratchet` skill

**Files:**
- Create: `skills/ratchet/SKILL.md`

**Interfaces:**
- Consumes: the `murex ratchet …` CLI (Tasks 1–5).
- Produces: the auto-invocable skill; no code depends on it.

- [ ] **Step 1: Write `skills/ratchet/SKILL.md`**

Mirror the shape of `skills/spiral/SKILL.md` (frontmatter, Description, Prerequisites, The loop, numbered steps, Notes). Frontmatter description MUST lead with triggers and point to the sibling mode:

```markdown
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

​```
start -> add* (decompose) -> [ next -> build (subagent) -> verify ]* -> complete
​```

### 1. Open the ratchet

​```bash
murex --root . ratchet start "ship CSV export" --requirement "a user downloads a valid CSV of their data"
​```

### 2. Decompose into verifiable components (leaves first)

Register the lowest parts first; a component may only depend on ids that already
exist. Each `--requirement` must be checkable.

​```bash
murex --root . ratchet add "CSV row encoder" --requirement "encodes one record to an RFC-4180 line, quotes embedded commas/quotes"
murex --root . ratchet add "CSV writer" --requirement "streams all rows with a header" --depends-on C1
murex --root . ratchet add "export endpoint" --requirement "GET /export returns text/csv for the current user" --depends-on C2
​```

### 3. Take the next buildable component

​```bash
murex --root . ratchet next
​```

Returns the build brief for the lowest buildable component - its requirement and
the evidence/interfaces of its already-verified dependencies.

### 4. Build it in a fresh subagent

Dispatch a fresh subagent with the brief and nothing else: it builds the
smallest implementation that satisfies the requirement and reports its evidence
(tests run, output shown). Your context stays on the register.

### 5. Verify, then gate

When it returns, **verify the evidence yourself** - run the checks - before
touching the gate.

​```bash
murex --root . ratchet verify C1 --evidence "cargo test csv_encoder green; RFC-4180 sample matches byte-for-byte" --cost 1.0
​```

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
```

(Note: the three `​``` ` fences above are shown with a zero-width mark so they render inside this plan; write them as ordinary triple-backticks in the SKILL.md.)

- [ ] **Step 2: Verify the skill file is well-formed**

Run: `head -5 skills/ratchet/SKILL.md` and confirm the frontmatter `name: ratchet` and a `description:` line are present and the file has real triple-backtick code fences.

- [ ] **Step 3: Commit**

```bash
git add skills/ratchet/SKILL.md
git commit -m "Add the ratchet skill"
```

---

## Task 9: Recognition rewrite, manifests, docs, version bumps

**Files:**
- Modify: `skills/spiral/SKILL.md`, `skills/audit/SKILL.md`, `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`, `Cargo.toml`, `README.md`, `README.ko.md`, `install.sh`

**Interfaces:**
- Consumes: nothing (content pass).
- Produces: the shipped 0.5.0 surface.

- [ ] **Step 1: Rewrite the `spiral` skill description + add pivot/handoff docs + stale-binary guard**

In `skills/spiral/SKILL.md`:
- Replace the frontmatter `description` (line 3) with a trigger-led one that points at the ratchet:

```
description: "Use when a coding task has real unknowns - an unproven integration, an unvalidated performance target, a vendor API that may not behave as its docs claim - and you want to de-risk before committing. Runs risk-driven spiral-model development: register and score the unknowns, spike the largest risk each cycle in a fresh subagent, gate on a commitment review, repeat until the exposure is drained. For clear, well-specified requirements, use ratchet instead."
```

- Change the Prerequisites check (line 34-36) to feature-detect `--adopt` (a 0.5.0 capability) rather than mere presence:

```bash
murex commit --help 2>/dev/null | grep -q -- --adopt || curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh
```

- In the commitment-review section (the `--decision` table, lines 93-103), document `--adopt`:

  > On `pivot`, name the alternative you are switching to with `--adopt "<alternative>"`; it becomes the spiral's current approach and is added to the alternatives list if new. `murex commit --decision pivot --adopt "OT with a central server" --cost 1.0 --outcome "CRDT memory too high"`.

- Add a short closing subsection after "Repeat until drained" (after line 117):

  > ### When the exposure drains
  >
  > A drained spiral (`remaining_exposure` 0) means the unknowns are retired and
  > the requirements are now clear. `status` prints a `handoff` line: switch to
  > `/murex:ratchet` to build the de-risked feature bottom-up, verifying each layer.

- [ ] **Step 2: Rewrite the `audit` skill description**

In `skills/audit/SKILL.md`, prepend a trigger cue to the frontmatter `description` (line 3), keeping the rest:

```
description: "Use when you want to check whether a running or finished spiral stayed risk-driven or became an incremental build wearing spiral clothing. Audits a spiral against the risk-driven invariants the binary cannot enforce: scores that actually rank, gate evidence that actually retires risks, exposure that actually drains, a stop that gets considered. Verdicts cite the ledger."
```

- [ ] **Step 3: Update the manifests to 0.5.0 + two-mode descriptions**

`.claude-plugin/plugin.json`:
- `"version": "0.4.0"` → `"0.5.0"`.
- `"description"` → `"Two disciplined build loops for coding agents: a risk-driven spiral (spike the largest unknown each cycle) for unclear requirements, and a verification-gated ratchet (build bottom-up, verify each layer) for clear ones."`

`.claude-plugin/marketplace.json`:
- top-level `"description"` and the plugin entry `"description"` → the same two-mode summary (keep them consistent, ≤ ~180 chars each).

- [ ] **Step 4: Bump `Cargo.toml`**

`version = "0.4.0"` → `version = "0.5.0"` (line 3). Leave `Cargo.lock` to `cargo build` (next step regenerates the `murex` entry).

- [ ] **Step 5: Update both READMEs**

- `README.md`: under "How it works", add a short "Two modes" note and a ratchet diagram mirroring the spiral one; document `murex ratchet` in "Use"; mention the spiral→ratchet handoff; list `/murex:ratchet` alongside `/murex:spiral` and `/murex:audit`.
- `README.ko.md`: the same, in Korean, mirroring the English structure.

(Keep it factual and short — a paragraph plus the command block. Match the existing tone.)

- [ ] **Step 6: Update `install.sh` closing message**

`install.sh:77` — change the hint line to list the ratchet:

```sh
echo "Done. In Claude Code: /murex:spiral, /murex:ratchet, /murex:audit. By hand:"
```

- [ ] **Step 7: Verify the whole build, tests, and JSON**

```bash
cargo build && cargo test
python3 -c "import json;[json.load(open(p)) for p in ['.claude-plugin/plugin.json','.claude-plugin/marketplace.json']];print('json ok')"
./target/debug/murex --version   # -> murex 0.5.0
```
Expected: build + full test suite PASS; `json ok`; version prints `0.5.0`.

- [ ] **Step 8: Commit**

```bash
git add skills/spiral/SKILL.md skills/audit/SKILL.md .claude-plugin/plugin.json .claude-plugin/marketplace.json Cargo.toml Cargo.lock README.md README.ko.md install.sh
git commit -m "Recognition rewrite, two-mode docs, and the 0.5.0 bump"
```

---

## Self-Review (completed during planning)

- **Spec coverage:** ratchet mode → Tasks 1–5, 8; recognition → Task 9 (+ ratchet description in Task 8); spiral gap 1 → Task 6; spiral gap 2 → Task 7; release/version/stale-binary guard → Tasks 5, 8, 9. All spec sections map to a task.
- **Type consistency:** the locked API block is the single source of truth; `open_step` (lib) ↔ `next` (CLI), `verify`/`rework` take `(id, …, cost)`, `commit` gains a trailing `adopt: &str` used identically in `src/main.rs` and all `tests/spiral.rs` call sites (Task 6 Step 7 fixes the existing 6-arg calls).
- **No placeholders:** every code step carries complete test code; mechanical mirrors cite exact `src/lib.rs` lines to copy; novel logic (frontier/depth, gate, handoff, pivot) is given in full.

## Release (after the plan is merged — not a code task)

Per the spec's §5: merge to `main` (updates the Claude Code plugin + Codex skills), then tag that commit and push to publish the binary:

```bash
git tag v0.5.0 && git push origin v0.5.0   # release.yml builds + uploads per-platform binaries
```
Confirm the GitHub release shows 0.5.0 assets before announcing, so the skills' feature-detect reinstall resolves to the new binary.
