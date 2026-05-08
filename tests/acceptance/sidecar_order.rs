use shore::model::{DiffFile, FileId, FileStatus};
use shore::sidecar::agent_context::{
    AgentContext, AgentFileContext, DiagnosticCode, DiagnosticLevel, Range, apply_file_order,
    parse_agent_context,
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
