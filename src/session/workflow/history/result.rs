use serde::Serialize;

use crate::session::state::ProjectionDiagnostic;

use super::ReviewHistoryEntry;
use super::options::ReviewHistoryFilters;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryResult {
    pub event_set_hash: String,
    pub event_count: usize,
    pub filters: ReviewHistoryFilters,
    pub entries: Vec<ReviewHistoryEntry>,
    /// Diagnostics describe the full replayed event set, not only filtered entries.
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

impl ReviewHistoryResult {
    pub fn history_count(&self) -> usize {
        self.entries.len()
    }
}
