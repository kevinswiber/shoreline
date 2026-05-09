use serde_json::Value;
use shore::dump::{DumpDocument, DumpInputSource, DumpInputSummary};
use shore::model::{
    Anchor, DiffFile, DiffRow, DiffRowKind, DiffSnapshot, FileId, FileStatus, HunkId, LineRange,
    ResolutionStatus, ReviewHunk, ReviewId, ReviewNote, ReviewNoteId, ReviewNoteSource,
    ReviewStream, Side, SnapshotId,
};
use shore::sidecar::{DiagnosticLevel, ReviewNotesDiagnostic, ReviewNotesDiagnosticCode};

#[test]
fn dump_document_serializes_summary_diagnostics_and_stream_rows() {
    let snapshot = snapshot_with_one_hunk();
    let hunk = &snapshot.files[0].hunks[0];
    let note = ReviewNote {
        id: ReviewNoteId::new("note:demo"),
        anchor: Anchor {
            file_id: FileId::new("src/lib.rs"),
            side: Side::New,
            line_range: LineRange::new(1, 1),
            hunk_signature: hunk.signature(),
            target_text_hash: "sha256:demo".to_owned(),
            status: ResolutionStatus::Exact,
        },
        source: ReviewNoteSource::Sidecar,
        title: "Demo note".to_owned(),
        body: Some("Details".to_owned()),
        tags: vec!["demo".to_owned()],
        confidence: Some("high".to_owned()),
        external_source: Some("reviewer".to_owned()),
        author: Some("human reviewer".to_owned()),
        created_at: Some("2026-05-09T03:16:51Z".to_owned()),
    };
    let stream =
        ReviewStream::from_snapshot_with_resolved_notes(&snapshot, std::slice::from_ref(&note));
    let diagnostic = ReviewNotesDiagnostic {
        level: DiagnosticLevel::Warning,
        code: ReviewNotesDiagnosticCode::MissingNoteTitle,
        path: "files[0].notes[0].title".to_owned(),
        message: "review note is missing title".to_owned(),
    };

    let document = DumpDocument::new(
        DumpInputSummary {
            source: DumpInputSource::ReviewNotes,
        },
        snapshot,
        vec![note],
        stream,
        vec![diagnostic],
    );

    let json = serde_json::to_value(&document).expect("dump document serializes");

    assert_eq!(json["schema"], "shore.dump");
    assert_eq!(json["version"], 1);
    assert_eq!(json["input"]["source"], "review_notes");
    assert_eq!(json["summary"]["file_count"], 1);
    assert_eq!(json["summary"]["hunk_count"], 1);
    assert_eq!(json["summary"]["row_count"], 4);
    assert_eq!(json["summary"]["note_count"], 1);
    assert_eq!(json["summary"]["diagnostic_count"], 1);
    assert_eq!(json["diagnostics"][0]["level"], "warning");
    assert_eq!(json["diagnostics"][0]["code"], "missing_note_title");
    assert_eq!(json["diagnostics"][0]["path"], "files[0].notes[0].title");
    assert_eq!(
        json["diagnostics"][0]["message"],
        "review note is missing title"
    );
    assert_eq!(
        json["stream"]["rows"]
            .as_array()
            .expect("rows are array")
            .len(),
        4
    );
}

#[test]
fn dump_input_source_serializes_as_snake_case() {
    assert_eq!(
        input_source_value(DumpInputSource::None),
        Value::String("none".to_owned())
    );
    assert_eq!(
        input_source_value(DumpInputSource::ReviewNotes),
        Value::String("review_notes".to_owned())
    );
    assert_eq!(
        input_source_value(DumpInputSource::LegacyHunkAgentContext),
        Value::String("legacy_hunk_agent_context".to_owned())
    );
}

fn input_source_value(source: DumpInputSource) -> Value {
    serde_json::to_value(DumpInputSummary { source })
        .expect("input summary serializes")
        .get("source")
        .expect("source field exists")
        .clone()
}

fn snapshot_with_one_hunk() -> DiffSnapshot {
    let hunk = ReviewHunk {
        id: HunkId::new("hunk:1"),
        header: "@@ -1 +1 @@".to_owned(),
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 1,
        rows: vec![DiffRow {
            kind: DiffRowKind::Added,
            old_line: None,
            new_line: Some(1),
            text: "pub fn demo() {}".to_owned(),
        }],
    };

    DiffSnapshot::new(
        ReviewId::new("review:test"),
        SnapshotId::new("snapshot:test"),
        vec![DiffFile {
            id: FileId::new("src/lib.rs"),
            status: FileStatus::Modified,
            old_path: Some("src/lib.rs".to_owned()),
            new_path: Some("src/lib.rs".to_owned()),
            old_mode: None,
            new_mode: None,
            old_oid: None,
            new_oid: None,
            similarity: None,
            is_binary: false,
            is_submodule: false,
            is_mode_only: false,
            synthetic: false,
            metadata_rows: Vec::new(),
            hunks: vec![hunk],
        }],
    )
}
