use serde::{Deserialize, Serialize};

use super::kind::EventType;
use super::payload::EventPayload;
use crate::model::{
    InterventionId, InterventionResolutionId, ReviewTargetRef, ReviewUnitId, TrackId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionMode {
    Blocking,
    Advisory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionReasonCode {
    AmbiguousState,
    UnsafeAction,
    StaleRevision,
    FailedGate,
    ExternalSideEffect,
    ConflictingEvent,
    MissingPermission,
    ManualDecisionRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionResolutionOutcome {
    Approved,
    Rejected,
    Dismissed,
    Superseded,
    Abandoned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionRequestedPayload {
    pub intervention_id: InterventionId,
    pub target: ReviewTargetRef,
    pub mode: InterventionMode,
    pub reason_code: InterventionReasonCode,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_byte_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_content_hash: Option<String>,
}

impl InterventionRequestedPayload {
    pub fn idempotency_key(
        review_unit_id: &ReviewUnitId,
        track_id: &TrackId,
        source_key: &str,
    ) -> String {
        format!(
            "intervention_requested:{}:{}:{}",
            review_unit_id.as_str(),
            track_id.as_str(),
            source_key
        )
    }
}

impl EventPayload for InterventionRequestedPayload {
    fn event_type(&self) -> EventType {
        EventType::InterventionRequested
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionResolvedPayload {
    pub intervention_resolution_id: InterventionResolutionId,
    pub intervention_id: InterventionId,
    pub outcome: InterventionResolutionOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_byte_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_content_hash: Option<String>,
}

impl InterventionResolvedPayload {
    pub fn idempotency_key(intervention_id: &InterventionId, source_key: &str) -> String {
        format!(
            "intervention_resolved:{}:{}",
            intervention_id.as_str(),
            source_key
        )
    }
}

impl EventPayload for InterventionResolvedPayload {
    fn event_type(&self) -> EventType {
        EventType::InterventionResolved
    }
}
