//! Self-check for the ratchet controller. Run: cargo test

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
