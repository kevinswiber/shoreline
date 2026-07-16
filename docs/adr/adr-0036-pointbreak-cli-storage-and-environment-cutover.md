# ADR-0036: Pointbreak CLI, Storage, and Environment Cutover

**Status:** Accepted (owner-approved 2026-07-15).
**Date:** 2026-07-15
**See also:** **ADR-0031** (the accepted flat review-surface grammar), **ADR-0032** (emitted
`pointbreak.*` documents and the at-rest naming boundary), **ADR-0029** (machine-output contract),
and **ADR-0004** (event-signature inputs).

## Context

The package, crate, repository, and emitted machine-document namespace are already named
Pointbreak, but the installed executable is still `shore` (`Cargo.toml:32`), the Clap identity and
test harness invoke `shore` (`src/cli/mod.rs:41`, `tests/support/mod.rs:15`), runtime environment
selectors use `SHORE_*`, and operational state lives under `.shore`, Git-common-dir `shore` paths,
and Shore-named user homes (`src/shore_home.rs:8`, `src/session/store/store_config.rs:23`). The
extension already uses the exact `pointbreak.version` v1 handshake but currently accepts CLI minor
`0.6` (`extensions/vscode/src/cli.ts:440`).

The earlier split was useful while a separate retired product occupied the `pointbreak` executable
name. That product is no longer part of the Review compatibility boundary. Pointbreak Review is
pre-1.0 and can make one coordinated breaking release before operational names become a larger
installed contract.

Operational naming and persisted identity have different physics. Paths, environment variables,
binary names, and prospective producer metadata can change without reinterpreting existing records.
Raw `shore.*` schemas, digest domains, signed payloads, stable IDs, and historical producer values
cannot: several are direct signature or identity inputs, including the event signing payload
(`src/session/event/tbs.rs:11`), worktree/revision/object fingerprints
(`src/session/store/fingerprint.rs:15`, `src/session/store/fingerprint.rs:17`, and
`src/session/store/fingerprint.rs:439`), event-set projection freshness
(`src/session/projection/freshness.rs:7`), and the historical producer string
(`src/session/identity/writer.rs:108`). ADR-0032 already distinguishes emitted Pointbreak documents
from frozen at-rest protocol identifiers, while the current emitted version document is
`pointbreak.version` v1 (`src/documents/version.rs:7`). A lexical cleanup of those bytes would
invalidate existing evidence.

## Decision

### 1. Release 0.7.0 is one hard operational cutover

Release `0.7.0` ships the complete executable, path, and environment cutover together. There is no
multi-release compatibility calendar:

| Release | Executable | Runtime paths | Product environment |
| --- | --- | --- | --- |
| `0.6.x` and earlier | `shore` | Shore-named operational paths | `SHORE_*` |
| `0.7.0` and later | `pointbreak` only | Pointbreak-named operational paths only | `POINTBREAK_*` only |

The sole Cargo target and runtime executable is `pointbreak` (`pointbreak.exe` on Windows). There is
no `shore` alias, wrapper, symlink, copied binary, second Cargo target, or forwarding shim. The flat
ADR-0031 grammar mounts directly beneath `pointbreak`; this decision does not add a `review` prefix
or redesign existing nouns and arguments.

### 2. Runtime storage placement is Pointbreak-only

Canonical operational placement is:

| Concern | Canonical placement |
| --- | --- |
| Repository config | `<repo>/.pointbreak/` |
| Ephemeral repository data | `<repo>/.pointbreak/data/` |
| Clone-family/common data | `<git-common-dir>/pointbreak/` |
| Family binding filename | `<git-common-dir>/pointbreak.link.json` |
| Explicit user root | `$POINTBREAK_HOME` |
| Unix XDG default | `$XDG_DATA_HOME/pointbreak` |
| Unix fallback | `$HOME/.pointbreak` |
| Windows default | `%APPDATA%\pointbreak` |

Runtime code does not inventory, classify, read, merge, warn about, or fall back to `.shore`,
Git-common-dir `shore`, `shore.link.json`, or Shore-named default homes. The superseded historical
`.shoreline` design is also never probed. There is one authority because only the Pointbreak
namespace participates in resolution.

Existing installations move their operational directories and binding filename before using
`0.7.0`. The move preserves the contents of persisted configuration and binding documents; their
raw `shore.*` schema identifiers remain frozen under Decision 6.

### 3. Product environment input is POINTBREAK_* only

The product-owned runtime variables are:

- `POINTBREAK_HOME`
- `POINTBREAK_ACTOR_ID`
- `POINTBREAK_SIGNING`
- `POINTBREAK_SIGNING_KEY`
- `POINTBREAK_FORMAT`
- `POINTBREAK_THEME`
- `POINTBREAK_LOG`
- `POINTBREAK_BACKEND`
- `POINTBREAK_PERF`

The corresponding `SHORE_*` names are not aliases and are not read or compared. Explicit CLI
options retain their existing precedence. Each Pointbreak variable preserves the old selector's
empty and non-Unicode behavior unless a separate decision changes that behavior. Developer-only
benchmark inputs become `POINTBREAK_BENCH_*` without aliases. Third-party inputs such as
`NO_COLOR`, `BAT_THEME`, and `RUST_LOG` are unchanged.

### 4. Pointbreak exposes paths, but does not migrate them

The read-only command is:

```text
pointbreak store paths --repo <path> --format json
```

Its machine document is `pointbreak.store-paths` version 1. It reports the resolved tier and exact
worktree store, common store, binding, home, and key paths. It is a supported shell/consumer seam;
existing machine-document schemas, versions, and registry members remain unchanged. Registering
`pointbreak.store-paths` v1 adds exactly that one member to the `pointbreak.version` v1 `documents`
map. The map's existing members and every other field remain unchanged; this is the sole approved
delta to an existing emitted document body.

Pointbreak does not ship a path-migration command, an automatic old-path detector, a migration
document, a migration lock, a user acknowledgement flag, source-retirement workflow, or dual
reads/writes. Existing single-owner state is moved offline while Review writers are stopped.
Pre/post path-size-SHA-256 manifests and readback with the exact `0.7.0` candidate are release
evidence, not product behavior. Rollback is the inverse filesystem move plus the prior executable.

### 5. Distribution and the extension use only pointbreak

Release archives, checksums, Cargo installation, generated help/completions, and the VS Code bundle
contain only `pointbreak[.exe]`. Installers stage and verify the exact candidate's
`pointbreak.version` v1 document before atomically replacing the `pointbreak` destination.

Installers do not inspect, identify, rename, or remove a neighboring `shore[.exe]`. Cleanup of an
old executable is an owner operation, not installer behavior. Pointbreak Debug receives no probe,
replacement flag, state/config migration, MCP conversion, or compatibility classifier; ordinary
replacement of a destination already named `pointbreak` is sufficient.

The extension compatible CLI minor becomes `0.7`. Bundled and PATH fallback resolve only
`pointbreak`; an explicitly configured path remains basename-agnostic but must satisfy the exact
`pointbreak.version` v1 and required-document handshake. Extension child processes sanitize the
canonical `POINTBREAK_ACTOR_ID` and `POINTBREAK_FORMAT` inputs.

### 6. Persisted shore.* identities and historical bytes remain frozen

The word Shore may appear publicly only when necessary to identify a pre-`0.7.0` operational input
or a stable persisted protocol identifier. It is not a current product name.

The historical/frozen allowlist includes:

- `shore.event`, its event-type vocabulary, and
  `application/vnd.shore.event-tbs.v1+json`;
- `shore.object`, object-identity, revision-identity, worktree-fingerprint, and related digest
  domains;
- `shore.event-set.v1`, `shore.state`, and note-body schemas;
- store-config, store-link, family, registry, actor-attribute, sensitivity, and export-manifest
  schemas;
- canonical TBS and PAE bytes, signature envelopes, event-record hashes, stable object/revision/event
  IDs, freshness values, and golden fixtures;
- historical `producer.name = "shore"` values and captured output;
- accepted ADR and changelog history, screenshot locks/assets, temporary `.shore-write` mechanics,
  fixture provenance, and nested historical source snapshots.

Existing emitted `pointbreak.*` documents keep their schema names, versions, and bodies except for
the additive `pointbreak.store-paths` registry member authorized by Decision 4. New native records
use producer name `pointbreak` prospectively; loaded historical records retain `shore`. Producer
metadata remains outside signed TBS/PAE and freshness identity while participating in new
event-record hashing exactly as the current schema defines.

### 7. Permission promises do not expand

Canonical store creation and installer replacement preserve the permissions currently supported on
each platform. This decision adds no cross-platform path-migration API and therefore makes no claim
that a Unix mode is byte-portable as a Windows ACL. Owner-operated moves must not intentionally
broaden access, but they are not a new Pointbreak ACL contract.

### 8. Retired documentation remains retired

The retired `docs.withpointbreak.com` host remains unavailable. Living documentation labels that
material archived/retired where needed. A redirect requires a separately identified owner and is not
created by this cutover.

## Consequences

### Accepted

- The package, executable, machine documents, paths, environment, release assets, extension, and
  maintained operational guidance share one product name after one pre-1.0 release.
- Runtime resolution becomes simpler: there is no compatibility pair, old/new marker classifier,
  two-authority state, or product migration workflow.
- Existing installations must move local state and update environment/config before using `0.7.0`.
  This is an intentional breaking cost accepted while the product has one owner/operator.
- Old `shore` binaries may remain on disk until the owner removes them; official artifacts never
  expose them as supported entry points.
- Persisted evidence remains readable and verifiable because identity-bearing bytes are not renamed.

### Rejected

- **A `shore` executable alias or warning shim:** it creates a second permanent entry point and
  weakens the hard boundary.
- **Path fallback or `SHORE_*` aliases:** they preserve hidden split authority and require another
  future retirement release.
- **A general-purpose migration command:** unnecessary for the current ownership model and much
  larger than an offline verified move.
- **Automatic old-binary cleanup:** installer ownership is limited to `pointbreak`; a basename or
  machine response is not permission to delete another file.
- **Renaming persisted `shore.*` bytes:** it breaks signatures, digests, IDs, provenance, and
  interoperability for no operational benefit.
- **Debug-specific handling:** the retired product is outside the Review system boundary.

## Revisit Triggers

- A future supported user base requires a migration utility for a new storage transition. That
  utility is designed for the then-current transition; it does not retroactively add `shore`
  fallback to `0.7.0`.
- The base-directory policy changes independently of naming and needs its own platform-specific
  migration decision.
- A store-format break is required for reasons other than branding. Frozen `shore.*` identities may
  be reconsidered only as part of that explicit format/signature/identity decision.
- The distribution model gains an authoritative package manager capable of safely owning obsolete
  installed targets. Until then, installers manage only `pointbreak`.
