use crate::session::state::ProjectionDiagnostic;
use crate::session::store::resolution::ReadStore;

pub(crate) const CLONE_LOCAL_UNSYNCED_LOCAL_EVENTS_CODE: &str = "clone_local_unsynced_local_events";

/// Linked-mode reads are store-only. After write-through (INV-1) new writes land
/// in the clone-local store, so this is a **residual-only** signal: it fires for
/// worktree-local events written before this worktree was linked (events absent
/// from the clone-local store), surfacing them honestly rather than unioning them
/// into the result (INV-6).
pub(crate) fn divergence_diagnostics(read_store: &ReadStore) -> Vec<ProjectionDiagnostic> {
    if read_store.local_only_event_files.is_empty() {
        return Vec::new();
    }
    vec![ProjectionDiagnostic {
        code: CLONE_LOCAL_UNSYNCED_LOCAL_EVENTS_CODE.to_owned(),
        message: format!(
            "{} local event(s) in this worktree are not yet in the linked clone-local store (written before this worktree was linked); run shore store link to copy them",
            read_store.local_only_event_files.len()
        ),
    }]
}
