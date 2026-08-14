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

use crate::{err, now, SpiralError};
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

// Not yet called - verify/rework land in a later task and will use this for
// id lookups by exact match (unlike add_component's existence-only check).
#[allow(dead_code)]
fn component_index(state: &Ratchet, id: &str) -> Result<usize> {
    state
        .components
        .iter()
        .position(|c| c.id == id)
        .ok_or_else(|| SpiralError(format!("unknown component {id:?}")))
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

/// Stub - full status view lands in a later task. Enough for now to report
/// the objective and confirm whether a ratchet exists at all.
pub fn status(root: &Path) -> Result<Value> {
    let state = load(root)?;
    Ok(json!({ "objective": state.objective, "ratchet_status": state.status }))
}
