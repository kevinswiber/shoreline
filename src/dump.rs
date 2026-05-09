use serde::{Deserialize, Serialize};

use crate::model::{DiffSnapshot, ReviewNote, ReviewStream};
use crate::sidecar::ReviewNotesDiagnostic;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DumpDocument {
    pub schema: String,
    pub version: u32,
    pub input: DumpInputSummary,
    pub summary: DumpSummary,
    pub diagnostics: Vec<ReviewNotesDiagnostic>,
    pub snapshot: DiffSnapshot,
    pub notes: Vec<ReviewNote>,
    pub stream: ReviewStream,
}

impl DumpDocument {
    pub fn new(
        input: DumpInputSummary,
        snapshot: DiffSnapshot,
        notes: Vec<ReviewNote>,
        stream: ReviewStream,
        diagnostics: Vec<ReviewNotesDiagnostic>,
    ) -> Self {
        let summary = DumpSummary::from_parts(&snapshot, &notes, &stream, &diagnostics);
        Self {
            schema: "shore.dump".to_owned(),
            version: 1,
            input,
            summary,
            diagnostics,
            snapshot,
            notes,
            stream,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DumpInputSummary {
    pub source: DumpInputSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DumpInputSource {
    None,
    ReviewNotes,
    LegacyHunkAgentContext,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DumpSummary {
    pub file_count: usize,
    pub hunk_count: usize,
    pub row_count: usize,
    pub note_count: usize,
    pub diagnostic_count: usize,
}

impl DumpSummary {
    fn from_parts(
        snapshot: &DiffSnapshot,
        notes: &[ReviewNote],
        stream: &ReviewStream,
        diagnostics: &[ReviewNotesDiagnostic],
    ) -> Self {
        Self {
            file_count: snapshot.files.len(),
            hunk_count: snapshot.files.iter().map(|file| file.hunks.len()).sum(),
            row_count: stream.rows.len(),
            note_count: notes.len(),
            diagnostic_count: diagnostics.len(),
        }
    }
}
