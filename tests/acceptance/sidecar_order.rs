use shore::model::{
    AnnotationSource, DiffFile, DiffRow, DiffRowKind, FileId, FileStatus, HunkId, LineRange,
    ResolutionStatus, ReviewHunk, Side,
};
use shore::sidecar::{
    AgentAnnotation, AgentContext, AgentFileContext, DiagnosticCode, DiagnosticLevel, Range,
    apply_file_order, parse_agent_context, resolve_annotations,
};

#[test]
fn hunk_compatible_agent_context_parses_file_order_and_annotations() {
    let parsed = parse_agent_context(include_str!(
        "../fixtures/sidecars/basic-agent-context.json"
    ))
    .expect("valid sidecar parses");

    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    assert_eq!(parsed.context.version, 1);
    assert_eq!(
        parsed.context.schema.as_deref(),
        Some("shore.agent-context")
    );
    assert_eq!(
        parsed.context.summary.as_deref(),
        Some("Phase 3 parser fixture")
    );

    let paths = parsed
        .context
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["src/new_name.rs", "src/git/raw.rs"]);

    let renamed = &parsed.context.files[0];
    assert_eq!(renamed.old_path.as_deref(), Some("src/old_name.rs"));
    assert_eq!(
        renamed.summary.as_deref(),
        Some("Narrative summary for renamed file")
    );
    assert_eq!(renamed.annotations.len(), 2);
    assert_eq!(renamed.annotations[0].id.as_deref(), Some("ann-new-range"));
    assert_eq!(
        renamed.annotations[0].new_range,
        Some(Range { start: 10, end: 12 })
    );
    assert_eq!(
        renamed.annotations[0].summary.as_deref(),
        Some("New-side annotation")
    );
    assert_eq!(
        renamed.annotations[0].rationale.as_deref(),
        Some("Explains why the new range matters")
    );
    assert_eq!(renamed.annotations[0].tags, vec!["risk", "parser"]);

    assert_eq!(
        renamed.annotations[1].old_range,
        Some(Range { start: 7, end: 7 })
    );
    assert_eq!(
        renamed.annotations[1].summary.as_deref(),
        Some("Old-side annotation")
    );
}

#[test]
fn invalid_sidecar_entries_return_recoverable_diagnostics() {
    let parsed = parse_agent_context(include_str!(
        "../fixtures/sidecars/invalid-agent-context.json"
    ))
    .expect("invalid annotations remain recoverable");

    assert_eq!(parsed.context.files.len(), 2);
    assert_eq!(parsed.diagnostics.len(), 4);

    let diagnostics = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.level.clone(),
                diagnostic.code.clone(),
                diagnostic.path.as_str(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostics,
        vec![
            (
                DiagnosticLevel::Warning,
                DiagnosticCode::InvalidRange,
                "files[0].annotations[0].newRange"
            ),
            (
                DiagnosticLevel::Warning,
                DiagnosticCode::InvalidRange,
                "files[0].annotations[1].oldRange"
            ),
            (
                DiagnosticLevel::Warning,
                DiagnosticCode::MissingAnnotationSummary,
                "files[0].annotations[2].summary"
            ),
            (
                DiagnosticLevel::Warning,
                DiagnosticCode::MissingFilePath,
                "files[1].path"
            ),
        ]
    );
}

#[test]
fn sidecar_order_reorders_matching_files_and_warns_for_stale_paths() {
    let files = vec![
        modified_file("src/a.rs"),
        modified_file("src/b.rs"),
        renamed_file("src/old_c.rs", "src/c.rs"),
        modified_file("src/d.rs"),
    ];
    let context = AgentContext {
        schema: Some("shore.agent-context".to_owned()),
        version: 1,
        summary: None,
        ownership: None,
        files: vec![
            sidecar_file("src/context-c.rs", Some("src/old_c.rs")),
            sidecar_file("src/a.rs", None),
            sidecar_file("src/stale.rs", None),
        ],
    };

    let ordered = apply_file_order(files, &context);

    let paths = ordered
        .files
        .iter()
        .map(|file| {
            file.new_path
                .as_deref()
                .or(file.old_path.as_deref())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["src/c.rs", "src/a.rs", "src/b.rs", "src/d.rs"]);
    assert_eq!(ordered.diagnostics.len(), 1);
    assert_eq!(ordered.diagnostics[0].level, DiagnosticLevel::Warning);
    assert_eq!(ordered.diagnostics[0].code, DiagnosticCode::StaleFilePath);
    assert_eq!(ordered.diagnostics[0].path, "files[2].path");
    assert!(ordered.diagnostics[0].message.contains("src/stale.rs"));
}

#[test]
fn sidecar_annotations_resolve_to_exact_minimal_anchors() {
    let files = vec![annotated_file()];
    let context = AgentContext {
        schema: Some("shore.agent-context".to_owned()),
        version: 1,
        summary: None,
        ownership: None,
        files: vec![AgentFileContext {
            path: "src/lib.rs".to_owned(),
            old_path: None,
            summary: None,
            annotations: vec![
                annotation("added row", None, Some(Range { start: 2, end: 2 })),
                annotation("removed row", Some(Range { start: 2, end: 2 }), None),
                annotation("context row", None, Some(Range { start: 3, end: 3 })),
                annotation("multi-line range", None, Some(Range { start: 3, end: 4 })),
            ],
        }],
    };

    let resolved = resolve_annotations(&files, &context);

    assert!(
        resolved.diagnostics.is_empty(),
        "{:#?}",
        resolved.diagnostics
    );
    assert_eq!(resolved.annotations.len(), 4);

    assert_annotation(
        annotation_by_summary(&resolved.annotations, "added row"),
        Side::New,
        LineRange::new(2, 2),
        "sha256:569dd3149acd6f05a7736e6ff2e3aed60f472171aeb74a3cc43c6e6813ca8c8c",
    );
    assert_annotation(
        annotation_by_summary(&resolved.annotations, "removed row"),
        Side::Old,
        LineRange::new(2, 2),
        "sha256:489f4336b2747c25479b8e1409c076c44baf968183ec052ade602214795fdde9",
    );
    assert_annotation(
        annotation_by_summary(&resolved.annotations, "context row"),
        Side::New,
        LineRange::new(3, 3),
        "sha256:c0e9a8fcaa59a634469252d037aba2004dfdb824b565c72102f58dba2a4134d0",
    );
    assert_annotation(
        annotation_by_summary(&resolved.annotations, "multi-line range"),
        Side::New,
        LineRange::new(3, 4),
        "sha256:7dfd8a717636663d5e7bc9b988060d22ad6fea1ca6e8034cda845e304f67c633",
    );
}

#[test]
fn unresolved_sidecar_annotations_return_diagnostics() {
    let files = vec![annotated_file()];
    let context = AgentContext {
        schema: Some("shore.agent-context".to_owned()),
        version: 1,
        summary: None,
        ownership: None,
        files: vec![AgentFileContext {
            path: "src/lib.rs".to_owned(),
            old_path: None,
            summary: None,
            annotations: vec![annotation(
                "out of range",
                None,
                Some(Range { start: 99, end: 99 }),
            )],
        }],
    };

    let resolved = resolve_annotations(&files, &context);

    assert!(resolved.annotations.is_empty());
    assert_eq!(resolved.diagnostics.len(), 1);
    assert_eq!(resolved.diagnostics[0].level, DiagnosticLevel::Warning);
    assert_eq!(
        resolved.diagnostics[0].code,
        DiagnosticCode::UnresolvedAnnotation
    );
    assert_eq!(
        resolved.diagnostics[0].path,
        "files[0].annotations[0].newRange"
    );
}

fn assert_annotation(
    annotation: &shore::model::Annotation,
    side: Side,
    line_range: LineRange,
    target_text_hash: &str,
) {
    assert_eq!(annotation.source, AnnotationSource::Sidecar);
    assert_eq!(annotation.anchor.file_id, FileId::new("src/lib.rs"));
    assert_eq!(annotation.anchor.side, side);
    assert_eq!(annotation.anchor.line_range, line_range);
    assert_eq!(annotation.anchor.status, ResolutionStatus::Exact);
    assert_eq!(
        annotation.anchor.hunk_signature,
        "sha256:0ed5a197d3bd2107a7250d2e36c33328dcf6c135e84bec170f342e22397a64f7"
    );
    assert_eq!(annotation.anchor.target_text_hash, target_text_hash);
}

fn annotation_by_summary<'a>(
    annotations: &'a [shore::model::Annotation],
    summary: &str,
) -> &'a shore::model::Annotation {
    annotations
        .iter()
        .find(|annotation| annotation.summary == summary)
        .expect("annotation exists")
}

fn annotation(
    summary: &str,
    old_range: Option<Range>,
    new_range: Option<Range>,
) -> AgentAnnotation {
    AgentAnnotation {
        id: Some(format!("annotation-{summary}")),
        old_range,
        new_range,
        summary: Some(summary.to_owned()),
        rationale: Some(format!("rationale for {summary}")),
        tags: vec!["fixture".to_owned()],
        confidence: Some("high".to_owned()),
        source: Some("test".to_owned()),
        author: Some("codex".to_owned()),
        created_at: Some("2026-05-08T00:00:00Z".to_owned()),
    }
}

fn annotated_file() -> DiffFile {
    DiffFile {
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
        hunks: vec![ReviewHunk {
            id: HunkId::new("src/lib.rs:1:1"),
            header: "@@ -1,4 +1,5 @@".to_owned(),
            old_start: 1,
            old_lines: 4,
            new_start: 1,
            new_lines: 5,
            rows: vec![
                context_row(1, 1, "fn main() {"),
                removed_row(2, "    old_call();"),
                added_row(2, "    new_call();"),
                context_row(3, 3, "    keep();"),
                added_row(4, "    extra();"),
                context_row(4, 5, "}"),
            ],
        }],
    }
}

fn context_row(old_line: u32, new_line: u32, text: &str) -> DiffRow {
    DiffRow {
        kind: DiffRowKind::Context,
        old_line: Some(old_line),
        new_line: Some(new_line),
        text: text.to_owned(),
    }
}

fn added_row(new_line: u32, text: &str) -> DiffRow {
    DiffRow {
        kind: DiffRowKind::Added,
        old_line: None,
        new_line: Some(new_line),
        text: text.to_owned(),
    }
}

fn removed_row(old_line: u32, text: &str) -> DiffRow {
    DiffRow {
        kind: DiffRowKind::Removed,
        old_line: Some(old_line),
        new_line: None,
        text: text.to_owned(),
    }
}

fn sidecar_file(path: &str, old_path: Option<&str>) -> AgentFileContext {
    AgentFileContext {
        path: path.to_owned(),
        old_path: old_path.map(str::to_owned),
        summary: None,
        annotations: Vec::new(),
    }
}

fn modified_file(path: &str) -> DiffFile {
    DiffFile {
        id: FileId::new(path),
        status: FileStatus::Modified,
        old_path: Some(path.to_owned()),
        new_path: Some(path.to_owned()),
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
        hunks: Vec::new(),
    }
}

fn renamed_file(old_path: &str, new_path: &str) -> DiffFile {
    DiffFile {
        id: FileId::new(new_path),
        status: FileStatus::Renamed,
        old_path: Some(old_path.to_owned()),
        new_path: Some(new_path.to_owned()),
        old_mode: None,
        new_mode: None,
        old_oid: None,
        new_oid: None,
        similarity: Some(92),
        is_binary: false,
        is_submodule: false,
        is_mode_only: false,
        synthetic: false,
        metadata_rows: Vec::new(),
        hunks: Vec::new(),
    }
}
