//! Command entrypoint for the spiral plugin.
//!
//! Ouroboros invokes plugins as `<entrypoint.command> <command> [args...]` and
//! reads a JSON document from stdout, so every path here prints JSON and
//! nothing else. Failures print `{"error": ...}` and exit 1.
//!
//! Argument *values* are validated in the library rather than by clap, so that
//! a bad value surfaces as a JSON error on the protocol channel instead of as
//! clap's plain-text usage message.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use ouroboros_spiral as spiral;

#[derive(Parser)]
#[command(name = "ooo spiral", about = "Risk-driven spiral-model cycles.")]
struct Cli {
    /// Repository root holding .ouroboros/spiral.json.
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
