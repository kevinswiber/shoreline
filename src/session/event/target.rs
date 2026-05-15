use serde::{Deserialize, Serialize};

use crate::model::{
    ReviewId, ReviewTargetRef, ReviewUnitId, RevisionId, SnapshotId, TrackId, WorkUnitId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTarget {
    pub review_id: ReviewId,
    /// Work-unit target used by review-level events that do not yet target a
    /// captured ReviewUnit, such as initialization and imported review notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_unit_id: Option<WorkUnitId>,
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
}

impl EventTarget {
    pub fn new(review_id: ReviewId, work_unit_id: WorkUnitId) -> Self {
        Self {
            review_id,
            work_unit_id: Some(work_unit_id),
            review_unit_id: None,
            revision_id: None,
            snapshot_id: None,
            track_id: None,
            subject: None,
        }
    }

    pub fn for_review_unit(
        review_id: ReviewId,
        review_unit_id: ReviewUnitId,
        revision_id: RevisionId,
        snapshot_id: SnapshotId,
    ) -> Self {
        Self {
            review_id,
            work_unit_id: None,
            review_unit_id: Some(review_unit_id.clone()),
            revision_id: Some(revision_id),
            snapshot_id: Some(snapshot_id),
            track_id: None,
            subject: Some(ReviewTargetRef::ReviewUnit { review_unit_id }),
        }
    }
}
