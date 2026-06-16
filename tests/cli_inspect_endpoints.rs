//! Endpoint contract coverage for the `shore inspect` JSON API (issue #110),
//! exercised over real HTTP against a store built at test time. The harness
//! lives in `support::inspect` so multiple inspector suites share one
//! spawn-the-real-server fixture.

mod support;

use support::git_repo::GitRepo;
use support::inspect::{Inspector, capture};

/// Smoke: the shared harness spawns the real `shore inspect --port 0` server and
/// serves a well-formed history payload for a minimal store.
#[test]
fn inspector_harness_serves_history_for_minimal_store() {
    let repo = GitRepo::new();
    repo.write("src/lib.rs", "pub fn value() -> u32 { 1 }\n");
    repo.commit_all("base");
    repo.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n");
    capture(repo.path());

    let inspector = Inspector::spawn(repo.path());
    let history = inspector.get_json("/api/history");

    assert_eq!(history["schema"], "shore.inspect-history");
    // A minimal store records exactly the capture event (no separate
    // `review_initialized` event exists in the current event model).
    let entries = history["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["eventType"], "review_unit_captured");
}
