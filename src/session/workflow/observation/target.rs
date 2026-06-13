use std::path::Path;

use crate::error::{Result, ShoreError};
use crate::model::{
    ReviewTargetRef, ReviewUnitId, ReviewUnitLineageId, RevisionId, SessionId, Side, SnapshotId,
};
use crate::session::event::{EventType, ReviewUnitCapturedPayload, ShoreEvent};
use crate::session::projection::lineage::ReviewUnitLineageProjection;
use crate::session::snapshot_artifact::read_snapshot_artifact_for_write_validation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedReviewUnit {
    pub session_id: SessionId,
    pub review_unit_id: ReviewUnitId,
    pub revision_id: RevisionId,
    pub snapshot_id: SnapshotId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewUnitSelection<'a> {
    Current,
    Exact(&'a ReviewUnitId),
    LineageHead(&'a ReviewUnitLineageId),
}

impl<'a> ReviewUnitSelection<'a> {
    pub(crate) fn from_review_unit_or_lineage(
        review_unit_id: Option<&'a ReviewUnitId>,
        lineage_id: Option<&'a ReviewUnitLineageId>,
    ) -> Result<Self> {
        match (review_unit_id, lineage_id) {
            (Some(_), Some(_)) => Err(ShoreError::WorkflowInputInvalid {
                reason: "cannot combine --review-unit and --lineage".to_owned(),
            }),
            (Some(review_unit_id), None) => Ok(Self::Exact(review_unit_id)),
            (None, Some(lineage_id)) => Ok(Self::LineageHead(lineage_id)),
            (None, None) => Ok(Self::Current),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationTargetSelector {
    pub file_path: Option<String>,
    pub side: Side,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

impl ObservationTargetSelector {
    pub fn review_unit() -> Self {
        Self {
            file_path: None,
            side: Side::New,
            start_line: None,
            end_line: None,
        }
    }

    pub fn file(path: impl Into<String>) -> Self {
        Self {
            file_path: Some(path.into()),
            side: Side::New,
            start_line: None,
            end_line: None,
        }
    }

    pub fn range(
        path: impl Into<String>,
        side: Side,
        start_line: u32,
        end_line: Option<u32>,
    ) -> Self {
        Self {
            file_path: Some(path.into()),
            side,
            start_line: Some(start_line),
            end_line,
        }
    }
}

pub(crate) fn resolve_review_unit(
    events: &[ShoreEvent],
    selection: ReviewUnitSelection<'_>,
) -> Result<ResolvedReviewUnit> {
    if let ReviewUnitSelection::LineageHead(lineage_id) = selection {
        let projection = ReviewUnitLineageProjection::from_events(events)?;
        let lineage = projection.lineage(lineage_id).ok_or_else(|| {
            ShoreError::Message(format!(
                "unknown ReviewUnit lineage: {}",
                lineage_id.as_str()
            ))
        })?;
        if !lineage.diagnostics.is_empty() {
            return Err(ShoreError::Message(format!(
                "ReviewUnit lineage {} is malformed",
                lineage_id.as_str()
            )));
        }
        let head = lineage.head_review_unit_id.as_ref().ok_or_else(|| {
            ShoreError::Message(format!(
                "ReviewUnit lineage {} has no current head",
                lineage_id.as_str()
            ))
        })?;
        return resolve_review_unit(events, ReviewUnitSelection::Exact(head));
    }

    let mut captured = Vec::new();
    for event in events
        .iter()
        .filter(|event| event.event_type == EventType::ReviewUnitCaptured)
    {
        let payload: ReviewUnitCapturedPayload = serde_json::from_value(event.payload.clone())?;
        let resolved = ResolvedReviewUnit {
            session_id: event.target.session_id.clone(),
            review_unit_id: payload.review_unit_id,
            revision_id: payload.revision_id,
            snapshot_id: payload.snapshot_id,
        };
        if matches!(selection, ReviewUnitSelection::Exact(requested) if requested == &resolved.review_unit_id)
        {
            return Ok(resolved);
        }
        captured.push(resolved);
    }

    if let ReviewUnitSelection::Exact(requested) = selection {
        return Err(ShoreError::Message(format!(
            "unknown review unit: {}",
            requested.as_str()
        )));
    }

    match captured.as_slice() {
        [] => Err(ShoreError::Message("no captured review unit".to_owned())),
        [resolved] => Ok(resolved.clone()),
        _ => Err(ShoreError::Message(
            "multiple captured review units; pass --review-unit".to_owned(),
        )),
    }
}

pub(crate) fn resolve_observation_target(
    repo: &Path,
    resolved: &ResolvedReviewUnit,
    selector: &ObservationTargetSelector,
) -> Result<ReviewTargetRef> {
    let Some(file_path) = selector.file_path.as_deref() else {
        if selector.start_line.is_some() || selector.end_line.is_some() {
            return Err(ShoreError::WorkflowInputInvalid {
                reason: "file is required when selecting observation lines".to_owned(),
            });
        }
        return Ok(ReviewTargetRef::ReviewUnit {
            review_unit_id: resolved.review_unit_id.clone(),
        });
    };

    let artifact = read_snapshot_artifact_for_write_validation(repo, &resolved.snapshot_id)?;
    if !artifact.snapshot.files.iter().any(|file| {
        file.new_path.as_deref() == Some(file_path) || file.old_path.as_deref() == Some(file_path)
    }) {
        return Err(ShoreError::Message(format!(
            "file target is not present in captured snapshot: {file_path}"
        )));
    }

    match selector.start_line {
        Some(start_line) => {
            if start_line == 0 {
                return Err(ShoreError::WorkflowInputInvalid {
                    reason: "start line must be greater than zero".to_owned(),
                });
            }
            let end_line = selector.end_line.unwrap_or(start_line);
            if end_line < start_line {
                return Err(ShoreError::WorkflowInputInvalid {
                    reason: "end line must be greater than or equal to start line".to_owned(),
                });
            }
            Ok(ReviewTargetRef::Range {
                review_unit_id: resolved.review_unit_id.clone(),
                file_path: file_path.to_owned(),
                side: selector.side,
                start_line,
                end_line,
            })
        }
        None => {
            if selector.end_line.is_some() {
                return Err(ShoreError::WorkflowInputInvalid {
                    reason: "start line is required when end line is supplied".to_owned(),
                });
            }
            Ok(ReviewTargetRef::File {
                review_unit_id: resolved.review_unit_id.clone(),
                file_path: file_path.to_owned(),
            })
        }
    }
}
