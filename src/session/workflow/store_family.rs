//! The user-level family store's lifecycle verbs outside the resolve/link path:
//! `forget` (whole-store destructive, deliberately outside content-targeted
//! removal — no store survives to hold a removal event) and `list`.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::{Result, ShoreError};
use crate::session::store::inventory::scan_store_inventory;
use crate::session::store::user_level::{
    family_last_write, family_liveness, read_family_manifest, stores_root, user_level_store_dir,
};
use crate::session::workflow::store_status::StoreStatusInventory;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreForgetOptions {
    slug: String,
    yes: bool,
    force: bool,
}

impl StoreForgetOptions {
    pub fn new(slug: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            yes: false,
            force: false,
        }
    }

    /// Perform the deletion. Without it, forget previews and deletes nothing.
    pub fn with_yes(mut self, yes: bool) -> Self {
        self.yes = yes;
        self
    }

    /// Forget even when live clones are still registered.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreForgetResult {
    pub family_ref: String,
    pub dry_run: bool,
    pub deleted: bool,
    pub live_clone_count: usize,
    pub orphaned: bool,
    pub inventory: StoreStatusInventory,
}

pub fn forget_family_store(options: StoreForgetOptions) -> Result<StoreForgetResult> {
    let family_dir = user_level_store_dir(&options.slug)?;
    forget_family_store_at(&family_dir, options)
}

/// Pure test seam: identical logic, but the family directory is injected rather than
/// resolved via `SHORE_HOME`, so unit tests never mutate process env.
fn forget_family_store_at(
    family_dir: &Path,
    options: StoreForgetOptions,
) -> Result<StoreForgetResult> {
    if read_family_manifest(family_dir)?.is_none() {
        return Err(ShoreError::Message(format!(
            "family {} was forgotten or never existed",
            options.slug
        )));
    }
    let liveness = family_liveness(family_dir, &options.slug)?;
    let inventory = StoreStatusInventory::from(scan_store_inventory(family_dir, None)?);

    if !options.yes {
        return Ok(StoreForgetResult {
            family_ref: options.slug,
            dry_run: true,
            deleted: false,
            live_clone_count: liveness.live_clone_count,
            orphaned: liveness.orphaned,
            inventory,
        });
    }
    if !liveness.orphaned && !options.force {
        return Err(ShoreError::Message(format!(
            "refusing to forget family {}: {} of {} registered clone(s) still live; \
             re-run with --force to forget anyway",
            options.slug, liveness.live_clone_count, liveness.total_entries
        )));
    }
    fs::remove_dir_all(family_dir).map_err(|error| {
        ShoreError::Message(format!(
            "remove family store {}: {error}",
            family_dir.display()
        ))
    })?;
    Ok(StoreForgetResult {
        family_ref: options.slug,
        dry_run: false,
        deleted: true,
        live_clone_count: liveness.live_clone_count,
        orphaned: liveness.orphaned,
        inventory,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreListResult {
    pub families: Vec<StoreListEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreListEntry {
    pub family_ref: String,
    pub live_clone_count: usize,
    pub orphaned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_write: Option<String>,
    pub inventory: StoreStatusInventory,
}

/// Repo-less by construction: walks `<shore-home-root>/stores/` directly and never
/// resolves a git repo, a worktree, or a per-clone store (never calls
/// `git_worktree_root` / `resolve_store`).
pub fn list_family_stores() -> Result<StoreListResult> {
    let root = stores_root()?;
    list_family_stores_under(&root)
}

fn list_family_stores_under(root: &Path) -> Result<StoreListResult> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoreListResult {
                families: Vec::new(),
            });
        }
        Err(error) => {
            return Err(ShoreError::Message(format!(
                "list {}: {error}",
                root.display()
            )));
        }
    };

    let mut families = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            ShoreError::Message(format!("read entry under {}: {error}", root.display()))
        })?;
        let family_dir = entry.path();
        if !family_dir.is_dir() {
            continue;
        }
        // No family.json: not a family directory. Skip silently — this is a
        // machine-wide walk of an arbitrary shared root, not a validated registry.
        let Some(manifest) = read_family_manifest(&family_dir)? else {
            continue;
        };
        let liveness = family_liveness(&family_dir, &manifest.family_id)?;
        let last_write = family_last_write(&family_dir)?;
        let inventory = StoreStatusInventory::from(scan_store_inventory(&family_dir, None)?);
        families.push(StoreListEntry {
            family_ref: manifest.family_id,
            live_clone_count: liveness.live_clone_count,
            orphaned: liveness.orphaned,
            last_write,
            inventory,
        });
    }
    families.sort_by(|left, right| left.family_ref.cmp(&right.family_ref));
    Ok(StoreListResult { families })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::session::store::user_level::{ensure_family_store_scaffold, register_clone};

    fn scaffolded_family(root: &tempfile::TempDir, slug: &str) -> std::path::PathBuf {
        let family_dir = root.path().join(slug);
        ensure_family_store_scaffold(&family_dir, slug, &[]).unwrap();
        family_dir
    }

    #[test]
    fn dry_run_by_default_reports_and_deletes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = scaffolded_family(&root, "acme");

        let result = forget_family_store_at(&family_dir, StoreForgetOptions::new("acme")).unwrap();

        assert!(result.dry_run);
        assert!(!result.deleted);
        assert_eq!(result.live_clone_count, 0);
        assert!(result.orphaned);
        assert!(
            family_dir.join("family.json").is_file(),
            "nothing was deleted"
        );
    }

    #[test]
    fn yes_deletes_an_orphaned_family_with_no_live_clones() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = scaffolded_family(&root, "acme");

        let result =
            forget_family_store_at(&family_dir, StoreForgetOptions::new("acme").with_yes(true))
                .unwrap();

        assert!(!result.dry_run);
        assert!(result.deleted);
        assert!(!family_dir.exists());
    }

    #[test]
    fn yes_with_a_live_clone_refuses_without_force() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = scaffolded_family(&root, "acme");
        let (clone_repo, clone_ref) = live_bound_clone(&family_dir, "acme");
        register_clone(&family_dir, "acme", &clone_ref, clone_repo.path()).unwrap();

        let error =
            forget_family_store_at(&family_dir, StoreForgetOptions::new("acme").with_yes(true))
                .expect_err("a live clone must refuse without --force");

        assert!(
            error.to_string().contains("force"),
            "names the override: {error}"
        );
        assert!(
            family_dir.join("family.json").is_file(),
            "nothing was deleted"
        );
    }

    #[test]
    fn force_overrides_the_live_clone_refusal() {
        let root = tempfile::tempdir().unwrap();
        let family_dir = scaffolded_family(&root, "acme");
        let (clone_repo, clone_ref) = live_bound_clone(&family_dir, "acme");
        register_clone(&family_dir, "acme", &clone_ref, clone_repo.path()).unwrap();

        let result = forget_family_store_at(
            &family_dir,
            StoreForgetOptions::new("acme")
                .with_yes(true)
                .with_force(true),
        )
        .unwrap();

        assert!(result.deleted);
        assert!(!family_dir.exists());
    }

    #[test]
    fn a_dangling_family_ref_is_a_hard_actionable_error() {
        let root = tempfile::tempdir().unwrap();
        let never_scaffolded = root.path().join("ghost");

        let error = forget_family_store_at(&never_scaffolded, StoreForgetOptions::new("ghost"))
            .expect_err("no family.json must be a hard error, never a silent no-op");

        assert!(
            error.to_string().contains("forgotten") || error.to_string().contains("never existed"),
            "names the condition: {error}"
        );
    }

    #[test]
    fn lists_two_scaffolded_families_with_correct_orphan_flags_and_skips_a_junk_dir() {
        let root = tempfile::tempdir().unwrap();
        let live_family = scaffolded_family(&root, "acme");
        let (clone_repo, clone_ref) = live_bound_clone(&live_family, "acme");
        register_clone(&live_family, "acme", &clone_ref, clone_repo.path()).unwrap();
        scaffolded_family(&root, "beta"); // no registered clones: stays orphaned
        std::fs::create_dir_all(root.path().join("not-a-family")).unwrap(); // no family.json

        let result = list_family_stores_under(root.path()).unwrap();

        assert_eq!(result.families.len(), 2, "the junk dir is skipped silently");
        let acme = result
            .families
            .iter()
            .find(|entry| entry.family_ref == "acme")
            .expect("acme is listed");
        assert_eq!(acme.live_clone_count, 1);
        assert!(!acme.orphaned);
        let beta = result
            .families
            .iter()
            .find(|entry| entry.family_ref == "beta")
            .expect("beta is listed");
        assert_eq!(beta.live_clone_count, 0);
        assert!(beta.orphaned);
    }

    #[test]
    fn an_absent_stores_root_returns_an_empty_list_not_an_error() {
        let parent = tempfile::tempdir().unwrap();
        let never_created = parent.path().join("stores");

        let result = list_family_stores_under(&never_created).unwrap();

        assert!(result.families.is_empty());
    }

    /// A real git repo whose `.shore/store.local.json` binds it back to `slug` with
    /// an arbitrary-but-fixed opaque clone ref, so `family_liveness`'s bidirectional
    /// check (git repo exists + binding names this family back) finds it live. The
    /// literal clone-ref string only needs to match what this same fixture registers
    /// via `register_clone`.
    fn live_bound_clone(_family_dir: &Path, slug: &str) -> (tempfile::TempDir, String) {
        let clone_ref = "clone-test-0001".to_owned();
        let repo = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .arg("init")
            .current_dir(repo.path())
            .output()
            .unwrap();
        let local_config = repo.path().join(".shore/store.local.json");
        std::fs::create_dir_all(local_config.parent().unwrap()).unwrap();
        std::fs::write(
            &local_config,
            format!(
                r#"{{"schema":"shore.store-config","version":1,"mode":"shared","familyRef":"{slug}","cloneRef":"{clone_ref}"}}"#
            ),
        )
        .unwrap();
        (repo, clone_ref)
    }
}
