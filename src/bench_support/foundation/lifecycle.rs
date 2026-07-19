use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    BACKUP_COMPLETION_FILE_V1, BACKUP_MANIFEST_FILE_V1, ExactBundleClosureV2,
    ExactBundleManifestV2, ExactLogicalManifestV1, ImportReceiptPolicyPrototypeV1,
    ImportReceiptPrototypeV1, MigrationCandidateV1, ProofEvidenceStateV1,
    QualificationCorpusManifestV1, QualificationProfile, QualificationProfileDescriptorV1,
    QualificationRecordKindV1, QualificationRecordV1, create_profile_record, is_event,
    open_candidate, read_profile_corpus, verify_completed_backup,
};

const LIFECYCLE_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/store-foundation/migration/lifecycle-states.json"
));

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentLifecycleStateV1 {
    Available,
    Missing,
    ValidOrphan,
    Removed,
    TypedAbsence,
    Recomputable,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContentLifecycleFixtureV1 {
    pub schema: String,
    pub entries: Vec<ContentLifecycleEntryV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentLifecycleEntryV1 {
    pub logical_key: String,
    pub record_kind: QualificationRecordKindV1,
    pub state: ContentLifecycleStateV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContentLifecycleReportV1 {
    pub required_states_complete: bool,
    pub entries: u64,
    pub proof_after_removal: ProofEvidenceStateV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateLifecycleReportV1 {
    pub candidate: MigrationCandidateV1,
    pub matrix: ContentLifecycleReportV1,
    pub available_readable: bool,
    pub missing_absent: bool,
    pub valid_orphan_readable: bool,
    pub removed_absent: bool,
    pub typed_absence_preserved: bool,
    pub recomputable_absent: bool,
    pub document_manifest_independent_of_blob: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CrossCandidateTransferReportV1 {
    pub paths: Vec<CandidateTransferPathReportV1>,
    pub hard_conflict_rejected_before_write: bool,
    pub omission_preserved_existing_content: bool,
    pub receipt_alternatives: Vec<TransferReceiptReportV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateTransferPathReportV1 {
    pub source_candidate: MigrationCandidateV1,
    pub destination_candidate: MigrationCandidateV1,
    pub exact_manifest_match: bool,
    pub bundle_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferReceiptReportV1 {
    pub policy: ImportReceiptPolicyPrototypeV1,
    pub receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveCopyFailurePointV1 {
    None,
    BeforeCompletion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImmutableArchiveCopyReportV1 {
    pub manifest_sha256: String,
    pub carrier_count: u64,
    pub completion_published_last: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateArchiveRehearsalReportV1 {
    pub candidate: MigrationCandidateV1,
    pub archive: ImmutableArchiveCopyReportV1,
    pub restore_verified: bool,
}

pub fn content_lifecycle_fixture_v1() -> ContentLifecycleFixtureV1 {
    serde_json::from_str(LIFECYCLE_FIXTURE).expect("checked-in lifecycle fixture must be valid")
}

pub fn validate_content_lifecycle_matrix(
    fixture: &ContentLifecycleFixtureV1,
) -> Result<ContentLifecycleReportV1, String> {
    if fixture.schema != "pointbreak.qualification-lifecycle-fixture.v1" {
        return Err(format!("unsupported lifecycle schema {}", fixture.schema));
    }
    let required = [
        ContentLifecycleStateV1::Available,
        ContentLifecycleStateV1::Missing,
        ContentLifecycleStateV1::ValidOrphan,
        ContentLifecycleStateV1::Removed,
        ContentLifecycleStateV1::TypedAbsence,
        ContentLifecycleStateV1::Recomputable,
    ];
    let mut keys = std::collections::BTreeSet::new();
    for entry in &fixture.entries {
        if entry.logical_key.trim().is_empty() || !keys.insert(entry.logical_key.as_str()) {
            return Err("lifecycle keys must be non-empty and unique".to_owned());
        }
    }
    let required_states_complete = required
        .iter()
        .all(|state| fixture.entries.iter().any(|entry| entry.state == *state));
    if !required_states_complete {
        return Err("lifecycle fixture omits a required state".to_owned());
    }
    let removed_proof = fixture.entries.iter().any(|entry| {
        entry.state == ContentLifecycleStateV1::Removed
            && entry.record_kind == QualificationRecordKindV1::RelationProof
    });
    if !removed_proof {
        return Err("lifecycle fixture omits removed proof evidence".to_owned());
    }
    Ok(ContentLifecycleReportV1 {
        required_states_complete,
        entries: fixture.entries.len() as u64,
        proof_after_removal: ProofEvidenceStateV1::Removed,
    })
}

pub fn rehearse_candidate_lifecycle(
    candidate: MigrationCandidateV1,
    root: &Path,
) -> Result<CandidateLifecycleReportV1, String> {
    let matrix = validate_content_lifecycle_matrix(&content_lifecycle_fixture_v1())?;
    let profile = open_candidate(candidate, root)?;
    let available = "content/available";
    let orphan = "content/orphan";
    let removed = "content/removed";
    let document_manifest = "content/document-manifest";
    let document_blob = "content/document-blob";
    profile.put_content_once(
        available,
        QualificationRecordKindV1::ObjectArtifact,
        b"available",
    )?;
    profile.put_content_once(orphan, QualificationRecordKindV1::DocumentBlob, b"orphan")?;
    profile.put_content_once(removed, QualificationRecordKindV1::RelationProof, b"proof")?;
    profile.put_content_once(
        document_manifest,
        QualificationRecordKindV1::DocumentManifest,
        b"manifest",
    )?;
    profile.put_content_once(
        document_blob,
        QualificationRecordKindV1::DocumentBlob,
        b"blob",
    )?;
    if !profile.remove_content(removed)? || !profile.remove_content(document_blob)? {
        return Err("content removal did not report an existing record".to_owned());
    }

    Ok(CandidateLifecycleReportV1 {
        candidate,
        matrix,
        available_readable: profile.read_content(available)?.is_some(),
        missing_absent: profile.read_content("content/missing")?.is_none(),
        valid_orphan_readable: profile.read_content(orphan)?.is_some(),
        removed_absent: profile.read_content(removed)?.is_none(),
        typed_absence_preserved: profile.read_content("content/typed-absence")?.is_none(),
        recomputable_absent: profile.read_content("content/recomputable")?.is_none(),
        document_manifest_independent_of_blob: profile.read_content(document_manifest)?.is_some()
            && profile.read_content(document_blob)?.is_none(),
    })
}

pub fn rehearse_cross_candidate_transfers(
    source: &QualificationCorpusManifestV1,
    root: &Path,
) -> Result<CrossCandidateTransferReportV1, String> {
    source.validate().map_err(|error| error.to_string())?;
    fs::create_dir_all(root).map_err(|error| format!("create transfer root: {error}"))?;
    let candidates = [
        MigrationCandidateV1::SqliteWal,
        MigrationCandidateV1::BoundedSegments,
    ];
    let mut paths = Vec::new();
    for source_candidate in candidates {
        for destination_candidate in candidates {
            let label = format!(
                "{}-to-{}",
                candidate_label(source_candidate),
                candidate_label(destination_candidate)
            );
            let source_profile =
                open_candidate(source_candidate, &root.join(&label).join("source"))?;
            publish_corpus(source_profile.as_ref(), source)?;
            let source_readback = read_profile_corpus(source_profile.as_ref(), source)?;
            let bundle = exact_bundle(&source_readback)?;
            let destination_profile = open_candidate(
                destination_candidate,
                &root.join(&label).join("destination"),
            )?;
            publish_bundle_to_profile(destination_profile.as_ref(), &bundle)?;
            let destination_readback = read_profile_corpus(destination_profile.as_ref(), source)?;
            let source_manifest = ExactLogicalManifestV1::from_corpus(&source_readback)?;
            let destination_manifest = ExactLogicalManifestV1::from_corpus(&destination_readback)?;
            paths.push(CandidateTransferPathReportV1 {
                source_candidate,
                destination_candidate,
                exact_manifest_match: source_manifest == destination_manifest,
                bundle_sha256: bundle.bundle_sha256,
            });
        }
    }

    let conflict_profile =
        open_candidate(MigrationCandidateV1::SqliteWal, &root.join("hard-conflict"))?;
    let bundle = exact_bundle(source)?;
    let conflict_record = bundle
        .events
        .last()
        .ok_or_else(|| "transfer fixture must contain an event".to_owned())?;
    conflict_profile
        .journal()
        .create_once(&conflict_record.logical_key, b"divergent")?;
    let head_before = conflict_profile.journal().head_marker()?;
    let conflict_rejected = publish_bundle_to_profile(conflict_profile.as_ref(), &bundle).is_err();
    let mut no_partial_write = conflict_profile.journal().head_marker()? == head_before;
    for record in &bundle.content {
        no_partial_write &= conflict_profile
            .read_content(&record.logical_key)?
            .is_none();
    }
    for record in &bundle.events {
        if record.logical_key != conflict_record.logical_key {
            no_partial_write &= conflict_profile
                .journal()
                .read(&record.logical_key)?
                .is_none();
        }
    }

    let omission_profile = open_candidate(
        MigrationCandidateV1::BoundedSegments,
        &root.join("omission"),
    )?;
    omission_profile.put_content_once(
        "content/not-in-bundle",
        QualificationRecordKindV1::NoteBody,
        b"preserve",
    )?;
    publish_bundle_to_profile(omission_profile.as_ref(), &bundle)?;
    let omission_preserved_existing_content = omission_profile
        .read_content("content/not-in-bundle")?
        .is_some();
    let receipt_alternatives = [
        ImportReceiptPolicyPrototypeV1::DurableOperational,
        ImportReceiptPolicyPrototypeV1::LocalProvenanceEvent,
    ]
    .into_iter()
    .map(|policy| {
        ImportReceiptPrototypeV1::new(policy, &bundle, "candidate-transfer")
            .map(|receipt| TransferReceiptReportV1 {
                policy,
                receipt_sha256: receipt.receipt_sha256,
            })
            .map_err(|error| error.to_string())
    })
    .collect::<Result<Vec<_>, String>>()?;

    Ok(CrossCandidateTransferReportV1 {
        paths,
        hard_conflict_rejected_before_write: conflict_rejected && no_partial_write,
        omission_preserved_existing_content,
        receipt_alternatives,
    })
}

pub fn copy_completed_backup_to_immutable_prefix(
    source: &Path,
    destination: &Path,
    descriptor: &QualificationProfileDescriptorV1,
    failure_point: ArchiveCopyFailurePointV1,
) -> Result<ImmutableArchiveCopyReportV1, String> {
    let manifest =
        verify_completed_backup(source, descriptor).map_err(|error| error.to_string())?;
    if destination
        .try_exists()
        .map_err(|error| format!("inspect archive destination: {error}"))?
    {
        return Err("archive destination must be a fresh prefix".to_owned());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("create archive destination: {error}"))?;
    copy_directory_shape(source, destination)?;

    for carrier in &manifest.carriers {
        copy_new(
            &source.join(&carrier.relative_path),
            &destination.join(&carrier.relative_path),
        )?;
    }
    copy_new(
        &source.join(BACKUP_MANIFEST_FILE_V1),
        &destination.join(BACKUP_MANIFEST_FILE_V1),
    )?;
    if failure_point == ArchiveCopyFailurePointV1::BeforeCompletion {
        return Err("archive copy stopped before completion publication".to_owned());
    }
    copy_new(
        &source.join(BACKUP_COMPLETION_FILE_V1),
        &destination.join(BACKUP_COMPLETION_FILE_V1),
    )?;
    let copied =
        verify_completed_backup(destination, descriptor).map_err(|error| error.to_string())?;
    if copied != manifest {
        return Err("archive manifest changed during opaque copy".to_owned());
    }
    Ok(ImmutableArchiveCopyReportV1 {
        manifest_sha256: copied.manifest_sha256,
        carrier_count: copied.carriers.len() as u64,
        completion_published_last: true,
    })
}

pub fn rehearse_candidate_archive(
    candidate: MigrationCandidateV1,
    root: &Path,
) -> Result<CandidateArchiveRehearsalReportV1, String> {
    let profile = open_candidate(candidate, &root.join("profile"))?;
    profile
        .journal()
        .create_once("events/archive-rehearsal.json", b"archive-rehearsal")?;
    let backup = root.join("backup");
    let archive_root = root.join("immutable-prefix");
    profile.backup_to(&backup)?;
    let descriptor = profile.descriptor()?;
    let archive = copy_completed_backup_to_immutable_prefix(
        &backup,
        &archive_root,
        &descriptor,
        ArchiveCopyFailurePointV1::None,
    )?;
    profile.verify_restore(&archive_root)?;
    Ok(CandidateArchiveRehearsalReportV1 {
        candidate,
        archive,
        restore_verified: true,
    })
}

fn publish_corpus(
    profile: &dyn QualificationProfile,
    source: &QualificationCorpusManifestV1,
) -> Result<(), String> {
    for record in source
        .records
        .iter()
        .filter(|record| !is_event(record.record_kind))
    {
        create_profile_record(profile, record)?;
    }
    for record in source
        .records
        .iter()
        .filter(|record| is_event(record.record_kind))
    {
        create_profile_record(profile, record)?;
    }
    Ok(())
}

fn exact_bundle(source: &QualificationCorpusManifestV1) -> Result<ExactBundleManifestV2, String> {
    let closure = [
        (
            "events/000-root.json",
            &[
                "artifacts/documents/blob-guide.md",
                "artifacts/documents/manifest-guide.json",
                "artifacts/objects/round-001.json",
            ][..],
        ),
        (
            "events/003-attestation-verified.json",
            &["artifacts/proofs/relation-001.json"][..],
        ),
    ]
    .into_iter()
    .filter(|(event, _)| {
        source
            .records
            .iter()
            .any(|record| record.logical_key == *event)
    })
    .map(|(event, content)| ExactBundleClosureV2 {
        event_logical_key: event.to_owned(),
        required_content_keys: content.iter().map(|key| (*key).to_owned()).collect(),
    })
    .collect();
    ExactBundleManifestV2::new(
        source.manifest_sha256.clone(),
        super::LogicalCapabilityEpochV1::foundation(),
        source.records.clone(),
        closure,
    )
    .map_err(|error| error.to_string())
}

fn publish_bundle_to_profile(
    profile: &dyn QualificationProfile,
    bundle: &ExactBundleManifestV2,
) -> Result<(), String> {
    bundle.validate().map_err(|error| error.to_string())?;
    let descriptor = profile.descriptor()?;
    if descriptor.logical_capabilities != bundle.required_capabilities {
        return Err("destination capability epoch does not satisfy bundle".to_owned());
    }

    for record in bundle.content.iter().chain(&bundle.events) {
        let existing = if is_event(record.record_kind) {
            profile.journal().read(&record.logical_key)?
        } else {
            profile.read_content(&record.logical_key)?
        };
        if let Some(existing) = existing
            && (existing.decoded_sha256 != record.decoded_sha256
                || existing.decoded_bytes != record.decoded_bytes)
        {
            return Err(format!("hard conflict for {}", record.logical_key));
        }
    }

    for record in &bundle.content {
        create_or_match(profile, record)?;
    }
    for record in &bundle.events {
        create_or_match(profile, record)?;
    }
    for record in bundle.content.iter().chain(&bundle.events) {
        let stored = if is_event(record.record_kind) {
            profile.journal().read(&record.logical_key)?
        } else {
            profile.read_content(&record.logical_key)?
        }
        .ok_or_else(|| format!("bundle publication omitted {}", record.logical_key))?;
        if stored.decoded_sha256 != record.decoded_sha256
            || stored.decoded_bytes != record.decoded_bytes
        {
            return Err(format!("bundle publication changed {}", record.logical_key));
        }
    }
    Ok(())
}

fn create_or_match(
    profile: &dyn QualificationProfile,
    record: &QualificationRecordV1,
) -> Result<(), String> {
    if is_event(record.record_kind) {
        profile
            .journal()
            .create_once(&record.logical_key, &record.decoded_bytes)?;
    } else {
        profile.put_content_once(
            &record.logical_key,
            record.record_kind,
            &record.decoded_bytes,
        )?;
    }
    Ok(())
}

pub(crate) fn copy_new(source: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|error| format!("read {}: {error}", source.display()))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|error| format!("create {}: {error}", destination.display()))?;
    output
        .write_all(&bytes)
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("write {}: {error}", destination.display()))
}

pub(crate) fn copy_directory_shape(source: &Path, destination: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("read directory {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read directory {}: {error}", source.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", source_path.display()))?;
        if file_type.is_dir() {
            let destination_path = destination.join(entry.file_name());
            fs::create_dir(&destination_path)
                .map_err(|error| format!("create {}: {error}", destination_path.display()))?;
            copy_directory_shape(&source_path, &destination_path)?;
        } else if !file_type.is_file() {
            return Err(format!(
                "archive source contains unsupported carrier {}",
                source_path.display()
            ));
        }
    }
    Ok(())
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
    use crate::bench_support::foundation::{QualificationProfile, SqliteQualificationProfile};

    #[test]
    fn archive_copy_fails_closed_for_changed_or_added_carriers() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let profile =
            SqliteQualificationProfile::open(&roots.path().join("profile")).expect("profile");
        profile
            .journal()
            .create_once("events/one.json", b"one")
            .expect("seed event");
        let backup = roots.path().join("backup");
        profile.backup_to(&backup).expect("backup");
        let descriptor = profile.descriptor().expect("descriptor");

        let changed = roots.path().join("changed");
        copy_completed_backup_to_immutable_prefix(
            &backup,
            &changed,
            &descriptor,
            ArchiveCopyFailurePointV1::None,
        )
        .expect("archive copy");
        let manifest = verify_completed_backup(&changed, &descriptor).expect("manifest");
        fs::write(
            changed.join(&manifest.carriers[0].relative_path),
            b"changed",
        )
        .expect("change carrier");
        assert!(verify_completed_backup(&changed, &descriptor).is_err());

        let added = roots.path().join("added");
        copy_completed_backup_to_immutable_prefix(
            &backup,
            &added,
            &descriptor,
            ArchiveCopyFailurePointV1::None,
        )
        .expect("archive copy");
        fs::write(added.join("unexpected"), b"added").expect("add carrier");
        assert!(verify_completed_backup(&added, &descriptor).is_err());
    }

    #[test]
    fn completed_archive_is_independent_of_later_source_changes() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let profile =
            SqliteQualificationProfile::open(&roots.path().join("profile")).expect("profile");
        profile
            .journal()
            .create_once("events/one.json", b"one")
            .expect("seed event");
        let backup = roots.path().join("backup");
        profile.backup_to(&backup).expect("backup");
        let descriptor = profile.descriptor().expect("descriptor");
        let archive = roots.path().join("archive");
        copy_completed_backup_to_immutable_prefix(
            &backup,
            &archive,
            &descriptor,
            ArchiveCopyFailurePointV1::None,
        )
        .expect("archive copy");
        fs::remove_dir_all(&backup).expect("remove disposable source backup");

        assert!(verify_completed_backup(&archive, &descriptor).is_ok());
        assert!(profile.verify_restore(&archive).is_ok());
    }

    #[test]
    fn archive_copy_fails_closed_for_missing_or_mismatched_completion() {
        let roots = tempfile::tempdir().expect("temporary roots");
        let profile =
            SqliteQualificationProfile::open(&roots.path().join("profile")).expect("profile");
        profile
            .journal()
            .create_once("events/one.json", b"one")
            .expect("seed event");
        let backup = roots.path().join("backup");
        profile.backup_to(&backup).expect("backup");
        let descriptor = profile.descriptor().expect("descriptor");

        let missing = roots.path().join("missing-completion");
        copy_completed_backup_to_immutable_prefix(
            &backup,
            &missing,
            &descriptor,
            ArchiveCopyFailurePointV1::None,
        )
        .expect("archive copy");
        fs::remove_file(missing.join(BACKUP_COMPLETION_FILE_V1)).expect("remove completion");
        assert!(verify_completed_backup(&missing, &descriptor).is_err());

        let mismatch = roots.path().join("mismatched-completion");
        copy_completed_backup_to_immutable_prefix(
            &backup,
            &mismatch,
            &descriptor,
            ArchiveCopyFailurePointV1::None,
        )
        .expect("archive copy");
        fs::write(
            mismatch.join(BACKUP_COMPLETION_FILE_V1),
            br#"{"schema":"pointbreak.qualification-backup-completion.v1","manifestSha256":"mismatch"}"#,
        )
        .expect("replace completion");
        assert!(verify_completed_backup(&mismatch, &descriptor).is_err());
    }
}
