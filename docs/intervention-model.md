# Intervention Model

## Status

V1 has a local durable intervention ledger. Shore can record `input_request_opened` events,
append `input_request_responded` events, and expose polling read surfaces through
`shore review input-request list` and `shore review input-request fetch`.

This document remains architecture guidance for the model around that V1 surface. Prompt delivery,
watch mode, cancellation, escalation, daemon behavior, and UI prompts are deferred.

## Goal

Shore needs a durable way to represent moments where normal review flow should pause, ask for a
decision, surface an escalation, or record that outside input changed the path forward.

Do not call this "human-in-the-loop" in the core model. The actor may be a human, reviewer, monitor
process, automated tool, cloud worker, or another Shore client. The model should describe the
workflow fact, not assume who resolves it.

## Core Terms

- **Intervention:** a durable request for attention, decision, override, or explicit response.
- **Blocking intervention:** an intervention that should stop a cooperative client before it
  continues a workflow step such as capturing review state, applying notes, pushing, or
  mutating state.
- **Advisory intervention:** an intervention that should be visible but does not block progress.
- **Resolution:** the durable answer to an intervention, such as approved, rejected, dismissed,
  superseded, or resolved by a later event.
- **Cancellation:** withdrawal of an intervention request before a decision is made, usually because
  the request was mistaken, superseded, or no longer relevant.
- **Escalation:** a higher-priority intervention, usually created because the original workflow
  cannot decide safely from local state.

## Architectural Principles

Interventions are durable events, not UI prompts. A TUI may render an intervention as a modal, a CLI
may print it to stderr, and a monitor may react in real time, but the durable model should be the
source of truth.

Interruption should be cooperative. Shore does not need to preempt another process mid-operation.
Clients should check durable state at safe boundaries and decide whether unresolved blocking
interventions require them to pause.

The model must support both real-time and polling clients:

- A monitor-style client can subscribe to stdout/stderr, filesystem notifications, or a future event
  stream and respond quickly.
- A turn-boundary client can poll at start, stop, or before risky operations.
- A cloud client can poll or receive backend-specific notifications without changing the event
  vocabulary.

The same durable event model should work for all three.

## Event Model

V1 intervention events use the same event envelope as other review/session state:

```text
input_request_opened
input_request_responded
```

Deferred event types may include:

```text
intervention_cancelled
intervention_escalated
```

`input_request_opened` records the durable request. The request has a stable input request ID, a
target reference, a required track, a blocking/advisory mode, a short title, an optional body, and a
structured `reasonCode`.

`input_request_responded` records a durable answer. The response has a stable response ID, targets
the input request, and carries an `outcome` such as approved, rejected, dismissed, superseded, or
abandoned. Response `outcome` is intentionally separate from request `reasonCode`: one describes
why the pause was requested, the other describes how it ended.

V1 response events keep the request event's review unit, revision, snapshot, and track context.
That anchors the decision to the captured material that caused the input request, not to whatever
worktree state happens to exist when the input request is answered.

Multiple different response events are preserved as append-only facts. Current V1 read surfaces
report that state as ambiguous rather than choosing a timestamp winner.

Duplicate events with the same semantic ID are different from multiple decisions. If a request is
written more than once with the same `inputRequestId`, `list` and `fetch` return one input request
and include a duplicate semantic diagnostic. If a response is written more than once with the same
`inputRequestResponseId`, `fetch` returns one response and keeps the input request `responded`.
Only distinct response IDs make an input request `ambiguous`.

Future `intervention_escalated` should target an existing intervention and change its routing or
urgency in the derived projection. It should not create a second intervention. If a separate decision
is needed, create another `input_request_opened` event with an explicit relationship to the first.

Future `intervention_cancelled` means the request was withdrawn without a decision. V1 expresses
cancellation-like outcomes through `dismissed`, `superseded`, or `abandoned` response outcomes.

Each event should carry:

- a stable input request ID and, for responses, a stable response ID
- target reference: ReviewUnit, file, range, observation, intervention, or event
- blocking/advisory mode
- request reason code
- short title
- body or structured details
- requesting actor or writer provenance
- responding actor or writer provenance, for response events
- timestamps using the same UTC timestamp policy as other Shore events
- idempotency key

Reason codes should stay workflow-oriented, not actor-oriented. Useful starting categories:

- `ambiguous_state`
- `unsafe_action`
- `stale_revision`
- `failed_gate`
- `external_side_effect`
- `conflicting_event`
- `missing_permission`
- `manual_decision_required`

The `blocking` mode is the control-flow signal. Urgency is advisory; it should not decide whether a
client may continue.

Interventions should not expire automatically. Clearing an unresolved intervention requires an
explicit `input_request_responded` event in V1, or a future `intervention_cancelled` event. A future
`expiresAt` field can be added if a concrete workflow needs advisory expiry, but it should not
silently unblock a client.

Every blocking intervention must have a defined exit event or escalation policy. That does not mean
blocking states should clear themselves on a timer. For review workflows, the external decision is
often the point. The requirement is that Shore can represent how the state ends: resolved,
cancelled, superseded, escalated, or explicitly abandoned.

Response events should preserve the audit trail even when the target is no longer live. For
example, an `input_request_responded` event targeting a closed work unit should still be recorded, but
any resume or apply action derived from it should be a no-op. Distinguish "the event happened" from
"the action still applies."

## V1 Commands And Derived State

The command surface is:

```bash
shore review input-request open
shore review input-request list
shore review input-request fetch
shore review input-request respond
```

The V1 read surface is polling-oriented. `list` and `fetch` replay `.shore/events/`; they do not
depend on `state.json` as authority. Bodies and response reasons may use internal
`shore.note-body` artifacts, but command output does not expose artifact paths.

`list` and `fetch` project semantic IDs, not raw event count. `idempotencyKey` decides whether a
write is the same event-file retry; `inputRequestId` and `inputRequestResponseId` decide whether
read output represents one logical request or response. Duplicate semantic IDs are preserved in
storage and reported through diagnostics rather than silently hidden.

Bounded `state.json` exposes only summary counters:

```text
inputRequestCount
openInputRequestCount
openBlockingInputRequestCount
```

A future fuller projection can expose:

```text
unresolved_input_requests
unresolved_blocking_input_requests
latest_input_request_event_id
```

A client should be able to ask:

- Are there unresolved blocking input requests for this work unit?
- Are there unresolved blocking input requests targeting the current revision?
- Has anything changed since my last event cursor?
- Which event or artifact caused the input request?

That implies Shore should eventually expose an `events_since(cursor)` style API or equivalent
cursor-based projection. V1 does not implement that API, but it should not choose a storage shape
that makes it awkward.

## Design Constraints For Local Durable State

The local durable-state model should preserve these future requirements:

- Use generic target references in event payloads rather than hard-coded single-target fields.
- Keep event IDs and idempotency keys stable enough for polling clients.
- Keep derived `state` rebuildable from durable events.
- Do not make terminal UI state the only place an interruption can live.
- Do not assume intervention actors are humans.
- Do not assume intervention delivery is real-time.
- Do not assume local filesystem notification is available.
- Do not require async storage yet, but avoid event semantics that depend on POSIX-only behavior such
  as atomic rename; remote backends may need conditional create, versioned writes, or transactions.
- Re-read target state before applying a resolution or resume action; stale targets should preserve
  the event but suppress the action.

Intervention transport is independent of review-exchange transport. An intervention is not a review
artifact, verdict, or review note. A future adapter may export or import intervention facts, but the
core model should keep them separate.

Native assessments may relate to input requests through `--related-input-request`, but that
relationship is evidence, not lifecycle. An assessment does not close an input request. Use
`shore review input-request respond` to append the explicit closure event.

## Non-Goals

This document does not require:

- a prompt system
- a daemon
- a notification service
- a lock or lease protocol
- a cloud backend
- a TUI modal
- note mutation
- a legacy decision command

Those may become useful later, but the architectural requirement is narrower: Shore's durable model
should be interruptible at safe workflow boundaries.
