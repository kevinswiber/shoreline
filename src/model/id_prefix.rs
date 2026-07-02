//! Internal registry of the prefix strings Shoreline mints into ids, artifact
//! references, and reserved tokens.
//!
//! Prefixes are opaque to consumers (see "IDs are opaque" in
//! `docs/review-workflow.md`); this registry exists for internal consistency
//! and discoverability. Prefix strings feed content-derived ids, so changing
//! any existing value changes the ids newly minted for identical content and
//! breaks cross-clone convergence with existing stores — that is an explicit
//! owner decision, recorded in `docs/adr/adr-0028-id-prefix-convention.md`.
//! The convention for new entries: spelled-out, hyphenated, lowercase domain
//! names; the abbreviation set below is closed.
//!
//! Production code consumes the constants; the `cfg(test)` table beneath them
//! is the enumerable contract the registry tests and the inspector drift test
//! check against (it is test-gated so the lib target carries no test-only
//! dead code — lift the gate if production ever needs to enumerate).

/// Event envelope ids: `evt:sha256:<hex>` (hash of the idempotency key).
pub(crate) const EVENT: &str = "evt";
/// Revision ids: `rev:sha256:<hex>` and the `rev:worktree:sha256:<hex>` variant.
pub(crate) const REVISION: &str = "rev";
/// Content-object ids: `obj:sha256:<hex>` and the `obj:git:sha256:<hex>` variant.
pub(crate) const OBJECT: &str = "obj";
/// Engagement grouping ids: `engagement:sha256:<hex>`.
pub(crate) const ENGAGEMENT: &str = "engagement";
/// Observation ids: `obs:sha256:<hex>`.
pub(crate) const OBSERVATION: &str = "obs";
/// Assessment ids: `assess:sha256:<hex>`.
pub(crate) const ASSESSMENT: &str = "assess";
/// Validation check ids: `validation:sha256:<hex>`.
pub(crate) const VALIDATION: &str = "validation";
/// Input request ids: `input-request:sha256:<hex>`.
pub(crate) const INPUT_REQUEST: &str = "input-request";
/// Input request response ids: `input-request-response:sha256:<hex>`.
pub(crate) const INPUT_REQUEST_RESPONSE: &str = "input-request-response";
/// Commit association ids: `assoc-commit:sha256:<hex>`.
pub(crate) const COMMIT_ASSOCIATION: &str = "assoc-commit";
/// Ref association ids: `assoc-ref:sha256:<hex>`.
pub(crate) const REF_ASSOCIATION: &str = "assoc-ref";
/// Commit withdrawal ids: `withdraw-commit:sha256:<hex>`.
pub(crate) const COMMIT_WITHDRAWAL: &str = "withdraw-commit";
/// Ref withdrawal ids: `withdraw-ref:sha256:<hex>`.
pub(crate) const REF_WITHDRAWAL: &str = "withdraw-ref";
/// Task attempt work-object ids: `task-attempt:sha256:<hex>` (adapter-minted).
pub(crate) const TASK_ATTEMPT: &str = "task-attempt";
/// Checkpoint ids: `checkpoint:sha256:<hex>` (adapter-minted).
pub(crate) const CHECKPOINT: &str = "checkpoint";
/// Review note ids, in three shapes: `note:sha256:<hex>` (imported content),
/// `note:<explicit_id>` (sidecar-supplied), `note:<file_index>:<note_index>`
/// (positional fallback).
pub(crate) const NOTE: &str = "note";
/// Journal ids: `journal:claude:<session_uuid>` and the `journal:default`
/// sentinel. Generic `JournalId::new(session)` callers pass strings through
/// as-is; this prefix covers the hardcoded mints.
pub(crate) const JOURNAL: &str = "journal";
/// Review ids: the `review:default` sentinel only — no content-derived shape.
pub(crate) const REVIEW: &str = "review";
/// Actor ids: `actor:git-email:<email>`, `actor:git-name:<name>`,
/// `actor:local`, `actor:claude_code:user`, `actor:claude_code:assistant`.
/// The `actor:agent:<name>` / `actor:env:<...>` shapes arrive from user
/// configuration and are validated, never minted here.
pub(crate) const ACTOR: &str = "actor";
/// Review-stream row ids: `row:<zero-padded ordinal>` (positional, not content).
pub(crate) const ROW: &str = "row";
/// Export/inventory artifact reference to an object artifact: `object:sha256:<hex>`.
pub(crate) const ARTIFACT_OBJECT: &str = "object";
/// Export artifact reference to a content body: `body:sha256:<hex>`.
pub(crate) const ARTIFACT_BODY: &str = "body";
/// Export/inventory artifact reference to a note body: `note-body:sha256:<hex>`.
pub(crate) const NOTE_BODY: &str = "note-body";
/// Redacted sensitive-path reference: `file:sha256:<hex>` (hash of the relative
/// path). Distinct from `FileId`, which is path-based and carries no prefix.
pub(crate) const REDACTED_FILE: &str = "file";
/// Event `occurredAt` wall-clock token: `unix-ms:<millis>`. Not an id.
pub(crate) const UNIX_MS: &str = "unix-ms";

/// What kind of string a registered prefix opens.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrefixKind {
    /// Content-derived id: `<prefix>:[infix:]sha256:<hex>`.
    ContentId,
    /// Structured, non-content id (positional, composite, or sentinel).
    StructuralId,
    /// Content-addressed artifact reference in export bundles and inventories.
    ArtifactRef,
    /// Reserved non-id token.
    Token,
}

/// One registered prefix: the enumerable row the registry tests check.
#[cfg(test)]
pub(crate) struct IdPrefix {
    pub(crate) prefix: &'static str,
    pub(crate) kind: PrefixKind,
    /// False only for legacy display entries that no production path mints.
    pub(crate) minted: bool,
}

/// Every prefix Shoreline has ever put in front of an id, artifact reference,
/// or reserved token — the single enumerable source. Content-id entries first,
/// then structural, artifact references, tokens, and the legacy display
/// entries last.
#[cfg(test)]
pub(crate) const ID_PREFIXES: &[IdPrefix] = &[
    IdPrefix {
        prefix: EVENT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: REVISION,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: OBJECT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: ENGAGEMENT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: OBSERVATION,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: ASSESSMENT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: VALIDATION,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: INPUT_REQUEST,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: INPUT_REQUEST_RESPONSE,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: COMMIT_ASSOCIATION,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: REF_ASSOCIATION,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: COMMIT_WITHDRAWAL,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: REF_WITHDRAWAL,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: TASK_ATTEMPT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: CHECKPOINT,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: NOTE,
        kind: PrefixKind::ContentId,
        minted: true,
    },
    IdPrefix {
        prefix: JOURNAL,
        kind: PrefixKind::StructuralId,
        minted: true,
    },
    IdPrefix {
        prefix: REVIEW,
        kind: PrefixKind::StructuralId,
        minted: true,
    },
    IdPrefix {
        prefix: ACTOR,
        kind: PrefixKind::StructuralId,
        minted: true,
    },
    IdPrefix {
        prefix: ROW,
        kind: PrefixKind::StructuralId,
        minted: true,
    },
    IdPrefix {
        prefix: ARTIFACT_OBJECT,
        kind: PrefixKind::ArtifactRef,
        minted: true,
    },
    IdPrefix {
        prefix: ARTIFACT_BODY,
        kind: PrefixKind::ArtifactRef,
        minted: true,
    },
    IdPrefix {
        prefix: NOTE_BODY,
        kind: PrefixKind::ArtifactRef,
        minted: true,
    },
    IdPrefix {
        prefix: REDACTED_FILE,
        kind: PrefixKind::ArtifactRef,
        minted: true,
    },
    IdPrefix {
        prefix: UNIX_MS,
        kind: PrefixKind::Token,
        minted: true,
    },
    IdPrefix {
        prefix: "review-unit",
        kind: PrefixKind::ContentId,
        minted: false,
    },
    IdPrefix {
        prefix: "snap",
        kind: PrefixKind::ContentId,
        minted: false,
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    // The ratification lock: every minted prefix string is frozen as-is (ADR-0028).
    // Changing a value here changes newly minted content ids and is an owner decision.
    #[test]
    fn prefix_constants_are_frozen() {
        assert_eq!(EVENT, "evt");
        assert_eq!(REVISION, "rev");
        assert_eq!(OBJECT, "obj");
        assert_eq!(ENGAGEMENT, "engagement");
        assert_eq!(OBSERVATION, "obs");
        assert_eq!(ASSESSMENT, "assess");
        assert_eq!(VALIDATION, "validation");
        assert_eq!(INPUT_REQUEST, "input-request");
        assert_eq!(INPUT_REQUEST_RESPONSE, "input-request-response");
        assert_eq!(COMMIT_ASSOCIATION, "assoc-commit");
        assert_eq!(REF_ASSOCIATION, "assoc-ref");
        assert_eq!(COMMIT_WITHDRAWAL, "withdraw-commit");
        assert_eq!(REF_WITHDRAWAL, "withdraw-ref");
        assert_eq!(TASK_ATTEMPT, "task-attempt");
        assert_eq!(CHECKPOINT, "checkpoint");
        assert_eq!(NOTE, "note");
        assert_eq!(JOURNAL, "journal");
        assert_eq!(REVIEW, "review");
        assert_eq!(ACTOR, "actor");
        assert_eq!(ROW, "row");
        assert_eq!(ARTIFACT_OBJECT, "object");
        assert_eq!(ARTIFACT_BODY, "body");
        assert_eq!(NOTE_BODY, "note-body");
        assert_eq!(REDACTED_FILE, "file");
        assert_eq!(UNIX_MS, "unix-ms");
    }

    #[test]
    fn registry_prefixes_are_unique() {
        let mut seen = HashSet::new();
        for entry in ID_PREFIXES {
            assert!(
                seen.insert(entry.prefix),
                "duplicate registry prefix: {}",
                entry.prefix
            );
        }
    }

    #[test]
    fn registry_prefixes_use_the_reserved_charset() {
        for entry in ID_PREFIXES {
            let mut chars = entry.prefix.chars();
            let first = chars.next().expect("prefix must be non-empty");
            assert!(
                first.is_ascii_lowercase(),
                "{} must open lowercase",
                entry.prefix
            );
            assert!(
                chars.all(|c| c.is_ascii_lowercase() || c == '-'),
                "{} must be lowercase letters and hyphens",
                entry.prefix
            );
            assert!(
                !entry.prefix.ends_with('-'),
                "{} must not end with a hyphen",
                entry.prefix
            );
        }
    }

    #[test]
    fn every_minted_constant_is_registered_exactly_once() {
        let minted = [
            EVENT,
            REVISION,
            OBJECT,
            ENGAGEMENT,
            OBSERVATION,
            ASSESSMENT,
            VALIDATION,
            INPUT_REQUEST,
            INPUT_REQUEST_RESPONSE,
            COMMIT_ASSOCIATION,
            REF_ASSOCIATION,
            COMMIT_WITHDRAWAL,
            REF_WITHDRAWAL,
            TASK_ATTEMPT,
            CHECKPOINT,
            NOTE,
            JOURNAL,
            REVIEW,
            ACTOR,
            ROW,
            ARTIFACT_OBJECT,
            ARTIFACT_BODY,
            NOTE_BODY,
            REDACTED_FILE,
            UNIX_MS,
        ];
        // 25 minted constants + the 2 legacy display literals.
        assert_eq!(ID_PREFIXES.len(), minted.len() + 2);
        for prefix in minted {
            let entry = ID_PREFIXES
                .iter()
                .find(|e| e.prefix == prefix)
                .unwrap_or_else(|| panic!("{prefix} missing from the table"));
            assert!(entry.minted, "{prefix} is a production-minted prefix");
            assert_eq!(
                ID_PREFIXES.iter().filter(|e| e.prefix == prefix).count(),
                1,
                "{prefix} must appear in the table exactly once"
            );
        }
    }

    #[test]
    fn registry_kind_partition_is_stable() {
        let count = |kind: PrefixKind| ID_PREFIXES.iter().filter(|e| e.kind == kind).count();
        // 16 minted content-id prefixes + the 2 legacy display entries.
        assert_eq!(count(PrefixKind::ContentId), 18);
        assert_eq!(count(PrefixKind::StructuralId), 4);
        assert_eq!(count(PrefixKind::ArtifactRef), 4);
        assert_eq!(count(PrefixKind::Token), 1);
    }

    #[test]
    fn legacy_entries_are_exactly_the_unminted_ones() {
        let legacy: Vec<&str> = ID_PREFIXES
            .iter()
            .filter(|e| !e.minted)
            .map(|e| e.prefix)
            .collect();
        assert_eq!(
            legacy,
            ["review-unit", "snap"],
            "minted: false is reserved for the legacy display entries"
        );
    }
}
