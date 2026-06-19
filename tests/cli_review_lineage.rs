mod support;

use serde_json::Value;
use support::{dump_repo, shore};

#[test]
fn review_lineage_attach_show_and_head_scoped_commands_work() {
    let repo = dump_repo();
    let repo_path = repo.path().to_str().unwrap();
    let lineage_id = "review-unit-lineage:random:test";
    let first = parse_json(&shore(["review", "capture", "--repo", repo_path]).stdout);

    let first_attach = shore([
        "review",
        "lineage",
        "attach",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
        "--review-unit",
        first["reviewUnit"]["id"].as_str().unwrap(),
    ]);
    assert!(
        first_attach.status.success(),
        "first attach stderr:\n{}",
        String::from_utf8_lossy(&first_attach.stderr)
    );
    let first_attach_json = parse_json(&first_attach.stdout);
    assert_eq!(first_attach_json["schema"], "shore.review-lineage-attach");
    assert_eq!(first_attach_json["lineageId"], lineage_id);
    assert_eq!(first_attach_json["eventsCreated"], 2);
    assert_eq!(
        first_attach_json["eventsCreatedByType"]["review_unit_lineage_declared"],
        1
    );
    assert_eq!(
        first_attach_json["eventsCreatedByType"]["review_unit_lineage_round_recorded"],
        1
    );

    repo.write("src/lib.rs", "pub fn value() -> u32 { 3 }\n");
    let second = parse_json(&shore(["review", "capture", "--repo", repo_path]).stdout);
    let second_attach = shore([
        "review",
        "lineage",
        "attach",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
        "--review-unit",
        second["reviewUnit"]["id"].as_str().unwrap(),
        "--predecessor",
        first["reviewUnit"]["id"].as_str().unwrap(),
    ]);
    assert!(
        second_attach.status.success(),
        "second attach stderr:\n{}",
        String::from_utf8_lossy(&second_attach.stderr)
    );

    let lineage = shore([
        "review",
        "lineage",
        "show",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
    ]);
    assert!(
        lineage.status.success(),
        "lineage show stderr:\n{}",
        String::from_utf8_lossy(&lineage.stderr)
    );
    let lineage_json = parse_json(&lineage.stdout);
    assert_eq!(lineage_json["schema"], "shore.review-lineage");
    assert_eq!(lineage_json["lineageId"], lineage_id);
    assert_eq!(lineage_json["headReviewUnitId"], second["reviewUnit"]["id"]);
    assert_eq!(lineage_json["rounds"].as_array().unwrap().len(), 2);

    let unit_show = shore([
        "review",
        "unit",
        "show",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
    ]);
    assert!(
        unit_show.status.success(),
        "unit show stderr:\n{}",
        String::from_utf8_lossy(&unit_show.stderr)
    );
    let unit_json = parse_json(&unit_show.stdout);
    assert_eq!(unit_json["reviewUnit"]["id"], second["reviewUnit"]["id"]);

    let observation = shore([
        "review",
        "observation",
        "add",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
        "--track",
        "agent:codex",
        "--title",
        "head fact",
    ]);
    assert!(
        observation.status.success(),
        "observation add stderr:\n{}",
        String::from_utf8_lossy(&observation.stderr)
    );
    let observation_json = parse_json(&observation.stdout);
    assert_eq!(observation_json["reviewUnitId"], second["reviewUnit"]["id"]);

    let conflict = shore([
        "review",
        "unit",
        "show",
        "--repo",
        repo_path,
        "--review-unit",
        first["reviewUnit"]["id"].as_str().unwrap(),
        "--lineage",
        lineage_id,
    ]);
    assert!(!conflict.status.success());
    assert!(conflict.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&conflict.stderr)
            .contains("cannot combine --review-unit and --lineage")
    );
}

#[test]
fn review_lineage_attach_reports_headless_fork_in_write_output() {
    let repo = dump_repo();
    let repo_path = repo.path().to_str().unwrap();
    let lineage_id = "review-unit-lineage:random:fork";
    let first = parse_json(&shore(["review", "capture", "--repo", repo_path]).stdout);
    assert!(
        shore([
            "review",
            "lineage",
            "attach",
            "--repo",
            repo_path,
            "--lineage",
            lineage_id,
            "--review-unit",
            first["reviewUnit"]["id"].as_str().unwrap(),
        ])
        .status
        .success()
    );

    repo.write("src/lib.rs", "pub fn value() -> u32 { 3 }\n");
    let second = parse_json(&shore(["review", "capture", "--repo", repo_path]).stdout);
    assert!(
        shore([
            "review",
            "lineage",
            "attach",
            "--repo",
            repo_path,
            "--lineage",
            lineage_id,
            "--review-unit",
            second["reviewUnit"]["id"].as_str().unwrap(),
            "--predecessor",
            first["reviewUnit"]["id"].as_str().unwrap(),
        ])
        .status
        .success()
    );

    repo.write("src/lib.rs", "pub fn value() -> u32 { 4 }\n");
    let third = parse_json(&shore(["review", "capture", "--repo", repo_path]).stdout);
    let third_attach = shore([
        "review",
        "lineage",
        "attach",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
        "--review-unit",
        third["reviewUnit"]["id"].as_str().unwrap(),
        "--predecessor",
        first["reviewUnit"]["id"].as_str().unwrap(),
    ]);
    assert!(
        third_attach.status.success(),
        "third attach stderr:\n{}",
        String::from_utf8_lossy(&third_attach.stderr)
    );
    let third_attach_json = parse_json(&third_attach.stdout);
    assert!(third_attach_json["headReviewUnitId"].is_null());
    let diagnostic_codes = third_attach_json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(diagnostic_codes.contains(&"lineage_forked_successor"));
    assert!(diagnostic_codes.contains(&"lineage_multiple_heads"));
}

#[test]
fn review_capture_can_attach_to_lineage_in_one_command() {
    let repo = dump_repo();
    let repo_path = repo.path().to_str().unwrap();
    let lineage_id = "review-unit-lineage:random:capture";

    let first_capture = shore([
        "review",
        "capture",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
    ]);
    assert!(
        first_capture.status.success(),
        "first capture stderr:\n{}",
        String::from_utf8_lossy(&first_capture.stderr)
    );
    let first = parse_json(&first_capture.stdout);
    assert_eq!(first["schema"], "shore.review-capture");
    assert_eq!(first["lineageAttach"]["lineageId"], lineage_id);
    assert_eq!(first["lineageAttach"]["eventsCreated"], 2);

    let duplicate_capture = shore([
        "review",
        "capture",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
    ]);
    assert!(
        duplicate_capture.status.success(),
        "duplicate capture stderr:\n{}",
        String::from_utf8_lossy(&duplicate_capture.stderr)
    );
    let duplicate = parse_json(&duplicate_capture.stdout);
    assert_eq!(duplicate["reviewUnit"]["id"], first["reviewUnit"]["id"]);
    assert_eq!(duplicate["eventsCreated"], 0);
    // The re-capture re-records both the capture event and the auto-recorded ref.
    assert_eq!(duplicate["eventsExisting"], 2);
    assert_eq!(duplicate["lineageAttach"]["eventsCreated"], 0);
    assert_eq!(duplicate["lineageAttach"]["eventsExisting"], 2);

    repo.write("src/lib.rs", "pub fn value() -> u32 { 3 }\n");
    let second_capture = shore([
        "review",
        "capture",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
        "--predecessor",
        first["reviewUnit"]["id"].as_str().unwrap(),
    ]);
    assert!(
        second_capture.status.success(),
        "second capture stderr:\n{}",
        String::from_utf8_lossy(&second_capture.stderr)
    );
    let second = parse_json(&second_capture.stdout);
    assert_eq!(
        second["lineageAttach"]["headReviewUnitId"],
        second["reviewUnit"]["id"]
    );

    let lineage_output = shore([
        "review",
        "lineage",
        "show",
        "--repo",
        repo_path,
        "--lineage",
        lineage_id,
    ]);
    let lineage_stdout = String::from_utf8_lossy(&lineage_output.stdout);
    let lineage = parse_json(&lineage_output.stdout);
    assert_eq!(lineage["headReviewUnitId"], second["reviewUnit"]["id"]);
    assert_eq!(lineage["rounds"].as_array().unwrap().len(), 2);
    assert!(!lineage_stdout.contains("worktreeRoot"));
    assert!(!lineage_stdout.contains(".shore/data"));
    assert!(!lineage_stdout.contains(".git"));
}

fn parse_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("parse CLI JSON")
}
