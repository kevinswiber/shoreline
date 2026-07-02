use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::canonical_hash::sha256_bytes_hex;
use crate::error::{Result, ShoreError};
use crate::git::git_tracked_and_untracked_inventory;
use crate::model::id_prefix;
use crate::session::store::sensitivity_config::{glob_matches, resolve_sensitivity_excludes};

const SCAN_READ_LIMIT: u64 = 64 * 1024;
const LARGE_GENERATED_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SensitivityScan {
    pub policy_outcome: String,
    pub findings: Vec<SensitivityFinding>,
    /// Unique inventory paths the configured exclude globs skipped — an
    /// excluded path is NOT scanned, so the count keeps an over-broad exclude
    /// visible rather than silent.
    pub excluded_path_count: usize,
    /// Every configured exclude glob with its match count, zero-count globs
    /// included (a dead glob is itself a finding for the operator). Glob
    /// strings are operator-authored config, safe to render; excluded paths
    /// keep the scan's redaction posture and are never listed.
    pub exclude_globs: Vec<SensitivityExcludeGlob>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SensitivityExcludeGlob {
    pub glob: String,
    pub matched: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SensitivityFinding {
    pub kind: String,
    pub severity: String,
    pub count: usize,
    pub policy_outcome: String,
    pub references: Vec<String>,
}

#[derive(Debug)]
struct FindingAccumulator {
    kind: &'static str,
    severity: &'static str,
    policy_outcome: &'static str,
    count: usize,
    references: BTreeSet<String>,
}

pub(crate) fn scan_worktree_sensitivity(worktree_root: &Path) -> Result<SensitivityScan> {
    let exclude_globs = resolve_sensitivity_excludes(worktree_root)?;
    let mut glob_match_counts = vec![0usize; exclude_globs.len()];
    let mut excluded_path_count = 0usize;
    let mut findings = BTreeMap::<&'static str, FindingAccumulator>::new();
    for relative_path in git_inventory_paths(worktree_root)? {
        // A path matching ANY configured glob is skipped before every
        // filename/content check — an explicit operator opt-out, surfaced via
        // the counts. Every matching glob's counter increments so overlapping
        // globs each show the work they do.
        if !exclude_globs.is_empty() {
            let relative = relative_path.to_string_lossy();
            let mut excluded = false;
            for (glob, count) in exclude_globs.iter().zip(glob_match_counts.iter_mut()) {
                if glob_matches(glob, &relative) {
                    *count += 1;
                    excluded = true;
                }
            }
            if excluded {
                excluded_path_count += 1;
                continue;
            }
        }
        let path = worktree_root.join(&relative_path);
        let metadata = fs::metadata(&path)
            .map_err(|error| io_error("read scan file metadata", &path, error))?;
        if !metadata.is_file() {
            continue;
        }

        let reference = redacted_file_ref(&relative_path);
        let relative_display = relative_path.to_string_lossy();
        let relative_lower = relative_display.to_ascii_lowercase();

        if sensitive_filename(&relative_lower) {
            add_finding(
                &mut findings,
                "sensitive_filename",
                "medium",
                "warn",
                &reference,
            );
        }
        if generated_path(&relative_lower) && metadata.len() > LARGE_GENERATED_BYTES {
            add_finding(
                &mut findings,
                "generated_path",
                "medium",
                "warn",
                &reference,
            );
        }

        let text = read_text_prefix(&path)?;
        if contains_known_token(&text) {
            add_finding(&mut findings, "known_token", "high", "block", &reference);
        }
        if contains_private_key_marker(&text) {
            add_finding(&mut findings, "private_key", "high", "block", &reference);
        }
        if contains_high_entropy_token(&text) {
            add_finding(&mut findings, "high_entropy", "medium", "warn", &reference);
        }
    }

    let findings = findings
        .into_values()
        .map(|finding| SensitivityFinding {
            kind: finding.kind.to_owned(),
            severity: finding.severity.to_owned(),
            count: finding.count,
            policy_outcome: finding.policy_outcome.to_owned(),
            references: finding.references.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    Ok(SensitivityScan {
        policy_outcome: combined_policy_outcome(&findings).to_owned(),
        findings,
        excluded_path_count,
        exclude_globs: exclude_globs
            .into_iter()
            .zip(glob_match_counts)
            .map(|(glob, matched)| SensitivityExcludeGlob { glob, matched })
            .collect(),
    })
}

fn add_finding(
    findings: &mut BTreeMap<&'static str, FindingAccumulator>,
    kind: &'static str,
    severity: &'static str,
    policy_outcome: &'static str,
    reference: &str,
) {
    let finding = findings.entry(kind).or_insert_with(|| FindingAccumulator {
        kind,
        severity,
        policy_outcome,
        count: 0,
        references: BTreeSet::new(),
    });
    finding.count += 1;
    finding.references.insert(reference.to_owned());
}

fn combined_policy_outcome(findings: &[SensitivityFinding]) -> &'static str {
    if findings
        .iter()
        .any(|finding| finding.policy_outcome == "block")
    {
        "block"
    } else if findings
        .iter()
        .any(|finding| finding.policy_outcome == "warn")
    {
        "warn"
    } else {
        "allow"
    }
}

fn git_inventory_paths(worktree_root: &Path) -> Result<Vec<PathBuf>> {
    git_tracked_and_untracked_inventory(worktree_root)?
        .into_iter()
        .map(|raw_path| {
            raw_path
                .into_utf8_string("sensitivity scan path")
                .map(PathBuf::from)
        })
        .collect()
}

fn read_text_prefix(path: &Path) -> Result<String> {
    let file = fs::File::open(path).map_err(|error| io_error("open scan file", path, error))?;
    let mut bytes = Vec::new();
    file.take(SCAN_READ_LIMIT)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read scan file", path, error))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn redacted_file_ref(relative_path: &Path) -> String {
    format!(
        "{}:sha256:{}",
        id_prefix::REDACTED_FILE,
        sha256_bytes_hex(relative_path.to_string_lossy().as_bytes())
    )
}

fn sensitive_filename(relative_lower: &str) -> bool {
    let file_name = relative_lower.rsplit('/').next().unwrap_or(relative_lower);
    matches!(file_name, ".env" | ".npmrc" | ".netrc" | "kubeconfig")
        || file_name.starts_with(".env.")
        || file_name.contains("credential")
        || file_name.contains("credentials")
        || file_name.contains("token")
        || file_name.contains("private-key")
        || relative_lower.contains(".config/gcloud/")
        || relative_lower.contains(".aws/credentials")
}

fn generated_path(relative_lower: &str) -> bool {
    relative_lower.starts_with("target/")
        || relative_lower.contains("/target/")
        || relative_lower.starts_with("node_modules/")
        || relative_lower.contains("/node_modules/")
        || relative_lower.starts_with("vendor/")
        || relative_lower.contains("/vendor/")
        || relative_lower.starts_with("dist/")
        || relative_lower.contains("/dist/")
        || relative_lower.starts_with("build/")
        || relative_lower.contains("/build/")
}

fn contains_known_token(text: &str) -> bool {
    token_candidates(text).any(|token| {
        (token.starts_with("sk-") && token.len() >= 20)
            || (token.starts_with("ghp_") && token.len() >= 20)
            || (token.starts_with("github_pat_") && token.len() >= 30)
            || (token.starts_with("AKIA") && token.len() >= 16)
    })
}

fn contains_private_key_marker(text: &str) -> bool {
    text.contains("-----BEGIN ") && text.contains("PRIVATE KEY-----")
}

fn contains_high_entropy_token(text: &str) -> bool {
    token_candidates(text).any(|token| {
        token.len() >= 32
            && token.bytes().any(|byte| byte.is_ascii_lowercase())
            && token.bytes().any(|byte| byte.is_ascii_uppercase())
            && token.bytes().any(|byte| byte.is_ascii_digit())
            && distinct_ascii_count(token) >= 16
    })
}

fn token_candidates(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| {
        !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '/' | '='))
    })
    .filter(|token| !token.is_empty())
}

fn distinct_ascii_count(token: &str) -> usize {
    token.bytes().collect::<BTreeSet<_>>().len()
}

fn io_error(action: &str, path: &Path, error: std::io::Error) -> ShoreError {
    ShoreError::Message(format!("{action} {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn sensitivity_inventory_preserves_git_order_and_excludes_ignored_paths() {
        let repo = TestRepo::new();
        repo.write("z-tracked.txt", "safe\n");
        repo.write("a-tracked.txt", "safe\n");
        repo.commit_all("base");
        repo.write("m-untracked.txt", "safe\n");
        repo.write("ignored-token.txt", "sk-test000000000000000000000000\n");
        fs::write(repo.path().join(".git/info/exclude"), "ignored-token.txt\n").unwrap();

        let paths = git_inventory_paths(repo.path()).unwrap();

        assert_eq!(
            paths,
            vec![
                PathBuf::from("m-untracked.txt"),
                PathBuf::from("a-tracked.txt"),
                PathBuf::from("z-tracked.txt"),
            ]
        );
    }

    #[test]
    fn excluded_globs_skip_matching_paths_before_content_checks() {
        let repo = TestRepo::new();
        // The motivating repro shape: a fixture file carrying a private-key marker.
        repo.write(
            "fixtures/dev.pem",
            "-----BEGIN PRIVATE KEY-----\nredacted\n",
        );
        repo.write(
            ".shore/sensitivity.json",
            r#"{"schema":"shore.sensitivity-config","version":1,"excludeGlobs":["fixtures/**"]}"#,
        );
        repo.commit_all("base");

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();

        assert_eq!(
            scan.policy_outcome, "allow",
            "the excluded fixture no longer blocks"
        );
        assert!(
            scan.findings.is_empty(),
            "no finding from an excluded path: {scan:?}"
        );
        assert_eq!(scan.excluded_path_count, 1);
        assert_eq!(scan.exclude_globs.len(), 1);
        assert_eq!(scan.exclude_globs[0].glob, "fixtures/**");
        assert_eq!(scan.exclude_globs[0].matched, 1);
    }

    #[test]
    fn non_excluded_sensitive_paths_still_block() {
        // The gate stays protective for the rest of the tree — a targeted
        // exclude is not a blanket override.
        let repo = TestRepo::new();
        repo.write(
            "fixtures/dev.pem",
            "-----BEGIN PRIVATE KEY-----\nredacted\n",
        );
        repo.write("keys/real.pem", "-----BEGIN PRIVATE KEY-----\nredacted\n");
        repo.write(
            ".shore/sensitivity.json",
            r#"{"schema":"shore.sensitivity-config","version":1,"excludeGlobs":["fixtures/**"]}"#,
        );
        repo.commit_all("base");

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();

        assert_eq!(scan.policy_outcome, "block");
        assert_eq!(scan.excluded_path_count, 1);
    }

    #[test]
    fn zero_count_globs_are_still_reported() {
        // A dead glob is itself a finding for the operator.
        let repo = TestRepo::new();
        repo.write("src/safe.txt", "safe\n");
        repo.write(
            ".shore/sensitivity.json",
            r#"{"schema":"shore.sensitivity-config","version":1,"excludeGlobs":["stale/**"]}"#,
        );
        repo.commit_all("base");

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();

        assert_eq!(scan.excluded_path_count, 0);
        assert_eq!(scan.exclude_globs.len(), 1);
        assert_eq!(scan.exclude_globs[0].matched, 0);
    }

    #[test]
    fn default_scan_without_config_reports_no_excludes_and_is_unchanged() {
        let repo = TestRepo::new();
        repo.write("keys/dev.pem", "-----BEGIN PRIVATE KEY-----\nredacted\n");
        repo.commit_all("base");

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();

        assert_eq!(scan.policy_outcome, "block");
        assert_eq!(scan.excluded_path_count, 0);
        assert!(scan.exclude_globs.is_empty());
    }

    #[test]
    fn excluded_paths_never_leak_into_the_serialized_scan() {
        // Globs are operator-authored config (safe to render); excluded PATHS
        // keep the scan's redaction posture — only counts appear.
        let repo = TestRepo::new();
        repo.write(
            "fixtures/dev.pem",
            "-----BEGIN PRIVATE KEY-----\nredacted\n",
        );
        repo.write(
            ".shore/sensitivity.json",
            r#"{"schema":"shore.sensitivity-config","version":1,"excludeGlobs":["fixtures/**"]}"#,
        );
        repo.commit_all("base");

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();
        let json = serde_json::to_string(&scan).unwrap();
        assert!(!json.contains("dev.pem"));
        assert!(json.contains("fixtures/**"));
    }

    #[test]
    fn sensitivity_scan_reports_redacted_findings_and_policy() {
        let repo = TestRepo::new();
        repo.write(
            "src/token.txt",
            "let key = \"sk-test000000000000000000000000\";\n",
        );
        repo.write("keys/dev.pem", "-----BEGIN PRIVATE KEY-----\nredacted\n");
        repo.write(".env", "DATABASE_URL=postgres://user:pass@example/db\n");
        repo.write(
            "config/value.txt",
            "token = hQ7x9Zp4Lm2N8vR5sT1aBcD3eFgH6jK0\n",
        );
        repo.write("target/generated/cache.bin", &"x".repeat(1024 * 1024 + 1));

        let scan = scan_worktree_sensitivity(repo.path()).unwrap();

        assert_eq!(scan.policy_outcome, "block");
        assert_finding(&scan, "known_token", "high", "block");
        assert_finding(&scan, "private_key", "high", "block");
        assert_finding(&scan, "sensitive_filename", "medium", "warn");
        assert_finding(&scan, "high_entropy", "medium", "warn");
        assert_finding(&scan, "generated_path", "medium", "warn");
        assert!(scan.findings.iter().all(|finding| {
            finding
                .references
                .iter()
                .all(|reference| reference.starts_with("file:sha256:"))
        }));

        let json = serde_json::to_string(&scan).unwrap();
        assert!(!json.contains("sk-test"));
        assert!(!json.contains("PRIVATE KEY"));
        assert!(!json.contains(".env"));
        assert!(!json.contains("target/generated"));
    }

    fn assert_finding(scan: &SensitivityScan, kind: &str, severity: &str, policy_outcome: &str) {
        let finding = scan
            .findings
            .iter()
            .find(|finding| finding.kind == kind)
            .unwrap_or_else(|| panic!("missing finding kind {kind}: {scan:?}"));
        assert_eq!(finding.severity, severity);
        assert_eq!(finding.policy_outcome, policy_outcome);
        assert!(finding.count >= 1);
    }

    struct TestRepo {
        root: TempDir,
    }

    impl TestRepo {
        fn new() -> Self {
            let root = TempDir::new().expect("create temp git repository directory");
            let repo = Self { root };
            repo.git(["init"]);
            repo.git(["config", "user.name", "Shore Tests"]);
            repo.git(["config", "user.email", "shore-tests@example.com"]);
            repo.git(["config", "commit.gpgsign", "false"]);
            repo
        }

        fn path(&self) -> &Path {
            self.root.path()
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.path().join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn commit_all(&self, message: &str) {
            self.git(["add", "--all"]);
            self.git(["commit", "-m", message]);
        }

        fn git<I, S>(&self, args: I)
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            let args = args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect::<Vec<_>>();
            let output = Command::new("git")
                .args(&args)
                .current_dir(self.root.path())
                .output()
                .unwrap_or_else(|error| {
                    panic!(
                        "run git {:?} in {}: {error}",
                        args,
                        self.root.path().display()
                    )
                });
            assert!(
                output.status.success(),
                "git {:?} failed in {}\nstdout:\n{}\nstderr:\n{}",
                args,
                self.root.path().display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
