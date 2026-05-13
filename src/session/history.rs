use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::model::{
    DispositionId, EventId, InterventionId, InterventionResolutionId, ObservationId,
    ReviewEndpoint, ReviewId, ReviewTargetRef, ReviewUnitId, ReviewUnitSource, RevisionId,
    SnapshotId, TrackId,
};
use crate::session::event::{
    EventType, ImportedNoteTarget, InterventionMode, InterventionReasonCode,
    InterventionRequestedPayload, InterventionResolutionOutcome, InterventionResolvedPayload,
    ReviewDisposition, ReviewDispositionRecordedPayload, ReviewInitializedPayload,
    ReviewNoteImportedPayload, ReviewObservationRecordedPayload, ReviewUnitCapturedPayload,
    ShoreEvent, SidecarSource, Writer,
};
use crate::session::observation::validated_track_id;
use crate::session::projection_freshness::event_set_hash_for_events;
use crate::session::state::{ProjectionDiagnostic, SessionState};
use crate::session::store_init::ShoreStorePaths;
use crate::storage::EventStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewHistoryOptions {
    repo: PathBuf,
    review_unit_id: Option<ReviewUnitId>,
    track: Option<String>,
    event_types: Vec<EventType>,
    include_body: bool,
}

impl ReviewHistoryOptions {
    pub fn new(repo: impl AsRef<Path>) -> Self {
        Self {
            repo: repo.as_ref().to_path_buf(),
            review_unit_id: None,
            track: None,
            event_types: Vec::new(),
            include_body: false,
        }
    }

    pub fn with_review_unit_id(mut self, review_unit_id: ReviewUnitId) -> Self {
        self.review_unit_id = Some(review_unit_id);
        self
    }

    pub fn with_track(mut self, track: impl Into<String>) -> Self {
        self.track = Some(track.into());
        self
    }

    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.event_types.push(event_type);
        self
    }

    pub fn with_include_body(mut self, include_body: bool) -> Self {
        self.include_body = include_body;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryResult {
    pub event_set_hash: String,
    pub event_count: usize,
    pub filters: ReviewHistoryFilters,
    pub entries: Vec<ReviewHistoryEntry>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

impl ReviewHistoryResult {
    pub fn history_count(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_unit_id: Option<ReviewUnitId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<TrackId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub event_types: Vec<EventType>,
    pub include_body: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ResolvedHistoryFilters {
    review_unit_id: Option<ReviewUnitId>,
    track_id: Option<TrackId>,
    event_types: Vec<EventType>,
    include_body: bool,
}

impl From<ResolvedHistoryFilters> for ReviewHistoryFilters {
    fn from(filters: ResolvedHistoryFilters) -> Self {
        Self {
            review_unit_id: filters.review_unit_id,
            track_id: filters.track_id,
            event_types: filters.event_types,
            include_body: filters.include_body,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryEntry {
    pub event_id: EventId,
    pub event_type: EventType,
    pub occurred_at: String,
    pub payload_hash: String,
    pub review_id: ReviewId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_unit_id: Option<ReviewUnitId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<RevisionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<SnapshotId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<TrackId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ReviewTargetRef>,
    pub writer: Writer,
    pub summary: ReviewHistorySummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ReviewHistorySummary {
    ReviewInitialized {},
    ReviewUnitCaptured {
        review_unit_id: ReviewUnitId,
        source: ReviewUnitSource,
        base: ReviewEndpoint,
        target: ReviewEndpoint,
        revision_id: RevisionId,
        snapshot_id: SnapshotId,
        snapshot_artifact_content_hash: String,
    },
    ObservationRecorded {
        observation_id: ObservationId,
        target: ReviewTargetRef,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body_byte_size: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body_content_hash: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        supersedes: Vec<ObservationId>,
    },
    InterventionRequested {
        intervention_id: InterventionId,
        target: ReviewTargetRef,
        mode: InterventionMode,
        reason_code: InterventionReasonCode,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body_byte_size: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body_content_hash: Option<String>,
    },
    InterventionResolved {
        intervention_resolution_id: InterventionResolutionId,
        intervention_id: InterventionId,
        outcome: InterventionResolutionOutcome,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason_byte_size: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason_content_hash: Option<String>,
    },
    DispositionRecorded {
        disposition_id: DispositionId,
        target: ReviewTargetRef,
        disposition: ReviewDisposition,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary_byte_size: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary_content_hash: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        replaces: Vec<DispositionId>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        related_observations: Vec<ObservationId>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        related_interventions: Vec<InterventionId>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        overrides: Vec<ReviewTargetRef>,
    },
    ReviewNoteImported {
        sidecar_source: SidecarSource,
        note_id: String,
        file_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_old_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<ImportedNoteTarget>,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body_byte_size: Option<usize>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        created_at: Option<String>,
        sidecar_content_hash: String,
    },
}

pub fn review_history(options: ReviewHistoryOptions) -> Result<ReviewHistoryResult> {
    let paths = ShoreStorePaths::resolve(&options.repo)?;
    let events = EventStore::open(paths.shore_dir()).list_events()?;
    let filters = ResolvedHistoryFilters {
        review_unit_id: options.review_unit_id,
        track_id: options
            .track
            .as_deref()
            .map(validated_track_id)
            .transpose()?,
        event_types: options.event_types,
        include_body: options.include_body,
    };
    history_from_events(&events, filters, Some(paths.shore_dir()))
}

fn history_from_events(
    events: &[ShoreEvent],
    filters: ResolvedHistoryFilters,
    _shore_dir: Option<&Path>,
) -> Result<ReviewHistoryResult> {
    let state = SessionState::from_events(events)?;
    let event_set_hash = state
        .event_set_hash
        .clone()
        .unwrap_or(event_set_hash_for_events(events)?);
    let mut entries = events
        .iter()
        .filter(|event| event_matches_filters(event, &filters))
        .map(history_entry_from_event)
        .collect::<Result<Vec<_>>>()?;

    entries.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
    });

    Ok(ReviewHistoryResult {
        event_set_hash,
        event_count: events.len(),
        filters: filters.into(),
        entries,
        diagnostics: state.diagnostics,
    })
}

fn history_entry_from_event(event: &ShoreEvent) -> Result<ReviewHistoryEntry> {
    let summary = match event.event_type {
        EventType::ReviewInitialized => {
            let _payload: ReviewInitializedPayload = serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::ReviewInitialized {}
        }
        EventType::ReviewUnitCaptured => {
            let payload: ReviewUnitCapturedPayload = serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::ReviewUnitCaptured {
                review_unit_id: payload.review_unit_id,
                source: payload.source,
                base: payload.base,
                target: payload.target,
                revision_id: payload.revision_id,
                snapshot_id: payload.snapshot_id,
                snapshot_artifact_content_hash: payload.snapshot_artifact_content_hash,
            }
        }
        EventType::ReviewObservationRecorded => {
            let payload: ReviewObservationRecordedPayload =
                serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::ObservationRecorded {
                observation_id: payload.observation_id,
                target: payload.target,
                title: payload.title,
                body: None,
                body_byte_size: payload.body_byte_size,
                body_content_hash: payload.body_content_hash,
                tags: payload.tags,
                confidence: payload.confidence,
                supersedes: payload.supersedes_observation_ids,
            }
        }
        EventType::InterventionRequested => {
            let payload: InterventionRequestedPayload =
                serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::InterventionRequested {
                intervention_id: payload.intervention_id,
                target: payload.target,
                mode: payload.mode,
                reason_code: payload.reason_code,
                title: payload.title,
                body: None,
                body_byte_size: payload.body_byte_size,
                body_content_hash: payload.body_content_hash,
            }
        }
        EventType::InterventionResolved => {
            let payload: InterventionResolvedPayload =
                serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::InterventionResolved {
                intervention_resolution_id: payload.intervention_resolution_id,
                intervention_id: payload.intervention_id,
                outcome: payload.outcome,
                reason: None,
                reason_byte_size: payload.reason_byte_size,
                reason_content_hash: payload.reason_content_hash,
            }
        }
        EventType::ReviewDispositionRecorded => {
            let payload: ReviewDispositionRecordedPayload =
                serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::DispositionRecorded {
                disposition_id: payload.disposition_id,
                target: payload.target,
                disposition: payload.disposition,
                summary: None,
                summary_byte_size: payload.summary_byte_size,
                summary_content_hash: payload.summary_content_hash,
                replaces: payload.replaces_disposition_ids,
                related_observations: payload.related_observation_ids,
                related_interventions: payload.related_intervention_ids,
                overrides: payload.overrides,
            }
        }
        EventType::ReviewNoteImported => {
            let payload: ReviewNoteImportedPayload = serde_json::from_value(event.payload.clone())?;
            ReviewHistorySummary::ReviewNoteImported {
                sidecar_source: payload.sidecar_source,
                note_id: payload.note_id,
                file_path: payload.file_path,
                file_old_path: payload.file_old_path,
                target: payload.target,
                title: payload.title,
                body: None,
                body_byte_size: payload.body_byte_size,
                tags: payload.tags,
                confidence: payload.confidence,
                external_source: payload.external_source,
                author: payload.author,
                created_at: payload.created_at,
                sidecar_content_hash: payload.sidecar_content_hash,
            }
        }
    };

    Ok(ReviewHistoryEntry {
        event_id: event.event_id.clone(),
        event_type: event.event_type,
        occurred_at: event.occurred_at.clone(),
        payload_hash: event.payload_hash.clone(),
        review_id: event.target.review_id.clone(),
        review_unit_id: event.target.review_unit_id.clone(),
        revision_id: event.target.revision_id.clone(),
        snapshot_id: event.target.snapshot_id.clone(),
        track_id: event.target.track_id.clone(),
        subject: event.target.subject.clone(),
        writer: event.writer.clone(),
        summary,
    })
}

fn event_matches_filters(event: &ShoreEvent, filters: &ResolvedHistoryFilters) -> bool {
    if filters
        .review_unit_id
        .as_ref()
        .is_some_and(|review_unit_id| event.target.review_unit_id.as_ref() != Some(review_unit_id))
    {
        return false;
    }
    if filters
        .track_id
        .as_ref()
        .is_some_and(|track_id| event.target.track_id.as_ref() != Some(track_id))
    {
        return false;
    }
    if !filters.event_types.is_empty() && !filters.event_types.contains(&event.event_type) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DispositionId, InterventionId, InterventionResolutionId, ObservationId, ReviewEndpoint,
        ReviewId, ReviewTargetRef, ReviewUnitId, ReviewUnitSource, RevisionId, Side, SnapshotId,
        TrackId, WorkUnitId, WorktreeCaptureMode,
    };
    use crate::session::event::{ImportedNoteTarget, ReviewNoteImportedPayload};
    use crate::session::{
        EventTarget, EventType, InterventionMode, InterventionReasonCode,
        InterventionRequestedPayload, InterventionResolutionOutcome, InterventionResolvedPayload,
        ReviewDisposition, ReviewDispositionRecordedPayload, ReviewInitializedPayload,
        ReviewObservationRecordedPayload, ReviewUnitCapturedPayload, ShoreEvent, SidecarSource,
        Writer,
    };

    #[test]
    fn review_history_returns_empty_freshness_metadata_without_events() {
        let result = history_from_events(&[], ResolvedHistoryFilters::default(), None).unwrap();

        assert_eq!(result.event_count, 0);
        assert_eq!(result.history_count(), 0);
        assert!(result.event_set_hash.starts_with("sha256:"));
        assert!(result.entries.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn review_history_metadata_uses_full_event_set() {
        let first = review_initialized_event("one");
        let second = review_initialized_event("two");

        let result = history_from_events(
            &[first, second],
            ResolvedHistoryFilters {
                event_types: vec![EventType::ReviewInitialized],
                ..ResolvedHistoryFilters::default()
            },
            None,
        )
        .unwrap();

        assert_eq!(result.event_count, 2);
        assert_eq!(result.history_count(), 2);
        assert!(result.event_set_hash.starts_with("sha256:"));
    }

    #[test]
    fn history_entry_summarizes_current_event_families() {
        let cases = [
            (
                review_initialized_event("init"),
                EventType::ReviewInitialized,
                "review_initialized",
            ),
            (
                review_unit_captured_event(),
                EventType::ReviewUnitCaptured,
                "review_unit_captured",
            ),
            (
                observation_event_with_body("body"),
                EventType::ReviewObservationRecorded,
                "observation_recorded",
            ),
            (
                intervention_requested_event(),
                EventType::InterventionRequested,
                "intervention_requested",
            ),
            (
                intervention_resolved_event(),
                EventType::InterventionResolved,
                "intervention_resolved",
            ),
            (
                disposition_event(),
                EventType::ReviewDispositionRecorded,
                "disposition_recorded",
            ),
            (
                review_note_imported_event(),
                EventType::ReviewNoteImported,
                "review_note_imported",
            ),
        ];

        for (event, event_type, summary_kind) in cases {
            let entry = history_entry_from_event(&event).unwrap();
            let summary_json = serde_json::to_value(&entry.summary).unwrap();

            assert_eq!(entry.event_type, event_type);
            assert_eq!(summary_json["kind"], summary_kind);
        }
    }

    #[test]
    fn history_entry_omits_internal_artifact_paths() {
        let event = observation_event_with_artifact_path("artifacts/notes/body.json");

        let entry = history_entry_from_event(&event).unwrap();
        let json = serde_json::to_string(&entry).unwrap();

        assert!(!json.contains("bodyArtifactPath"));
        assert!(!json.contains("artifacts/notes"));
    }

    #[test]
    fn history_sorts_by_occurred_at_then_event_id() {
        let late = event_with_time_and_key("2026-05-13T10:00:02Z", "late");
        let tie_b = event_with_time_and_key("2026-05-13T10:00:01Z", "b");
        let tie_a = event_with_time_and_key("2026-05-13T10:00:01Z", "a");

        let result = history_from_events(
            &[late, tie_b, tie_a],
            ResolvedHistoryFilters::default(),
            None,
        )
        .unwrap();

        assert_eq!(
            result
                .entries
                .iter()
                .map(|entry| entry.occurred_at.as_str())
                .collect::<Vec<_>>(),
            vec![
                "2026-05-13T10:00:01Z",
                "2026-05-13T10:00:01Z",
                "2026-05-13T10:00:02Z",
            ]
        );
        assert!(result.entries[0].event_id.as_str() < result.entries[1].event_id.as_str());
    }

    #[test]
    fn history_filters_by_review_unit_track_and_event_type() {
        let keep = observation_event("review-unit:sha256:one", "agent:codex", "Keep");
        let other_track = observation_event("review-unit:sha256:one", "agent:claude", "Drop track");
        let other_unit = observation_event("review-unit:sha256:two", "agent:codex", "Drop unit");
        let capture = review_unit_captured_event_for("review-unit:sha256:one");

        let filters = ResolvedHistoryFilters {
            review_unit_id: Some(ReviewUnitId::new("review-unit:sha256:one")),
            track_id: Some(TrackId::new("agent:codex")),
            event_types: vec![EventType::ReviewObservationRecorded],
            include_body: false,
        };

        let result =
            history_from_events(&[keep, other_track, other_unit, capture], filters, None).unwrap();

        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].track_id.as_ref().map(TrackId::as_str),
            Some("agent:codex")
        );
        assert_eq!(
            result.entries[0].event_type,
            EventType::ReviewObservationRecorded
        );
    }

    fn review_initialized_event(key: &str) -> ShoreEvent {
        let review_id = ReviewId::new("review:default");
        let work_unit_id = WorkUnitId::new(format!("work:{key}"));
        ShoreEvent::new(
            EventType::ReviewInitialized,
            ReviewInitializedPayload::idempotency_key(&review_id, &work_unit_id),
            EventTarget::new(review_id, work_unit_id),
            Writer::shore_local_author("test"),
            ReviewInitializedPayload {},
            format!("2026-05-13T10:00:0{key}Z"),
        )
        .unwrap()
    }

    fn review_unit_captured_event() -> ShoreEvent {
        review_unit_captured_event_for("review-unit:sha256:one")
    }

    fn review_unit_captured_event_for(review_unit_id: &str) -> ShoreEvent {
        let review_unit_id = ReviewUnitId::new(review_unit_id);
        let payload = ReviewUnitCapturedPayload {
            review_unit_id: review_unit_id.clone(),
            source: ReviewUnitSource::GitWorktree {
                mode: WorktreeCaptureMode::CombinedHeadToWorkingTree,
                include_untracked: true,
            },
            base: ReviewEndpoint::GitCommit {
                commit_oid: "base".to_owned(),
                tree_oid: "base-tree".to_owned(),
            },
            target: ReviewEndpoint::GitWorkingTree {
                worktree_root: "/repo".to_owned(),
            },
            revision_id: RevisionId::new(format!("rev:{}", review_unit_id.as_str())),
            snapshot_id: SnapshotId::new(format!("snap:{}", review_unit_id.as_str())),
            snapshot_artifact_content_hash: "sha256:artifact".to_owned(),
        };
        ShoreEvent::new(
            EventType::ReviewUnitCaptured,
            "capture:one",
            EventTarget::for_review_unit(
                ReviewId::new("review:default"),
                review_unit_id,
                payload.revision_id.clone(),
                payload.snapshot_id.clone(),
            ),
            Writer::shore_local_author("test"),
            payload,
            "2026-05-13T10:00:00Z",
        )
        .unwrap()
    }

    fn event_with_time_and_key(occurred_at: &str, key: &str) -> ShoreEvent {
        let mut event = observation_event("review-unit:sha256:one", "agent:codex", key);
        event.occurred_at = occurred_at.to_owned();
        event
    }

    fn observation_event(review_unit_id: &str, track_id: &str, title: &str) -> ShoreEvent {
        let review_unit_id = ReviewUnitId::new(review_unit_id);
        let payload = ReviewObservationRecordedPayload {
            observation_id: ObservationId::new(format!("obs:sha256:{title}")),
            target: ReviewTargetRef::ReviewUnit {
                review_unit_id: review_unit_id.clone(),
            },
            title: title.to_owned(),
            body: None,
            body_artifact_path: None,
            body_byte_size: None,
            body_content_hash: None,
            tags: vec![],
            confidence: None,
            supersedes_observation_ids: vec![],
        };
        tracked_event_for_unit(
            EventType::ReviewObservationRecorded,
            &format!("observation:{title}:{track_id}"),
            track_id,
            review_unit_id,
            payload,
            "2026-05-13T10:00:01Z",
        )
    }

    fn observation_event_with_body(body: &str) -> ShoreEvent {
        let payload = ReviewObservationRecordedPayload {
            observation_id: ObservationId::new("obs:sha256:one"),
            target: ReviewTargetRef::ReviewUnit {
                review_unit_id: review_unit_id("one"),
            },
            title: "Observation".to_owned(),
            body: Some(body.to_owned()),
            body_artifact_path: None,
            body_byte_size: Some(body.len() as u64),
            body_content_hash: Some("sha256:body".to_owned()),
            tags: vec!["correctness".to_owned()],
            confidence: Some("high".to_owned()),
            supersedes_observation_ids: vec![],
        };
        tracked_event(
            EventType::ReviewObservationRecorded,
            "observation:one",
            "agent:codex",
            payload,
            "2026-05-13T10:00:01Z",
        )
    }

    fn observation_event_with_artifact_path(path: &str) -> ShoreEvent {
        let payload = ReviewObservationRecordedPayload {
            observation_id: ObservationId::new("obs:sha256:artifact"),
            target: ReviewTargetRef::ReviewUnit {
                review_unit_id: review_unit_id("one"),
            },
            title: "Observation".to_owned(),
            body: None,
            body_artifact_path: Some(path.to_owned()),
            body_byte_size: Some(5000),
            body_content_hash: Some("sha256:body".to_owned()),
            tags: vec![],
            confidence: None,
            supersedes_observation_ids: vec![],
        };
        tracked_event(
            EventType::ReviewObservationRecorded,
            "observation:artifact",
            "agent:codex",
            payload,
            "2026-05-13T10:00:01Z",
        )
    }

    fn intervention_requested_event() -> ShoreEvent {
        let payload = InterventionRequestedPayload {
            intervention_id: InterventionId::new("intervention:sha256:one"),
            target: ReviewTargetRef::ReviewUnit {
                review_unit_id: review_unit_id("one"),
            },
            mode: InterventionMode::Blocking,
            reason_code: InterventionReasonCode::ManualDecisionRequired,
            title: "Need decision".to_owned(),
            body: Some("body".to_owned()),
            body_artifact_path: None,
            body_byte_size: Some(4),
            body_content_hash: Some("sha256:body".to_owned()),
        };
        tracked_event(
            EventType::InterventionRequested,
            "intervention:request",
            "human:kevin",
            payload,
            "2026-05-13T10:00:02Z",
        )
    }

    fn intervention_resolved_event() -> ShoreEvent {
        let payload = InterventionResolvedPayload {
            intervention_resolution_id: InterventionResolutionId::new(
                "intervention-resolution:sha256:one",
            ),
            intervention_id: InterventionId::new("intervention:sha256:one"),
            outcome: InterventionResolutionOutcome::Approved,
            reason: Some("approved".to_owned()),
            reason_artifact_path: None,
            reason_byte_size: Some(8),
            reason_content_hash: Some("sha256:reason".to_owned()),
        };
        tracked_event(
            EventType::InterventionResolved,
            "intervention:resolved",
            "human:kevin",
            payload,
            "2026-05-13T10:00:03Z",
        )
    }

    fn disposition_event() -> ShoreEvent {
        let payload = ReviewDispositionRecordedPayload {
            disposition_id: DispositionId::new("disp:sha256:one"),
            target: ReviewTargetRef::ReviewUnit {
                review_unit_id: review_unit_id("one"),
            },
            disposition: ReviewDisposition::Accepted,
            summary: Some("ship it".to_owned()),
            summary_artifact_path: None,
            summary_byte_size: Some(7),
            summary_content_hash: Some("sha256:summary".to_owned()),
            replaces_disposition_ids: vec![],
            related_observation_ids: vec![ObservationId::new("obs:sha256:one")],
            related_intervention_ids: vec![InterventionId::new("intervention:sha256:one")],
            overrides: vec![],
        };
        tracked_event(
            EventType::ReviewDispositionRecorded,
            "disposition:one",
            "human:kevin",
            payload,
            "2026-05-13T10:00:04Z",
        )
    }

    fn review_note_imported_event() -> ShoreEvent {
        let payload = ReviewNoteImportedPayload {
            sidecar_source: SidecarSource::ReviewNotes,
            note_id: "note-1".to_owned(),
            file_path: "src/lib.rs".to_owned(),
            file_old_path: None,
            target: Some(ImportedNoteTarget {
                side: Side::New,
                start_line: 1,
                end_line: 1,
            }),
            title: "Imported note".to_owned(),
            body: Some("body".to_owned()),
            body_artifact_path: None,
            body_byte_size: Some(4),
            tags: vec!["imported".to_owned()],
            confidence: Some("medium".to_owned()),
            external_source: Some("review-notes.json".to_owned()),
            author: Some("reviewer".to_owned()),
            created_at: Some("2026-05-13T09:00:00Z".to_owned()),
            sidecar_content_hash: "sha256:sidecar".to_owned(),
        };
        ShoreEvent::new(
            EventType::ReviewNoteImported,
            "review-note:one",
            EventTarget::new(
                ReviewId::new("review:default"),
                WorkUnitId::new("work:default"),
            ),
            Writer::shore_local_reviewer("test"),
            payload,
            "2026-05-13T10:00:05Z",
        )
        .unwrap()
    }

    fn tracked_event<P>(
        event_type: EventType,
        idempotency_key: &str,
        track_id: &str,
        payload: P,
        occurred_at: &str,
    ) -> ShoreEvent
    where
        P: crate::session::EventPayload,
    {
        tracked_event_for_unit(
            event_type,
            idempotency_key,
            track_id,
            review_unit_id("one"),
            payload,
            occurred_at,
        )
    }

    fn tracked_event_for_unit<P>(
        event_type: EventType,
        idempotency_key: &str,
        track_id: &str,
        review_unit_id: ReviewUnitId,
        payload: P,
        occurred_at: &str,
    ) -> ShoreEvent
    where
        P: crate::session::EventPayload,
    {
        let mut target = EventTarget::for_review_unit(
            ReviewId::new("review:default"),
            review_unit_id.clone(),
            revision_id("one"),
            snapshot_id("one"),
        );
        target.track_id = Some(TrackId::new(track_id));
        target.subject = Some(ReviewTargetRef::ReviewUnit { review_unit_id });
        ShoreEvent::new(
            event_type,
            idempotency_key,
            target,
            Writer::shore_local_reviewer("test"),
            payload,
            occurred_at,
        )
        .unwrap()
    }

    fn review_unit_id(suffix: &str) -> ReviewUnitId {
        ReviewUnitId::new(format!("review-unit:sha256:{suffix}"))
    }

    fn revision_id(suffix: &str) -> RevisionId {
        RevisionId::new(format!("rev:sha256:{suffix}"))
    }

    fn snapshot_id(suffix: &str) -> SnapshotId {
        SnapshotId::new(format!("snap:sha256:{suffix}"))
    }
}
