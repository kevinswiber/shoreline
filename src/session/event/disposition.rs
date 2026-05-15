use serde::{Deserialize, Serialize};

use super::kind::EventType;
use super::payload::EventPayload;
use crate::model::{
    DispositionId, InterventionId, ObservationId, ReviewTargetRef, ReviewUnitId, TrackId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDisposition {
    Accepted,
    AcceptedWithFollowUp,
    NeedsChanges,
    NeedsClarification,
    Overridden,
    Deferred,
    SplitOut,
    Superseded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDispositionRecordedPayload {
    pub disposition_id: DispositionId,
    pub target: ReviewTargetRef,
    pub disposition: ReviewDisposition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_byte_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces_disposition_ids: Vec<DispositionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_observation_ids: Vec<ObservationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_intervention_ids: Vec<InterventionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<ReviewTargetRef>,
}

impl ReviewDispositionRecordedPayload {
    pub fn idempotency_key(
        review_unit_id: &ReviewUnitId,
        track_id: &TrackId,
        source_key: &str,
    ) -> String {
        format!(
            "review_disposition_recorded:{}:{}:{}",
            review_unit_id.as_str(),
            track_id.as_str(),
            source_key
        )
    }
}

impl EventPayload for ReviewDispositionRecordedPayload {
    fn event_type(&self) -> EventType {
        EventType::ReviewDispositionRecorded
    }
}
