use shore::sidecar::agent_context::{DiagnosticCode, DiagnosticLevel, Range, parse_agent_context};

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
