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
