//! Boehm spiral-model cycle controller for Ouroboros.
//!
//! Ouroboros already owns execution (`ooo auto`, `ooo run`, `ooo evolve`). Its
//! evolve loop is quality-driven: it iterates until an evaluation gate passes.
//! The spiral model is risk-driven instead - each cycle exists to retire the
//! single largest risk, and a commitment review decides whether the next cycle
//! is worth its cost.
//!
//! This crate adds only that missing layer: the risk register, top-risk
//! selection, and the commitment gate. It never executes work. [`open_cycle`]
//! emits a spike brief that the operator (or agent) hands to the runtime.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const STATE_PATH: &str = ".murex/spiral.json";
pub const DECISIONS: [&str; 3] = ["continue", "pivot", "stop"];
/// A risk still steers cycles until it is resolved or explicitly accepted.
pub const OPEN_STATES: [&str; 2] = ["open", "mitigating"];

/// Operator-facing failure. The binary renders it as a JSON error.
#[derive(Debug)]
pub struct SpiralError(pub String);

impl fmt::Display for SpiralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SpiralError {}

pub type Result<T> = std::result::Result<T, SpiralError>;

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(SpiralError(message.into()))
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub id: String,
    pub description: String,
    pub probability: f64,
    pub impact: f64,
    pub status: String,
    pub mitigation: String,
    pub evidence: String,
    pub cycle_opened: u32,
    pub cycle_closed: Option<u32>,
}

impl Risk {
    /// Boehm's risk exposure: probability of loss x size of loss.
    pub fn exposure(&self) -> f64 {
        round4(self.probability * self.impact)
    }

    pub fn is_open(&self) -> bool {
        OPEN_STATES.contains(&self.status.as_str())
    }

    fn to_value(&self) -> Value {
        let mut value = serde_json::to_value(self).expect("risk serializes");
        value["exposure"] = json!(self.exposure());
        value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cycle {
    pub n: u32,
    pub opened_at: String,
    pub objectives: Vec<String>,
    pub top_risk: String,
    pub top_risk_exposure: f64,
    pub decision: Option<String>,
    pub cost: f64,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_risks: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spiral {
    pub objective: String,
    pub created_at: String,
    pub status: String,
    pub cycle: u32,
    pub cumulative_cost: f64,
    pub constraints: Vec<String>,
    pub alternatives: Vec<String>,
    pub risks: Vec<Risk>,
    pub cycles: Vec<Cycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_reason: Option<String>,
}

fn state_file(root: &Path) -> PathBuf {
    root.join(STATE_PATH)
}

pub fn load(root: &Path) -> Result<Spiral> {
    let path = state_file(root);
    if !path.exists() {
        return err("no spiral here - run `ooo murex start \"<objective>\"` first");
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| SpiralError(format!("cannot read {}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| SpiralError(format!("corrupt state at {}: {e}", path.display())))
}

pub fn save(root: &Path, state: &Spiral) -> Result<()> {
    let path = state_file(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SpiralError(format!("cannot create {}: {e}", parent.display())))?;
    }
    let text = serde_json::to_string_pretty(state).map_err(|e| SpiralError(e.to_string()))?;
    fs::write(&path, text + "\n")
        .map_err(|e| SpiralError(format!("cannot write {}: {e}", path.display())))
}

/// Risk ids are `R<n>`; compare the number so R10 sorts after R2.
fn id_order(id: &str) -> u32 {
    id.trim_start_matches('R').parse().unwrap_or(u32::MAX)
}

/// Open risks, highest exposure first. Ties break by id for determinism.
pub fn ranked_open_risks(state: &Spiral) -> Vec<&Risk> {
    let mut open: Vec<&Risk> = state.risks.iter().filter(|r| r.is_open()).collect();
    open.sort_by(|a, b| {
        b.exposure()
            .total_cmp(&a.exposure())
            .then_with(|| id_order(&a.id).cmp(&id_order(&b.id)))
    });
    open
}

fn total_exposure(state: &Spiral) -> f64 {
    // Sum for f64 uses -0.0 as its identity, so an empty register would report
    // -0.0 at exactly the moment the docs say to look for zero; + 0.0 fixes the sign.
    round4(ranked_open_risks(state).iter().map(|r| r.exposure()).sum::<f64>() + 0.0)
}

fn pending_index(state: &Spiral) -> Option<usize> {
    state.cycles.iter().rposition(|c| c.decision.is_none())
}

fn risk_index(state: &Spiral, id: &str) -> Result<usize> {
    state
        .risks
        .iter()
        .position(|r| r.id == id)
        .ok_or_else(|| SpiralError(format!("unknown risk {id:?}")))
}

fn check_unit(value: f64, label: &str) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        return err(format!("{label} must be within 0.0..1.0, got {value}"));
    }
    Ok(())
}

/// Quadrant 1 of cycle 0: fix objectives, alternatives, constraints.
pub fn start(
    root: &Path,
    objective: &str,
    constraints: Vec<String>,
    alternatives: Vec<String>,
) -> Result<Value> {
    if objective.trim().is_empty() {
        return err("objective must not be empty");
    }
    let path = state_file(root);
    if path.exists() {
        return err(format!(
            "spiral already exists at {} - see `ooo murex status`",
            path.display()
        ));
    }
    let state = Spiral {
        objective: objective.to_string(),
        created_at: now(),
        status: "active".to_string(),
        cycle: 0,
        cumulative_cost: 0.0,
        constraints,
        alternatives,
        risks: Vec::new(),
        cycles: Vec::new(),
        stopped_reason: None,
    };
    save(root, &state)?;
    Ok(json!({
        "objective": objective,
        "status": "active",
        "state_file": path.display().to_string(),
        "next": "ooo murex risk add \"<risk>\" --probability 0.7 --impact 0.9",
    }))
}

/// Quadrant 2: register a risk so it can compete for the next cycle.
pub fn add_risk(
    root: &Path,
    description: &str,
    probability: f64,
    impact: f64,
    mitigation: &str,
) -> Result<Value> {
    if description.trim().is_empty() {
        return err("risk description must not be empty");
    }
    check_unit(probability, "probability")?;
    check_unit(impact, "impact")?;
    let mut state = load(root)?;
    let risk = Risk {
        // Risks are never deleted, so a monotonic counter cannot collide.
        id: format!("R{}", state.risks.len() + 1),
        description: description.to_string(),
        probability,
        impact,
        status: "open".to_string(),
        mitigation: mitigation.to_string(),
        evidence: String::new(),
        cycle_opened: state.cycle,
        cycle_closed: None,
    };
    let value = risk.to_value();
    state.risks.push(risk);
    save(root, &state)?;
    Ok(json!({ "risk": value }))
}

pub fn close_risk(root: &Path, risk_id: &str, status: &str, evidence: &str) -> Result<Value> {
    if status != "resolved" && status != "accepted" {
        return err("status must be 'resolved' or 'accepted'");
    }
    let mut state = load(root)?;
    let index = risk_index(&state, risk_id)?;
    let cycle = state.cycle;
    let risk = &mut state.risks[index];
    risk.status = status.to_string();
    risk.evidence = evidence.to_string();
    risk.cycle_closed = Some(cycle);
    let value = risk.to_value();
    save(root, &state)?;
    Ok(json!({ "risk": value }))
}

pub fn list_risks(root: &Path) -> Result<Value> {
    let state = load(root)?;
    let open: Vec<Value> = ranked_open_risks(&state).iter().map(|r| r.to_value()).collect();
    let closed: Vec<Value> = state
        .risks
        .iter()
        .filter(|r| !r.is_open())
        .map(|r| r.to_value())
        .collect();
    Ok(json!({ "open": open, "closed": closed }))
}

fn brief(state: &Spiral, risk: &Risk, cycle_n: u32) -> Value {
    json!({
        "cycle": cycle_n,
        "objective": state.objective,
        "de_risk": risk.description,
        "risk_id": risk.id,
        "exposure": risk.exposure(),
        "planned_mitigation": risk.mitigation,
        "constraints": state.constraints,
        "instruction": format!(
            "Cycle {cycle_n} exists to retire exactly one risk: {} (exposure {:.2}). \
             Build the smallest prototype or spike that produces evidence for or \
             against it, then verify. Do not broaden scope to unrelated work - \
             other risks get their own cycles.",
            risk.description,
            risk.exposure(),
        ),
    })
}

/// Quadrants 1-3: pick the top risk and emit the spike brief for it.
pub fn open_cycle(root: &Path, objectives: Vec<String>) -> Result<Value> {
    let mut state = load(root)?;
    if state.status != "active" {
        return err(format!("spiral is {} - no further cycles", state.status));
    }
    if let Some(index) = pending_index(&state) {
        return err(format!(
            "cycle {} is still open - close it with \
             `ooo murex commit --decision <continue|pivot|stop>`",
            state.cycles[index].n
        ));
    }
    let top_id = match ranked_open_risks(&state).first() {
        Some(risk) => risk.id.clone(),
        None => {
            return err(
                "no open risks - a spiral cycle must be driven by one. Add a risk \
                 with `ooo murex risk add`, or close out with `ooo murex stop`.",
            )
        }
    };
    let top = risk_index(&state, &top_id)?;
    state.cycle += 1;
    let entry = Cycle {
        n: state.cycle,
        opened_at: now(),
        objectives: if objectives.is_empty() {
            vec![state.objective.clone()]
        } else {
            objectives
        },
        top_risk: top_id.clone(),
        top_risk_exposure: state.risks[top].exposure(),
        decision: None,
        cost: 0.0,
        outcome: String::new(),
        resolved_risks: None,
        closed_at: None,
    };
    state.cycles.push(entry);
    // Selecting a risk marks it as being worked, not resolved.
    if state.risks[top].status == "open" {
        state.risks[top].status = "mitigating".to_string();
    }
    save(root, &state)?;
    Ok(json!({
        "cycle": state.cycle,
        "brief": brief(&state, &state.risks[top], state.cycle),
        "next": [
            "hand the brief to the runtime, e.g. `ooo auto \"<instruction>\"`",
            format!(
                "then `ooo murex commit --decision continue --cost <n> --resolve {top_id}`"
            ),
        ],
    }))
}

/// Quadrant 4: the commitment review that closes a cycle.
pub fn commit(
    root: &Path,
    decision: &str,
    cost: f64,
    outcome: &str,
    resolve: Vec<String>,
    evidence: &str,
) -> Result<Value> {
    if !DECISIONS.contains(&decision) {
        return err(format!("decision must be one of {DECISIONS:?}"));
    }
    if cost < 0.0 {
        return err(format!("cost must not be negative, got {cost}"));
    }
    let mut state = load(root)?;
    let Some(index) = pending_index(&state) else {
        return err("no open cycle - start one with `ooo murex cycle`");
    };
    let n = state.cycles[index].n;
    // Resolve every id before mutating, so one bad id cannot half-apply.
    let targets = resolve
        .iter()
        .map(|id| risk_index(&state, id))
        .collect::<Result<Vec<usize>>>()?;
    for target in targets {
        let risk = &mut state.risks[target];
        risk.status = "resolved".to_string();
        risk.evidence = evidence.to_string();
        risk.cycle_closed = Some(n);
    }
    let entry = &mut state.cycles[index];
    entry.decision = Some(decision.to_string());
    entry.cost = cost;
    entry.outcome = outcome.to_string();
    entry.resolved_risks = Some(resolve.clone());
    entry.closed_at = Some(now());
    // The spiral's radius: cost accumulated across every cycle so far.
    state.cumulative_cost = round4(state.cumulative_cost + cost);
    if decision == "stop" {
        state.status = "stopped".to_string();
    }
    save(root, &state)?;
    Ok(json!({
        "cycle": n,
        "decision": decision,
        "resolved_risks": resolve,
        "cumulative_cost": state.cumulative_cost,
        "spiral_status": state.status,
        "remaining_exposure": total_exposure(&state),
    }))
}

/// Abandon the spiral without a cycle in flight.
pub fn stop(root: &Path, reason: &str) -> Result<Value> {
    let mut state = load(root)?;
    state.status = "stopped".to_string();
    state.stopped_reason = Some(reason.to_string());
    save(root, &state)?;
    Ok(json!({ "spiral_status": "stopped", "reason": reason }))
}

pub fn status(root: &Path) -> Result<Value> {
    let state = load(root)?;
    let open_risks: Vec<Value> = ranked_open_risks(&state)
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "description": r.description,
                "exposure": r.exposure(),
                "status": r.status,
            })
        })
        .collect();
    let history: Vec<Value> = state
        .cycles
        .iter()
        .map(|c| {
            json!({
                "n": c.n,
                "top_risk": c.top_risk,
                "decision": c.decision,
                "cost": c.cost,
            })
        })
        .collect();
    Ok(json!({
        "objective": state.objective,
        "spiral_status": state.status,
        "cycle": state.cycle,
        // Radius grows with cost; convergence shows as falling exposure.
        "radius": {
            "cycles_completed": state.cycles.iter().filter(|c| c.decision.is_some()).count(),
            "cumulative_cost": state.cumulative_cost,
        },
        "remaining_exposure": total_exposure(&state),
        "open_risks": open_risks,
        "pending_cycle": pending_index(&state).map(|i| state.cycles[i].n),
        "history": history,
    }))
}
