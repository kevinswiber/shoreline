# ADR-0027: At-Rest Encryption Boundaries — Decided Before Building

**Status:** Accepted (owner-approved 2026-07-01); decision-only — no encryption is implemented by
this ADR, and none is scheduled until a real demand for compressed or encrypted storage exists.
**Date:** 2026-07-01
**See also:** **ADR-0016** (content-targeted removal — §5 two-phase remove/compact, §6 convergence,
and the "Removal Convergence, Reference Stability, and Erasure Inventory" amendment whose pack and
read-index invariants this ADR restates as encryption boundaries), **ADR-0020** (durable-storage
backend seam — the byte traits below the wrapper; O3: the content address is the decoded-content
sha256, so `contentEncoding` sits below the identity layer and composes without re-keying; D3: the
compact re-hash floor is wrapper logic), **ADR-0004**, amendment "Opaque-Coded Signed Identity, the
View-Upcast Seam, and the Storage Descriptor" (the `contentEncoding` storage descriptor this ADR
builds on: an ordered, hash-excluded content-coding token list, reversed on read, encryption
outermost, envelope plaintext). Grounding issue: **#214**.

## Context

Shoreline stores everything as plaintext today. The storage codec is global and implicit
(`serde_json` at the wrapper layer, e.g. `src/session/store/event_store.rs`); `src/keys/` holds
Ed25519 **signing** keys only (`~/.shore/keys`); erasure is plaintext whole-blob deletion. No
requirement for at-rest encryption or compression exists yet — but the boundaries interact with
content-targeted removal (ADR-0016) and content-addressed convergence, and are expensive to
retrofit, so they are decided now, before anything is built.

The storage layout this ADR binds to:

- **Journal event records** — one JSON file per event under `events/`, behind the append-only
  `Journal` trait (`src/session/store/backend/mod.rs`), which has **no remove**: content removal
  targets the content store, never the journal.
- **Content-store blobs** — per-blob files (`artifacts/objects/<hash>.json` object artifacts,
  `artifacts/notes/<hash>.json` note bodies) behind `ContentStore`, whose `remove` is a plain
  per-file unlink; the re-hash-before-unlink floor lives in the wrapper above it
  (`src/session/workflow/artifact_removal/mod.rs`), and today counts **any decode failure as a
  hash mismatch** (`SweepOutcome::HashMismatchSkipped`).
- **Identity digests are already computed over decoded content, never stored bytes**: the object
  artifact hashes its typed `{schema, version, snapshot}` body (`object_artifact.rs`), a note body
  hashes the decoded body string (`body_artifact.rs`), and every event digest is built from the
  typed event.

A cautionary prior-art reference (from #214): KurrentDB's redaction refuses to rewrite a
non-identity-transform (encrypted) chunk — encryption implemented without deciding the erasure
boundary first can make physical erasure *harder*. These decisions exist to make the opposite
true here.

## Decision

### D1 — Erasure stays whole-blob delete; per-blob granularity is load-bearing

Erasure remains the deletion of one whole content-addressed file via `ContentStore::remove`,
codec-independent. Three standing invariants are restated here as encryption boundaries:

1. **No pack/segment files.** Coalescing blobs into a shared physical file would re-import the
   decrypt-and-rewrite problem (erasing one blob means rewriting an encrypted segment). Per-blob
   files are what make deletion codec-independent. (ADR-0016's erasure-inventory amendment already
   bans durable references to pack offsets; this ADR makes the no-packs rule an encryption
   precondition.)
2. **The read-index never holds blob bytes.** A secondary read-index caches positions/presence
   only, and the erasure sweep enumerates **physical artifact directories, never the index** — a
   stale or deduplicating index must not be able to hide a physical copy from erasure.
3. **The journal is not an erasure surface** (D4).

### D2 — Sign-then-encrypt; identity hashes are over decoded plaintext

Recorded as a forward-binding invariant (it is already how the code behaves): every content hash,
event digest, and signature is computed over the **fully decoded canonical content**, never over
stored (possibly encoded) bytes. Encryption, when it arrives, is an **outermost content-coding**
in the `contentEncoding` pipeline — applied after signing/hashing on write, reversed before
verifying/hashing on read.

Consequences that make this the only workable order:

- **Convergence is codec-independent.** Two stores encrypting the same object under different keys
  produce different ciphertext but the same decoded-content hash; `contentEncoding` differs and is
  hash-excluded; `contentHash` converges. ADR-0016 §6 survives untouched: `ArtifactRemoved` keys on
  the plaintext content hash, so two peers removing the same content still emit byte-identical
  removal facts.
- **Re-encoding is fork-free.** Recompression and encryption-key rotation never re-key the store.

The inverse order (encrypt-then-sign, integrity over ciphertext) would bind identity to per-store
ciphertext and break content-addressed convergence and dedup; it is rejected below.

### D3 — Crypto-shred per content-hash; key-shred is the erasure primitive

If keys are ever dropped to achieve erasure, keying is **per content-hash** (or per coherent
removable set), held in a **new keystore namespace** for symmetric content keys — never a
store-wide master key, which would make crypto-shred all-or-nothing and useless for
content-targeted removal.

Under encryption, the erasure semantics of ADR-0016 §5 sharpen: **the key-shred is the point of no
return; the ciphertext unlink becomes best-effort cleanup.** A blob whose key is destroyed is
already unrecoverable noise; confidentiality is achieved when the key dies, not when the file
disappears.

This forces one concrete change to the compact sweep's protection floor. Today the floor re-reads
a blob, re-derives its content hash, and refuses to erase on any mismatch — and any decode failure
counts as a mismatch. With an encrypted blob, re-hashing requires decoding, which requires the key;
a blob whose key was deliberately shredded can never decode, would be misreported as corruption
drift, and its ciphertext would survive forever. So the floor gains a **third outcome**:

- `matches` → erase-eligible, unlink;
- `mismatch` → corruption/tamper drift, withhold from erasure (unchanged);
- **`shredded`** → decode failed because the content key was **deliberately destroyed**, per an
  **authoritative shred record** — report as shredded (not drift) and allow the ciphertext cleanup.

The classification must be sound in one direction above all: a genuinely corrupt, never-shredded
blob must never be classifiable as "shredded, safe to reclaim." Proving that — including that a
shredded key id yields a clean typed "key shredded" error rather than a generic decode failure,
and choosing the shred record's shape (the removal fact, a new fact, or a local ledger — noting
that ADR-0016 deliberately has `compact` emit no event) — is the **prototype obligation** attached
to the first real encryption demand. This ADR decides the ordering rule; it does not claim the
classification is built.

### D4 — The journal is out of scope for erasure

The journal is append-only with no remove; crypto-shred cannot erase what cannot be removed.
At-rest encryption of event payloads would therefore buy **confidentiality only, no erasure** —
and no journal encryption is planned: the confidentiality need, if it arrives, is expected to be
served by content-store encryption plus the metadata-redaction work ADR-0016 defers (Tier-2),
which remains the home for erasing event-borne sensitive bytes (inline note bodies under the
materialization threshold, file paths, provenance fields). Content erasure and crypto-shred apply
to the two content-store blob classes only.

### D5 — Cross-store blob transport carries decoded content, never per-store ciphertext

`contentEncoding` is a **per-record/per-blob**, hash-excluded storage descriptor (per the ADR-0004
amendment); what is **store-local** is the *choice* of encodings, their in-band frame parameters,
and any content-key material. A transport that moved opaque ciphertext would therefore hand the
receiver bytes it cannot decode or re-verify. Any cross-store blob path must carry decoded
content — or the content plus a shared key — so the receiver can verify the plaintext hash and
apply its own storage encoding.

One such path already exists: **store bundle export/import**
(`src/session/store/bundle.rs` — `import_store_bundle_with_verification` reads source artifacts
and commits their bytes into the target store). Today both sides are plaintext, so the byte copy
is trivially compliant; when `contentEncoding`/encryption lands, bundle transport must **decode
and verify the source content and re-encode under the target store's policy**, never copy stored
ciphertext. The relay currently moves documented CLI JSON, not raw blobs; a future sealed
artifact-fetch plane inherits the same rule.

## Consequences

### Accepted

- Encryption can be added later as an additive, outermost content-coding with **no store break**:
  identity, convergence, dedup, and removal semantics are all pinned independent of the codec.
- Content-targeted erasure keeps working under encryption, with the erasure primitive shifting
  from the unlink to the key-shred — decided now so the sweep's protection floor is extended (a
  third outcome) rather than silently misclassifying shredded blobs as corruption.
- The cost accepted: per-content-hash keys are real machinery (derivation, storage, atomic shred,
  shred bookkeeping) deferred with its prototype; until then this ADR constrains future designs
  without shipping code.

### Rejected

- **A store-wide master key** — makes crypto-shred all-or-nothing; useless for content-targeted
  removal.
- **Encrypt-then-sign / identity over ciphertext** — binds identity to per-store bytes; breaks
  content-addressed convergence, dedup, and the convergent removal fact; also KurrentDB's
  integrity-over-ciphertext posture, consciously inverted here.
- **Pack/segment files** — erasing one blob would mean decrypting and rewriting a segment;
  whole-blob delete is the boundary that keeps erasure codec-independent.
- **In-place rewrite as an erasure mechanism** — same objection; erasure is delete (or key-shred),
  never rewrite.
- **Treating every decode failure as corruption once encryption exists** — without the `shredded`
  outcome, deliberate crypto-shred is misreported as drift and ciphertext is never reclaimed.
- **Enumerating erasure targets from the read-index** — a stale/deduplicating index could hide a
  physical copy; the sweep walks disk.
- **Journal encryption as an erasure story** — append-only storage cannot erase; claiming
  otherwise would misstate what encryption buys there.

## Revisit Triggers

- **A real demand for compressed or encrypted storage arrives** → fires the deferred prototype
  before any encryption ships: prove the shred-vs-corruption classification is sound (a corrupt
  blob can never masquerade as shredded), that a shredded key id surfaces as a typed "key shredded"
  error, that `shore store remove`/`gc` keep working unchanged with per-content-hash keys +
  whole-blob delete, and that plaintext-hash convergence holds end to end; choose the shred-record
  shape (reconciling with ADR-0016's compact-emits-no-event stance); and measure whether
  per-content-hash key volume warrants per-removable-set granularity.
- **Any pack/segment or blob-coalescing proposal** → blocked by D1 unless it preserves per-blob
  erasure; reopen here first.
- **A journal-confidentiality demand appears** → a separate decision; D4 records that encryption
  there buys no erasure, and Tier-2 metadata redaction (ADR-0016, deferred) is the erasure home
  for event-borne bytes.
- **The prototype lands** → decide then whether the key-shred point-of-no-return refinement is
  also recorded as an ADR-0016 §5 amendment, or stands here alone.
