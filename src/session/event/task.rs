use serde::{Deserialize, Serialize};

use super::kind::EventType;
use super::payload::EventPayload;
use crate::model::{WorkObjectId, WorkObjectType};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAttemptCapturedPayload {
    pub task_attempt_id: WorkObjectId,
    pub project_path: String,
    pub claude_session_uuid: String,
    pub initial_prompt_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<WorkObjectId>,
}

impl TaskAttemptCapturedPayload {
    pub fn idempotency_key_for_work_object(
        work_object_id: &WorkObjectId,
        work_object_type: WorkObjectType,
        source_key: &str,
    ) -> String {
        let kind = match work_object_type {
            WorkObjectType::ReviewUnit => "review_unit",
            WorkObjectType::TaskAttempt => "task_attempt",
        };
        format!(
            "task_attempt_captured:{}:{}:{}",
            work_object_id.as_str(),
            kind,
            source_key
        )
    }
}

impl EventPayload for TaskAttemptCapturedPayload {
    fn event_type(&self) -> EventType {
        EventType::TaskAttemptCaptured
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionId, WorkObjectId, WorkObjectType};
    use crate::session::event::{
        AssertionMode, EventPayload, EventTarget, EventType, InterventionRequestedPayload,
        ShoreEvent, Writer,
    };

    fn sample_payload() -> TaskAttemptCapturedPayload {
        TaskAttemptCapturedPayload {
            task_attempt_id: WorkObjectId::new("task-attempt:sha256:abc"),
            project_path: "/repo".to_owned(),
            claude_session_uuid: "uuid-1".to_owned(),
            initial_prompt_hash: "sha256:prompt".to_owned(),
            predecessor: None,
        }
    }

    #[test]
    fn task_attempt_captured_payload_round_trips_through_serde() {
        let payload = sample_payload();
        let json = serde_json::to_string(&payload).unwrap();
        let round: TaskAttemptCapturedPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(round, payload);
    }

    #[test]
    fn task_attempt_captured_payload_serializes_camel_case_fields() {
        let json = serde_json::to_value(sample_payload()).unwrap();

        assert_eq!(json["taskAttemptId"], "task-attempt:sha256:abc");
        assert_eq!(json["projectPath"], "/repo");
        assert_eq!(json["claudeSessionUuid"], "uuid-1");
        assert_eq!(json["initialPromptHash"], "sha256:prompt");
        assert!(json.get("predecessor").is_none());
        assert!(json.get("assertionMode").is_none());
        assert!(json.get("sourceRef").is_none());
        assert!(json.get("submissionId").is_none());
    }

    #[test]
    fn task_attempt_captured_idempotency_key_for_work_object_uses_substrate_form() {
        let key = TaskAttemptCapturedPayload::idempotency_key_for_work_object(
            &WorkObjectId::new("task-attempt:sha256:abc"),
            WorkObjectType::TaskAttempt,
            "source-1",
        );
        assert_eq!(
            key,
            "task_attempt_captured:task-attempt:sha256:abc:task_attempt:source-1"
        );
    }

    #[test]
    fn task_attempt_captured_idempotency_key_does_not_collide_with_intervention_substrate_form() {
        let task_key = TaskAttemptCapturedPayload::idempotency_key_for_work_object(
            &WorkObjectId::new("shared"),
            WorkObjectType::TaskAttempt,
            "source-1",
        );
        let intervention_key = InterventionRequestedPayload::idempotency_key_for_work_object(
            &WorkObjectId::new("shared"),
            WorkObjectType::TaskAttempt,
            "source-1",
        );
        assert_ne!(task_key, intervention_key);
    }

    #[test]
    fn task_attempt_captured_payload_reports_matching_event_type() {
        assert_eq!(
            sample_payload().event_type(),
            EventType::TaskAttemptCaptured
        );
    }

    #[test]
    fn task_attempt_captured_event_builds_through_shore_event_new() {
        let target = EventTarget::for_work_object(
            SessionId::new("session:claude:uuid-1"),
            WorkObjectId::new("task-attempt:sha256:abc"),
            WorkObjectType::TaskAttempt,
        );
        let idempotency_key = TaskAttemptCapturedPayload::idempotency_key_for_work_object(
            &WorkObjectId::new("task-attempt:sha256:abc"),
            WorkObjectType::TaskAttempt,
            "uuid-1",
        );

        let event = ShoreEvent::new(
            EventType::TaskAttemptCaptured,
            idempotency_key,
            target,
            Writer::shore_local_author("test"),
            sample_payload(),
            "2026-05-18T00:00:00Z",
        )
        .unwrap();

        assert_eq!(event.event_type, EventType::TaskAttemptCaptured);
        assert_eq!(event.assertion_mode, AssertionMode::Advisory);
        assert!(event.source_ref.is_none());

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["target"]["workObjectId"], "task-attempt:sha256:abc");
        assert_eq!(json["target"]["workObjectType"], "task_attempt");
        assert_eq!(json["target"]["sessionId"], "session:claude:uuid-1");
        assert!(json["target"].get("reviewUnitId").is_none());
    }
}
