//! The opaque signed-target subject id (seam **S2**).
//!
//! The signed envelope binds an opaque `subjectId` — a sha256 over the subject's
//! **identity-bearing fields only** — in place of the structural subject, so a
//! future display rename of a subject's kind tag is projection-only. The
//! structural subject is carried in the event payload and reconstructed by the
//! projection for display.
//!
//! The derivation excludes the renamable variant/kind tag (the sub-anchor kind
//! and the `Review`/`Task` domain tag): domain stays structurally derived
//! (ADR-0017 §A4), never folded here. A `TargetRef::Journal` carrier has no
//! subject, so it yields `None`.

use serde_json::json;

use crate::canonical_hash::sha256_json_prefixed;
use crate::error::{Result, ShoreError};
use crate::model::{TargetRef, TaskTargetRef, WorkObjectId, id_prefix};

/// The identity-bearing material a [`subject_id`] is derived from, assembled by
/// the caller (the write path / migrator) rather than read from the renamable
/// tag. `TaskTargetRef::TaskAttempt` is fieldless, so a task subject's identity
/// is the work-object (attempt) id, threaded here from the write path;
/// [`work_object_id`](Self::work_object_id) is ignored for review and journal
/// subjects.
pub(crate) struct SubjectIdentity<'a> {
    pub(crate) subject: &'a TargetRef,
    pub(crate) work_object_id: Option<&'a WorkObjectId>,
}

impl<'a> SubjectIdentity<'a> {
    /// A review or journal subject, whose identity is fully carried by the
    /// `TargetRef` itself.
    pub(crate) fn new(subject: &'a TargetRef) -> Self {
        Self {
            subject,
            work_object_id: None,
        }
    }

    /// A subject whose identity needs the work-object (attempt) id threaded in —
    /// required for `TargetRef::Task` subjects (the fieldless `TaskAttempt`).
    pub(crate) fn with_work_object_id(subject: &'a TargetRef, work_object_id: &'a WorkObjectId) -> Self {
        Self {
            subject,
            work_object_id: Some(work_object_id),
        }
    }
}

/// Derive the opaque `subjectId` for a subject, or `None` for the fieldless
/// `TargetRef::Journal` carrier. The digest folds identity-bearing fields only,
/// never the renamable kind tag or the derived domain.
pub(crate) fn subject_id(identity: &SubjectIdentity<'_>) -> Result<Option<String>> {
    let material = match identity.subject {
        TargetRef::Journal => return Ok(None),
        TargetRef::Review(review) => {
            // Fold the review sub-anchor's identity fields; drop the renamable
            // `kind` tag so a future rename of the sub-anchor is projection-only.
            let mut value = serde_json::to_value(review)?;
            if let Some(object) = value.as_object_mut() {
                object.remove("kind");
            }
            value
        }
        TargetRef::Task(task) => {
            // A task subject's identity is the work-object (attempt) id — the
            // `TaskAttempt` variant is fieldless — plus the checkpoint id for a
            // checkpoint sub-target. The `task`/`checkpoint` kind tag is excluded.
            let work_object_id = identity.work_object_id.ok_or_else(|| {
                ShoreError::Message(
                    "subject_id for a task subject requires the work-object id".to_owned(),
                )
            })?;
            let mut value = json!({ "workObjectId": work_object_id.as_str() });
            if let TaskTargetRef::Checkpoint { checkpoint_id } = task {
                value["checkpointId"] = json!(checkpoint_id.as_str());
            }
            value
        }
    };

    let digest = sha256_json_prefixed(&material)?;
    Ok(Some(format!("{}:{digest}", id_prefix::SUBJECT)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CheckpointId, ObservationId, ReviewTargetRef, RevisionId, Side, WorkObjectId,
    };

    fn revision(id: &str) -> RevisionId {
        RevisionId::new(id)
    }

    #[test]
    fn subject_id_absent_for_journal_carrier() {
        let subject = TargetRef::Journal;
        let identity = SubjectIdentity::new(&subject);

        assert_eq!(subject_id(&identity).unwrap(), None);
    }

    #[test]
    fn subject_id_is_a_prefixed_content_id() {
        let subject = TargetRef::Review(ReviewTargetRef::Revision {
            revision_id: revision("rev:sha256:abc"),
        });
        let identity = SubjectIdentity::new(&subject);

        let id = subject_id(&identity).unwrap().unwrap();
        assert!(id.starts_with("subject:sha256:"), "got {id}");
    }

    #[test]
    fn subject_id_excludes_the_renamable_kind_tag() {
        // The digest must fold identity fields only, never the `kind` tag: a
        // subject_id equals the sha256 of the same fields with `kind` stripped.
        let subject = TargetRef::Review(ReviewTargetRef::File {
            revision_id: revision("rev:sha256:abc"),
            file_path: "src/lib.rs".to_owned(),
        });
        let identity = SubjectIdentity::new(&subject);

        let expected_material = json!({
            "revisionId": "rev:sha256:abc",
            "filePath": "src/lib.rs",
        });
        let expected = format!(
            "{}:{}",
            id_prefix::SUBJECT,
            sha256_json_prefixed(&expected_material).unwrap()
        );

        assert_eq!(subject_id(&identity).unwrap().unwrap(), expected);
    }

    #[test]
    fn subject_id_changes_with_revision_id() {
        let subject_x = TargetRef::Review(ReviewTargetRef::Revision {
            revision_id: revision("rev:sha256:x"),
        });
        let subject_y = TargetRef::Review(ReviewTargetRef::Revision {
            revision_id: revision("rev:sha256:y"),
        });

        assert_ne!(
            subject_id(&SubjectIdentity::new(&subject_x)).unwrap(),
            subject_id(&SubjectIdentity::new(&subject_y)).unwrap()
        );
    }

    #[test]
    fn subject_id_distinguishes_review_sub_anchors() {
        // A File anchor and a Range anchor on the same revision+path are different
        // subjects (Range carries side/line fields), so their ids must differ.
        let file = TargetRef::Review(ReviewTargetRef::File {
            revision_id: revision("rev:sha256:abc"),
            file_path: "src/lib.rs".to_owned(),
        });
        let range = TargetRef::Review(ReviewTargetRef::Range {
            revision_id: revision("rev:sha256:abc"),
            file_path: "src/lib.rs".to_owned(),
            side: Side::New,
            start_line: 1,
            end_line: 4,
        });

        assert_ne!(
            subject_id(&SubjectIdentity::new(&file)).unwrap(),
            subject_id(&SubjectIdentity::new(&range)).unwrap()
        );
    }

    #[test]
    fn review_and_task_subjects_do_not_collide() {
        let review = TargetRef::Review(ReviewTargetRef::Observation {
            revision_id: revision("rev:sha256:abc"),
            observation_id: ObservationId::new("obs:sha256:abc"),
        });
        let attempt_id = WorkObjectId::new("task-attempt:sha256:abc");
        let task = TargetRef::Task(TaskTargetRef::TaskAttempt);

        assert_ne!(
            subject_id(&SubjectIdentity::new(&review)).unwrap(),
            subject_id(&SubjectIdentity::with_work_object_id(&task, &attempt_id)).unwrap()
        );
    }

    #[test]
    fn subject_id_distinguishes_task_attempts() {
        let task = TargetRef::Task(TaskTargetRef::TaskAttempt);
        let attempt_a = WorkObjectId::new("task-attempt:sha256:a");
        let attempt_b = WorkObjectId::new("task-attempt:sha256:b");

        assert_ne!(
            subject_id(&SubjectIdentity::with_work_object_id(&task, &attempt_a)).unwrap(),
            subject_id(&SubjectIdentity::with_work_object_id(&task, &attempt_b)).unwrap()
        );
    }

    #[test]
    fn task_checkpoint_distinguishes_from_bare_attempt() {
        let attempt_id = WorkObjectId::new("task-attempt:sha256:abc");
        let bare = TargetRef::Task(TaskTargetRef::TaskAttempt);
        let checkpoint = TargetRef::Task(TaskTargetRef::Checkpoint {
            checkpoint_id: CheckpointId::new("checkpoint:sha256:c"),
        });

        assert_ne!(
            subject_id(&SubjectIdentity::with_work_object_id(&bare, &attempt_id)).unwrap(),
            subject_id(&SubjectIdentity::with_work_object_id(&checkpoint, &attempt_id)).unwrap()
        );
    }

    #[test]
    fn task_subject_without_work_object_id_is_an_error() {
        let task = TargetRef::Task(TaskTargetRef::TaskAttempt);
        let identity = SubjectIdentity::new(&task);

        assert!(subject_id(&identity).is_err());
    }
}
