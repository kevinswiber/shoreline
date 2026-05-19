//! Sibling task-domain projection over `ShoreEvent`s.
//!
//! Phase 5 reads already-written task-domain events into a per-attempt summary.
//! `SessionState` remains review-domain; `review_history` filters task events
//! out unconditionally. This module is the sibling task entry point.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::model::{ActorId, CheckpointId, EventId, ObservationId, WorkObjectId, WorkObjectType};
use crate::session::event::{
    AssertionMode, EventTarget, EventType, ShoreEvent, SourceRef, TaskAttemptCapturedPayload,
    TaskCheckpointCapturedPayload, TaskObservationRecordedPayload, Writer,
};

/// Envelope-level fields preserved on every projected event, so the projection
/// does not silently lose envelope identity / authorship / source provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskProjectionEventEnvelope {
    pub event_id: EventId,
    pub event_type: EventType,
    pub occurred_at: String,
    pub payload_hash: String,
    pub writer: Writer,
    pub assertion_mode: AssertionMode,
    pub source_ref: Option<SourceRef>,
    pub target: EventTarget,
}

impl TaskProjectionEventEnvelope {
    fn from_event(event: &ShoreEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            event_type: event.event_type,
            occurred_at: event.occurred_at.clone(),
            payload_hash: event.payload_hash.clone(),
            writer: event.writer.clone(),
            assertion_mode: event.assertion_mode,
            source_ref: event.source_ref.clone(),
            target: event.target.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskObservationSummary {
    pub envelope: TaskProjectionEventEnvelope,
    pub observation_id: ObservationId,
    pub checkpoint_id: Option<CheckpointId>,
    pub title: String,
    pub body: Option<String>,
    pub body_artifact_path: Option<String>,
    pub body_byte_size: Option<u64>,
    pub body_content_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskCheckpointSummary {
    pub envelope: TaskProjectionEventEnvelope,
    pub checkpoint_id: CheckpointId,
    pub assistant_message_id: String,
    pub tool_use_ids: Vec<String>,
    pub observations: Vec<TaskObservationSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskProjectionDiagnostic {
    pub code: String,
    pub message: String,
    pub event_id: Option<EventId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskAttemptSummary {
    pub reader_actor_id: ActorId,
    pub task_attempt_id: WorkObjectId,
    pub attempt_event: TaskProjectionEventEnvelope,
    pub project_path: String,
    pub claude_session_uuid: String,
    pub initial_prompt_hash: String,
    pub predecessor: Option<WorkObjectId>,
    pub latest_checkpoint: Option<TaskCheckpointSummary>,
    pub checkpoints: Vec<TaskCheckpointSummary>,
    pub observations_without_checkpoint: Vec<TaskObservationSummary>,
    pub diagnostics: Vec<TaskProjectionDiagnostic>,
}

/// Roll up `TaskAttemptCaptured`, `TaskCheckpointCaptured`, and
/// `TaskObservationRecorded` events for a single `TaskAttempt` into a
/// human/agent-readable summary.
///
/// Returns `Ok(None)` if no `TaskAttemptCaptured` event for the requested
/// `task_attempt_id` is present.
#[allow(dead_code)]
pub(crate) fn task_attempt_summary_from_events(
    events: &[ShoreEvent],
    task_attempt_id: &WorkObjectId,
    reader_actor_id: &ActorId,
) -> Result<Option<TaskAttemptSummary>> {
    let mut attempt: Option<(TaskProjectionEventEnvelope, TaskAttemptCapturedPayload)> = None;
    let mut checkpoint_envelopes: BTreeMap<
        CheckpointId,
        (TaskProjectionEventEnvelope, TaskCheckpointCapturedPayload),
    > = BTreeMap::new();
    let mut observation_records: Vec<(
        TaskProjectionEventEnvelope,
        TaskObservationRecordedPayload,
    )> = Vec::new();

    for event in events {
        event.validate_schema_version()?;

        if !targets_task_attempt(event, task_attempt_id) {
            continue;
        }

        match event.event_type {
            EventType::TaskAttemptCaptured => {
                let payload: TaskAttemptCapturedPayload =
                    serde_json::from_value(event.payload.clone())?;
                if payload.task_attempt_id == *task_attempt_id {
                    attempt = Some((TaskProjectionEventEnvelope::from_event(event), payload));
                }
            }
            EventType::TaskCheckpointCaptured => {
                let payload: TaskCheckpointCapturedPayload =
                    serde_json::from_value(event.payload.clone())?;
                if payload.parent_task_attempt_id == *task_attempt_id {
                    checkpoint_envelopes.insert(
                        payload.checkpoint_id.clone(),
                        (TaskProjectionEventEnvelope::from_event(event), payload),
                    );
                }
            }
            EventType::TaskObservationRecorded => {
                let payload: TaskObservationRecordedPayload =
                    serde_json::from_value(event.payload.clone())?;
                observation_records.push((TaskProjectionEventEnvelope::from_event(event), payload));
            }
            _ => continue,
        }
    }

    let Some((attempt_envelope, attempt_payload)) = attempt else {
        return Ok(None);
    };

    let mut diagnostics: Vec<TaskProjectionDiagnostic> = Vec::new();
    let mut observations_by_checkpoint: BTreeMap<CheckpointId, Vec<TaskObservationSummary>> =
        BTreeMap::new();
    let mut observations_without_checkpoint: Vec<TaskObservationSummary> = Vec::new();

    for (envelope, payload) in observation_records {
        let summary = TaskObservationSummary {
            envelope: envelope.clone(),
            observation_id: payload.observation_id,
            checkpoint_id: payload.checkpoint_id.clone(),
            title: payload.title,
            body: payload.body,
            body_artifact_path: payload.body_artifact_path,
            body_byte_size: payload.body_byte_size,
            body_content_hash: payload.body_content_hash,
        };

        match &summary.checkpoint_id {
            Some(checkpoint_id) => {
                if !checkpoint_envelopes.contains_key(checkpoint_id) {
                    diagnostics.push(TaskProjectionDiagnostic {
                        code: "observation_checkpoint_missing".to_owned(),
                        message: format!(
                            "observation {} names checkpoint {} which has no \
                             TaskCheckpointCaptured event under this attempt",
                            summary.observation_id.as_str(),
                            checkpoint_id.as_str()
                        ),
                        event_id: Some(envelope.event_id.clone()),
                    });
                    observations_without_checkpoint.push(summary);
                } else {
                    observations_by_checkpoint
                        .entry(checkpoint_id.clone())
                        .or_default()
                        .push(summary);
                }
            }
            None => observations_without_checkpoint.push(summary),
        }
    }

    sort_observations_recent_first(&mut observations_without_checkpoint);
    for bucket in observations_by_checkpoint.values_mut() {
        sort_observations_recent_first(bucket);
    }

    let mut checkpoints: Vec<TaskCheckpointSummary> = checkpoint_envelopes
        .into_values()
        .map(|(envelope, payload)| {
            let observations = observations_by_checkpoint
                .remove(&payload.checkpoint_id)
                .unwrap_or_default();
            TaskCheckpointSummary {
                envelope,
                checkpoint_id: payload.checkpoint_id,
                assistant_message_id: payload.assistant_message_id,
                tool_use_ids: payload.tool_use_ids,
                observations,
            }
        })
        .collect();

    checkpoints
        .sort_by(|left, right| envelope_chronological_order(&left.envelope, &right.envelope));

    let latest_checkpoint = checkpoints
        .iter()
        .max_by(|left, right| envelope_chronological_order(&left.envelope, &right.envelope))
        .cloned();

    Ok(Some(TaskAttemptSummary {
        reader_actor_id: reader_actor_id.clone(),
        task_attempt_id: task_attempt_id.clone(),
        attempt_event: attempt_envelope,
        project_path: attempt_payload.project_path,
        claude_session_uuid: attempt_payload.claude_session_uuid,
        initial_prompt_hash: attempt_payload.initial_prompt_hash,
        predecessor: attempt_payload.predecessor,
        latest_checkpoint,
        checkpoints,
        observations_without_checkpoint,
        diagnostics,
    }))
}

fn targets_task_attempt(event: &ShoreEvent, task_attempt_id: &WorkObjectId) -> bool {
    matches!(
        event.event_type,
        EventType::TaskAttemptCaptured
            | EventType::TaskCheckpointCaptured
            | EventType::TaskObservationRecorded
    ) && event.target.work_object_id.as_ref() == Some(task_attempt_id)
        && event.target.work_object_type == Some(WorkObjectType::TaskAttempt)
}

fn envelope_chronological_order(
    left: &TaskProjectionEventEnvelope,
    right: &TaskProjectionEventEnvelope,
) -> std::cmp::Ordering {
    left.occurred_at
        .cmp(&right.occurred_at)
        .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
}

fn sort_observations_recent_first(observations: &mut [TaskObservationSummary]) {
    observations.sort_by(|left, right| {
        right
            .envelope
            .occurred_at
            .cmp(&left.envelope.occurred_at)
            .then_with(|| {
                left.envelope
                    .event_id
                    .as_str()
                    .cmp(right.envelope.event_id.as_str())
            })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_hash::sha256_bytes_hex;
    use crate::model::{
        ActorId, CheckpointId, ObservationId, SessionId, TargetRef, TaskTargetRef, WorkObjectId,
    };
    use crate::session::event::{
        AssertionMode, EventTarget, EventType, ShoreEvent, SourceRef, TaskAttemptCapturedPayload,
        TaskCheckpointCapturedPayload, TaskObservationRecordedPayload, Writer, WriterRole,
        WriterTool,
    };

    fn writer_user() -> Writer {
        Writer {
            actor_id: ActorId::new("actor:claude_code:user"),
            role: WriterRole::User,
            tool: WriterTool {
                name: "claude_code".to_owned(),
                version: String::new(),
            },
        }
    }

    fn reader_actor() -> ActorId {
        ActorId::new("actor:shore:reader")
    }

    fn task_attempt_event(
        task_attempt_id: &WorkObjectId,
        session_id: &SessionId,
        claude_session_uuid: &str,
        occurred_at: &str,
    ) -> ShoreEvent {
        let target = EventTarget::for_work_object(
            session_id.clone(),
            task_attempt_id.clone(),
            WorkObjectType::TaskAttempt,
        );
        let payload = TaskAttemptCapturedPayload {
            task_attempt_id: task_attempt_id.clone(),
            project_path: "/repo".to_owned(),
            claude_session_uuid: claude_session_uuid.to_owned(),
            initial_prompt_hash: "sha256:prompt".to_owned(),
            predecessor: None,
        };
        let idempotency_key = TaskAttemptCapturedPayload::idempotency_key_for_work_object(
            task_attempt_id,
            WorkObjectType::TaskAttempt,
            claude_session_uuid,
        );
        let mut event = ShoreEvent::new(
            EventType::TaskAttemptCaptured,
            idempotency_key,
            target,
            writer_user(),
            payload,
            occurred_at,
        )
        .unwrap();
        event.source_ref = Some(SourceRef::new("claude_code", claude_session_uuid));
        event.assertion_mode = AssertionMode::Advisory;
        event
    }

    fn checkpoint_event(
        task_attempt_id: &WorkObjectId,
        session_id: &SessionId,
        checkpoint_id: &CheckpointId,
        assistant_message_id: &str,
        tool_use_ids: Vec<String>,
        occurred_at: &str,
    ) -> ShoreEvent {
        let mut target = EventTarget::for_work_object(
            session_id.clone(),
            task_attempt_id.clone(),
            WorkObjectType::TaskAttempt,
        );
        target.subject = Some(TargetRef::Task(TaskTargetRef::Checkpoint {
            checkpoint_id: checkpoint_id.clone(),
        }));
        let payload = TaskCheckpointCapturedPayload {
            checkpoint_id: checkpoint_id.clone(),
            parent_task_attempt_id: task_attempt_id.clone(),
            assistant_message_id: assistant_message_id.to_owned(),
            tool_use_ids,
        };
        let idempotency_key = TaskCheckpointCapturedPayload::idempotency_key_for_work_object(
            task_attempt_id,
            WorkObjectType::TaskAttempt,
            checkpoint_id.as_str(),
        );
        let mut event = ShoreEvent::new(
            EventType::TaskCheckpointCaptured,
            idempotency_key,
            target,
            Writer::shore_local_reviewer("test"),
            payload,
            occurred_at,
        )
        .unwrap();
        event.source_ref = Some(SourceRef::new(
            "claude_code",
            format!("session:assistant:{assistant_message_id}"),
        ));
        event.assertion_mode = AssertionMode::Advisory;
        event
    }

    fn observation_event(
        task_attempt_id: &WorkObjectId,
        session_id: &SessionId,
        checkpoint_id: Option<&CheckpointId>,
        source_id: &str,
        title: &str,
        occurred_at: &str,
    ) -> ShoreEvent {
        let observation_id = ObservationId::new(format!(
            "obs:sha256:{}",
            sha256_bytes_hex(source_id.as_bytes())
        ));
        let mut target = EventTarget::for_work_object(
            session_id.clone(),
            task_attempt_id.clone(),
            WorkObjectType::TaskAttempt,
        );
        target.subject = Some(match checkpoint_id {
            Some(checkpoint_id) => TargetRef::Task(TaskTargetRef::Checkpoint {
                checkpoint_id: checkpoint_id.clone(),
            }),
            None => TargetRef::Task(TaskTargetRef::TaskAttempt),
        });
        let payload = TaskObservationRecordedPayload {
            observation_id: observation_id.clone(),
            checkpoint_id: checkpoint_id.cloned(),
            title: title.to_owned(),
            body: None,
            body_artifact_path: None,
            body_byte_size: None,
            body_content_hash: None,
        };
        let idempotency_key = TaskObservationRecordedPayload::idempotency_key_for_work_object(
            task_attempt_id,
            WorkObjectType::TaskAttempt,
            observation_id.as_str(),
        );
        let mut event = ShoreEvent::new(
            EventType::TaskObservationRecorded,
            idempotency_key,
            target,
            Writer::shore_local_reviewer("test"),
            payload,
            occurred_at,
        )
        .unwrap();
        event.source_ref = Some(SourceRef::new("claude_code", source_id));
        event.assertion_mode = AssertionMode::Advisory;
        event
    }

    #[test]
    fn task_attempt_summary_rolls_up_one_attempt() {
        let task_attempt_id = WorkObjectId::new("task-attempt:sha256:ta");
        let session_id = SessionId::new("session:claude:uuid-1");
        let checkpoint_a = CheckpointId::new("checkpoint:sha256:cp-a");
        let checkpoint_b = CheckpointId::new("checkpoint:sha256:cp-b");

        let events = vec![
            task_attempt_event(
                &task_attempt_id,
                &session_id,
                "uuid-1",
                "2026-05-18T00:00:00Z",
            ),
            checkpoint_event(
                &task_attempt_id,
                &session_id,
                &checkpoint_a,
                "msg_1",
                vec!["tu_1".to_owned()],
                "2026-05-18T00:00:01Z",
            ),
            checkpoint_event(
                &task_attempt_id,
                &session_id,
                &checkpoint_b,
                "msg_2",
                vec!["tu_2".to_owned()],
                "2026-05-18T00:00:03Z",
            ),
            observation_event(
                &task_attempt_id,
                &session_id,
                Some(&checkpoint_a),
                "uuid-1#tool_result:tu_1",
                "tool_result: Bash",
                "2026-05-18T00:00:02Z",
            ),
            observation_event(
                &task_attempt_id,
                &session_id,
                Some(&checkpoint_b),
                "uuid-1#tool_result:tu_2",
                "tool_result: Read",
                "2026-05-18T00:00:04Z",
            ),
        ];

        let summary = task_attempt_summary_from_events(&events, &task_attempt_id, &reader_actor())
            .unwrap()
            .expect("attempt is present");

        assert_eq!(summary.reader_actor_id, reader_actor());
        assert_eq!(summary.task_attempt_id, task_attempt_id);
        assert_eq!(summary.project_path, "/repo");
        assert_eq!(summary.claude_session_uuid, "uuid-1");
        assert_eq!(summary.initial_prompt_hash, "sha256:prompt");
        assert_eq!(summary.predecessor, None);
        assert_eq!(summary.checkpoints.len(), 2);
        assert_eq!(
            summary
                .latest_checkpoint
                .as_ref()
                .map(|cp| cp.checkpoint_id.clone()),
            Some(checkpoint_b.clone())
        );

        let cp_a = summary
            .checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id == checkpoint_a)
            .expect("checkpoint a present");
        assert_eq!(cp_a.observations.len(), 1);
        assert_eq!(cp_a.observations[0].title, "tool_result: Bash");

        let cp_b = summary
            .checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id == checkpoint_b)
            .expect("checkpoint b present");
        assert_eq!(cp_b.observations.len(), 1);
        assert_eq!(cp_b.observations[0].title, "tool_result: Read");

        assert!(summary.observations_without_checkpoint.is_empty());
        assert!(summary.diagnostics.is_empty());
    }

    #[test]
    fn task_attempt_summary_ignores_other_task_attempts() {
        let attempt_a = WorkObjectId::new("task-attempt:sha256:a");
        let attempt_b = WorkObjectId::new("task-attempt:sha256:b");
        let session_a = SessionId::new("session:claude:uuid-a");
        let session_b = SessionId::new("session:claude:uuid-b");
        let checkpoint_a = CheckpointId::new("checkpoint:sha256:cp-a");
        let checkpoint_b = CheckpointId::new("checkpoint:sha256:cp-b");

        let events = vec![
            task_attempt_event(&attempt_a, &session_a, "uuid-a", "2026-05-18T00:00:00Z"),
            task_attempt_event(&attempt_b, &session_b, "uuid-b", "2026-05-18T00:00:00Z"),
            checkpoint_event(
                &attempt_a,
                &session_a,
                &checkpoint_a,
                "msg_a1",
                vec![],
                "2026-05-18T00:00:01Z",
            ),
            checkpoint_event(
                &attempt_b,
                &session_b,
                &checkpoint_b,
                "msg_b1",
                vec![],
                "2026-05-18T00:00:01Z",
            ),
            observation_event(
                &attempt_a,
                &session_a,
                Some(&checkpoint_a),
                "uuid-a#tool_result:1",
                "obs-a",
                "2026-05-18T00:00:02Z",
            ),
            observation_event(
                &attempt_b,
                &session_b,
                Some(&checkpoint_b),
                "uuid-b#tool_result:1",
                "obs-b",
                "2026-05-18T00:00:02Z",
            ),
        ];

        let summary = task_attempt_summary_from_events(&events, &attempt_a, &reader_actor())
            .unwrap()
            .expect("attempt a is present");

        assert_eq!(summary.task_attempt_id, attempt_a);
        assert_eq!(summary.checkpoints.len(), 1);
        assert_eq!(summary.checkpoints[0].checkpoint_id, checkpoint_a);
        assert_eq!(summary.checkpoints[0].observations.len(), 1);
        assert_eq!(summary.checkpoints[0].observations[0].title, "obs-a");

        for cp in &summary.checkpoints {
            assert_ne!(cp.checkpoint_id, checkpoint_b);
            for obs in &cp.observations {
                assert_ne!(obs.title, "obs-b");
            }
        }
    }

    #[test]
    fn task_attempt_summary_preserves_envelope_and_payload_fields() {
        let task_attempt_id = WorkObjectId::new("task-attempt:sha256:ta");
        let session_id = SessionId::new("session:claude:uuid-1");
        let checkpoint = CheckpointId::new("checkpoint:sha256:cp");

        let attempt = task_attempt_event(
            &task_attempt_id,
            &session_id,
            "uuid-1",
            "2026-05-18T00:00:00Z",
        );
        let checkpoint_event_ev = checkpoint_event(
            &task_attempt_id,
            &session_id,
            &checkpoint,
            "msg_1",
            vec!["tu_1".to_owned()],
            "2026-05-18T00:00:01Z",
        );
        let observation = observation_event(
            &task_attempt_id,
            &session_id,
            Some(&checkpoint),
            "uuid-1#tool_result:tu_1",
            "tool_result: Bash",
            "2026-05-18T00:00:02Z",
        );

        let events = vec![
            attempt.clone(),
            checkpoint_event_ev.clone(),
            observation.clone(),
        ];
        let summary = task_attempt_summary_from_events(&events, &task_attempt_id, &reader_actor())
            .unwrap()
            .expect("attempt present");

        let env = &summary.attempt_event;
        assert_eq!(env.event_id, attempt.event_id);
        assert_eq!(env.event_type, EventType::TaskAttemptCaptured);
        assert_eq!(env.occurred_at, attempt.occurred_at);
        assert_eq!(env.payload_hash, attempt.payload_hash);
        assert_eq!(env.writer, attempt.writer);
        assert_eq!(env.assertion_mode, AssertionMode::Advisory);
        assert_eq!(env.source_ref, attempt.source_ref);
        assert_eq!(env.target, attempt.target);

        let cp = summary.checkpoints.first().expect("checkpoint present");
        assert_eq!(cp.envelope.event_id, checkpoint_event_ev.event_id);
        assert_eq!(cp.envelope.event_type, EventType::TaskCheckpointCaptured);
        assert_eq!(cp.envelope.payload_hash, checkpoint_event_ev.payload_hash);
        assert_eq!(cp.envelope.writer, checkpoint_event_ev.writer);
        assert_eq!(cp.envelope.source_ref, checkpoint_event_ev.source_ref);
        assert_eq!(cp.envelope.target, checkpoint_event_ev.target);
        assert_eq!(cp.assistant_message_id, "msg_1");
        assert_eq!(cp.tool_use_ids, vec!["tu_1".to_owned()]);

        let obs = cp.observations.first().expect("observation present");
        assert_eq!(obs.envelope.event_id, observation.event_id);
        assert_eq!(obs.envelope.event_type, EventType::TaskObservationRecorded);
        assert_eq!(obs.envelope.payload_hash, observation.payload_hash);
        assert_eq!(obs.envelope.writer, observation.writer);
        assert_eq!(obs.envelope.source_ref, observation.source_ref);
        assert_eq!(obs.envelope.target, observation.target);
        assert_eq!(obs.title, "tool_result: Bash");
        assert_eq!(obs.checkpoint_id.as_ref(), Some(&checkpoint));
        assert_eq!(obs.body, None);
        assert_eq!(obs.body_artifact_path, None);
        assert_eq!(obs.body_byte_size, None);
        assert_eq!(obs.body_content_hash, None);
    }

    #[test]
    fn task_attempt_summary_orders_latest_checkpoint_and_recent_observations() {
        let task_attempt_id = WorkObjectId::new("task-attempt:sha256:ta");
        let session_id = SessionId::new("session:claude:uuid-1");
        let cp_early = CheckpointId::new("checkpoint:sha256:cp-early");
        let cp_late = CheckpointId::new("checkpoint:sha256:cp-late");

        // Feed events out of chronological order.
        let events = vec![
            observation_event(
                &task_attempt_id,
                &session_id,
                Some(&cp_late),
                "uuid-1#tool_result:later",
                "later observation",
                "2026-05-18T00:00:05Z",
            ),
            checkpoint_event(
                &task_attempt_id,
                &session_id,
                &cp_late,
                "msg_late",
                vec![],
                "2026-05-18T00:00:04Z",
            ),
            observation_event(
                &task_attempt_id,
                &session_id,
                Some(&cp_late),
                "uuid-1#tool_result:earlier-under-late",
                "earlier observation under late checkpoint",
                "2026-05-18T00:00:03Z",
            ),
            checkpoint_event(
                &task_attempt_id,
                &session_id,
                &cp_early,
                "msg_early",
                vec![],
                "2026-05-18T00:00:01Z",
            ),
            task_attempt_event(
                &task_attempt_id,
                &session_id,
                "uuid-1",
                "2026-05-18T00:00:00Z",
            ),
        ];

        let summary = task_attempt_summary_from_events(&events, &task_attempt_id, &reader_actor())
            .unwrap()
            .expect("attempt present");

        assert_eq!(
            summary
                .latest_checkpoint
                .as_ref()
                .map(|cp| cp.checkpoint_id.clone()),
            Some(cp_late.clone()),
            "latest_checkpoint is the highest occurred_at checkpoint"
        );

        let cp_late_summary = summary
            .checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id == cp_late)
            .expect("late checkpoint present");
        let titles: Vec<&str> = cp_late_summary
            .observations
            .iter()
            .map(|obs| obs.title.as_str())
            .collect();
        assert_eq!(
            titles,
            vec![
                "later observation",
                "earlier observation under late checkpoint",
            ],
            "observations sort by occurred_at descending"
        );
    }

    #[test]
    fn task_attempt_summary_does_not_depend_on_adapter_intents() {
        // Build inputs as already-written ShoreEvents only. This is a documentation
        // pin: the projection lives downstream of the write seam.
        let task_attempt_id = WorkObjectId::new("task-attempt:sha256:ta");
        let session_id = SessionId::new("session:claude:uuid-1");

        let events = vec![task_attempt_event(
            &task_attempt_id,
            &session_id,
            "uuid-1",
            "2026-05-18T00:00:00Z",
        )];

        let summary = task_attempt_summary_from_events(&events, &task_attempt_id, &reader_actor())
            .unwrap()
            .expect("attempt present");

        assert_eq!(summary.task_attempt_id, task_attempt_id);
        assert!(summary.checkpoints.is_empty());
        assert!(summary.observations_without_checkpoint.is_empty());
        assert!(summary.latest_checkpoint.is_none());
    }

    #[test]
    fn task_attempt_summary_returns_none_when_attempt_not_present() {
        let other_attempt = WorkObjectId::new("task-attempt:sha256:other");
        let queried_attempt = WorkObjectId::new("task-attempt:sha256:queried");
        let session_id = SessionId::new("session:claude:uuid-other");

        let events = vec![task_attempt_event(
            &other_attempt,
            &session_id,
            "uuid-other",
            "2026-05-18T00:00:00Z",
        )];

        let summary =
            task_attempt_summary_from_events(&events, &queried_attempt, &reader_actor()).unwrap();
        assert!(summary.is_none());
    }
}
