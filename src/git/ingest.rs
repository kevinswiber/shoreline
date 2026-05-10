use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::error::{Result, ShoreError};
use crate::git::command::{git_worktree_root, run_git, run_git_allowing_statuses};
use crate::git::patch::{PatchFile, parse_patch};
use crate::git::raw::{RawFile, parse_raw};
use crate::model::{DiffFile, DiffSnapshot, FileId, FileMetadataKind, FileMetadataRow, ReviewId};
use crate::session::worktree_fingerprint_for_files;

#[derive(Clone, Debug, Default)]
pub struct IngestOptions {
    helper_paths: Vec<PathBuf>,
}

impl IngestOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exclude_helper_path(mut self, path: impl AsRef<Path>) -> Self {
        self.helper_paths.push(path.as_ref().to_path_buf());
        self
    }
}

pub fn ingest_tracked_diff(repo: impl AsRef<Path>) -> Result<DiffSnapshot> {
    ingest_tracked_diff_with_options(repo, IngestOptions::new())
}

pub fn ingest_tracked_diff_with_options(
    repo: impl AsRef<Path>,
    options: IngestOptions,
) -> Result<DiffSnapshot> {
    let repo = repo.as_ref();
    let files = filter_helper_paths(
        capture_worktree_diff_files(repo)?,
        repo,
        &options.helper_paths,
    )?;
    let fingerprint = worktree_fingerprint_for_files(repo, &files)?;

    Ok(DiffSnapshot::new(
        ReviewId::new("working-tree"),
        fingerprint.snapshot_id,
        files,
    ))
}

fn filter_helper_paths(
    files: Vec<DiffFile>,
    repo: &Path,
    helper_paths: &[PathBuf],
) -> Result<Vec<DiffFile>> {
    let excluded_paths = excluded_git_paths(repo, helper_paths)?;
    if excluded_paths.is_empty() {
        return Ok(files);
    }

    Ok(files
        .into_iter()
        .filter(|file| {
            !file
                .old_path
                .as_deref()
                .is_some_and(|path| excluded_paths.contains(path))
                && !file
                    .new_path
                    .as_deref()
                    .is_some_and(|path| excluded_paths.contains(path))
        })
        .collect())
}

fn excluded_git_paths(repo: &Path, helper_paths: &[PathBuf]) -> Result<BTreeSet<String>> {
    let worktree_root = git_worktree_root(repo)?;
    let worktree_root = worktree_root.canonicalize().map_err(|error| {
        ShoreError::Message(format!(
            "canonicalize git worktree root {}: {error}",
            worktree_root.display()
        ))
    })?;
    let paths = helper_paths
        .iter()
        .filter_map(|path| {
            let helper_path = path.canonicalize().ok()?;
            let relative = helper_path.strip_prefix(&worktree_root).ok()?;
            Some(
                relative
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            )
        })
        .collect();
    Ok(paths)
}

pub(crate) fn capture_worktree_diff_files(repo: &Path) -> Result<Vec<DiffFile>> {
    let raw_output = run_git(
        repo,
        [
            "diff",
            "--raw",
            "-z",
            "--no-ext-diff",
            "--no-color",
            "--full-index",
            "-M",
            "-C",
            "--submodule=short",
            "HEAD",
            "--",
        ],
    )?;
    let patch_output = run_git(
        repo,
        [
            "diff",
            "--patch",
            "--no-ext-diff",
            "--no-color",
            "--full-index",
            "-M",
            "-C",
            "--submodule=short",
            "HEAD",
            "--",
        ],
    )?;

    let raw_files = parse_raw(&raw_output.stdout)?;
    let patch_files = parse_patch(&String::from_utf8_lossy(&patch_output.stdout))?
        .into_iter()
        .map(|file| (file.key(), file))
        .collect::<BTreeMap<_, _>>();

    let mut files = raw_files
        .into_iter()
        .map(|raw_file| {
            let key = raw_file.key();
            let patch_file = patch_files
                .get(&key)
                .ok_or_else(|| ShoreError::Message(format!("missing patch entry for {key}")))?;
            diff_file(raw_file, patch_file, false)
        })
        .collect::<Result<Vec<_>>>()?;
    files.extend(synthesize_untracked_files(repo)?);

    Ok(files)
}

fn synthesize_untracked_files(repo: &Path) -> Result<Vec<DiffFile>> {
    discover_untracked_files(repo)?
        .into_iter()
        .map(|path| synthesize_untracked_file(repo, &path))
        .collect()
}

fn discover_untracked_files(repo: &Path) -> Result<Vec<String>> {
    let output = run_git(
        repo,
        ["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| {
            std::str::from_utf8(field)
                .map(str::to_owned)
                .map_err(|error| {
                    ShoreError::Message(format!("untracked path is not utf-8: {error}"))
                })
        })
        .collect()
}

fn synthesize_untracked_file(repo: &Path, path: &str) -> Result<DiffFile> {
    let patch = no_index_patch(repo, path)?;
    let patch_file = parse_patch(&String::from_utf8_lossy(&patch.stdout))?
        .into_iter()
        .next()
        .ok_or_else(|| ShoreError::Message(format!("missing no-index patch for {path}")))?;
    let raw_file = RawFile {
        status: crate::model::FileStatus::Added,
        old_mode: None,
        new_mode: patch_file.new_mode.clone(),
        old_oid: None,
        new_oid: None,
        similarity: None,
        old_path: None,
        new_path: Some(path.to_owned()),
    };
    diff_file(raw_file, &patch_file, true)
}

fn no_index_patch(repo: &Path, path: &str) -> Result<crate::git::command::GitOutput> {
    let args = vec![
        OsString::from("diff"),
        OsString::from("--no-index"),
        OsString::from("--no-ext-diff"),
        OsString::from("--no-color"),
        OsString::from("--full-index"),
        OsString::from("--"),
        OsString::from("/dev/null"),
        OsString::from(PathBuf::from(path)),
    ];
    run_git_allowing_statuses(repo, args, &[0, 1])
}

fn diff_file(raw_file: RawFile, patch_file: &PatchFile, synthetic: bool) -> Result<DiffFile> {
    let is_submodule = raw_file.is_submodule();
    let is_mode_only = raw_file.is_mode_only();
    let is_binary = patch_file.is_binary;
    let metadata_rows = metadata_rows(&raw_file, patch_file);
    let hunks = if metadata_rows.is_empty() {
        patch_file.hunks.clone()
    } else {
        Vec::new()
    };

    Ok(DiffFile {
        id: FileId::new(raw_file.key()),
        status: raw_file.status,
        old_path: raw_file.old_path,
        new_path: raw_file.new_path,
        old_mode: raw_file.old_mode.or_else(|| patch_file.old_mode.clone()),
        new_mode: raw_file.new_mode.or_else(|| patch_file.new_mode.clone()),
        old_oid: raw_file.old_oid,
        new_oid: raw_file.new_oid,
        similarity: raw_file.similarity.or(patch_file.similarity),
        is_binary,
        is_submodule,
        is_mode_only,
        synthetic,
        metadata_rows,
        hunks,
    })
}

fn metadata_rows(
    raw_file: &crate::git::raw::RawFile,
    patch_file: &PatchFile,
) -> Vec<FileMetadataRow> {
    let mut rows = Vec::new();
    if matches!(
        raw_file.status,
        crate::model::FileStatus::Renamed | crate::model::FileStatus::Copied
    ) {
        rows.push(FileMetadataRow {
            kind: FileMetadataKind::RenameSummary,
            text: match (&raw_file.old_path, &raw_file.new_path, raw_file.similarity) {
                (Some(old), Some(new), Some(similarity)) => {
                    format!("renamed {old} -> {new} ({similarity}%)")
                }
                (Some(old), Some(new), None) => format!("renamed {old} -> {new}"),
                _ => "renamed file".to_owned(),
            },
        });
    }
    if patch_file.is_binary {
        rows.push(FileMetadataRow {
            kind: FileMetadataKind::BinarySummary,
            text: "binary files differ".to_owned(),
        });
    }
    if raw_file.is_mode_only() {
        rows.push(FileMetadataRow {
            kind: FileMetadataKind::ModeChange,
            text: match (&raw_file.old_mode, &raw_file.new_mode) {
                (Some(old), Some(new)) => format!("mode changed {old} -> {new}"),
                _ => "mode changed".to_owned(),
            },
        });
    }
    if raw_file.is_submodule() {
        rows.push(FileMetadataRow {
            kind: FileMetadataKind::SubmoduleSummary,
            text: match (&raw_file.old_oid, &raw_file.new_oid) {
                (Some(old), Some(new)) => format!("submodule changed {old} -> {new}"),
                _ => "submodule changed".to_owned(),
            },
        });
    }
    rows
}
