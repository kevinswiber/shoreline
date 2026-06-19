use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::model::{
    ReviewEndpoint, ReviewUnitId, ReviewUnitSource, RevisionId, SessionId, SnapshotId,
};
use crate::session::event::{EventType, ReviewUnitCapturedPayload, ShoreEvent};
use crate::session::state::{ProjectionDiagnostic, SessionState};
use crate::session::store::resolution::resolve_read_store;
use crate::session::workflow::association::normalize_ref;
use crate::session::workflow::commit_range_liveness::{CommitGraphCondition, enrich_liveness};
use crate::session::workflow::read_store::divergence_diagnostics;
use crate::session::{EventStore, ReviewUnitCommitRangeProjection, ReviewUnitCommitRangeView};

/// How a `--ref` read filter matches: by the recorded label (offline, answerable
/// even after the branch is deleted) or by reachability from the ref's live tip.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RefFilterMode {
    #[default]
    Label,
    Liveness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RefFilter {
    name: String,
    mode: RefFilterMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewUnitListOptions {
    repo: PathBuf,
    ref_filter: Option<RefFilter>,
}

impl ReviewUnitListOptions {
    pub fn new(repo: impl AsRef<Path>) -> Self {
        Self {
            repo: repo.as_ref().to_path_buf(),
            ref_filter: None,
        }
    }

    /// Filter to units associated with `name`; the name is normalized to its full
    /// ref before matching the stored `ref_name`.
    pub fn with_ref_filter(mut self, name: impl Into<String>, mode: RefFilterMode) -> Self {
        self.ref_filter = Some(RefFilter {
            name: name.into(),
            mode,
        });
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewUnitListEntry {
    pub review_unit_id: ReviewUnitId,
    pub session_id: SessionId,
    pub captured_at: String,
    pub revision_id: RevisionId,
    pub snapshot_id: SnapshotId,
    pub source: ReviewUnitSource,
    pub base: ReviewEndpoint,
    pub target: ReviewEndpoint,
    pub snapshot_artifact_content_hash: String,
    /// Git-free commit-range lifecycle view for this unit (anchored/floating,
    /// current and withdrawn associations). Liveness is layered caller-side.
    pub commit_range: ReviewUnitCommitRangeView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewUnitListResult {
    pub event_set_hash: String,
    pub event_count: usize,
    pub review_unit_count: usize,
    pub entries: Vec<ReviewUnitListEntry>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

pub fn list_review_units(options: ReviewUnitListOptions) -> Result<ReviewUnitListResult> {
    let read_store = resolve_read_store(&options.repo)?;
    let events = EventStore::open(read_store.store_dir()).list_events()?;
    let projection = ReviewUnitCommitRangeProjection::from_events(&events)?;
    let mut result = list_from_events(&events, &projection)?;

    if let Some(ref_filter) = &options.ref_filter {
        let matching = review_units_matching_ref(
            &projection,
            &ref_filter.name,
            ref_filter.mode,
            &options.repo,
        )?;
        result
            .entries
            .retain(|entry| matching.contains(&entry.review_unit_id));
        result.review_unit_count = result.entries.len();
    }

    result
        .diagnostics
        .extend(divergence_diagnostics(&read_store));
    Ok(result)
}

/// Convenience entry point for "which units are associated with this ref?".
/// Delegates to [`list_review_units`] with a `--ref` filter applied.
pub fn list_units_for_ref(
    repo: impl AsRef<Path>,
    ref_name: impl Into<String>,
    mode: RefFilterMode,
) -> Result<ReviewUnitListResult> {
    list_review_units(ReviewUnitListOptions::new(repo).with_ref_filter(ref_name, mode))
}

/// The review-unit ids matching a ref under the chosen mode. The name is
/// normalized to its full ref first. `Label` is fully offline (current ref
/// labels); `Liveness` joins `enrich_liveness` against the ref's tip and keeps
/// units with at least one reachable commit. Shared by `unit list` and history.
pub(crate) fn review_units_matching_ref(
    projection: &ReviewUnitCommitRangeProjection,
    name: &str,
    mode: RefFilterMode,
    repo: &Path,
) -> Result<BTreeSet<ReviewUnitId>> {
    let normalized_ref = normalize_ref(name);
    match mode {
        RefFilterMode::Label => Ok(projection
            .units_for_ref(&normalized_ref)
            .into_iter()
            .map(|view| view.review_unit_id.clone())
            .collect()),
        RefFilterMode::Liveness => {
            let mut matching = BTreeSet::new();
            for view in projection.units.values() {
                let enrichment = enrich_liveness(view, repo, Some(&normalized_ref))?;
                if enrichment.per_commit.iter().any(|commit| {
                    matches!(
                        commit.condition,
                        CommitGraphCondition::Merged | CommitGraphCondition::Live
                    )
                }) {
                    matching.insert(view.review_unit_id.clone());
                }
            }
            Ok(matching)
        }
    }
}

fn list_from_events(
    events: &[ShoreEvent],
    projection: &ReviewUnitCommitRangeProjection,
) -> Result<ReviewUnitListResult> {
    let state = SessionState::from_events(events)?;
    let event_set_hash = state
        .event_set_hash
        .clone()
        .expect("SessionState::from_events sets event_set_hash");

    let mut entries = events
        .iter()
        .filter(|event| event.event_type == EventType::ReviewUnitCaptured)
        .map(|event| entry_from_event(event, projection))
        .collect::<Result<Vec<_>>>()?;

    entries.sort_by(|left, right| {
        left.captured_at.cmp(&right.captured_at).then_with(|| {
            left.review_unit_id
                .as_str()
                .cmp(right.review_unit_id.as_str())
        })
    });

    Ok(ReviewUnitListResult {
        event_set_hash,
        event_count: events.len(),
        review_unit_count: entries.len(),
        entries,
        diagnostics: state.diagnostics,
    })
}

fn entry_from_event(
    event: &ShoreEvent,
    projection: &ReviewUnitCommitRangeProjection,
) -> Result<ReviewUnitListEntry> {
    let payload: ReviewUnitCapturedPayload = serde_json::from_value(event.payload.clone())?;
    let commit_range = projection
        .unit(&payload.review_unit_id)
        .cloned()
        .unwrap_or_else(|| empty_view(payload.review_unit_id.clone()));
    Ok(ReviewUnitListEntry {
        review_unit_id: payload.review_unit_id,
        session_id: event.target.session_id.clone(),
        captured_at: event.occurred_at.clone(),
        revision_id: payload.revision_id,
        snapshot_id: payload.snapshot_id,
        source: payload.source,
        base: payload.base,
        target: payload.target,
        snapshot_artifact_content_hash: payload.snapshot_artifact_content_hash,
        commit_range,
    })
}

fn empty_view(review_unit_id: ReviewUnitId) -> ReviewUnitCommitRangeView {
    ReviewUnitCommitRangeView {
        review_unit_id,
        anchored: false,
        current_commits: Vec::new(),
        current_refs: Vec::new(),
        withdrawn_commits: Vec::new(),
        withdrawn_refs: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ReviewEndpoint, ReviewUnitSource, WorktreeCaptureMode};
    use crate::session::event::{EventTarget, Writer};

    #[test]
    fn empty_event_set_returns_no_entries() {
        let result = list_from_events(
            &[],
            &ReviewUnitCommitRangeProjection::from_events(&[]).unwrap(),
        )
        .unwrap();

        assert_eq!(result.event_count, 0);
        assert_eq!(result.review_unit_count, 0);
        assert!(result.entries.is_empty());
        assert!(result.event_set_hash.starts_with("sha256:"));
    }

    #[test]
    fn includes_only_review_unit_captured_events() {
        let capture = captured_event("a", "2026-05-13T10:00:00Z");
        let events = [capture];
        let projection = ReviewUnitCommitRangeProjection::from_events(&events).unwrap();
        let result = list_from_events(&events, &projection).unwrap();

        assert_eq!(result.event_count, 1);
        assert_eq!(result.review_unit_count, 1);
        assert_eq!(
            result.entries[0].review_unit_id.as_str(),
            "review-unit:sha256:a"
        );
        assert_eq!(result.entries[0].captured_at, "2026-05-13T10:00:00Z");
        assert_eq!(
            result.entries[0].snapshot_artifact_content_hash,
            "sha256:artifact:a"
        );
    }

    #[test]
    fn sorts_entries_by_captured_at_then_review_unit_id() {
        let later = captured_event("z-later", "2026-05-13T10:00:05Z");
        let tie_b = captured_event("b-tie", "2026-05-13T10:00:01Z");
        let tie_a = captured_event("a-tie", "2026-05-13T10:00:01Z");

        let events = [later, tie_b, tie_a];
        let projection = ReviewUnitCommitRangeProjection::from_events(&events).unwrap();
        let result = list_from_events(&events, &projection).unwrap();

        let order: Vec<&str> = result
            .entries
            .iter()
            .map(|entry| entry.review_unit_id.as_str())
            .collect();
        assert_eq!(
            order,
            vec![
                "review-unit:sha256:a-tie",
                "review-unit:sha256:b-tie",
                "review-unit:sha256:z-later",
            ]
        );
    }

    #[test]
    fn entry_serializes_with_camel_case_and_no_internal_paths() {
        let events = [captured_event("one", "2026-05-13T10:00:00Z")];
        let projection = ReviewUnitCommitRangeProjection::from_events(&events).unwrap();
        let result = list_from_events(&events, &projection).unwrap();
        let json = serde_json::to_string(&result.entries[0]).unwrap();

        assert!(json.contains("reviewUnitId"));
        assert!(json.contains("capturedAt"));
        assert!(json.contains("snapshotArtifactContentHash"));
        assert!(!json.contains("artifacts/"));
        assert!(!json.contains("statePath"));
        assert!(!json.contains("payloadHash"));
    }

    fn captured_event(suffix: &str, occurred_at: &str) -> ShoreEvent {
        let review_unit_id = ReviewUnitId::new(format!("review-unit:sha256:{suffix}"));
        let revision_id = RevisionId::new(format!("rev:sha256:{suffix}"));
        let snapshot_id = SnapshotId::new(format!("snap:sha256:{suffix}"));
        let payload = ReviewUnitCapturedPayload {
            review_unit_id: review_unit_id.clone(),
            source: ReviewUnitSource::GitWorktree {
                mode: WorktreeCaptureMode::CombinedHeadToWorkingTree,
                include_untracked: true,
            },
            base: ReviewEndpoint::GitCommit {
                commit_oid: format!("base:{suffix}"),
                tree_oid: format!("base-tree:{suffix}"),
            },
            target: ReviewEndpoint::GitWorkingTree {
                worktree_root: "/repo".to_owned(),
            },
            revision_id: revision_id.clone(),
            snapshot_id: snapshot_id.clone(),
            snapshot_artifact_content_hash: format!("sha256:artifact:{suffix}"),
        };
        ShoreEvent::new(
            EventType::ReviewUnitCaptured,
            format!("capture:{suffix}"),
            EventTarget::for_review_unit(
                SessionId::new("session:default"),
                review_unit_id,
                revision_id,
                snapshot_id,
            ),
            Writer::shore_local("test"),
            payload,
            occurred_at,
        )
        .unwrap()
    }
}
