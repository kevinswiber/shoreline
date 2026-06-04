// Document builders for `shore review lineage attach` and `show`.
use crate::documents::{DiagnosticDocument, EventWriteDocument};
use crate::session::{LineageAttachResult, LineageRoundView, LineageShowResult};

/// Documented body for `shore.review-lineage-attach`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageAttachBody {
    lineage_id: String,
    head_review_unit_id: Option<String>,
}

/// Documented body for `shore.review-lineage`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageShowBody {
    event_set_hash: String,
    event_count: usize,
    lineage_id: String,
    head_review_unit_id: Option<String>,
    rounds: Vec<LineageRoundDocument>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LineageRoundDocument {
    lineage_id: String,
    round_id: String,
    review_unit_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    predecessor_review_unit_id: Option<String>,
    round_index: Option<usize>,
    is_head: bool,
}

/// Build the `shore.review-lineage-attach` document from an attach result.
pub fn lineage_attach_document(
    result: LineageAttachResult,
) -> EventWriteDocument<LineageAttachBody> {
    EventWriteDocument::new(
        "shore.review-lineage-attach",
        LineageAttachBody {
            lineage_id: result.lineage_id.as_str().to_owned(),
            head_review_unit_id: result
                .head_review_unit_id
                .map(|review_unit_id| review_unit_id.as_str().to_owned()),
        },
        result.events_created,
        result.events_existing,
        result.events_created_by_type,
        result.diagnostics,
    )
}

/// Build the `shore.review-lineage` document from a lineage show result.
pub fn lineage_show_document(result: LineageShowResult) -> DiagnosticDocument<LineageShowBody> {
    DiagnosticDocument::new(
        "shore.review-lineage",
        LineageShowBody {
            event_set_hash: result.event_set_hash,
            event_count: result.event_count,
            lineage_id: result.lineage_id.as_str().to_owned(),
            head_review_unit_id: result
                .head_review_unit_id
                .map(|review_unit_id| review_unit_id.as_str().to_owned()),
            rounds: result
                .rounds
                .into_iter()
                .map(LineageRoundDocument::from)
                .collect(),
        },
        result.diagnostics,
    )
}

impl From<LineageRoundView> for LineageRoundDocument {
    fn from(round: LineageRoundView) -> Self {
        Self {
            lineage_id: round.lineage_id.as_str().to_owned(),
            round_id: round.round_id.as_str().to_owned(),
            review_unit_id: round.review_unit_id.as_str().to_owned(),
            predecessor_review_unit_id: round
                .predecessor_review_unit_id
                .map(|review_unit_id| review_unit_id.as_str().to_owned()),
            round_index: round.round_index,
            is_head: round.is_head,
        }
    }
}
