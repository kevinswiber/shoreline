use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    QualificationCorpusManifestV1, QualificationCreateOutcome, QualificationProfile,
    QualificationRecordKindV1, QualificationRecordV1, SegmentQualificationProfile,
    SqliteQualificationProfile, load_external_legacy_manifest, verify_completed_backup,
};
use crate::canonical_hash::{canonical_json_bytes, sha256_bytes_hex};

pub const EXACT_LOGICAL_MANIFEST_SCHEMA_V1: &str =
    "pointbreak.qualification-exact-logical-manifest.v1";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ExactLogicalManifestV1 {
    pub schema: String,
    pub source: String,
    pub record_count: u64,
    pub decoded_bytes: u64,
    pub by_kind: Vec<ExactLogicalKindSummaryV1>,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ExactLogicalKindSummaryV1 {
    pub record_kind: QualificationRecordKindV1,
    pub record_count: u64,
    pub decoded_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationCandidateV1 {
    SqliteWal,
    BoundedSegments,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationFailurePointV1 {
    None,
    AfterFirstWrite,
    BeforeActivation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationRollbackBoundaryV1 {
    SafeBeforeDestinationMutation,
    ForwardRepairRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateMigrationReportV1 {
    pub candidate: MigrationCandidateV1,
    pub source: ExactLogicalManifestV1,
    pub destination: ExactLogicalManifestV1,
    pub restore: ExactLogicalManifestV1,
    pub inverse: ExactLogicalManifestV1,
    pub backup_manifest_sha256: String,
    pub source_unchanged: bool,
    pub activation_published_last: bool,
    pub rollback_boundary: MigrationRollbackBoundaryV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationWorkloadReportV1 {
    pub workload: String,
    pub report: CandidateMigrationReportV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationLifecycleRehearsalReportV1 {
    pub schema: String,
    pub mode: String,
    pub migrations: Vec<MigrationWorkloadReportV1>,
    pub transfers: super::CrossCandidateTransferReportV1,
    pub lifecycle: Vec<super::CandidateLifecycleReportV1>,
    pub archives: Vec<super::CandidateArchiveRehearsalReportV1>,
    pub external_corpus_coverage: ExternalCorpusCoverageV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExternalCorpusCoverageV1 {
    NotConfigured,
    Validated {
        manifest_sha256: String,
        record_count: u64,
        decoded_bytes: u64,
    },
}

#[derive(Serialize)]
struct ExactManifestHashPreimage<'a> {
    schema: &'a str,
    source: &'a str,
    records: &'a [ExactLogicalRecordPreimageV1],
    record_count: u64,
    decoded_bytes: u64,
}

#[derive(Serialize)]
struct ExactLogicalRecordPreimageV1 {
    logical_key: String,
    record_kind: QualificationRecordKindV1,
    decoded_sha256: String,
    decoded_bytes: u64,
}

impl ExactLogicalManifestV1 {
    pub fn from_corpus(source: &QualificationCorpusManifestV1) -> Result<Self, String> {
        source.validate().map_err(|error| error.to_string())?;
        let records = source
            .records
            .iter()
            .map(|record| ExactLogicalRecordPreimageV1 {
                logical_key: record.logical_key.clone(),
                record_kind: record.record_kind,
                decoded_sha256: record.decoded_sha256.clone(),
                decoded_bytes: record.decoded_bytes.len() as u64,
            })
            .collect::<Vec<_>>();
        let record_count = records.len() as u64;
        let decoded_bytes = records.iter().try_fold(0_u64, |total, record| {
            total
                .checked_add(record.decoded_bytes)
                .ok_or_else(|| "exact logical manifest byte count overflow".to_owned())
        })?;
        let manifest_sha256 =
            exact_manifest_sha256(&source.source, &records, record_count, decoded_bytes)?;
        let mut by_kind = BTreeMap::<QualificationRecordKindV1, (u64, u64)>::new();
        for record in &records {
            let totals = by_kind.entry(record.record_kind).or_default();
            totals.0 += 1;
            totals.1 = totals
                .1
                .checked_add(record.decoded_bytes)
                .ok_or_else(|| "exact logical kind byte count overflow".to_owned())?;
        }
        Ok(Self {
            schema: EXACT_LOGICAL_MANIFEST_SCHEMA_V1.to_owned(),
            source: source.source.clone(),
            record_count,
            decoded_bytes,
            by_kind: by_kind
                .into_iter()
                .map(
                    |(record_kind, (record_count, decoded_bytes))| ExactLogicalKindSummaryV1 {
                        record_kind,
                        record_count,
                        decoded_bytes,
                    },
                )
                .collect(),
            manifest_sha256,
        })
    }

    pub fn compare_corpus(&self, corpus: &QualificationCorpusManifestV1) -> Result<(), String> {
        let actual = Self::from_corpus(corpus)?;
        if &actual == self {
            Ok(())
        } else {
            Err(format!(
                "logical manifest {} differs from expected {}",
                actual.manifest_sha256, self.manifest_sha256
            ))
        }
    }
}

pub fn classify_rollback_boundary(destination_mutated: bool) -> MigrationRollbackBoundaryV1 {
    if destination_mutated {
        MigrationRollbackBoundaryV1::ForwardRepairRequired
    } else {
        MigrationRollbackBoundaryV1::SafeBeforeDestinationMutation
    }
}

pub fn rehearse_candidate_migration(
    source: &QualificationCorpusManifestV1,
    candidate: MigrationCandidateV1,
    destination: &Path,
    failure_point: MigrationFailurePointV1,
) -> Result<CandidateMigrationReportV1, String> {
    let source_before = source.clone();
    let source_manifest = ExactLogicalManifestV1::from_corpus(source)?;
    if destination
        .try_exists()
        .map_err(|error| format!("inspect migration destination: {error}"))?
    {
        return Err("migration destination must be fresh".to_owned());
    }

    fs::create_dir_all(destination)
        .map_err(|error| format!("create migration destination: {error}"))?;
    let staging = destination.join("staging");
    let backup = destination.join("backup");
    let active = destination.join("active");
    let profile = open_candidate(candidate, &staging)?;

    let mut writes = 0_usize;
    for record in source
        .records
        .iter()
        .filter(|record| !is_event(record.record_kind))
    {
        create_profile_record(profile.as_ref(), record)?;
        writes += 1;
        if writes == 1 && failure_point == MigrationFailurePointV1::AfterFirstWrite {
            return Err("migration stopped after the first destination write".to_owned());
        }
    }
    for record in source
        .records
        .iter()
        .filter(|record| is_event(record.record_kind))
    {
        create_profile_record(profile.as_ref(), record)?;
        writes += 1;
        if writes == 1 && failure_point == MigrationFailurePointV1::AfterFirstWrite {
            return Err("migration stopped after the first destination write".to_owned());
        }
    }

    let destination_corpus = read_profile_corpus(profile.as_ref(), source)?;
    let destination_manifest = ExactLogicalManifestV1::from_corpus(&destination_corpus)?;
    ensure_exact(&source_manifest, &destination_manifest, "destination")?;

    profile.backup_to(&backup)?;
    profile.verify_restore(&backup)?;
    let backup_manifest = verify_completed_backup(&backup, &profile.descriptor()?)
        .map_err(|error| error.to_string())?;
    let restore_root = destination.join("restore");
    materialize_completed_backup(&backup, &restore_root, &backup_manifest)?;
    let restored_profile = open_candidate(candidate, &restore_root)?;
    let restore_corpus = read_profile_corpus(restored_profile.as_ref(), source)?;
    let restore_manifest = ExactLogicalManifestV1::from_corpus(&restore_corpus)?;
    ensure_exact(&source_manifest, &restore_manifest, "restore")?;
    let inverse_manifest = ExactLogicalManifestV1::from_corpus(&restore_corpus)?;
    ensure_exact(&source_manifest, &inverse_manifest, "inverse")?;
    drop(restored_profile);
    verify_completed_backup(&backup, &profile.descriptor()?).map_err(|error| error.to_string())?;
    drop(profile);

    if failure_point == MigrationFailurePointV1::BeforeActivation {
        return Err("migration stopped before destination activation".to_owned());
    }
    fs::rename(&staging, &active)
        .map_err(|error| format!("publish migration activation: {error}"))?;

    Ok(CandidateMigrationReportV1 {
        candidate,
        source: source_manifest,
        destination: destination_manifest,
        restore: restore_manifest,
        inverse: inverse_manifest,
        backup_manifest_sha256: backup_manifest.manifest_sha256,
        source_unchanged: source == &source_before,
        activation_published_last: active.is_dir(),
        rollback_boundary: classify_rollback_boundary(writes > 0),
    })
}

pub fn external_corpus_coverage(
    explicitly_supplied_path: Option<&Path>,
) -> Result<ExternalCorpusCoverageV1, String> {
    let Some(path) = explicitly_supplied_path else {
        return Ok(ExternalCorpusCoverageV1::NotConfigured);
    };
    let manifest = load_external_legacy_manifest(path).map_err(|error| error.to_string())?;
    let exact = ExactLogicalManifestV1::from_corpus(&manifest)?;
    Ok(ExternalCorpusCoverageV1::Validated {
        manifest_sha256: exact.manifest_sha256,
        record_count: exact.record_count,
        decoded_bytes: exact.decoded_bytes,
    })
}

pub fn run_migration_lifecycle_rehearsal(
    root: &Path,
    explicitly_supplied_external_corpus: Option<&Path>,
) -> Result<MigrationLifecycleRehearsalReportV1, String> {
    fs::create_dir_all(root).map_err(|error| format!("create rehearsal root: {error}"))?;
    let synthetic = super::synthetic_legacy_manifest().map_err(|error| error.to_string())?;
    let modeled = super::modeled_post_foundation_manifest().map_err(|error| error.to_string())?;
    let candidates = [
        MigrationCandidateV1::SqliteWal,
        MigrationCandidateV1::BoundedSegments,
    ];
    let mut migrations = Vec::new();
    for (workload, source) in [
        ("synthetic_legacy", &synthetic),
        ("modeled_foundation", &modeled),
    ] {
        for candidate in candidates {
            migrations.push(MigrationWorkloadReportV1 {
                workload: workload.to_owned(),
                report: rehearse_candidate_migration(
                    source,
                    candidate,
                    &root
                        .join("migrations")
                        .join(format!("{workload}-{}", candidate_label(candidate))),
                    MigrationFailurePointV1::None,
                )?,
            });
        }
    }
    let transfers =
        super::rehearse_cross_candidate_transfers(&modeled, &root.join("candidate-transfers"))?;
    let lifecycle = candidates
        .into_iter()
        .map(|candidate| {
            super::rehearse_candidate_lifecycle(
                candidate,
                &root.join("lifecycle").join(candidate_label(candidate)),
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let archives = candidates
        .into_iter()
        .map(|candidate| {
            super::rehearse_candidate_archive(
                candidate,
                &root.join("archives").join(candidate_label(candidate)),
            )
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(MigrationLifecycleRehearsalReportV1 {
        schema: "pointbreak.store-foundation-migration-lifecycle-rehearsal.v1".to_owned(),
        mode: "non_timing_exact_migration_and_lifecycle".to_owned(),
        migrations,
        transfers,
        lifecycle,
        archives,
        external_corpus_coverage: external_corpus_coverage(explicitly_supplied_external_corpus)?,
    })
}

pub(crate) fn open_candidate(
    candidate: MigrationCandidateV1,
    root: &Path,
) -> Result<Box<dyn QualificationProfile>, String> {
    match candidate {
        MigrationCandidateV1::SqliteWal => SqliteQualificationProfile::open(root)
            .map(|profile| Box::new(profile) as Box<dyn QualificationProfile>)
            .map_err(|error| error.to_string()),
        MigrationCandidateV1::BoundedSegments => SegmentQualificationProfile::open(root)
            .map(|profile| Box::new(profile) as Box<dyn QualificationProfile>)
            .map_err(|error| error.to_string()),
    }
}

pub(crate) fn read_profile_corpus(
    profile: &dyn QualificationProfile,
    expected: &QualificationCorpusManifestV1,
) -> Result<QualificationCorpusManifestV1, String> {
    let actual_events = profile.journal().list()?;
    let expected_event_count = expected
        .records
        .iter()
        .filter(|record| is_event(record.record_kind))
        .count();
    if actual_events.len() != expected_event_count {
        return Err(format!(
            "profile event count {} differs from expected {expected_event_count}",
            actual_events.len()
        ));
    }

    let mut records = Vec::with_capacity(expected.records.len());
    for expected_record in &expected.records {
        let entry = if is_event(expected_record.record_kind) {
            profile.journal().read(&expected_record.logical_key)?
        } else {
            let entry = profile.read_content(&expected_record.logical_key)?;
            if entry.is_some()
                && profile.put_content_once(
                    &expected_record.logical_key,
                    expected_record.record_kind,
                    &expected_record.decoded_bytes,
                )? != QualificationCreateOutcome::AlreadyExists
            {
                return Err(format!(
                    "profile changed content kind for {}",
                    expected_record.logical_key
                ));
            }
            entry
        }
        .ok_or_else(|| format!("profile omitted {}", expected_record.logical_key))?;
        if entry.logical_key != expected_record.logical_key
            || entry.decoded_sha256 != expected_record.decoded_sha256
            || entry.decoded_bytes != expected_record.decoded_bytes
        {
            return Err(format!(
                "profile changed logical record {}",
                expected_record.logical_key
            ));
        }
        records.push(QualificationRecordV1::new(
            entry.logical_key,
            expected_record.record_kind,
            entry.decoded_bytes,
        ));
    }
    QualificationCorpusManifestV1::new(expected.source.clone(), records)
        .map_err(|error| error.to_string())
}

pub(crate) fn create_profile_record(
    profile: &dyn QualificationProfile,
    record: &QualificationRecordV1,
) -> Result<(), String> {
    let outcome = if is_event(record.record_kind) {
        profile
            .journal()
            .create_once(&record.logical_key, &record.decoded_bytes)?
    } else {
        profile.put_content_once(
            &record.logical_key,
            record.record_kind,
            &record.decoded_bytes,
        )?
    };
    if outcome != QualificationCreateOutcome::Created {
        return Err(format!(
            "fresh profile already contained {}",
            record.logical_key
        ));
    }
    Ok(())
}

pub(crate) fn is_event(kind: QualificationRecordKindV1) -> bool {
    matches!(
        kind,
        QualificationRecordKindV1::LegacyEvent
            | QualificationRecordKindV1::GenerationProposal
            | QualificationRecordKindV1::RelationAttestation
            | QualificationRecordKindV1::FactPort
    )
}

fn exact_manifest_sha256(
    source: &str,
    records: &[ExactLogicalRecordPreimageV1],
    record_count: u64,
    decoded_bytes: u64,
) -> Result<String, String> {
    let value = serde_json::to_value(ExactManifestHashPreimage {
        schema: EXACT_LOGICAL_MANIFEST_SCHEMA_V1,
        source,
        records,
        record_count,
        decoded_bytes,
    })
    .map_err(|error| error.to_string())?;
    canonical_json_bytes(&value)
        .map(|bytes| sha256_bytes_hex(&bytes))
        .map_err(|error| error.to_string())
}

fn materialize_completed_backup(
    source: &Path,
    destination: &Path,
    manifest: &super::CompletedBackupManifestV1,
) -> Result<(), String> {
    if destination
        .try_exists()
        .map_err(|error| format!("inspect restore destination: {error}"))?
    {
        return Err("restore destination must be fresh".to_owned());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("create restore destination: {error}"))?;
    super::copy_directory_shape(source, destination)?;
    for carrier in &manifest.carriers {
        super::copy_new(
            &source.join(&carrier.relative_path),
            &destination.join(&carrier.relative_path),
        )?;
    }
    Ok(())
}

fn ensure_exact(
    expected: &ExactLogicalManifestV1,
    actual: &ExactLogicalManifestV1,
    stage: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{stage} manifest {} differs from source {}",
            actual.manifest_sha256, expected.manifest_sha256
        ))
    }
}

fn candidate_label(candidate: MigrationCandidateV1) -> &'static str {
    match candidate {
        MigrationCandidateV1::SqliteWal => "sqlite-wal",
        MigrationCandidateV1::BoundedSegments => "bounded-segments",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench_support::foundation::{
        QualificationCorpusManifestV1, QualificationRecordV1, modeled_post_foundation_manifest,
        synthetic_legacy_manifest,
    };

    #[test]
    fn exact_manifest_detects_rehashed_byte_and_identity_changes() {
        let source = synthetic_legacy_manifest().expect("source fixture");
        let expected = ExactLogicalManifestV1::from_corpus(&source).expect("source manifest");

        let mut changed_bytes = source.records.clone();
        let original = &changed_bytes[0];
        changed_bytes[0] = QualificationRecordV1::new(
            original.logical_key.clone(),
            original.record_kind,
            b"different bytes".to_vec(),
        );
        let changed_bytes =
            QualificationCorpusManifestV1::new(source.source.clone(), changed_bytes)
                .expect("valid changed-byte manifest");
        assert!(expected.compare_corpus(&changed_bytes).is_err());

        let mut changed_identity = source.records.clone();
        changed_identity[0].logical_key = "artifacts/notes/note-002.md".to_owned();
        let changed_identity =
            QualificationCorpusManifestV1::new(source.source.clone(), changed_identity)
                .expect("valid changed-identity manifest");
        assert!(expected.compare_corpus(&changed_identity).is_err());
    }

    #[test]
    fn both_candidates_restore_and_invert_one_exact_manifest() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let source = modeled_post_foundation_manifest().expect("modeled fixture");
        for candidate in [
            MigrationCandidateV1::SqliteWal,
            MigrationCandidateV1::BoundedSegments,
        ] {
            let report = rehearse_candidate_migration(
                &source,
                candidate,
                &roots.path().join(format!("{candidate:?}")),
                MigrationFailurePointV1::None,
            )
            .expect("candidate migration");
            assert_eq!(report.source, report.destination);
            assert_eq!(report.source, report.restore);
            assert_eq!(report.source, report.inverse);
            assert!(report.source_unchanged);
            assert!(report.activation_published_last);
        }
    }

    #[test]
    fn full_staging_failure_still_does_not_publish_activation() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let source = synthetic_legacy_manifest().expect("source fixture");
        let destination = roots.path().join("destination");
        assert!(
            rehearse_candidate_migration(
                &source,
                MigrationCandidateV1::BoundedSegments,
                &destination,
                MigrationFailurePointV1::BeforeActivation,
            )
            .is_err()
        );
        assert!(!destination.join("active").exists());
        assert!(destination.join("staging").exists());
    }

    #[test]
    fn aggregate_rehearsal_emits_sanitized_machine_readable_evidence() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let report =
            run_migration_lifecycle_rehearsal(roots.path(), None).expect("aggregate rehearsal");
        assert_eq!(report.migrations.len(), 4);
        assert!(report.migrations.iter().all(|row| {
            row.report.source == row.report.destination
                && row.report.source == row.report.restore
                && row.report.source == row.report.inverse
        }));
        assert!(
            report
                .transfers
                .paths
                .iter()
                .all(|path| path.exact_manifest_match)
        );
        assert!(report.lifecycle.iter().all(|row| {
            row.matrix.required_states_complete
                && row.typed_absence_preserved
                && row.recomputable_absent
        }));
        assert!(
            report
                .archives
                .iter()
                .all(|row| { row.archive.completion_published_last && row.restore_verified })
        );
        assert_eq!(
            report.external_corpus_coverage,
            ExternalCorpusCoverageV1::NotConfigured
        );
        println!(
            "{}",
            serde_json::to_string(&report).expect("rehearsal report serializes")
        );
    }
}
