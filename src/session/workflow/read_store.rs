use crate::session::state::ProjectionDiagnostic;
use crate::session::store::resolution::ReadStore;

pub(crate) const CLONE_LOCAL_UNSYNCED_LOCAL_EVENTS_CODE: &str = "clone_local_unsynced_local_events";

/// Linked-mode reads are store-only: worktree-local events invisible to this
/// read are surfaced as a diagnostic, never unioned into the result.
pub(crate) fn divergence_diagnostics(read_store: &ReadStore) -> Vec<ProjectionDiagnostic> {
    if read_store.local_only_event_files.is_empty() {
        return Vec::new();
    }
    vec![ProjectionDiagnostic {
        code: CLONE_LOCAL_UNSYNCED_LOCAL_EVENTS_CODE.to_owned(),
        message: format!(
            "{} local event(s) in this worktree are not yet in the linked clone-local store; run shore store link to copy them",
            read_store.local_only_event_files.len()
        ),
    }]
}
