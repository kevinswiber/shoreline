//! One-shot migration driver: reshape a pre-break store onto the opaque-coded
//! signed-identity wire in a single pass. Owner-run; NOT part of the `shore`
//! binary — this is a run-once, throwaway operation, so it ships as a tested
//! library function plus this thin driver rather than a permanent CLI subcommand.
//!
//! Usage:
//!   migrate-opaque <source-store-dir> <target-store-dir> <keystore-dir>
//!
//! `<source-store-dir>` and `<target-store-dir>` are the directories holding
//! `events/` and `artifacts/` (e.g. a repo's `.git/shore`); `<target-store-dir>`
//! must be fresh/empty. `<keystore-dir>` holds the signers' private keys used to
//! re-sign inline signatures and re-attest held-key co-signatures. All paths are
//! arguments — the driver carries no built-in locations.
use std::path::PathBuf;
use std::process::ExitCode;

use shoreline::session::{MigrateOptions, migrate_opaque_identity};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, target, keystore] = args.as_slice() else {
        eprintln!("usage: migrate-opaque <source-store-dir> <target-store-dir> <keystore-dir>");
        return ExitCode::FAILURE;
    };

    let options = MigrateOptions {
        source_store_dir: PathBuf::from(source),
        target_store_dir: PathBuf::from(target),
        keystore_dir: PathBuf::from(keystore),
    };

    match migrate_opaque_identity(options) {
        Ok(summary) => {
            println!(
                "migrated {source} -> {target}: events_migrated={} events_passed_through={} \
                 content_ids_rederived={} inline_signatures_resigned={} \
                 cosignatures_reattested={} cosignatures_dropped={} \
                 content_removed_preserved={} self_check_passed={}",
                summary.events_migrated,
                summary.events_passed_through,
                summary.content_ids_rederived,
                summary.inline_signatures_resigned,
                summary.cosignatures_reattested,
                summary.cosignatures_dropped,
                summary.content_removed_preserved,
                summary.self_check_passed,
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("migrate-opaque failed: {error}");
            ExitCode::FAILURE
        }
    }
}
