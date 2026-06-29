# ADR-0022: Inspector Front-End TypeScript Migration — Build, Serve, and Test Architecture

**Status:** Accepted
**Date:** 2026-06-28
**See also:** [ADR-0021](./adr-0021-inspector-server-side-projection-direction.md) (inspector server-side projection — establishes the "no bundler, no npm, no build step, no JS test harness" baseline this decision changes), **ADR-0003** (advisory-first), **ADR-0019** (blackboard liveness; pull-only localhost tool).

## Context

The inspector front-end is `include_str!`'d vanilla JS/CSS — no framework, no bundler, no npm, no build step, and no JS test harness (the baseline ADR-0021 records). The pain is concentrated in one file: `src/cli/inspect/assets/app.js` is **3219 lines, a single `"use strict"` global scope, ~200 functions, zero `import`/`export`**, untyped and unlinted. It builds HTML by template-literal concatenation guarded by a hand-rolled `escapeHtml`. The owner wants it on the same TypeScript + Biome paradigm used in the `mmdflux` packages.

Two hard constraints shape every option:

- **`include_str!` is compile-time** (`src/cli/inspect/server.rs:18-21` embeds the four assets; `:142-146` routes them). `include_str!("assets/app.js")` requires `app.js` to exist on disk at `cargo build` time.
- **`shoreline` publishes to crates.io** (`Cargo.toml`, no `publish = false`), so `cargo build` / `cargo install shoreline` / `cargo publish --locked` must stay **Node-free** for end users and from-source contributors. `Cargo.toml` `exclude` ships `src/cli/inspect/assets/**` (required for `include_str!`) and drops `src/cli/inspect/design-system/**`.

Together these mean a **committed, git-tracked `assets/app.js` is mandatory** in every viable option — a gitignored or release-only artifact breaks `git clone && cargo build` and is dropped by `cargo package`. The "generate-then-ignore" precedent of `bake.sh` (whose outputs are git-ignored) does **not** transfer.

The migration's biggest blocker is the test coupling: **~50 `#[test]` functions** across `tests/cli_inspect_*.rs` (plus 3 in `server.rs`) assert on `app.js` **source text** — literal identifiers, function signatures, and template fragments (`function closeDiff()`, `navigate({ diff: null, … })`, `class="dfile-head"`). Any emit step (tsc or esbuild) reflows those bytes and fails them. They are a known stand-in for the missing JS execution harness (issue **#276**). Crucially, only `app.js`-source greps break; the ~55 tests asserting `index.html`/`app.css`/`tokens.css`/JSON-API/gallery survive an emit untouched, because the TS build regenerates *only* `app.js`.

The CSS/token/design-system layer is well-kept and out of scope: the JS layer is a pure **sink** (`bake.sh` has zero `app.js` references; `app.js` reads no CSS/token/design-system file), so the migration leaves `tokens.css`, `app.css`, `bake.sh`, `styles.css`, and the gallery **byte-stable**.

## Decision

### 1. Author TypeScript; ship a single committed bundle

TypeScript modules live under a new `src/cli/inspect/web/` tree (`web/src/*.ts`, `web/package.json`, a committed `web/package-lock.json`, `web/tsconfig.json`, `web/biome.json`), **added to `Cargo.toml` `exclude`** (a sibling of `design-system/**`) so the sources, `package.json`, the lockfile, and `node_modules` are dropped from the published tarball. The build is **esbuild** to **one non-minified, `--keep-names`, IIFE** artifact committed at the unchanged path `src/cli/inspect/assets/app.js`. Because the path, the `<script src="/app.js">` tag (`index.html:108`), and the `include_str!` + route are unchanged, **`server.rs` is untouched**. Tooling mirrors mmdflux: **Biome 2.x** (recommended ruleset, double quotes, `organizeImports`) and a **strict `tsconfig`** (ES2022). Minification is prohibited — the committed artifact must stay reviewable/diffable.

### 2. Decompose `app.js` behind a `store.ts` commit/subscribe core

`app.js` splits into ~18–24 cohesive modules with an acyclic import graph. The state core is a **`store.ts` with `commit`/`subscribe`** (not a bare shared-`state` module, not full DI): `navigate()` commits to the store and `render` is the **single subscriber** registered once in `main.ts`, so the router no longer imports render. This severs the four real cycles with two moves — the store-subscribe breaks router↔render, render↔diff, and (with an `http.ts` split) data↔render; an **overlay manager that owns teardown** breaks diff↔palette↔help. Route state lives on the store; the ~12 transient view caches become module-local fields. The two `document` delegates (`keydown`→router, `data-ref-kind` click→`resolveRef`) stay; per-render timeline/card/type-toggle listeners migrate to a delegated `#master`/`#filter-types` handler; `wireDagInteractions` stays imperative.

### 3. A read-only CI freshness gate is the sole enforcement

Stale committed bundles are made impossible to merge by a **read-only CI freshness gate**: a single ubuntu Node job runs `biome check` + `tsc --noEmit` + the JS tests + `npm run build && git diff --exit-code src/cli/inspect/assets/app.js`, kept **off the Windows shard long pole**. It is fork-safe because it only reads/diffs (read-only token). **Contributors who touch the inspector rebuild and commit `app.js` themselves.** An optional pre-push hook (no-ops without Node, `--no-verify`-skippable) is a local convenience, not the guarantee. The release pipeline adds an **A3 verify-not-mutate** step before `cargo publish` (rebuild into a scratch dir, assert byte-equality with the committed artifact, fail the release on mismatch). Toolchain versions (`@biomejs/biome`, `typescript`, `esbuild`) are pinned exactly and the full transitive tree is frozen by the committed `web/package-lock.json` (the gate runs `npm ci` against it), with `.nvmrc` + a pinned `setup-node`, so the freshness diff is deterministic and never flaps.

### 4. Test architecture: vitest + happy-dom, snapshot fixtures, types as a gate

The harness is **vitest with the `happy-dom` environment** (owner choice; the one DOM-bound mmdflux package, `mmds-browser-text-metrics`, uses exactly this). It runs the ~70 pure-function unit tests, the DOM/rendered-output assertions (parse a pure renderer's returned HTML, assert `aria-*`/`role`/visible copy — strictly stronger than the old substring greps), and the ~6–8 live-DOM behavior tests. **`tsc --noEmit --strict`** is the type gate that subsumes the "client consumes field X / does not read removed field Y" greps. **Fixtures are snapshots of real `/api/revision` + `/api/objects` payloads** checked into `web/test/fixtures/`, with a Rust test asserting the live wire still matches the snapshot — tying the JS tests to the Rust-owned wire shape. The load-bearing advisory framing (`read-only · advisory`, `never gates a write`, `reader-relative`) is asserted at the **JS DOM layer only** (no Rust byte-smoke). Standing up this harness **resolves #276**.

### 5. Phased rollout (coverage stays monotonic)

- **P0 — no emit.** Add `web/` tooling and apply Biome + `// @ts-check` + JSDoc to the existing hand-written `app.js`; `tsc --noEmit` type-checks it. The served behavior and path are unchanged and the served bytes change only by additive comments (`// @ts-check`, JSDoc, `// biome-ignore`), so all ~50 grep tests stay green; one ubuntu CI leg runs check-only. Zero stale-artifact surface, zero cargo Node. Immediately adoptable.
- **P1 — harness + decomposition under green greps.** Stand up vitest+happy-dom (resolving #276) and develop the TypeScript port — the modules behind `store.ts`, in dependency order (pure leaves → projection/query/diff-render/cards → store → overlay+help → router → http/data → render/lenses/detail/diff-controller → keyboard/palette/navigation → `main.ts`) — as the **emerging** source, tested in isolation. **Source-of-truth rule during P1:** the hand-written `assets/app.js` stays the *served* artifact (no emit), so the Rust source-greps keep passing; the TS port is migrated behaviour-for-behaviour and is **not yet served**. The new JS tests therefore cover the port that P2 will ship, while the Rust greps continue to cover the served file — coverage is additive, never reduced.
- **P2 — atomic flip (one PR).** Wire esbuild to emit the committed `app.js` **from the TS port, making it the source of truth**; delete the ~50 `app.js`-source greps + their `served_app_js`/`spawn_and_get_app_js`/`slice_between` helpers + the `//! no JS harness` headers + the 3 `server.rs` source greps (keep the content-type smoke); enable the CI freshness gate and the A3 release verify. The served bytes change exactly once, and the swap is behaviour-checked on **both** layers — the Rust source-greps must still be green up to the moment they are deleted, and the JS tests green for the port that replaces them. The ~55 stays-Rust tests stay green across the flip.

## Consequences

### Accepted

- **First Node toolchain in the repo**, scoped to dev + one CI job; `cargo build`/`install`/`publish` stay Node-free. The CI leg is ~25–50s (mostly `npm ci`), runs concurrently, and never touches the Windows long pole. Contributors who edit the inspector must rebuild and commit `app.js` (the gate enforces it; a clear failure message tells them to run `just web-build`).
- A **typed, linted, tested, modular** front-end with ~70 pure functions under real unit tests and rendered-output asserted against a parsed DOM. Net Rust win: P2 removes ~50 server-spawn tests from the spawn-bound suite.
- **vitest is a heavier dependency than `node:test`** — accepted for its DOM/event ergonomics on the behavior tests.
- The **advisory posture is guarded only at the JS layer** — accepted; a coarse Rust byte-smoke is dropped to avoid a redundant cross-layer check.
- The CSS/token/design-system contract is **byte-stable** (a post-migration checklist confirms `tokens.css`/`app.css`/`bake.sh` are unchanged, the single `:root` stays in `tokens.css`, and the served class strings are still present); these files are untouched.

### Rejected

- **`build.rs` runs the toolchain (option B)** — forces Node onto every consumer's `cargo install`/`publish` and would have to mutate a source-relative `include_str!` path (`build.rs` may only write `OUT_DIR`). Hard reject.
- **CI builds *and commits* the bundle (A2 commit-back)** — owner-declined (2026-06-28). It works only on same-repo branches (fork PRs get a read-only token), lands an unsigned bot commit, and adds nothing the read-only gate already enforces.
- **A3 as a standalone mechanism** — a release-only `app.js` breaks `git clone && cargo build` and is dropped by `cargo publish`; A3 is retained only as a verify-not-mutate gate over the committed artifact.
- **Native multi-file ESM, no bundler (option C)** — would add 8–15 new `include_str!` constants + route arms and multiply the test-coupling surface, for no gain over a single bundle; the inspector has zero npm runtime deps.
- **Minified bundle** — an unreviewable committed artifact; non-minified `--keep-names` is required.
- **`node:test` as the runner** — superseded by the owner's vitest + happy-dom choice.
- **Editing `app.css`/`tokens.css`/the design-system bake** — out of scope; the JS migration is provably isolated from them.

## Revisit Triggers

- If `shoreline` stops publishing to crates.io, or accepts a Node prerequisite for from-source builds, the committed-bundle constraint relaxes and a `build.rs`/release-build option could be reconsidered.
- If the CI freshness diff flaps (a non-deterministic bundle), revisit toolchain version pinning before relaxing the gate.
- If the inspector acquires npm **runtime** dependencies, revisit the zero-runtime-deps assumption behind the single esbuild IIFE bundle.
- If the front-end grows enough surface to warrant a framework, revisit the vanilla-DOM + `store.ts` approach.
- **Filed follow-ups** (post-migration, not in scope here): a typed `classNames.ts` for the ~104 JS-emitted class names (catches intra-JS drift) — issue **#277**; and a cross-artifact test asserting each emitted class has a selector in `app.css`/`styles.css` (catches JS-vs-CSS drift) — issue **#278**, the higher-value of the two, and it needs no `app.css` change.
