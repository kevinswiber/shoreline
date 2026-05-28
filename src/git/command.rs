use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Result, ShoreError};

#[derive(Debug)]
pub(crate) struct GitOutput {
    pub stdout: Vec<u8>,
}

pub(crate) fn run_git<I, S>(cwd: &Path, args: I) -> Result<GitOutput>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_git_allowing_statuses(cwd, args, &[0])
}

pub fn git_worktree_root(repo: &Path) -> Result<PathBuf> {
    let output = run_git(repo, ["rev-parse", "--show-toplevel"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim_end_matches(['\r', '\n']);
    if root.is_empty() {
        return Err(ShoreError::Message(format!(
            "git rev-parse returned empty worktree root for {}",
            repo.display()
        )));
    }

    Ok(PathBuf::from(root))
}

pub fn git_info_exclude_path(repo: &Path) -> Result<PathBuf> {
    let output = run_git(repo, ["rev-parse", "--git-path", "info/exclude"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let relative = stdout.trim_end_matches(['\r', '\n']);
    if relative.is_empty() {
        return Err(ShoreError::Message(format!(
            "git rev-parse returned empty info/exclude path for {}",
            repo.display()
        )));
    }

    // `git rev-parse --git-path` resolves against the working directory we ran
    // it from (the worktree root). Joining keeps relative results anchored to
    // `repo` while preserving absolute results (linked worktrees share the
    // common `info/exclude`), since `Path::join` discards the base for an
    // absolute child.
    Ok(repo.join(relative))
}

/// Reports whether `pathspec` is ignored by the standard Git exclude sources
/// (the worktree `.gitignore`, the global excludes file, and the repository
/// `.git/info/exclude`). This mirrors the `--exclude-standard` rules used when
/// Shoreline discovers untracked files.
pub fn git_path_is_ignored(repo: &Path, pathspec: &str) -> Result<bool> {
    // `git check-ignore` prints matching paths to stdout and exits 1 (no error)
    // when nothing matches, so a non-empty stdout is the "ignored" signal.
    let output = run_git_allowing_statuses(repo, ["check-ignore", pathspec], &[0, 1])?;
    Ok(!output.stdout.is_empty())
}

pub fn git_head_oid(repo: &Path) -> Result<String> {
    let output = run_git(repo, ["rev-parse", "HEAD"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let oid = stdout.trim_end_matches(['\r', '\n']);
    if oid.is_empty() {
        return Err(ShoreError::Message(format!(
            "git rev-parse returned empty HEAD oid for {}",
            repo.display()
        )));
    }

    Ok(oid.to_owned())
}

pub fn git_head_tree_oid(repo: &Path) -> Result<String> {
    let output = run_git(repo, ["rev-parse", "HEAD^{tree}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let oid = stdout.trim_end_matches(['\r', '\n']);
    if oid.is_empty() {
        return Err(ShoreError::Message(format!(
            "git rev-parse returned empty HEAD tree oid for {}",
            repo.display()
        )));
    }

    Ok(oid.to_owned())
}

pub(crate) fn run_git_allowing_statuses<I, S>(
    cwd: &Path,
    args: I,
    allowed_statuses: &[i32],
) -> Result<GitOutput>
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
        .current_dir(cwd)
        .output()
        .map_err(|error| ShoreError::Message(format!("run git {:?}: {error}", args)))?;

    let status_code = output.status.code();
    if !status_code.is_some_and(|code| allowed_statuses.contains(&code)) {
        return Err(ShoreError::GitCommand {
            command: format!("{args:?}"),
            status: output.status.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(GitOutput {
        stdout: output.stdout,
    })
}
