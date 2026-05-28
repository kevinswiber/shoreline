mod command;
mod ingest;
mod patch;
mod raw;

pub use command::git_worktree_root;
pub(crate) use command::{
    git_head_oid, git_head_tree_oid, git_info_exclude_path, git_path_is_ignored,
};
pub(crate) use ingest::capture_worktree_diff_files;
pub use ingest::{IngestOptions, ingest_tracked_diff, ingest_tracked_diff_with_options};
