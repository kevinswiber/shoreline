mod support;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use support::git_repo::GitRepo;
use support::shore;

/// Linked review fixture: the seed worktree captures one review unit and
/// links it into the clone-local store, then the reader worktree links with
/// nothing local to push. Reads from the reader exercise the linked store.
struct LinkedFixture {
    _main: GitRepo,
    _worktree_parent: tempfile::TempDir,
    seed: PathBuf,
    reader: PathBuf,
    seed_review_unit_id: String,
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
            _main: main,
            _worktree_parent: worktree_parent,
            seed,
            reader,
            seed_review_unit_id: String::new(),
        };
        fs::write(fixture.seed.join("README.md"), "changed in seed\n").unwrap();
        let capture = fixture.capture(&fixture.seed);
        fixture.seed_review_unit_id = capture["reviewUnit"]["id"]
            .as_str()
            .expect("capture has review unit id")
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
