//! Self-check for the spiral controller. Run: cargo test

use std::path::Path;

use murex as sp;
use tempfile::TempDir;

fn expect_err<T>(result: sp::Result<T>, needle: &str) {
    match result {
        Ok(_) => panic!("expected SpiralError containing {needle:?}"),
        Err(error) => assert!(
            error.to_string().contains(needle),
            "expected {needle:?} in {error}"
        ),
    }
}

fn open_ids(root: &Path) -> Vec<String> {
    let state = sp::load(root).expect("state loads");
    sp::ranked_open_risks(&state)
        .iter()
        .map(|r| r.id.clone())
        .collect()
}

fn number(value: &serde_json::Value, pointer: &str) -> f64 {
    value.pointer(pointer).and_then(|v| v.as_f64()).unwrap_or_else(|| {
        panic!("no number at {pointer} in {value}");
    })
}

#[test]
fn spiral_drives_cycles_by_risk_exposure() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();

    expect_err(sp::status(root), "no spiral here");
    sp::start(root, "ship realtime editing", vec!["one Postgres box".into()], vec![])
        .expect("start");
    expect_err(sp::start(root, "again", vec![], vec![]), "already exists");

    expect_err(sp::add_risk(root, "bad", 1.5, 0.5, ""), "probability");
    expect_err(sp::add_risk(root, "", 0.5, 0.5, ""), "must not be empty");

    // A cycle needs a risk to drive it.
    expect_err(sp::open_cycle(root, vec![]), "no open risks");

    sp::add_risk(root, "low risk", 0.2, 0.3, "").expect("R1"); // exposure 0.06
    sp::add_risk(root, "CRDT memory blowup", 0.6, 0.9, "spike").expect("R2"); // 0.54
    sp::add_risk(root, "auth mismatch", 0.4, 0.7, "").expect("R3"); // 0.28

    // Highest exposure wins, regardless of insertion order.
    assert_eq!(open_ids(root), ["R2", "R3", "R1"]);

    let opened = sp::open_cycle(root, vec![]).expect("cycle 1");
    assert_eq!(number(&opened, "/cycle"), 1.0);
    assert_eq!(opened.pointer("/brief/risk_id").unwrap(), "R2");
    assert_eq!(number(&opened, "/brief/exposure"), 0.54);
    // Selecting a risk marks it as being worked, not resolved.
    assert_eq!(sp::load(root).unwrap().risks[1].status, "mitigating");

    // One cycle at a time: the commitment gate is not skippable.
    expect_err(sp::open_cycle(root, vec![]), "still open");

    expect_err(
        sp::commit(root, "maybe", 0.0, "", vec![], ""),
        "decision must be one of",
    );
    expect_err(
        sp::commit(root, "continue", -1.0, "", vec![], ""),
        "must not be negative",
    );
    expect_err(
        sp::commit(root, "continue", 0.0, "", vec!["R99".into()], ""),
        "unknown risk",
    );
    // A rejected commit must leave the cycle open, not half-closed.
    assert_eq!(number(&sp::status(root).unwrap(), "/pending_cycle"), 1.0);

    let before = number(&sp::status(root).unwrap(), "/remaining_exposure");
    let done = sp::commit(root, "continue", 1.5, "", vec!["R2".into()], "380MB").expect("commit");
    assert_eq!(done.pointer("/resolved_risks").unwrap().as_array().unwrap().len(), 1);
    assert_eq!(number(&done, "/cumulative_cost"), 1.5);
    // Retiring a risk must shrink remaining exposure.
    let after = number(&done, "/remaining_exposure");
    assert!(after < before);
    assert_eq!(after, ((before - 0.54) * 10_000.0).round() / 10_000.0);

    // Next cycle picks the new leader and cost accumulates as radius.
    let second = sp::open_cycle(root, vec![]).expect("cycle 2");
    assert_eq!(second.pointer("/brief/risk_id").unwrap(), "R3");
    sp::commit(root, "continue", 2.0, "", vec![], "").expect("commit 2");
    let status = sp::status(root).expect("status");
    assert_eq!(number(&status, "/radius/cycles_completed"), 2.0);
    assert_eq!(number(&status, "/radius/cumulative_cost"), 3.5);
    // Inconclusive spike: R3 was not resolved, so it stays in the running.
    assert_eq!(status.pointer("/open_risks/0/id").unwrap(), "R3");

    // An accepted risk stops steering cycles but stays in the record.
    sp::close_risk(root, "R3", "accepted", "ship with fallback").expect("accept");
    assert_eq!(open_ids(root), ["R1"]);

    // `stop` ends the spiral; no further cycles.
    sp::open_cycle(root, vec![]).expect("cycle 3");
    sp::commit(root, "stop", 0.5, "not worth it", vec![], "").expect("stop decision");
    assert_eq!(sp::status(root).unwrap().pointer("/spiral_status").unwrap(), "stopped");
    expect_err(sp::open_cycle(root, vec![]), "no further cycles");
}

#[test]
fn risk_ids_rank_numerically_past_ten() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    sp::start(root, "many risks", vec![], vec![]).expect("start");
    // Eleven risks of identical exposure: the tie-break must be R1 before R10.
    for i in 1..=11 {
        sp::add_risk(root, &format!("risk {i}"), 0.5, 0.5, "").expect("add");
    }
    assert_eq!(open_ids(root)[..3], ["R1", "R2", "R3"]);
}

#[test]
fn state_lands_under_the_tool_directory() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    sp::start(root, "path check", vec![], vec![]).expect("start");
    assert_eq!(sp::STATE_PATH, ".murex/spiral.json");
    assert!(root.join(".murex/spiral.json").exists());
    // The host's directory is no longer ours to write into.
    assert!(!root.join(".ouroboros").exists());
}
