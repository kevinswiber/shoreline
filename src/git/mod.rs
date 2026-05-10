mod command;
mod ingest;
mod patch;
mod raw;

pub(crate) use command::git_head_oid;
pub use command::git_worktree_root;
pub(crate) use ingest::capture_worktree_diff_files;
pub use ingest::{IngestOptions, ingest_tracked_diff, ingest_tracked_diff_with_options};
