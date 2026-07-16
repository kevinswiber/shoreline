# Naming cutover compatibility fixture

These bytes freeze the compatibility boundary immediately before the operational naming cutover.
They distinguish renameable placement (`.shore`, common-dir `shore`, `shore.link.json`, and the
user home) from identity-bearing `shore.*` protocol bytes that must remain readable unchanged.

The fixture was captured from source commit `b767f0d7c1b2d8c7496eea3bb547d8cea8548290` with
Pointbreak CLI `0.6.0`. `baseline.json` pins the existing Cargo target, signature vectors, event
record hash, identity IDs, historical producer, and emitted version document. The topology uses
fixed paths and timestamps so it is platform-independent; it contains no executable or secret key.

To verify provenance, check out the pinned commit, run
`cargo nextest run --test event_signature_vectors`, and run the focused
`object|revision|fingerprint|store|version` tests. The version bytes come from
`target/debug/shore version --format json`; the protocol and placement documents come from their
existing serializers and golden suites. `manifest.sha256` hashes every fixture payload, including
trailing newlines, relative to this directory.

Do not regenerate these files to satisfy an unrelated test failure. Change them only alongside an
explicit compatibility decision, retain the old vectors when historical reads still depend on
them, and document the new provenance.
