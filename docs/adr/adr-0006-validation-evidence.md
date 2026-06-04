# ADR-0006: Validation Evidence

**Status:** Accepted
**Date:** 2026-06-04
**See also:** [ADR-0004](./adr-0004-event-signatures.md),
[ADR-0005](./adr-0005-review-unit-lineage.md)

## Context

Reviewers and agents often run local checks after capturing a ReviewUnit. Those checks are useful
review evidence, but without a durable event family they remain conversational notes or ephemeral
terminal output. Shoreline needs a local, network-free way to record completed check facts against
the exact captured ReviewUnit they evaluated.

Validation evidence must not become an approval gate. Review acceptance remains a review assessment,
and merge or write authority belongs outside this evidence family.

## Decision

Add a review-domain event family named `validation_check_recorded`. Each event records a completed
check's facts, including check identity, status, trigger, optional source fingerprint, timing, and
optional summary or log references. The event attaches to one exact captured ReviewUnit.

Validation evidence is advisory. Read surfaces may display it on ReviewUnit views, validation lists,
and history, but it never changes `currentAssessment`, assessment ambiguity, operative
input-request counts, review acceptance, merge authority, or write authority.

Validation targets and stable identity fields are path-free. They carry opaque content-addressed IDs
such as `reviewUnitId`, `trackId`, and `validationCheckId`; they do not derive from worktree paths,
raw `.git` layout, raw `.shore` paths, clone-local store paths, raw artifact paths, or
machine-local route names.

Validation summaries use the existing inline-or-artifact text-body policy. Short summaries may be
stored inline; larger summaries are externalized to `artifacts/notes/<sha256(body)>.json` with the
`shore.note-body` envelope. Large logs and reports are referenced by `sha256:<hex>` content hashes
only and are not inlined in validation events.

Validation events remain signable under ADR-0004's generic `EventToBeSigned` contract. This family
adds no per-family signing code, signing payload type, or `sigVersion`. Lineage from ADR-0005 may
help callers select a ReviewUnit head before writing validation evidence, but the recorded evidence
still targets the exact captured ReviewUnit.

## Consequences

### Accepted

- Validation evidence is durable and replayable like other ReviewUnit ledger facts.
- Evidence can support reviewer judgment without being treated as acceptance or merge authority.
- Path-free identity keeps events portable across local stores and future forwarding paths.
- Content-hash log references avoid unbounded event payload growth and keep sensitive log bodies out
  of small status records.
- Generic signing coverage continues to apply to this event family.

### Rejected

- Treating validation status as review acceptance.
- Adding a broad path-bearing validation target.
- Inlining large logs or reports in event payloads.
- Adding a validation-specific signing payload type or signature version.

## Revisit Triggers

Reopen this ADR if future runner orchestration needs request/status lifecycle events, if validation
scope must intentionally include path-bearing targets, or if remote log retrieval promotes log
content hashes into a stable fetch contract.
