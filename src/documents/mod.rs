//! Command-output document layer for the `shore review-*` command family.
//!
//! This module owns the serializable documents the `shore review-*` commands
//! emit: the shared envelopes ([`DiagnosticDocument`], [`EventWriteDocument`]),
//! the per-item view-document mappers, the per-command body structs, and the
//! `*_document()` builders that wrap a `shoreline::session` result into the
//! documented JSON shape.
//!
//! Consumers can produce **byte-identical** `shore review-*` JSON in-process by
//! calling a builder and serializing the returned document with `serde_json`.
//! The CLI is a thin caller over these same builders, so the documented JSON
//! contract has a single source of truth.
//!
//! Stdout serialization (`write_json`) stays in the CLI; this module exposes the
//! serializable documents, not terminal IO.

use std::collections::BTreeMap;

use crate::session::ProjectionDiagnostic;

mod assessment;
mod capture;
mod history;
mod input_request;
mod observation;
mod unit;
mod view;

pub use assessment::{
    AssessmentAddBody, AssessmentShowBody, assessment_add_document, assessment_show_document,
};
pub use capture::{CaptureBody, capture_document};
pub use history::{HistoryBody, history_document};
pub use input_request::{
    InputRequestFetchBody, InputRequestListBody, InputRequestOpenBody, InputRequestRespondBody,
    input_request_fetch_document, input_request_list_document, input_request_open_document,
    input_request_respond_document,
};
pub use observation::{
    ObservationAddBody, ObservationListBody, observation_add_document, observation_list_document,
};
pub use unit::{UnitListBody, UnitShowBody, unit_list_document, unit_show_document};
pub use view::{
    AssessmentViewDocument, CurrentAssessmentDocument, InputRequestAssertionModeDocument,
    InputRequestResponseViewDocument, InputRequestViewDocument, ObservationViewDocument,
};

/// Envelope for a read/diagnostic document: `{ schema, version, <flattened
/// body>, diagnostics }`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticDocument<T> {
    schema: &'static str,
    version: u32,
    #[serde(flatten)]
    body: T,
    diagnostics: Vec<ProjectionDiagnostic>,
}

/// Envelope for an event-write document: the diagnostic envelope plus the
/// `eventsCreated`/`eventsExisting`/`eventsCreatedByType` write counts.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventWriteDocument<T> {
    schema: &'static str,
    version: u32,
    #[serde(flatten)]
    body: T,
    events_created: usize,
    events_existing: usize,
    events_created_by_type: BTreeMap<String, usize>,
    diagnostics: Vec<ProjectionDiagnostic>,
}

impl<T> DiagnosticDocument<T> {
    /// Wrap `body` in the diagnostic envelope under `schema` at version 1.
    pub fn new(schema: &'static str, body: T, diagnostics: Vec<ProjectionDiagnostic>) -> Self {
        Self {
            schema,
            version: 1,
            body,
            diagnostics,
        }
    }
}

impl<T> EventWriteDocument<T> {
    /// Wrap `body` in the event-write envelope under `schema` at version 1.
    pub fn new(
        schema: &'static str,
        body: T,
        events_created: usize,
        events_existing: usize,
        events_created_by_type: BTreeMap<String, usize>,
        diagnostics: Vec<ProjectionDiagnostic>,
    ) -> Self {
        Self {
            schema,
            version: 1,
            body,
            events_created,
            events_existing,
            events_created_by_type,
            diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    fn write_compact<T: serde::Serialize>(document: &T) -> String {
        let mut buf = Vec::new();
        serde_json::to_writer(&mut buf, document).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn event_write_document_preserves_field_order() {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            review_unit_id: &'static str,
            event_id: &'static str,
        }

        let doc = super::EventWriteDocument::new(
            "shore.test-write",
            Body {
                review_unit_id: "unit:1",
                event_id: "evt:1",
            },
            1,
            2,
            BTreeMap::new(),
            Vec::new(),
        );

        assert_eq!(
            write_compact(&doc),
            "{\"schema\":\"shore.test-write\",\"version\":1,\"reviewUnitId\":\"unit:1\",\"eventId\":\"evt:1\",\"eventsCreated\":1,\"eventsExisting\":2,\"eventsCreatedByType\":{},\"diagnostics\":[]}"
        );
    }

    #[test]
    fn diagnostic_document_preserves_trailing_diagnostics() {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            review_unit_id: &'static str,
            count: usize,
        }

        let doc = super::DiagnosticDocument::new(
            "shore.test-read",
            Body {
                review_unit_id: "unit:1",
                count: 3,
            },
            Vec::new(),
        );

        assert_eq!(
            write_compact(&doc),
            "{\"schema\":\"shore.test-read\",\"version\":1,\"reviewUnitId\":\"unit:1\",\"count\":3,\"diagnostics\":[]}"
        );
    }
}
