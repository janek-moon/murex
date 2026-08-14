//! Ratchet-mode component controller for coding agents.
//!
//! Bottom-up verified construction: components are declared up front along
//! with their dependencies, then built one at a time from the buildable
//! frontier and verified before the next component can start. A component's
//! status only ever moves forward - unbuilt -> building -> verified - hence
//! "ratchet".
//!
//! This module is only that layer: the component register and step
//! bookkeeping. It never executes work itself.

use crate::{err, now, round4, SpiralError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

// Re-exported so `ratchet::Result` reads naturally for callers of this module.
pub use crate::Result;

pub const STATE_PATH: &str = ".murex/ratchet.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub description: String,
    pub requirement: String,
    pub depends_on: Vec<String>,
    pub status: String,
    pub evidence: String,
    pub step_built: Option<u32>,
    pub step_verified: Option<u32>,
}

impl Component {
    fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("component serializes")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub n: u32,
    pub opened_at: String,
    pub component: String,
    pub result: Option<String>,
    pub cost: f64,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ratchet {
    pub objective: String,
    pub requirement: String,
    pub created_at: String,
    pub status: String,
    pub step: u32,
    pub cumulative_cost: f64,
    pub components: Vec<Component>,
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_reason: Option<String>,
}

fn state_file(root: &Path) -> PathBuf {
    root.join(STATE_PATH)
}

pub fn load(root: &Path) -> Result<Ratchet> {
    let path = state_file(root);
    if !path.exists() {
        return err("no ratchet here - run `murex ratchet start \"<objective>\"` first");
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| SpiralError(format!("cannot read {}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| SpiralError(format!("corrupt state at {}: {e}", path.display())))
}

pub fn save(root: &Path, state: &Ratchet) -> Result<()> {
    let path = state_file(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SpiralError(format!("cannot create {}: {e}", parent.display())))?;
    }
    let text = serde_json::to_string_pretty(state).map_err(|e| SpiralError(e.to_string()))?;
    fs::write(&path, text + "\n")
        .map_err(|e| SpiralError(format!("cannot write {}: {e}", path.display())))
}

fn component_index(state: &Ratchet, id: &str) -> Result<usize> {
    state
        .components
        .iter()
        .position(|c| c.id == id)
        .ok_or_else(|| SpiralError(format!("unknown component {id:?}")))
}

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

/// Fix the objective and requirement that every component must serve.
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

/// Register a component. Dependencies must already exist.
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

/// Selection + build brief: pick the top of the buildable frontier and open a
/// step against it.
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

/// Shared close: assert an open step exists and that it targets `id`.
fn take_pending(state: &Ratchet, id: &str) -> Result<usize> {
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

/// Close the open step by verifying its component: locks the component,
/// records evidence, and ratchets the whole thing to `complete` once every
/// component has been verified.
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

/// Close the open step as a rework: the component returns to `unbuilt` to be
/// picked again. A ratchet never slips a *verified* level - only a still-open
/// step can fail, so this cannot undo a previous verification.
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

/// Stub - full status view lands in a later task. Enough for now to report
/// the objective and confirm whether a ratchet exists at all.
pub fn status(root: &Path) -> Result<Value> {
    let state = load(root)?;
    Ok(json!({ "objective": state.objective, "ratchet_status": state.status }))
}
