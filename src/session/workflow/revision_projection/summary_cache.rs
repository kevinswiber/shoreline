//! Content-hash-keyed cache of snapshot-derived overview counts (#426).
//!
//! The overview batch's dominant cost is resolving every listed revision's
//! snapshot artifact — a full JSON decode plus a canonical-hash revalidation —
//! only to keep two counts (`file_count` and the snapshot row count). Those
//! counts are a pure function of the content-addressed artifact body, so they
//! are keyed by the bound `contentHash` and never invalidated: a hash that
//! reappears across rebuilds (every rebuild after a store write) reuses the
//! counts without touching the artifact. Removal is decided per read *outside*
//! the cache — an operatively-removed snapshot never consults it — so a
//! trust-set or policy change cannot serve stale suppression decisions.
//!
//! This is an in-process cache over content-addressed derivations, the same
//! shape as the inspector's history projection cache (#255): recomputable,
//! never persisted, no store-dir lock (ADR-0024 records-only still holds).

use std::collections::HashMap;
use std::sync::Mutex;

/// The snapshot-derived slice of a revision's overview summary: exactly the
/// fields `build_snapshot_rows` computes from the decoded artifact body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotSummaryCounts {
    pub file_count: usize,
    /// The snapshot's own row count (`snapshot_row_count` ==
    /// `snapshot_remainder_row_count` at build time; narrative rows are layered
    /// on per read).
    pub snapshot_row_count: usize,
}

/// Shared, thread-safe map from artifact `contentHash` to its
/// [`SnapshotSummaryCounts`]. Entries are tiny (a hash string and two counts),
/// grow with distinct captured contents, and are recomputable, so there is no
/// eviction.
#[derive(Debug, Default)]
pub struct SnapshotSummaryCache {
    entries: Mutex<HashMap<String, SnapshotSummaryCounts>>,
}

impl SnapshotSummaryCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, content_hash: &str) -> Option<SnapshotSummaryCounts> {
        self.entries
            .lock()
            .expect("snapshot summary cache mutex is not poisoned")
            .get(content_hash)
            .copied()
    }

    pub(crate) fn insert(&self, content_hash: String, counts: SnapshotSummaryCounts) {
        self.entries
            .lock()
            .expect("snapshot summary cache mutex is not poisoned")
            .insert(content_hash, counts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_returns_counts_by_content_hash() {
        let cache = SnapshotSummaryCache::new();
        assert_eq!(cache.get("sha256:abc"), None);

        let counts = SnapshotSummaryCounts {
            file_count: 3,
            snapshot_row_count: 42,
        };
        cache.insert("sha256:abc".to_owned(), counts);
        assert_eq!(cache.get("sha256:abc"), Some(counts));
        assert_eq!(cache.get("sha256:other"), None);
    }
}
