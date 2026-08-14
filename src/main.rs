//! Command entrypoint for the spiral plugin.
//!
//! The contract is argv in, JSON on stdout: every path here prints JSON and
//! nothing else. Failures print `{"error": ...}` and exit 1.
//!
//! Argument *values* are validated in the library rather than by clap, so that
//! a bad value surfaces as a JSON error on the protocol channel instead of as
//! clap's plain-text usage message.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use murex as spiral;

#[derive(Parser)]
#[command(name = "murex", version, about = "Risk-driven spiral-model cycles.")]
struct Cli {
    /// Repository root holding .murex/spiral.json.
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open a spiral for an objective.
    Start {
        objective: String,
        #[arg(long)]
        constraint: Vec<String>,
        #[arg(long)]
        alternative: Vec<String>,
    },
    /// Add, list, or close entries in the risk register.
    Risk {
        #[command(subcommand)]
        action: RiskAction,
    },
    /// Open the next cycle against the highest-exposure open risk.
    Cycle {
        #[arg(long)]
        objective: Vec<String>,
    },
    /// Commitment review that closes the open cycle.
    Commit {
        #[arg(long)]
        decision: String,
        #[arg(long, default_value_t = 0.0)]
        cost: f64,
        #[arg(long, default_value = "")]
        outcome: String,
        #[arg(long)]
        resolve: Vec<String>,
        #[arg(long, default_value = "")]
        evidence: String,
    },
    /// Abandon the spiral.
    Stop {
        #[arg(long, default_value = "")]
        reason: String,
    },
    /// Radius, remaining exposure, and history.
    Status,
    /// Bottom-up, verification-gated build for clear requirements.
    Ratchet {
        #[command(subcommand)]
        action: RatchetAction,
    },
}

#[derive(Subcommand)]
enum RiskAction {
    Add {
        description: String,
        #[arg(long)]
        probability: f64,
        #[arg(long)]
        impact: f64,
        #[arg(long, default_value = "")]
        mitigation: String,
    },
    List,
    Close {
        risk_id: String,
        #[arg(long, default_value = "resolved")]
        status: String,
        #[arg(long, default_value = "")]
        evidence: String,
    },
}

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

fn dispatch(cli: &Cli) -> spiral::Result<Value> {
    let root = cli.root.as_path();
    match &cli.command {
        Command::Start {
            objective,
            constraint,
            alternative,
        } => spiral::start(root, objective, constraint.clone(), alternative.clone()),
        Command::Risk { action } => match action {
            RiskAction::Add {
                description,
                probability,
                impact,
                mitigation,
            } => spiral::add_risk(root, description, *probability, *impact, mitigation),
            RiskAction::List => spiral::list_risks(root),
            RiskAction::Close {
                risk_id,
                status,
                evidence,
            } => spiral::close_risk(root, risk_id, status, evidence),
        },
        Command::Cycle { objective } => spiral::open_cycle(root, objective.clone()),
        Command::Commit {
            decision,
            cost,
            outcome,
            resolve,
            evidence,
        } => spiral::commit(root, decision, *cost, outcome, resolve.clone(), evidence),
        Command::Stop { reason } => spiral::stop(root, reason),
        Command::Status => spiral::status(root),
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
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).expect("value serializes"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", json!({ "error": error.to_string() }));
            ExitCode::FAILURE
        }
    }
}
