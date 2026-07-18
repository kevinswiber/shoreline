use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{
    QualificationCreateOutcome, QualificationInventoryV1, QualificationJournal,
    QualificationProfile, QualificationProfileDescriptorV1, QualificationRecordKindV1,
};
use crate::canonical_hash::{canonical_json_bytes, sha256_bytes_hex};

pub const QUALIFICATION_SCENARIO_RESULT_SCHEMA_V1: &str =
    "pointbreak.qualification-scenario-result.v1";
pub const BACKUP_MANIFEST_SCHEMA_V1: &str = "pointbreak.qualification-backup-manifest.v1";
pub const BACKUP_COMPLETION_SCHEMA_V1: &str = "pointbreak.qualification-backup-completion.v1";
pub const BACKUP_MANIFEST_FILE_V1: &str = "pointbreak-backup-manifest-v1.json";
pub const BACKUP_COMPLETION_FILE_V1: &str = "pointbreak-backup-complete-v1.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationScenarioOutcomeV1 {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationEnvironmentV1 {
    pub operating_system: String,
    pub architecture: String,
    pub filesystem: String,
    pub source_commit: String,
    pub candidate_build_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationWorkloadSummaryV1 {
    pub manifest_sha256: String,
    pub records: u64,
    pub decoded_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationTimingV1 {
    pub operation: String,
    pub elapsed_nanos: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationScenarioFailureV1 {
    pub stage: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationScenarioResultV1 {
    pub schema: String,
    pub scenario: String,
    pub outcome: QualificationScenarioOutcomeV1,
    pub environment: QualificationEnvironmentV1,
    pub workload: QualificationWorkloadSummaryV1,
    pub timings: Vec<QualificationTimingV1>,
    pub inventory: Option<QualificationInventoryV1>,
    pub failure: Option<QualificationScenarioFailureV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCarrierV1 {
    pub relative_path: String,
    pub encoded_bytes: u64,
    pub encoded_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedBackupManifestV1 {
    pub schema: String,
    pub descriptor: QualificationProfileDescriptorV1,
    pub carriers: Vec<BackupCarrierV1>,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupCompletionV1 {
    schema: String,
    manifest_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifestPreimage<'a> {
    schema: &'a str,
    descriptor: &'a QualificationProfileDescriptorV1,
    carriers: &'a [BackupCarrierV1],
}

pub fn run_journal_contract_vectors(
    journal: &dyn QualificationJournal,
) -> Result<QualificationScenarioResultV1, String> {
    let started = Instant::now();
    if journal.create_once("journal/b", b"second")? != QualificationCreateOutcome::Created {
        return Err("first create did not return Created".to_owned());
    }
    if journal.create_once("journal/a", b"first")? != QualificationCreateOutcome::Created {
        return Err("second create did not return Created".to_owned());
    }
    if journal.create_once("journal/a", b"first")? != QualificationCreateOutcome::AlreadyExists {
        return Err("idempotent create did not return AlreadyExists".to_owned());
    }
    if journal.create_once("journal/a", b"conflict").is_ok() {
        return Err("divergent create did not fail".to_owned());
    }
    let first = journal
        .read("journal/a")?
        .ok_or_else(|| "keyed read omitted journal/a".to_owned())?;
    if first.decoded_bytes != b"first" {
        return Err("keyed read returned different bytes".to_owned());
    }
    let listed = journal.list()?;
    if listed.len() != 2
        || listed[0].logical_key != "journal/a"
        || listed[1].logical_key != "journal/b"
    {
        return Err("journal list is incomplete or nondeterministic".to_owned());
    }
    if journal.head_marker()? != 2 {
        return Err("journal head marker does not reflect two creates".to_owned());
    }
    journal.integrity_check()?;
    let decoded_bytes = listed.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.decoded_bytes.len() as u64)
            .ok_or_else(|| "workload byte count overflow".to_owned())
    })?;
    Ok(QualificationScenarioResultV1 {
        schema: QUALIFICATION_SCENARIO_RESULT_SCHEMA_V1.to_owned(),
        scenario: "shared-journal-contract-v1".to_owned(),
        outcome: QualificationScenarioOutcomeV1::Passed,
        environment: QualificationEnvironmentV1 {
            operating_system: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            filesystem: "not-probed".to_owned(),
            source_commit: option_env!("POINTBREAK_BUILD_COMMIT")
                .unwrap_or("unavailable")
                .to_owned(),
            candidate_build_id: "shared-contract-v1".to_owned(),
        },
        workload: QualificationWorkloadSummaryV1 {
            manifest_sha256: sha256_bytes_hex(b"journal/a\0first\0journal/b\0second"),
            records: listed.len() as u64,
            decoded_bytes,
        },
        timings: vec![QualificationTimingV1 {
            operation: "shared-journal-contract".to_owned(),
            elapsed_nanos: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        }],
        inventory: None,
        failure: None,
    })
}

pub fn run_profile_contract_vectors(
    profile: &dyn QualificationProfile,
    backup_destination: &Path,
) -> Result<QualificationScenarioResultV1, String> {
    let started = Instant::now();
    let descriptor = profile.descriptor()?;
    if descriptor.physical_profile_id.trim().is_empty() {
        return Err("physical profile id must not be empty".to_owned());
    }
    descriptor
        .logical_capabilities
        .validate()
        .map_err(|error| error.to_string())?;

    let journal_report = run_journal_contract_vectors(profile.journal())?;
    let content_records = [
        (
            QualificationRecordKindV1::ObjectArtifact,
            "sha256:0000000000000000000000000000000000000000000000000000000000000101",
            b"object".as_slice(),
        ),
        (
            QualificationRecordKindV1::NoteBody,
            "sha256:0000000000000000000000000000000000000000000000000000000000000102",
            b"note".as_slice(),
        ),
        (
            QualificationRecordKindV1::RelationProof,
            "sha256:0000000000000000000000000000000000000000000000000000000000000103",
            b"proof".as_slice(),
        ),
        (
            QualificationRecordKindV1::DocumentManifest,
            "sha256:0000000000000000000000000000000000000000000000000000000000000104",
            b"manifest".as_slice(),
        ),
        (
            QualificationRecordKindV1::DocumentBlob,
            "sha256:0000000000000000000000000000000000000000000000000000000000000105",
            b"document".as_slice(),
        ),
    ];
    for (kind, key, bytes) in content_records {
        if profile.put_content_once(key, kind, bytes)? != QualificationCreateOutcome::Created {
            return Err(format!(
                "first content create for {key} did not return Created"
            ));
        }
        if profile.put_content_once(key, kind, bytes)? != QualificationCreateOutcome::AlreadyExists
        {
            return Err(format!(
                "idempotent content create for {key} did not return AlreadyExists"
            ));
        }
        let read = profile
            .read_content(key)?
            .ok_or_else(|| format!("content read omitted {key}"))?;
        if read.decoded_bytes != bytes {
            return Err(format!("content read returned different bytes for {key}"));
        }
    }
    let proof_key = content_records[2].1;
    if !profile.remove_content(proof_key)? || profile.read_content(proof_key)?.is_some() {
        return Err("relation-proof carrier removal did not remain independent".to_owned());
    }

    profile.backup_to(backup_destination)?;
    profile.verify_restore(backup_destination)?;
    let inventory = profile.inventory()?;
    validate_inventory(&inventory)?;
    let content_decoded_bytes = content_records.iter().try_fold(0_u64, |total, record| {
        total
            .checked_add(record.2.len() as u64)
            .ok_or_else(|| "profile workload byte count overflow".to_owned())
    })?;
    Ok(QualificationScenarioResultV1 {
        schema: QUALIFICATION_SCENARIO_RESULT_SCHEMA_V1.to_owned(),
        scenario: "shared-profile-contract-v1".to_owned(),
        outcome: QualificationScenarioOutcomeV1::Passed,
        environment: journal_report.environment,
        workload: QualificationWorkloadSummaryV1 {
            manifest_sha256: sha256_bytes_hex(b"shared-profile-contract-v1"),
            records: journal_report.workload.records + content_records.len() as u64,
            decoded_bytes: journal_report.workload.decoded_bytes + content_decoded_bytes,
        },
        timings: vec![QualificationTimingV1 {
            operation: "shared-profile-contract".to_owned(),
            elapsed_nanos: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        }],
        inventory: Some(inventory),
        failure: None,
    })
}

fn validate_inventory(inventory: &QualificationInventoryV1) -> Result<(), String> {
    if inventory.carriers.is_empty() {
        return Err("profile inventory omitted every carrier".to_owned());
    }
    for carriers in inventory.carriers.windows(2) {
        if carriers[0].as_bytes() >= carriers[1].as_bytes() {
            return Err("profile inventory carriers are duplicate or unsorted".to_owned());
        }
    }
    if inventory.allocated_bytes < inventory.encoded_bytes
        || inventory.high_water_bytes < inventory.allocated_bytes
    {
        return Err("profile inventory byte totals are not monotonic".to_owned());
    }
    Ok(())
}

pub fn publish_completed_backup<F>(
    destination: &Path,
    descriptor: &QualificationProfileDescriptorV1,
    populate_candidate_backup: F,
) -> Result<CompletedBackupManifestV1, BackupContractError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    validate_backup_descriptor(descriptor)?;
    if destination
        .try_exists()
        .map_err(|error| BackupContractError::Io {
            path: destination.to_path_buf(),
            message: error.to_string(),
        })?
    {
        return Err(BackupContractError::DestinationExists {
            path: destination.to_path_buf(),
        });
    }
    fs::create_dir_all(destination).map_err(|error| BackupContractError::Io {
        path: destination.to_path_buf(),
        message: error.to_string(),
    })?;
    populate_candidate_backup(destination).map_err(BackupContractError::CandidateBackup)?;
    for reserved in [BACKUP_MANIFEST_FILE_V1, BACKUP_COMPLETION_FILE_V1] {
        if destination
            .join(reserved)
            .try_exists()
            .map_err(|error| BackupContractError::Io {
                path: destination.join(reserved),
                message: error.to_string(),
            })?
        {
            return Err(BackupContractError::ReservedCarrier {
                relative_path: reserved.to_owned(),
            });
        }
    }

    let carriers = inventory_backup_carriers(destination)?;
    if carriers.is_empty() {
        return Err(BackupContractError::EmptyBackup);
    }
    let manifest_sha256 = backup_manifest_sha256(descriptor, &carriers)?;
    let manifest = CompletedBackupManifestV1 {
        schema: BACKUP_MANIFEST_SCHEMA_V1.to_owned(),
        descriptor: descriptor.clone(),
        carriers,
        manifest_sha256: manifest_sha256.clone(),
    };
    write_canonical_new(&destination.join(BACKUP_MANIFEST_FILE_V1), &manifest)?;
    sync_directory(destination)?;
    let completion = BackupCompletionV1 {
        schema: BACKUP_COMPLETION_SCHEMA_V1.to_owned(),
        manifest_sha256,
    };
    write_canonical_new(&destination.join(BACKUP_COMPLETION_FILE_V1), &completion)?;
    sync_directory(destination)?;
    Ok(manifest)
}

pub fn verify_completed_backup(
    destination: &Path,
    expected_descriptor: &QualificationProfileDescriptorV1,
) -> Result<CompletedBackupManifestV1, BackupContractError> {
    validate_backup_descriptor(expected_descriptor)?;
    let completion_path = destination.join(BACKUP_COMPLETION_FILE_V1);
    let completion: BackupCompletionV1 = read_json(&completion_path)?;
    if completion.schema != BACKUP_COMPLETION_SCHEMA_V1 {
        return Err(BackupContractError::UnsupportedSchema {
            schema: completion.schema,
        });
    }

    let manifest_path = destination.join(BACKUP_MANIFEST_FILE_V1);
    let manifest: CompletedBackupManifestV1 = read_json(&manifest_path)?;
    if manifest.schema != BACKUP_MANIFEST_SCHEMA_V1 {
        return Err(BackupContractError::UnsupportedSchema {
            schema: manifest.schema,
        });
    }
    if &manifest.descriptor != expected_descriptor {
        return Err(BackupContractError::DescriptorMismatch);
    }
    let expected_hash = backup_manifest_sha256(&manifest.descriptor, &manifest.carriers)?;
    if manifest.manifest_sha256 != expected_hash
        || completion.manifest_sha256 != manifest.manifest_sha256
    {
        return Err(BackupContractError::CompletionMismatch);
    }
    let actual_carriers = inventory_backup_carriers(destination)?;
    if actual_carriers != manifest.carriers {
        return Err(BackupContractError::CarrierSetMismatch);
    }
    Ok(manifest)
}

#[derive(Debug, thiserror::Error)]
pub enum BackupContractError {
    #[error("backup destination already exists: {path}", path = .path.display())]
    DestinationExists { path: PathBuf },
    #[error("candidate backup failed: {0}")]
    CandidateBackup(String),
    #[error("candidate backup used reserved carrier {relative_path}")]
    ReservedCarrier { relative_path: String },
    #[error("candidate backup produced no carriers")]
    EmptyBackup,
    #[error("unsupported backup schema {schema}")]
    UnsupportedSchema { schema: String },
    #[error("backup descriptor does not match the expected profile and capabilities")]
    DescriptorMismatch,
    #[error("backup completion marker does not bind the manifest")]
    CompletionMismatch,
    #[error("backup carrier set or hashes do not match the manifest")]
    CarrierSetMismatch,
    #[error("backup I/O failed at {path}: {message}", path = .path.display())]
    Io { path: PathBuf, message: String },
    #[error("backup document could not be serialized: {message}")]
    Serialization { message: String },
}

fn backup_manifest_sha256(
    descriptor: &QualificationProfileDescriptorV1,
    carriers: &[BackupCarrierV1],
) -> Result<String, BackupContractError> {
    let value = serde_json::to_value(BackupManifestPreimage {
        schema: BACKUP_MANIFEST_SCHEMA_V1,
        descriptor,
        carriers,
    })
    .map_err(|error| BackupContractError::Serialization {
        message: error.to_string(),
    })?;
    let bytes =
        canonical_json_bytes(&value).map_err(|error| BackupContractError::Serialization {
            message: error.to_string(),
        })?;
    Ok(sha256_bytes_hex(&bytes))
}

fn validate_backup_descriptor(
    descriptor: &QualificationProfileDescriptorV1,
) -> Result<(), BackupContractError> {
    if descriptor.physical_profile_id.trim().is_empty()
        || descriptor.logical_capabilities.validate().is_err()
    {
        return Err(BackupContractError::DescriptorMismatch);
    }
    Ok(())
}

fn inventory_backup_carriers(root: &Path) -> Result<Vec<BackupCarrierV1>, BackupContractError> {
    let mut paths = Vec::new();
    collect_files(root, &mut paths)?;
    paths.retain(|path| {
        path != &root.join(BACKUP_MANIFEST_FILE_V1) && path != &root.join(BACKUP_COMPLETION_FILE_V1)
    });
    let mut carriers = paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).map_err(|error| BackupContractError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
            Ok(BackupCarrierV1 {
                relative_path: relative_path_string(root, &path)?,
                encoded_bytes: bytes.len() as u64,
                encoded_sha256: sha256_bytes_hex(&bytes),
            })
        })
        .collect::<Result<Vec<_>, BackupContractError>>()?;
    carriers.sort_by(|left, right| {
        left.relative_path
            .as_bytes()
            .cmp(right.relative_path.as_bytes())
    });
    Ok(carriers)
}

fn collect_files(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), BackupContractError> {
    let entries = fs::read_dir(directory).map_err(|error| BackupContractError::Io {
        path: directory.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| BackupContractError::Io {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| BackupContractError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if file_type.is_dir() {
            collect_files(&path, paths)?;
        } else if file_type.is_file() {
            paths.push(path);
        } else {
            return Err(BackupContractError::CarrierSetMismatch);
        }
    }
    Ok(())
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, BackupContractError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| BackupContractError::Serialization {
            message: error.to_string(),
        })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn write_canonical_new<T: Serialize>(path: &Path, value: &T) -> Result<(), BackupContractError> {
    let json = serde_json::to_value(value).map_err(|error| BackupContractError::Serialization {
        message: error.to_string(),
    })?;
    let bytes =
        canonical_json_bytes(&json).map_err(|error| BackupContractError::Serialization {
            message: error.to_string(),
        })?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| BackupContractError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| BackupContractError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), BackupContractError> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| BackupContractError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), BackupContractError> {
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, BackupContractError> {
    let bytes = fs::read(path).map_err(|error| BackupContractError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    serde_json::from_slice(&bytes).map_err(|error| BackupContractError::Serialization {
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::bench_support::foundation::{
        IndependentContentStoreV1, LogicalCapabilityEpochV1, QualificationEntry,
        QualificationProfile, QualificationRecordKindV1,
    };

    #[derive(Debug, Default)]
    struct MemoryJournal {
        entries: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    impl QualificationJournal for MemoryJournal {
        fn create_once(
            &self,
            logical_key: &str,
            decoded_bytes: &[u8],
        ) -> Result<QualificationCreateOutcome, String> {
            let mut entries = self.entries.lock().map_err(|error| error.to_string())?;
            match entries.get(logical_key) {
                Some(existing) if existing == decoded_bytes => {
                    Ok(QualificationCreateOutcome::AlreadyExists)
                }
                Some(_) => Err(format!("conflict for {logical_key}")),
                None => {
                    entries.insert(logical_key.to_owned(), decoded_bytes.to_vec());
                    Ok(QualificationCreateOutcome::Created)
                }
            }
        }

        fn read(&self, logical_key: &str) -> Result<Option<QualificationEntry>, String> {
            let entries = self.entries.lock().map_err(|error| error.to_string())?;
            Ok(entries.get(logical_key).map(|bytes| QualificationEntry {
                logical_key: logical_key.to_owned(),
                decoded_sha256: crate::canonical_hash::sha256_bytes_hex(bytes),
                decoded_bytes: bytes.clone(),
            }))
        }

        fn list(&self) -> Result<Vec<QualificationEntry>, String> {
            let entries = self.entries.lock().map_err(|error| error.to_string())?;
            Ok(entries
                .iter()
                .map(|(key, bytes)| QualificationEntry {
                    logical_key: key.clone(),
                    decoded_sha256: crate::canonical_hash::sha256_bytes_hex(bytes),
                    decoded_bytes: bytes.clone(),
                })
                .collect())
        }

        fn head_marker(&self) -> Result<u64, String> {
            self.entries
                .lock()
                .map(|entries| entries.len() as u64)
                .map_err(|error| error.to_string())
        }

        fn integrity_check(&self) -> Result<(), String> {
            Ok(())
        }
    }

    fn descriptor() -> QualificationProfileDescriptorV1 {
        QualificationProfileDescriptorV1 {
            physical_profile_id: "qualification-memory-v1".to_owned(),
            logical_capabilities: LogicalCapabilityEpochV1::foundation(),
        }
    }

    #[derive(Debug)]
    struct MemoryProfile {
        descriptor: QualificationProfileDescriptorV1,
        descriptor_read: AtomicBool,
        journal: MemoryJournal,
        content: IndependentContentStoreV1,
    }

    impl QualificationProfile for MemoryProfile {
        fn descriptor(&self) -> Result<QualificationProfileDescriptorV1, String> {
            self.descriptor_read.store(true, Ordering::SeqCst);
            Ok(self.descriptor.clone())
        }

        fn journal(&self) -> &dyn QualificationJournal {
            assert!(
                self.descriptor_read.load(Ordering::SeqCst),
                "descriptor must be validated before child access"
            );
            &self.journal
        }

        fn put_content_once(
            &self,
            content_key: &str,
            record_kind: QualificationRecordKindV1,
            decoded_bytes: &[u8],
        ) -> Result<QualificationCreateOutcome, String> {
            self.content
                .put_once(content_key, record_kind, decoded_bytes)
                .map_err(|error| error.to_string())
        }

        fn read_content(&self, content_key: &str) -> Result<Option<QualificationEntry>, String> {
            self.content
                .read(content_key)
                .map_err(|error| error.to_string())
        }

        fn remove_content(&self, content_key: &str) -> Result<bool, String> {
            self.content
                .remove(content_key)
                .map_err(|error| error.to_string())
        }

        fn backup_to(&self, destination: &Path) -> Result<(), String> {
            publish_completed_backup(destination, &self.descriptor, |root| {
                std::fs::write(root.join("profile.pbst"), b"profile")
                    .map_err(|error| error.to_string())?;
                std::fs::write(root.join("journal.mem"), b"journal")
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .map(|_| ())
            .map_err(|error| error.to_string())
        }

        fn verify_restore(&self, restored_root: &Path) -> Result<(), String> {
            verify_completed_backup(restored_root, &self.descriptor)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }

        fn inventory(&self) -> Result<QualificationInventoryV1, String> {
            let mut inventory = self
                .content
                .inventory()
                .map_err(|error| error.to_string())?;
            inventory
                .carriers
                .splice(0..0, ["journal.mem".to_owned(), "profile.pbst".to_owned()]);
            inventory.carriers.sort();
            inventory.encoded_bytes += 14;
            inventory.allocated_bytes += 14;
            inventory.high_water_bytes += 14;
            Ok(inventory)
        }
    }

    #[test]
    fn journal_contract_driver_enforces_the_shared_semantics() {
        let report =
            run_journal_contract_vectors(&MemoryJournal::default()).expect("journal vectors");

        assert_eq!(report.schema, QUALIFICATION_SCENARIO_RESULT_SCHEMA_V1);
        assert_eq!(report.outcome, QualificationScenarioOutcomeV1::Passed);
        assert_eq!(report.workload.records, 2);
        assert!(report.failure.is_none());
    }

    #[test]
    fn composed_profile_driver_validates_descriptor_children_backup_and_inventory() {
        let profile_root = tempfile::tempdir().expect("profile root");
        let backup_parent = tempfile::tempdir().expect("backup parent");
        let profile = MemoryProfile {
            descriptor: descriptor(),
            descriptor_read: AtomicBool::new(false),
            journal: MemoryJournal::default(),
            content: IndependentContentStoreV1::open(profile_root.path()).expect("content"),
        };

        let report =
            run_profile_contract_vectors(&profile, &backup_parent.path().join("completed-backup"))
                .expect("profile vectors");

        assert_eq!(report.outcome, QualificationScenarioOutcomeV1::Passed);
        assert!(report.inventory.expect("inventory").carriers.len() >= 2);
    }

    #[test]
    fn composed_profile_driver_rejects_capabilities_before_child_access() {
        let profile_root = tempfile::tempdir().expect("profile root");
        let backup_parent = tempfile::tempdir().expect("backup parent");
        let mut invalid_descriptor = descriptor();
        invalid_descriptor.logical_capabilities.required.pop();
        let profile = MemoryProfile {
            descriptor: invalid_descriptor,
            descriptor_read: AtomicBool::new(false),
            journal: MemoryJournal::default(),
            content: IndependentContentStoreV1::open(profile_root.path()).expect("content"),
        };

        assert!(
            run_profile_contract_vectors(&profile, &backup_parent.path().join("backup")).is_err()
        );
        assert_eq!(profile.journal.head_marker().expect("head"), 0);
    }

    #[test]
    fn completed_backup_manifest_is_bound_and_published_last() {
        let parent = tempfile::tempdir().expect("tempdir");
        let destination = parent.path().join("backup");
        let manifest = publish_completed_backup(&destination, &descriptor(), |root| {
            std::fs::create_dir_all(root.join("journal")).map_err(|error| error.to_string())?;
            std::fs::write(root.join("profile.pbst"), b"profile")
                .map_err(|error| error.to_string())?;
            std::fs::write(root.join("journal/data"), b"journal")
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .expect("publish backup");

        assert_eq!(manifest.carriers.len(), 2);
        assert!(destination.join(BACKUP_MANIFEST_FILE_V1).is_file());
        assert!(destination.join(BACKUP_COMPLETION_FILE_V1).is_file());
        assert_eq!(
            verify_completed_backup(&destination, &descriptor()).expect("verify"),
            manifest
        );
    }

    #[test]
    fn completed_backup_rejects_missing_added_changed_and_mismatched_roots() {
        for mutation in ["missing", "added", "changed", "marker"] {
            let parent = tempfile::tempdir().expect("tempdir");
            let destination = parent.path().join("backup");
            publish_completed_backup(&destination, &descriptor(), |root| {
                std::fs::write(root.join("carrier"), b"original").map_err(|error| error.to_string())
            })
            .expect("publish backup");

            match mutation {
                "missing" => std::fs::remove_file(destination.join("carrier")).expect("remove"),
                "added" => std::fs::write(destination.join("added"), b"extra").expect("add"),
                "changed" => {
                    std::fs::write(destination.join("carrier"), b"changed").expect("change")
                }
                "marker" => std::fs::write(
                    destination.join(BACKUP_COMPLETION_FILE_V1),
                    br#"{"schema":"bad"}"#,
                )
                .expect("marker"),
                _ => unreachable!(),
            }

            assert!(
                verify_completed_backup(&destination, &descriptor()).is_err(),
                "{mutation} must fail closed"
            );
        }
    }
}
