use std::path::Path;

#[test]
fn agent_skills_and_docs_adopt_validation_evidence_workflow() {
    assert_contains(
        "skills/shoreline-author/SKILL.md",
        "shore review validation add",
    );
    assert_contains(
        "skills/shoreline-author/SKILL.md",
        "shore review validation list",
    );
    assert_contains(
        "skills/shoreline-reviewer/SKILL.md",
        "shore review validation list",
    );
    assert_contains(
        "skills/shoreline-reviewer/SKILL.md",
        "shore review validation add",
    );
    assert_contains(
        "skills/shoreline-author-response/SKILL.md",
        "shore review validation list",
    );
    assert_contains("docs/agent-authoring.md", "shore review validation add");
    assert_contains("docs/agent-authoring.md", "shore review validation list");
    assert_contains("skills/README.md", "validation evidence");
}

fn assert_contains(relative_path: &str, needle: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {relative_path}: {error}"));

    assert!(
        contents.contains(needle),
        "{relative_path} should contain {needle:?}"
    );
}
