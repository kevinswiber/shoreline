mod support;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use shoreline::model::SnapshotId;
use shoreline::session::{
    ArtifactKind, ArtifactRef, ImportArtifactOptions, export_artifact, import_artifact,
    read_events, read_snapshot_artifact, referenced_artifacts,
};
use support::git_repo::GitRepo;
use support::shore;

/// Linked review fixture: the seed worktree captures one review unit and
/// links it into the clone-local store, then the reader worktree links with
/// nothing local to push. Reads from the reader exercise the linked store.
struct LinkedFixture {
    main: GitRepo,
    _worktree_parent: tempfile::TempDir,
    seed: PathBuf,
    reader: PathBuf,
    seed_review_unit_id: String,
    seed_snapshot_id: String,
}

impl LinkedFixture {
    fn new() -> Self {
        let main = GitRepo::new();
        main.write("README.md", "base\n");
        main.commit_all("base");

        let worktree_parent = tempfile::tempdir().expect("create worktree parent");
        let seed = worktree_parent.path().join("seed");
        add_worktree(main.path(), &seed, "seed");
        let reader = worktree_parent.path().join("reader");
        add_worktree(main.path(), &reader, "reader");

        let mut fixture = Self {
            main,
            _worktree_parent: worktree_parent,
            seed,
            reader,
            seed_review_unit_id: String::new(),
            seed_snapshot_id: String::new(),
        };
        fs::write(fixture.seed.join("README.md"), "changed in seed\n").unwrap();
        let capture = fixture.capture(&fixture.seed);
        fixture.seed_review_unit_id = capture["reviewUnit"]["id"]
            .as_str()
            .expect("capture has review unit id")
            .to_owned();
        fixture.seed_snapshot_id = capture["reviewUnit"]["snapshotId"]
            .as_str()
            .expect("capture has snapshot id")
            .to_owned();
        fixture.link(&fixture.seed);
        fixture.link(&fixture.reader);
        fixture
    }

    fn capture(&self, worktree: &Path) -> Value {
        run_shore_json(&["review", "capture", "--repo", worktree.to_str().unwrap()])
    }

    fn link(&self, worktree: &Path) -> Value {
        run_shore_json(&["store", "link", "--repo", worktree.to_str().unwrap()])
    }

    fn observation_add(&self, worktree: &Path, review_unit_id: &str, body: &str) -> Value {
        run_shore_json(&[
            "review",
            "observation",
            "add",
            "--repo",
            worktree.to_str().unwrap(),
            "--review-unit",
            review_unit_id,
            "--track",
            "agent:test-fixture",
            "--title",
            "linked body artifact",
            "--body",
            body,
        ])
    }

    fn linked_store_dir(&self) -> PathBuf {
        self.main.path().join(".git/shoreline")
    }

    fn unit_list_json(&self, worktree: &Path) -> Value {
        run_shore_json(&[
            "review",
            "unit",
            "list",
            "--repo",
            worktree.to_str().unwrap(),
        ])
    }
}

fn run_shore_json(args: &[&str]) -> Value {
    let output = shore(args.iter().copied());
    assert!(
        output.status.success(),
        "shore {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("shore stdout is json")
}

fn diagnostic_codes(json: &Value) -> Vec<&str> {
    json["diagnostics"]
        .as_array()
        .map(|diagnostics| {
            diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic["code"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

fn diagnostic_message(json: &Value, code: &str) -> String {
    json["diagnostics"]
        .as_array()
        .and_then(|diagnostics| {
            diagnostics
                .iter()
                .find(|diagnostic| diagnostic["code"] == code)
        })
        .and_then(|diagnostic| diagnostic["message"].as_str())
        .unwrap_or_else(|| panic!("no diagnostic with code {code}"))
        .to_owned()
}

fn add_worktree(repo: &Path, path: &Path, branch: &str) {
    run_git_os(
        repo,
        [
            OsString::from("worktree"),
            OsString::from("add"),
            OsString::from("-b"),
            OsString::from(branch),
            path.as_os_str().to_owned(),
        ],
    );
}

fn run_git_os<I>(cwd: &Path, args: I)
where
    I: IntoIterator<Item = OsString>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("run git in {}: {error}", cwd.display()));
    assert!(
        output.status.success(),
        "git failed in {}\nstdout:\n{}\nstderr:\n{}",
        cwd.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn linked_unit_list_reports_unsynced_local_events_diagnostic() {
    let fixture = LinkedFixture::new();
    fs::write(fixture.reader.join("README.md"), "changed in reader\n").unwrap();
    fixture.capture(&fixture.reader);

    let json = fixture.unit_list_json(&fixture.reader);

    // Store-only: the reader's local capture is not listed; only the seed's
    // linked unit is.
    assert_eq!(json["reviewUnitCount"], 1);
    assert_eq!(
        json["entries"][0]["reviewUnitId"],
        Value::String(fixture.seed_review_unit_id.clone())
    );
    let codes = diagnostic_codes(&json);
    assert!(
        codes.contains(&"clone_local_unsynced_local_events"),
        "diagnostics: {}",
        json["diagnostics"]
    );
    let message = diagnostic_message(&json, "clone_local_unsynced_local_events");
    assert!(message.contains("1 local event"), "message: {message}");
    assert!(message.contains("shore store link"), "message: {message}");
}

#[test]
fn linked_unit_list_without_local_events_has_no_divergence_diagnostic() {
    let fixture = LinkedFixture::new();

    let json = fixture.unit_list_json(&fixture.reader);

    assert_eq!(json["reviewUnitCount"], 1);
    assert_eq!(json["eventCount"], 1);
    assert_eq!(
        json["entries"][0]["reviewUnitId"],
        Value::String(fixture.seed_review_unit_id.clone())
    );
    assert!(
        json["eventSetHash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(
        !diagnostic_codes(&json).contains(&"clone_local_unsynced_local_events"),
        "diagnostics: {}",
        json["diagnostics"]
    );
}

#[test]
fn snapshot_artifact_reads_from_linked_store() {
    let fixture = LinkedFixture::new();
    let snapshot_id = SnapshotId::new(fixture.seed_snapshot_id.clone());

    let artifact = read_snapshot_artifact(&fixture.reader, &snapshot_id)
        .expect("snapshot artifact reads from the linked store");

    assert_eq!(artifact.snapshot.snapshot_id, snapshot_id);
    assert_eq!(
        artifact.review_unit_id.as_str(),
        fixture.seed_review_unit_id
    );
}

#[test]
fn export_artifact_body_reads_from_linked_store() {
    let fixture = LinkedFixture::new();
    let body = "x".repeat(5000);
    fixture.observation_add(&fixture.seed, &fixture.seed_review_unit_id, &body);
    fixture.link(&fixture.seed);

    let body_ref = seed_body_artifact_ref(&fixture);
    let bytes = export_artifact(&fixture.reader, &body_ref)
        .expect("body artifact exports from the linked store");

    assert!(!bytes.is_empty());
}

#[test]
fn import_artifact_still_writes_worktree_local_in_linked_mode() {
    let fixture = LinkedFixture::new();
    let body = "y".repeat(5000);
    fixture.observation_add(&fixture.seed, &fixture.seed_review_unit_id, &body);

    let body_ref = seed_body_artifact_ref(&fixture);
    let artifact_relative_path = format!(
        "artifacts/notes/{}.json",
        body_ref
            .content_hash()
            .strip_prefix("sha256:")
            .expect("body content hash is sha256-prefixed")
    );
    let bytes = fs::read(fixture.seed.join(".shore").join(&artifact_relative_path)).unwrap();

    import_artifact(ImportArtifactOptions::new(
        &fixture.reader,
        body_ref,
        bytes,
    ))
    .expect("import into the linked reader succeeds");

    // Writes stay worktree-local until shared-store writes land: the artifact
    // lands in the reader's own .shore, not the linked clone-local store.
    assert!(
        fixture
            .reader
            .join(".shore")
            .join(&artifact_relative_path)
            .is_file()
    );
    assert!(
        !fixture
            .linked_store_dir()
            .join(&artifact_relative_path)
            .exists()
    );
}

fn seed_body_artifact_ref(fixture: &LinkedFixture) -> ArtifactRef {
    let events = read_events(&fixture.seed).expect("read seed worktree events");
    referenced_artifacts(&events)
        .expect("derive artifact refs from seed events")
        .into_iter()
        .find(|artifact| artifact.kind() == ArtifactKind::Body)
        .expect("seed events reference a body artifact")
}

#[test]
fn worktree_local_unit_list_is_unchanged() {
    let repo = GitRepo::new();
    repo.write("src/lib.rs", "pub fn value() -> u32 { 1 }\n");
    repo.commit_all("base");
    repo.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n");
    run_shore_json(&["review", "capture", "--repo", repo.path().to_str().unwrap()]);

    let json = run_shore_json(&[
        "review",
        "unit",
        "list",
        "--repo",
        repo.path().to_str().unwrap(),
    ]);

    assert_eq!(json["schema"], "shore.review-unit-list");
    assert_eq!(json["version"], 1);
    assert_eq!(json["eventCount"], 1);
    assert_eq!(json["reviewUnitCount"], 1);
    assert!(
        !diagnostic_codes(&json).contains(&"clone_local_unsynced_local_events"),
        "diagnostics: {}",
        json["diagnostics"]
    );
}
