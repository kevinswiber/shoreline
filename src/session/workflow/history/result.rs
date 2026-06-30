use serde::Serialize;

use super::options::ReviewHistoryFilters;
use super::summary::ReviewHistoryEntry;
use crate::session::state::ProjectionDiagnostic;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryResult {
    pub event_set_hash: String,
    pub event_count: usize,
    pub filters: ReviewHistoryFilters,
    pub entries: Vec<ReviewHistoryEntry>,
    /// An opaque continuation token for the next page when a window was applied
    /// and entries remain after it; `null` for an unwindowed or final page.
    pub next_cursor: Option<String>,
    /// Diagnostics describe the full replayed event set, not only filtered entries.
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

impl ReviewHistoryResult {
    pub fn history_count(&self) -> usize {
        self.entries.len()
    }
}
