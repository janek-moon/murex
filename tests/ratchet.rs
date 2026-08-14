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
