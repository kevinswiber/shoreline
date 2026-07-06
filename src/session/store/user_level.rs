//! User-level family-store placement primitives.
//!
//! Every family store lives at `<shore-home-root>/stores/<slug>/`. This module is
//! the single family-path derivation site: nothing else composes a family path.
//! The `stores/` segment keeps `<root>/{keys,stores}` disjoint by construction, so
//! a family named `keys` can never collide with the keystore. The slug is a
//! non-identity placement label — never folded into a content hash, id, or signed
//! bytes — so it is freely renameable.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, ShoreError};
use crate::git::git_worktree_root;
use crate::session::store::store_config::resolve_family_binding;
use crate::session::store::store_init::ensure_store_dirs;
use crate::session::{format_rfc3339_utc_millis, now_rfc3339_utc};
use crate::storage::{Durability, LocalStorage};

/// Longest permitted family slug, in bytes. The slug is a directory name and a
/// non-identity placement label; it is never folded into any content hash, id, or
/// signed bytes.
const MAX_SLUG_LEN: usize = 64;

/// The machine-wide root under which every family store lives:
/// `<shore-home-root>/stores/`. The `stores/` segment keeps `<root>/{keys,stores}`
/// disjoint from the keystore by construction.
pub(crate) fn stores_root() -> Result<PathBuf> {
    Ok(crate::shore_home::shore_home_root()?.join("stores"))
}

/// The family store directory for `slug`: `<shore-home-root>/stores/<slug>`. The
/// single family-path derivation site. Validates the slug before joining.
pub(crate) fn user_level_store_dir(slug: &str) -> Result<PathBuf> {
    user_level_store_dir_under(&crate::shore_home::shore_home_root()?, slug)
}

/// Pure derivation seam: `<root>/stores/<slug>` after slug validation. Kept
/// env-free so resolution and lifecycle tests can inject a tempdir root without
/// mutating process env.
fn user_level_store_dir_under(root: &Path, slug: &str) -> Result<PathBuf> {
    validate_family_slug(slug)?;
    Ok(root.join("stores").join(slug))
}

/// Validate a family slug: non-empty, at most 64 bytes, charset `[a-z0-9-]`. Every
/// error is actionable — it names the charset and the offending slug so the user
/// can pick a legal name.
pub(crate) fn validate_family_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(ShoreError::Message(
            "a family slug must not be empty; use lowercase letters, digits, and hyphens \
             (for example `acme-web`)"
                .to_owned(),
        ));
    }
    if slug.len() > MAX_SLUG_LEN {
        return Err(ShoreError::Message(format!(
            "family slug `{slug}` is {} bytes; the maximum is {MAX_SLUG_LEN}",
            slug.len(),
        )));
    }
    if !slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(ShoreError::Message(format!(
            "family slug `{slug}` is invalid; use only lowercase letters, digits, and hyphens \
             (`[a-z0-9-]`)"
        )));
    }
    Ok(())
}

const FAMILY_MANIFEST_SCHEMA: &str = "shore.family-manifest";
const FAMILY_MANIFEST_VERSION: u32 = 1;
const FAMILY_MANIFEST_FILE: &str = "family.json";
const FAMILY_GITIGNORE_BODY: &str = "state.json\nregistry.json\n";

/// The identity stamp for a family store. `family_id` is the placement slug (never
/// an identity token); `created_at` is an RFC 3339 UTC instant.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FamilyManifest {
    pub schema: String,
    pub version: u32,
    pub family_id: String,
    pub created_at: String,
    /// The founding clone's root-commit OIDs (`git rev-list --max-parents=0`), the
    /// anchor set the advisory history-overlap check compares against. May be empty
    /// (an anchorless family skips the advisory).
    #[serde(default)]
    pub root_commit_oids: Vec<String>,
}

/// Read `<family_dir>/family.json`. Absent → `None`; an unsupported schema/version
/// is a hard, actionable error naming the file (a family store must never be
/// misread as belonging to another slug).
pub(crate) fn read_family_manifest(family_dir: &Path) -> Result<Option<FamilyManifest>> {
    let path = family_dir.join(FAMILY_MANIFEST_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ShoreError::Message(format!(
                "read family manifest {}: {error}",
                path.display()
            )));
        }
    };
    let manifest: FamilyManifest = serde_json::from_slice(&bytes).map_err(|error| {
        ShoreError::Message(format!(
            "family manifest {} is malformed: {error}",
            path.display()
        ))
    })?;
    if manifest.schema != FAMILY_MANIFEST_SCHEMA || manifest.version != FAMILY_MANIFEST_VERSION {
        return Err(ShoreError::Message(format!(
            "family manifest {} has unsupported schema/version {} v{} (expected {} v{})",
            path.display(),
            manifest.schema,
            manifest.version,
            FAMILY_MANIFEST_SCHEMA,
            FAMILY_MANIFEST_VERSION
        )));
    }
    Ok(Some(manifest))
}

/// Write a fresh `family.json` stamped with `slug` and the founding clone's
/// root-commit anchors. Pretty-printed with a trailing newline, like
/// `write_store_config`, so the stamp inspects cleanly.
pub(crate) fn write_family_manifest(
    family_dir: &Path,
    slug: &str,
    root_commit_oids: &[String],
) -> Result<()> {
    let manifest = FamilyManifest {
        schema: FAMILY_MANIFEST_SCHEMA.to_owned(),
        version: FAMILY_MANIFEST_VERSION,
        family_id: slug.to_owned(),
        created_at: now_rfc3339_utc(),
        root_commit_oids: root_commit_oids.to_vec(),
    };
    let mut bytes = serde_json::to_vec_pretty(&manifest)?;
    bytes.push(b'\n');
    let path = family_dir.join(FAMILY_MANIFEST_FILE);
    std::fs::write(&path, &bytes)
        .map_err(|error| ShoreError::Message(format!("write {}: {error}", path.display())))
}

/// Eager, idempotent family-store scaffold: create the store dir layout, stamp the
/// manifest **once**, and drop the machine-local `.gitignore`. Returns `true` when
/// this call created the family (wrote a new manifest) and `false` when the family
/// already existed. A pre-existing manifest stamped for a different slug is a hard
/// error (the family-stamp guard — no override), so re-stamping never silently
/// unions two families under one directory.
pub(crate) fn ensure_family_store_scaffold(
    family_dir: &Path,
    slug: &str,
    root_commit_oids: &[String],
) -> Result<bool> {
    ensure_store_dirs(family_dir)?;
    let created = match read_family_manifest(family_dir)? {
        Some(existing) if existing.family_id != slug => {
            return Err(ShoreError::Message(format!(
                "family store {} is already stamped for family `{}`, not `{}`; refusing to reuse it",
                family_dir.join(FAMILY_MANIFEST_FILE).display(),
                existing.family_id,
                slug
            )));
        }
        Some(_) => false,
        None => {
            write_family_manifest(family_dir, slug, root_commit_oids)?;
            true
        }
    };
    write_family_gitignore(family_dir)?;
    Ok(created)
}

/// The family `.gitignore`: covers exactly the machine-local files (`state.json`,
/// `registry.json`). Fixed body → writing it unconditionally is idempotent by
/// content.
fn write_family_gitignore(family_dir: &Path) -> Result<()> {
    let path = family_dir.join(".gitignore");
    std::fs::write(&path, FAMILY_GITIGNORE_BODY)
        .map_err(|error| ShoreError::Message(format!("write {}: {error}", path.display())))
}

const FAMILY_REGISTRY_SCHEMA: &str = "shore.family-registry";
const FAMILY_REGISTRY_VERSION: u32 = 1;
const FAMILY_REGISTRY_FILE: &str = "registry.json";

/// Machine-local membership bookkeeping for a family store — outside the event
/// log, gitignored. Whole-set read-modify-write; no lock.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FamilyRegistry {
    pub schema: String,
    pub version: u32,
    pub entries: Vec<FamilyRegistryEntry>,
}

impl FamilyRegistry {
    fn empty() -> Self {
        Self {
            schema: FAMILY_REGISTRY_SCHEMA.to_owned(),
            version: FAMILY_REGISTRY_VERSION,
            entries: Vec::new(),
        }
    }
}

/// One member clone. `worktree_path` is a raw local path that lives ONLY inside the
/// gitignored registry (raw paths stay off the wire); `clone_ref` is the opaque
/// dedup key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FamilyRegistryEntry {
    pub clone_ref: String,
    pub worktree_path: PathBuf,
    pub linked_at: String,
}

/// Read `<family_dir>/registry.json`. Absent → an empty registry (a family with no
/// members reads cleanly). Unsupported schema/version is a hard error naming the
/// file.
pub(crate) fn read_family_registry(family_dir: &Path) -> Result<FamilyRegistry> {
    let path = family_dir.join(FAMILY_REGISTRY_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FamilyRegistry::empty());
        }
        Err(error) => {
            return Err(ShoreError::Message(format!(
                "read family registry {}: {error}",
                path.display()
            )));
        }
    };
    let registry: FamilyRegistry = serde_json::from_slice(&bytes).map_err(|error| {
        ShoreError::Message(format!(
            "family registry {} is malformed: {error}",
            path.display()
        ))
    })?;
    if registry.schema != FAMILY_REGISTRY_SCHEMA || registry.version != FAMILY_REGISTRY_VERSION {
        return Err(ShoreError::Message(format!(
            "family registry {} has unsupported schema/version {} v{} (expected {} v{})",
            path.display(),
            registry.schema,
            registry.version,
            FAMILY_REGISTRY_SCHEMA,
            FAMILY_REGISTRY_VERSION
        )));
    }
    Ok(registry)
}

/// Register (or refresh) a clone in the family, deduped by `clone_ref`: a re-link of
/// the same clone updates its recorded path and `linked_at` in place. Atomic
/// whole-set RMW. The `slug` is validated defensively so a clone can never be
/// registered into a bad-slug family.
pub(crate) fn register_clone(
    family_dir: &Path,
    slug: &str,
    clone_ref: &str,
    worktree_path: &Path,
) -> Result<()> {
    validate_family_slug(slug)?;
    let mut registry = read_family_registry(family_dir)?;
    let linked_at = now_rfc3339_utc();
    match registry
        .entries
        .iter_mut()
        .find(|entry| entry.clone_ref == clone_ref)
    {
        Some(entry) => {
            entry.worktree_path = worktree_path.to_path_buf();
            entry.linked_at = linked_at;
        }
        None => registry.entries.push(FamilyRegistryEntry {
            clone_ref: clone_ref.to_owned(),
            worktree_path: worktree_path.to_path_buf(),
            linked_at,
        }),
    }
    write_family_registry(family_dir, &registry)
}

/// Remove a clone from the family by `clone_ref`. Returns whether an entry was
/// removed. An absent entry or absent registry is a clean `false` no-op — `unlink`
/// (and a re-run of it) must never fail because the clone was never registered or
/// the registry is already gone.
pub(crate) fn deregister_clone(family_dir: &Path, clone_ref: &str) -> Result<bool> {
    let mut registry = read_family_registry(family_dir)?;
    let before = registry.entries.len();
    registry
        .entries
        .retain(|entry| entry.clone_ref != clone_ref);
    let removed = registry.entries.len() != before;
    if removed {
        write_family_registry(family_dir, &registry)?;
    }
    Ok(removed)
}

fn write_family_registry(family_dir: &Path, registry: &FamilyRegistry) -> Result<()> {
    LocalStorage::new(family_dir).write_json_atomic(
        Path::new(FAMILY_REGISTRY_FILE),
        registry,
        Durability::Projection,
    )
}

/// Re-derived-on-demand liveness facts for a family store (no stored idle state).
/// `orphaned` is binary: ORPHANED ⇔ zero live entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FamilyLiveness {
    pub live_clone_count: usize,
    pub total_entries: usize,
    pub orphaned: bool,
}

/// Count the live member clones of `slug`'s family. An entry is live iff its
/// recorded `worktree_path` still exists AND is a git repo AND that clone's local
/// binding still names THIS family back (a mutual back-pointer). ORPHANED ⇔ zero
/// live entries — including a never-linked/empty registry.
pub(crate) fn family_liveness(family_dir: &Path, slug: &str) -> Result<FamilyLiveness> {
    let registry = read_family_registry(family_dir)?;
    let total_entries = registry.entries.len();
    let mut live_clone_count = 0usize;
    for entry in &registry.entries {
        if clone_entry_is_live(&entry.worktree_path, slug)? {
            live_clone_count += 1;
        }
    }
    Ok(FamilyLiveness {
        live_clone_count,
        total_entries,
        orphaned: live_clone_count == 0,
    })
}

fn clone_entry_is_live(worktree_path: &Path, slug: &str) -> Result<bool> {
    if !worktree_path.exists() {
        return Ok(false);
    }
    // A recorded path that is no longer a git repo (rm -rf'd, replaced) is dead.
    if git_worktree_root(worktree_path).is_err() {
        return Ok(false);
    }
    // Bidirectional: the clone's local binding must still name this family.
    Ok(resolve_family_binding(worktree_path)?.is_some_and(|binding| binding.family_ref == slug))
}

/// The newest `events/` file mtime as an RFC 3339 UTC string — raw idle facts,
/// never a coded label. `None` when the family has no events (absent dir or no
/// `*.json`).
pub(crate) fn family_last_write(family_dir: &Path) -> Result<Option<String>> {
    let events_dir = family_dir.join("events");
    let entries = match std::fs::read_dir(&events_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ShoreError::Message(format!(
                "read family events dir {}: {error}",
                events_dir.display()
            )));
        }
    };
    let mut max_millis: Option<i64> = None;
    for entry in entries {
        let path = entry
            .map_err(|error| {
                ShoreError::Message(format!(
                    "read family events entry under {}: {error}",
                    events_dir.display()
                ))
            })?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let millis = file_mtime_millis(&path)?;
        max_millis = Some(max_millis.map_or(millis, |current| current.max(millis)));
    }
    Ok(max_millis.map(format_rfc3339_utc_millis))
}

fn file_mtime_millis(path: &Path) -> Result<i64> {
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            ShoreError::Message(format!("stat family event {}: {error}", path.display()))
        })?;
    Ok(modified
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0))
}

/// Best-effort, warn-only heuristic: does `root`'s path look like it lives on a
/// sync-managed filesystem? Case-insensitive path-substring match only — it cannot
/// see the real mount, so it never blocks; it exists to catch the
/// `~/.shore`-looks-syncable footgun.
pub(crate) fn flag_unsupported_filesystem(root: &Path) -> Option<String> {
    let haystack = root.to_string_lossy().to_ascii_lowercase();
    const MARKERS: &[(&str, &str)] = &[
        ("dropbox", "Dropbox"),
        ("library/mobile documents", "iCloud Drive"),
        ("icloud", "iCloud"),
        ("onedrive", "OneDrive"),
        ("google drive", "Google Drive"),
    ];
    for (needle, label) in MARKERS {
        if haystack.contains(needle) {
            return Some(format!(
                "the family store path looks like it lives on {label}, a sync-managed \
                 filesystem; family stores must live on a local POSIX filesystem \
                 (best-effort path heuristic, warn-only)"
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::session::store::store_config::set_family_binding_for_repo;

    fn init_git_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().expect("temp git repo");
        for args in [
            vec!["init"],
            vec!["config", "user.name", "Shore Tests"],
            vec!["config", "user.email", "shore-tests@example.com"],
            vec!["config", "commit.gpgsign", "false"],
        ] {
            let out = std::process::Command::new("git")
                .args(&args)
                .current_dir(repo.path())
                .output()
                .expect("run git");
            assert!(out.status.success(), "git {args:?} failed");
        }
        std::fs::write(repo.path().join("README.md"), "base\n").unwrap();
        for args in [vec!["add", "--all"], vec!["commit", "-m", "base"]] {
            let out = std::process::Command::new("git")
                .args(&args)
                .current_dir(repo.path())
                .output()
                .expect("run git");
            assert!(out.status.success(), "git {args:?} failed");
        }
        repo
    }

    #[test]
    fn store_dir_under_root_is_stores_slash_slug() {
        let root = PathBuf::from("/home/dev/.shore");
        let dir = user_level_store_dir_under(&root, "acme-web").unwrap();
        assert_eq!(dir, PathBuf::from("/home/dev/.shore/stores/acme-web"));
    }

    #[test]
    fn a_family_named_keys_lands_under_stores_disjoint_from_the_keystore() {
        // The `stores/` segment keeps `<root>/{keys,stores}` disjoint by
        // construction: a family literally named "keys" is `<root>/stores/keys`,
        // never `<root>/keys`.
        let root = PathBuf::from("/home/dev/.shore");
        let dir = user_level_store_dir_under(&root, "keys").unwrap();
        assert_eq!(dir, root.join("stores").join("keys"));
        assert_ne!(dir, root.join("keys"));
    }

    #[test]
    fn valid_slugs_are_accepted() {
        for slug in ["a", "acme-web", "repo-42", "0", "-", &"a".repeat(64)] {
            assert!(
                validate_family_slug(slug).is_ok(),
                "slug {slug:?} must be valid"
            );
        }
    }

    #[test]
    fn invalid_slugs_are_rejected() {
        // Uppercase, whitespace, empty, over-64-bytes, and path separators are all
        // rejected — the slug is a directory name and a placement label only.
        for slug in ["Acme", "a b", "", &"a".repeat(65), "a/b", "a.b", "a_b"] {
            assert!(
                validate_family_slug(slug).is_err(),
                "slug {slug:?} must be rejected"
            );
        }
    }

    #[test]
    fn store_dir_under_root_rejects_an_invalid_slug() {
        let root = PathBuf::from("/home/dev/.shore");
        assert!(user_level_store_dir_under(&root, "a/b").is_err());
    }

    #[test]
    fn scaffold_creates_store_dirs_manifest_and_gitignore() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("my-family");

        let created =
            ensure_family_store_scaffold(&family_dir, "my-family", &["a".repeat(40)]).unwrap();

        assert!(created, "a fresh family writes its manifest");
        assert!(family_dir.join("events").is_dir());
        assert!(family_dir.join("artifacts/objects").is_dir());
        assert!(family_dir.join("artifacts/notes").is_dir());

        let manifest = read_family_manifest(&family_dir)
            .unwrap()
            .expect("manifest present");
        assert_eq!(manifest.schema, "shore.family-manifest");
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.family_id, "my-family");
        // The founding clone's history anchors.
        assert_eq!(manifest.root_commit_oids, vec!["a".repeat(40)]);
        // RFC 3339 UTC instant (minted via the crate clock idiom).
        assert!(manifest.created_at.ends_with('Z') && manifest.created_at.contains('T'));
    }

    #[test]
    fn family_gitignore_body_is_exactly_the_machine_local_files() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        ensure_family_store_scaffold(&family_dir, "fam", &[]).unwrap();

        let body = std::fs::read_to_string(family_dir.join(".gitignore")).unwrap();
        assert_eq!(body, "state.json\nregistry.json\n");
    }

    #[test]
    fn re_scaffold_same_slug_is_a_no_op_and_preserves_created_at() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");

        assert!(ensure_family_store_scaffold(&family_dir, "fam", &["a".repeat(40)]).unwrap());
        let first = read_family_manifest(&family_dir).unwrap().unwrap();

        // A second scaffold must NOT rewrite the manifest (no fresh created_at, no
        // replaced anchors — the manifest records the FOUNDING clone's set) and must
        // report the family already existed.
        assert!(!ensure_family_store_scaffold(&family_dir, "fam", &["b".repeat(40)]).unwrap());
        let second = read_family_manifest(&family_dir).unwrap().unwrap();
        assert_eq!(
            first.created_at, second.created_at,
            "re-scaffold never re-stamps created_at"
        );
        assert_eq!(
            second.root_commit_oids,
            vec!["a".repeat(40)],
            "anchors are founding-clone-only"
        );
    }

    #[test]
    fn re_scaffold_with_a_different_slug_is_a_hard_error() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        ensure_family_store_scaffold(&family_dir, "fam", &[]).unwrap();

        let error = ensure_family_store_scaffold(&family_dir, "other", &[])
            .expect_err("a family-stamp mismatch must refuse, never re-stamp");
        let message = error.to_string();
        assert!(
            message.contains("fam"),
            "names the stamped family: {message}"
        );
        assert!(
            message.contains("other"),
            "names the requested slug: {message}"
        );
    }

    #[test]
    fn read_family_manifest_absent_is_none() {
        let root = tempfile::tempdir().unwrap();
        assert!(read_family_manifest(root.path()).unwrap().is_none());
    }

    #[test]
    fn read_family_manifest_wrong_schema_is_a_hard_error() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path()).unwrap();
        std::fs::write(
            root.path().join("family.json"),
            r#"{"schema":"shore.family-manifest","version":999,"familyId":"x","createdAt":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();

        let error =
            read_family_manifest(root.path()).expect_err("an unsupported version is a hard error");
        let message = error.to_string();
        assert!(
            message.contains("family.json"),
            "names the offending file: {message}"
        );
    }

    #[test]
    fn register_creates_the_registry_with_one_entry() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();
        let worktree = root.path().join("clone-a");

        register_clone(&family_dir, "fam", "abcdef0123456789", &worktree).unwrap();

        let registry = read_family_registry(&family_dir).unwrap();
        assert_eq!(registry.schema, "shore.family-registry");
        assert_eq!(registry.version, 1);
        assert_eq!(registry.entries.len(), 1);
        let entry = &registry.entries[0];
        assert_eq!(entry.clone_ref, "abcdef0123456789");
        assert_eq!(entry.worktree_path, worktree);
        assert!(entry.linked_at.ends_with('Z') && entry.linked_at.contains('T'));
        // Persisted atomically as a real file on disk.
        assert!(family_dir.join("registry.json").is_file());
    }

    #[test]
    fn re_registering_the_same_clone_ref_dedups_and_updates_in_place() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();

        register_clone(
            &family_dir,
            "fam",
            "dupref0000000000",
            &root.path().join("old"),
        )
        .unwrap();
        register_clone(
            &family_dir,
            "fam",
            "dupref0000000000",
            &root.path().join("new"),
        )
        .unwrap();

        let registry = read_family_registry(&family_dir).unwrap();
        assert_eq!(
            registry.entries.len(),
            1,
            "same clone_ref is not duplicated"
        );
        assert_eq!(registry.entries[0].worktree_path, root.path().join("new"));
    }

    #[test]
    fn deregister_removes_the_entry_and_reports_it() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();
        register_clone(
            &family_dir,
            "fam",
            "gone000000000000",
            &root.path().join("c"),
        )
        .unwrap();

        let removed = deregister_clone(&family_dir, "gone000000000000").unwrap();

        assert!(removed, "deregister reports it removed an entry");
        assert!(
            read_family_registry(&family_dir)
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn deregister_absent_entry_is_a_clean_no_op() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();

        // Absent registry AND absent entry both succeed as a no-op: unlink must
        // detach a clone whose registry was never written or already lost.
        let removed = deregister_clone(&family_dir, "never000000000000").unwrap();
        assert!(!removed);
    }

    #[test]
    fn read_registry_absent_is_empty() {
        let root = tempfile::tempdir().unwrap();
        let registry = read_family_registry(root.path()).unwrap();
        assert!(registry.entries.is_empty());
        assert_eq!(registry.schema, "shore.family-registry");
        assert_eq!(registry.version, 1);
    }

    #[test]
    fn read_registry_wrong_schema_is_a_hard_error() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("registry.json"),
            r#"{"schema":"shore.family-registry","version":999,"entries":[]}"#,
        )
        .unwrap();

        let error =
            read_family_registry(root.path()).expect_err("an unsupported version is a hard error");
        assert!(error.to_string().contains("registry.json"));
    }

    #[test]
    fn a_bound_live_clone_counts_as_live() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();

        // A real git repo whose local binding names THIS family back.
        let clone = init_git_repo();
        set_family_binding_for_repo(clone.path(), "fam", "cloneref00000000").unwrap();
        register_clone(&family_dir, "fam", "cloneref00000000", clone.path()).unwrap();

        let liveness = family_liveness(&family_dir, "fam").unwrap();
        assert_eq!(liveness.total_entries, 1);
        assert_eq!(liveness.live_clone_count, 1);
        assert!(!liveness.orphaned);
    }

    #[test]
    fn a_vanished_clone_path_counts_as_dead_and_orphans_the_family() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();
        // Registered path never existed (or was rm -rf'd): not live.
        register_clone(
            &family_dir,
            "fam",
            "ghost00000000000",
            &root.path().join("gone"),
        )
        .unwrap();

        let liveness = family_liveness(&family_dir, "fam").unwrap();
        assert_eq!(liveness.total_entries, 1);
        assert_eq!(liveness.live_clone_count, 0);
        assert!(liveness.orphaned, "zero live entries ⇒ ORPHANED");
    }

    #[test]
    fn a_clone_rebound_to_another_family_counts_as_dead() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();

        // A real git repo, but its local binding names a DIFFERENT family — the
        // bidirectional back-pointer fails, so it is not live under "fam".
        let clone = init_git_repo();
        set_family_binding_for_repo(clone.path(), "other", "cloneref00000000").unwrap();
        register_clone(&family_dir, "fam", "cloneref00000000", clone.path()).unwrap();

        let liveness = family_liveness(&family_dir, "fam").unwrap();
        assert_eq!(liveness.live_clone_count, 0);
        assert!(liveness.orphaned);
    }

    #[test]
    fn empty_registry_is_orphaned() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");
        std::fs::create_dir_all(&family_dir).unwrap();

        let liveness = family_liveness(&family_dir, "fam").unwrap();
        assert_eq!(liveness.total_entries, 0);
        assert_eq!(liveness.live_clone_count, 0);
        assert!(liveness.orphaned, "a never-linked family is orphaned too");
    }

    #[test]
    fn last_write_is_none_for_an_empty_family_and_some_for_a_written_one() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = root.path().join("stores").join("fam");

        // No events dir at all.
        assert!(family_last_write(&family_dir).unwrap().is_none());

        // One event file → an RFC 3339 last-write.
        std::fs::create_dir_all(family_dir.join("events")).unwrap();
        std::fs::write(family_dir.join("events").join("aaaa.json"), "{}").unwrap();
        let last = family_last_write(&family_dir)
            .unwrap()
            .expect("some last-write");
        assert!(
            last.ends_with('Z') && last.contains('T'),
            "RFC 3339 UTC: {last}"
        );
    }

    #[test]
    fn filesystem_heuristic_flags_sync_managed_roots_and_clears_plain_ones() {
        // A path component naming a known sync app trips the warning…
        let dropbox = std::path::Path::new("/Users/x/Dropbox/shore/stores/fam");
        assert!(flag_unsupported_filesystem(dropbox).is_some());
        let icloud = std::path::Path::new("/Users/x/Library/Mobile Documents/shore/stores/fam");
        assert!(flag_unsupported_filesystem(icloud).is_some());

        // …a plain temp path does not.
        let plain = tempfile::tempdir().unwrap();
        assert!(flag_unsupported_filesystem(plain.path()).is_none());
    }
}
