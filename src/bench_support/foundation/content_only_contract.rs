use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::contract::QualificationRecordKindV1;
use super::generated::{
    QUALIFICATION_G0_MANIFEST_SHA256_V1, QUALIFICATION_G0_SCHEDULE_SHA256_V1,
    QUALIFICATION_G0_SPEC_SHA256_V1, QUALIFICATION_G1_MANIFEST_SHA256_V1,
    QUALIFICATION_G1_SCHEDULE_SHA256_V1, QUALIFICATION_G1_SPEC_SHA256_V1,
    QUALIFICATION_GENERATOR_SCHEMA_V1, QUALIFICATION_PUBLIC_SEED_HEX_V1,
    QualificationGeneratedWorkloadV1,
};
use crate::canonical_hash::{canonical_json_bytes, sha256_bytes_hex};

pub const QUALIFICATION_CONTENT_ONLY_PROFILE_ID_V1: &str =
    "qualification-loose-journal-pbrf-content-v1";
pub const QUALIFICATION_CONTENT_ONLY_PHYSICAL_PROFILE_ID_V1: u32 = 3;
pub const QUALIFICATION_CONTENT_ONLY_MIN_COMPLETE_SAVINGS_BPS_V1: u16 = 1_000;

pub const QUALIFICATION_CONTENT_ONLY_CONTRACT_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-contract.v1";
pub const QUALIFICATION_CONTENT_ONLY_EVIDENCE_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-evidence.v1";
pub const QUALIFICATION_CONTENT_ONLY_PACKAGE_ADMISSION_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-package-admission.v1";
pub const QUALIFICATION_CONTENT_ONLY_PACKAGE_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-package.v1";
pub const QUALIFICATION_CONTENT_ONLY_EVALUATION_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-evaluation.v1";
pub const QUALIFICATION_CONTENT_ONLY_CONTRACT_PUBLICATION_SCHEMA_V1: &str =
    "pointbreak.qualification-content-only-contract-publication.v1";
pub const QUALIFICATION_CONTENT_ONLY_CONTRACT_PUBLICATION_MODE_V1: &str = "--content-only-contract";
pub const QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1: &str =
    "77ce55dd47363bc924d0612c3b508db92bd7969a0ca9bac8d9c7e096e985f654";

const QUALIFICATION_CONTENT_ONLY_DERIVATION_COMMIT_V1: &str =
    "8ac504bf318b73f3559dcc7c4e25fb827e452bcc";
const QUALIFICATION_CONTENT_ONLY_DERIVATION_TREE_V1: &str =
    "9e422cee6dfd72c463dfce4f2d77db1a8af6c2f8";
const QUALIFICATION_CONTENT_ONLY_DERIVATION_CARGO_LOCK_SHA256_V1: &str =
    "27a080ac57d757d19d0f386fd9d48e00cae1866654beeb112a36d0f07f63327f";
const QUALIFICATION_CONTENT_ONLY_LOGICAL_CAPABILITY_EPOCH_V1: &str = "pointbreak.foundation.v1";
const QUALIFICATION_CONTENT_ONLY_OPERATING_SYSTEM_V1: &str = "macos";
const QUALIFICATION_CONTENT_ONLY_FILESYSTEM_V1: &str = "apfs";
const QUALIFICATION_CONTENT_ONLY_ALLOCATION_API_V1: &str = "stat_blocks_512";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyStatusV1 {
    Passed,
    Failed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyWorkloadRoleV1 {
    G0Admission,
    G1Allocation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyInventoryStateV1 {
    Steady,
    Reopened,
    HighWater,
}

impl QualificationContentOnlyInventoryStateV1 {
    pub const ALL: [Self; 3] = [Self::Steady, Self::Reopened, Self::HighWater];

    fn as_str(self) -> &'static str {
        match self {
            Self::Steady => "steady",
            Self::Reopened => "reopened",
            Self::HighWater => "high-water",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyG0CriterionV1 {
    EventCarrierByteCreateOnceOrderHeadReceiptEquality,
    ContentCreateRetryConflictRead,
    ContentRawZstdRoundTrip,
    ContentAbsenceOrphanRemoval,
    ManifestBlobIndependence,
    ExhaustiveContentValidation,
    InterruptionBeforeAcknowledgement,
    InterruptionAfterAcknowledgement,
    StaleTempCleanupAndSafeReopen,
    ExactDecodedExportImport,
    CompletionLastBackupRestore,
    CopyOutRepair,
    SourcePreservingMigrationRollback,
    Privacy,
    Provenance,
    CompleteInventory,
}

impl QualificationContentOnlyG0CriterionV1 {
    pub const ALL: [Self; 16] = [
        Self::EventCarrierByteCreateOnceOrderHeadReceiptEquality,
        Self::ContentCreateRetryConflictRead,
        Self::ContentRawZstdRoundTrip,
        Self::ContentAbsenceOrphanRemoval,
        Self::ManifestBlobIndependence,
        Self::ExhaustiveContentValidation,
        Self::InterruptionBeforeAcknowledgement,
        Self::InterruptionAfterAcknowledgement,
        Self::StaleTempCleanupAndSafeReopen,
        Self::ExactDecodedExportImport,
        Self::CompletionLastBackupRestore,
        Self::CopyOutRepair,
        Self::SourcePreservingMigrationRollback,
        Self::Privacy,
        Self::Provenance,
        Self::CompleteInventory,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EventCarrierByteCreateOnceOrderHeadReceiptEquality => {
                "event_carrier_byte_create_once_order_head_receipt_equality"
            }
            Self::ContentCreateRetryConflictRead => "content_create_retry_conflict_read",
            Self::ContentRawZstdRoundTrip => "content_raw_zstd_round_trip",
            Self::ContentAbsenceOrphanRemoval => "content_absence_orphan_removal",
            Self::ManifestBlobIndependence => "manifest_blob_independence",
            Self::ExhaustiveContentValidation => "exhaustive_content_validation",
            Self::InterruptionBeforeAcknowledgement => "interruption_before_acknowledgement",
            Self::InterruptionAfterAcknowledgement => "interruption_after_acknowledgement",
            Self::StaleTempCleanupAndSafeReopen => "stale_temp_cleanup_and_safe_reopen",
            Self::ExactDecodedExportImport => "exact_decoded_export_import",
            Self::CompletionLastBackupRestore => "completion_last_backup_restore",
            Self::CopyOutRepair => "copy_out_repair",
            Self::SourcePreservingMigrationRollback => "source_preserving_migration_rollback",
            Self::Privacy => "privacy",
            Self::Provenance => "provenance",
            Self::CompleteInventory => "complete_inventory",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyCreateOutcomeV1 {
    Created,
    AlreadyExists,
    Conflict,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyPackageTargetV1 {
    MacosAarch64,
    MacosX8664,
}

impl QualificationContentOnlyPackageTargetV1 {
    pub const ALL: [Self; 2] = [Self::MacosAarch64, Self::MacosX8664];

    fn as_str(self) -> &'static str {
        match self {
            Self::MacosAarch64 => "macOS aarch64",
            Self::MacosX8664 => "macOS x86_64",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationContentOnlyCriterionKindV1 {
    G0,
    EventEquality,
    CompleteProfileSavings,
    PackageAdmission,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyDerivationV1 {
    pub pointbreak_commit: String,
    pub pointbreak_tree: String,
    pub cargo_lock_sha256: String,
    pub generator_schema: String,
    pub public_seed_hex: String,
    pub candidate_measurements_used: bool,
    pub historical_candidate_results_used: bool,
    pub private_corpus_used: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyContentRequirementV1 {
    pub record_kind: QualificationRecordKindV1,
    pub directory: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyWorkloadRequirementV1 {
    pub workload: QualificationGeneratedWorkloadV1,
    pub role: QualificationContentOnlyWorkloadRoleV1,
    pub generator_spec_sha256: String,
    pub manifest_sha256: String,
    pub operation_schedule_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyPlatformRequirementV1 {
    pub operating_system: String,
    pub filesystem: String,
    pub allocation_api: String,
    pub native_execution_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyCodecRequirementV1 {
    pub format: String,
    pub header_bytes: u16,
    pub adaptive_encoding: String,
    pub zstd_level: i8,
    pub checksum_required: bool,
    pub pledged_decoded_size_required: bool,
    pub dictionary_allowed: bool,
    pub exact_existing_bounds_and_hashes_required: bool,
    pub trailing_bytes_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyPublicationRequirementV1 {
    pub complete_same_directory_temp_required: bool,
    pub durable_create_once_required: bool,
    pub cleanup_required: bool,
    pub parent_directory_durability_required: bool,
    pub retry_compares_decoded_kind_key_and_bytes: bool,
    pub outcomes: Vec<QualificationContentOnlyCreateOutcomeV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyContractV1 {
    pub schema: String,
    pub derivation: QualificationContentOnlyDerivationV1,
    pub profile_id: String,
    pub physical_profile_id: u32,
    pub logical_capability_epoch: String,
    pub profile_descriptor: String,
    pub event_layout: String,
    pub event_requirement: String,
    pub content_layout: String,
    pub content_carrier_format: String,
    pub content_requirements: Vec<QualificationContentOnlyContentRequirementV1>,
    pub codec: QualificationContentOnlyCodecRequirementV1,
    pub publication: QualificationContentOnlyPublicationRequirementV1,
    pub forbidden_profile_shapes: Vec<String>,
    pub workloads: Vec<QualificationContentOnlyWorkloadRequirementV1>,
    pub platform: QualificationContentOnlyPlatformRequirementV1,
    pub independent_runs: u8,
    pub run_indices: Vec<u8>,
    pub inventory_states: Vec<QualificationContentOnlyInventoryStateV1>,
    pub g0_criteria: Vec<QualificationContentOnlyG0CriterionV1>,
    pub minimum_complete_profile_savings_bps: u16,
    pub event_savings_bps: u16,
    pub package_targets: Vec<QualificationContentOnlyPackageTargetV1>,
    pub complete_profile_scope: String,
    pub public_inputs_only: bool,
    pub fresh_disposable_roots_required: bool,
    pub timing_claim_authorized: bool,
    pub recovery_speed_claim_authorized: bool,
    pub physical_profile_selection_authorized: bool,
}

impl QualificationContentOnlyContractV1 {
    fn frozen() -> Self {
        Self {
            schema: QUALIFICATION_CONTENT_ONLY_CONTRACT_SCHEMA_V1.to_owned(),
            derivation: QualificationContentOnlyDerivationV1 {
                pointbreak_commit: QUALIFICATION_CONTENT_ONLY_DERIVATION_COMMIT_V1.to_owned(),
                pointbreak_tree: QUALIFICATION_CONTENT_ONLY_DERIVATION_TREE_V1.to_owned(),
                cargo_lock_sha256: QUALIFICATION_CONTENT_ONLY_DERIVATION_CARGO_LOCK_SHA256_V1
                    .to_owned(),
                generator_schema: QUALIFICATION_GENERATOR_SCHEMA_V1.to_owned(),
                public_seed_hex: QUALIFICATION_PUBLIC_SEED_HEX_V1.to_owned(),
                candidate_measurements_used: false,
                historical_candidate_results_used: false,
                private_corpus_used: false,
            },
            profile_id: QUALIFICATION_CONTENT_ONLY_PROFILE_ID_V1.to_owned(),
            physical_profile_id: QUALIFICATION_CONTENT_ONLY_PHYSICAL_PROFILE_ID_V1,
            logical_capability_epoch: QUALIFICATION_CONTENT_ONLY_LOGICAL_CAPABILITY_EPOCH_V1
                .to_owned(),
            profile_descriptor:
                "profile.pbst: PhysicalStoreHeaderV1 { profile_id: 3, store_uuid }".to_owned(),
            event_layout: "events/<sha256(logical-key)>.json".to_owned(),
            event_requirement:
                "unchanged raw loose carriers and journal receipt; candidate and loose are byte-equal"
                    .to_owned(),
            content_layout:
                "content/{object,note,relation-proof,document-manifest,document-blob}/<hex(logical-key)>.pbrf"
                    .to_owned(),
            content_carrier_format: "one_pbrf_v1_carrier_per_logical_key".to_owned(),
            content_requirements: vec![
                QualificationContentOnlyContentRequirementV1 {
                    record_kind: QualificationRecordKindV1::ObjectArtifact,
                    directory: "object".to_owned(),
                },
                QualificationContentOnlyContentRequirementV1 {
                    record_kind: QualificationRecordKindV1::NoteBody,
                    directory: "note".to_owned(),
                },
                QualificationContentOnlyContentRequirementV1 {
                    record_kind: QualificationRecordKindV1::RelationProof,
                    directory: "relation-proof".to_owned(),
                },
                QualificationContentOnlyContentRequirementV1 {
                    record_kind: QualificationRecordKindV1::DocumentManifest,
                    directory: "document-manifest".to_owned(),
                },
                QualificationContentOnlyContentRequirementV1 {
                    record_kind: QualificationRecordKindV1::DocumentBlob,
                    directory: "document-blob".to_owned(),
                },
            ],
            codec: QualificationContentOnlyCodecRequirementV1 {
                format: "pbrf_v1".to_owned(),
                header_bytes: 192,
                adaptive_encoding: "raw_or_zstd".to_owned(),
                zstd_level: 1,
                checksum_required: true,
                pledged_decoded_size_required: true,
                dictionary_allowed: false,
                exact_existing_bounds_and_hashes_required: true,
                trailing_bytes_allowed: false,
            },
            publication: QualificationContentOnlyPublicationRequirementV1 {
                complete_same_directory_temp_required: true,
                durable_create_once_required: true,
                cleanup_required: true,
                parent_directory_durability_required: true,
                retry_compares_decoded_kind_key_and_bytes: true,
                outcomes: vec![
                    QualificationContentOnlyCreateOutcomeV1::Created,
                    QualificationContentOnlyCreateOutcomeV1::AlreadyExists,
                    QualificationContentOnlyCreateOutcomeV1::Conflict,
                ],
            },
            forbidden_profile_shapes: [
                "mixed_raw_pbrf_content_root",
                "sniffing_fallback",
                "runtime_selector",
                "pack_or_shared_carrier",
                "content_page_or_value_log",
                "journal_change",
                "projection_authority",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            workloads: vec![
                QualificationContentOnlyWorkloadRequirementV1 {
                    workload: QualificationGeneratedWorkloadV1::G0,
                    role: QualificationContentOnlyWorkloadRoleV1::G0Admission,
                    generator_spec_sha256: QUALIFICATION_G0_SPEC_SHA256_V1.to_owned(),
                    manifest_sha256: QUALIFICATION_G0_MANIFEST_SHA256_V1.to_owned(),
                    operation_schedule_sha256: QUALIFICATION_G0_SCHEDULE_SHA256_V1.to_owned(),
                },
                QualificationContentOnlyWorkloadRequirementV1 {
                    workload: QualificationGeneratedWorkloadV1::G1,
                    role: QualificationContentOnlyWorkloadRoleV1::G1Allocation,
                    generator_spec_sha256: QUALIFICATION_G1_SPEC_SHA256_V1.to_owned(),
                    manifest_sha256: QUALIFICATION_G1_MANIFEST_SHA256_V1.to_owned(),
                    operation_schedule_sha256: QUALIFICATION_G1_SCHEDULE_SHA256_V1.to_owned(),
                },
            ],
            platform: QualificationContentOnlyPlatformRequirementV1 {
                operating_system: QUALIFICATION_CONTENT_ONLY_OPERATING_SYSTEM_V1.to_owned(),
                filesystem: QUALIFICATION_CONTENT_ONLY_FILESYSTEM_V1.to_owned(),
                allocation_api: QUALIFICATION_CONTENT_ONLY_ALLOCATION_API_V1.to_owned(),
                native_execution_required: true,
            },
            independent_runs: 2,
            run_indices: vec![0, 1],
            inventory_states: QualificationContentOnlyInventoryStateV1::ALL.to_vec(),
            g0_criteria: QualificationContentOnlyG0CriterionV1::ALL.to_vec(),
            minimum_complete_profile_savings_bps:
                QUALIFICATION_CONTENT_ONLY_MIN_COMPLETE_SAVINGS_BPS_V1,
            event_savings_bps: 0,
            package_targets: QualificationContentOnlyPackageTargetV1::ALL.to_vec(),
            complete_profile_scope: "profile.pbst, events, content, receipts, temporary/orphan, and operational carriers"
                .to_owned(),
            public_inputs_only: true,
            fresh_disposable_roots_required: true,
            timing_claim_authorized: false,
            recovery_speed_claim_authorized: false,
            physical_profile_selection_authorized: false,
        }
    }

    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self != &Self::frozen() {
            return Err("unsupported content-only contract".to_owned());
        }
        if self.canonical_sha256()? != QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1 {
            return Err("content-only contract hash is not frozen".to_owned());
        }
        Ok(())
    }

    pub fn decision_table_markdown(&self) -> String {
        let states = self
            .inventory_states
            .iter()
            .map(|state| state.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let targets = self
            .package_targets
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        [
            "| Decision | Required value |".to_owned(),
            "| --- | --- |".to_owned(),
            format!("| Profile | `{}` |", self.profile_id),
            format!("| Physical profile ID | {} |", self.physical_profile_id),
            format!(
                "| Logical capability epoch | `{}` |",
                self.logical_capability_epoch
            ),
            "| Events | unchanged raw loose carriers and receipt; byte-equal to loose; 0 bps informational only |"
                .to_owned(),
            "| Content | one PBRF v1 carrier per logical key across object, note, relation-proof, document-manifest, and document-blob |"
                .to_owned(),
            format!(
                "| Content codec | {}-byte PBRF v1 header; adaptive raw or zstd level {}; checksum and pledged decoded size; no dictionary or trailing bytes |",
                self.codec.header_bytes, self.codec.zstd_level
            ),
            "| Publication | complete same-directory temp, durable create-once, cleanup, and parent-directory durability; retry compares decoded kind, key, and bytes |"
                .to_owned(),
            "| Public workloads | G0 admission, then G1 allocation; frozen manifests and schedules |"
                .to_owned(),
            format!(
                "| Native platform | {}/{} via `{}` |",
                self.platform.operating_system,
                self.platform.filesystem,
                self.platform.allocation_api
            ),
            format!("| Independent runs | exactly {:?} |", self.run_indices),
            format!("| Allocation states | {states}; each gates independently |"),
            format!(
                "| Complete-profile floor | at least {} basis points in every run/state; no pooling or reruns |",
                self.minimum_complete_profile_savings_bps
            ),
            "| G0 | every named semantic, lifecycle, transfer, repair, migration, privacy, provenance, and inventory row passes |"
                .to_owned(),
            format!("| Package admission | {targets}; required only for aggregate pass |"),
            "| Meaning | bounded content-only APFS falsifier; no timing, recovery-speed, physical-profile selection, migration, activation, or rollout claim |"
                .to_owned(),
        ]
        .join("\n")
    }
}

pub fn qualification_content_only_contract_v1() -> QualificationContentOnlyContractV1 {
    QualificationContentOnlyContractV1::frozen()
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyContractPublicationV1 {
    pub schema: String,
    pub mode: String,
    pub contract: QualificationContentOnlyContractV1,
    pub contract_sha256: String,
    pub decision_table_markdown: String,
}

pub fn qualification_content_only_contract_v1_publication()
-> QualificationContentOnlyContractPublicationV1 {
    let contract = qualification_content_only_contract_v1();
    QualificationContentOnlyContractPublicationV1 {
        schema: QUALIFICATION_CONTENT_ONLY_CONTRACT_PUBLICATION_SCHEMA_V1.to_owned(),
        mode: "non_timing_contract_publication".to_owned(),
        contract_sha256: contract
            .canonical_sha256()
            .expect("the frozen content-only contract is canonical"),
        decision_table_markdown: contract.decision_table_markdown(),
        contract,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyExecutionIdentityV1 {
    pub source_commit: String,
    pub source_tree: String,
    pub source_dirty: bool,
    pub cargo_lock_sha256: String,
    pub contract_schema: String,
    pub contract_sha256: String,
    pub profile_id: String,
    pub physical_profile_id: u32,
    pub logical_capability_epoch: String,
    pub profile_source_sha256: String,
    pub codec_source_sha256: String,
    pub runner_source_sha256: String,
    pub generator_schema: String,
    pub public_seed_hex: String,
    pub g0_generator_spec_sha256: String,
    pub g0_manifest_sha256: String,
    pub g0_operation_schedule_sha256: String,
    pub g1_generator_spec_sha256: String,
    pub g1_manifest_sha256: String,
    pub g1_operation_schedule_sha256: String,
    pub operating_system: String,
    pub architecture: String,
    pub filesystem: String,
    pub allocation_api: String,
    pub private_corpus_configured: bool,
}

impl QualificationContentOnlyExecutionIdentityV1 {
    pub fn validate(&self) -> Result<(), String> {
        validate_hex(&self.source_commit, 40, "source commit")?;
        validate_hex(&self.source_tree, 40, "source tree")?;
        validate_hex(&self.cargo_lock_sha256, 64, "Cargo.lock SHA-256")?;
        validate_hex(&self.contract_sha256, 64, "contract SHA-256")?;
        validate_hex(&self.profile_source_sha256, 64, "profile source SHA-256")?;
        validate_hex(&self.codec_source_sha256, 64, "codec source SHA-256")?;
        validate_hex(&self.runner_source_sha256, 64, "runner source SHA-256")?;
        for (hash, name) in [
            (&self.g0_generator_spec_sha256, "G0 generator spec SHA-256"),
            (&self.g0_manifest_sha256, "G0 manifest SHA-256"),
            (&self.g0_operation_schedule_sha256, "G0 schedule SHA-256"),
            (&self.g1_generator_spec_sha256, "G1 generator spec SHA-256"),
            (&self.g1_manifest_sha256, "G1 manifest SHA-256"),
            (&self.g1_operation_schedule_sha256, "G1 schedule SHA-256"),
        ] {
            validate_hex(hash, 64, name)?;
        }
        validate_hex(&self.public_seed_hex, 64, "public seed")?;
        if self.source_dirty
            || self.private_corpus_configured
            || self.contract_schema != QUALIFICATION_CONTENT_ONLY_CONTRACT_SCHEMA_V1
            || self.contract_sha256 != QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1
            || self.profile_id != QUALIFICATION_CONTENT_ONLY_PROFILE_ID_V1
            || self.physical_profile_id != QUALIFICATION_CONTENT_ONLY_PHYSICAL_PROFILE_ID_V1
            || self.logical_capability_epoch
                != QUALIFICATION_CONTENT_ONLY_LOGICAL_CAPABILITY_EPOCH_V1
            || self.generator_schema != QUALIFICATION_GENERATOR_SCHEMA_V1
            || self.public_seed_hex != QUALIFICATION_PUBLIC_SEED_HEX_V1
            || self.g0_generator_spec_sha256 != QUALIFICATION_G0_SPEC_SHA256_V1
            || self.g0_manifest_sha256 != QUALIFICATION_G0_MANIFEST_SHA256_V1
            || self.g0_operation_schedule_sha256 != QUALIFICATION_G0_SCHEDULE_SHA256_V1
            || self.g1_generator_spec_sha256 != QUALIFICATION_G1_SPEC_SHA256_V1
            || self.g1_manifest_sha256 != QUALIFICATION_G1_MANIFEST_SHA256_V1
            || self.g1_operation_schedule_sha256 != QUALIFICATION_G1_SCHEDULE_SHA256_V1
            || self.operating_system != QUALIFICATION_CONTENT_ONLY_OPERATING_SYSTEM_V1
            || self.filesystem != QUALIFICATION_CONTENT_ONLY_FILESYSTEM_V1
            || self.allocation_api != QUALIFICATION_CONTENT_ONLY_ALLOCATION_API_V1
            || !matches!(self.architecture.as_str(), "aarch64" | "x86_64")
        {
            return Err("content-only execution identity is stale or unsupported".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyG0RowV1 {
    pub criterion: QualificationContentOnlyG0CriterionV1,
    pub status: QualificationContentOnlyStatusV1,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyG0ReceiptV1 {
    pub schema: String,
    pub identity: QualificationContentOnlyExecutionIdentityV1,
    pub rows: Vec<QualificationContentOnlyG0RowV1>,
    pub evidence_sha256: String,
}

impl QualificationContentOnlyG0ReceiptV1 {
    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256_without(&self.evidence_sha256, |evidence_sha256| {
            let mut preimage = self.clone();
            preimage.evidence_sha256 = evidence_sha256;
            preimage
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != QUALIFICATION_CONTENT_ONLY_EVIDENCE_SCHEMA_V1 {
            return Err("unsupported content-only G0 evidence schema".to_owned());
        }
        self.identity.validate()?;
        validate_rows_once(
            self.rows.iter().map(|row| row.criterion),
            "content-only G0 criterion",
        )?;
        for row in &self.rows {
            validate_hex(&row.receipt_sha256, 64, "G0 receipt SHA-256")?;
        }
        validate_bound_hash(
            &self.evidence_sha256,
            &self.canonical_sha256()?,
            "content-only G0 evidence",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyEventEqualityV1 {
    pub candidate_carrier_set_sha256: String,
    pub baseline_carrier_set_sha256: String,
    pub candidate_receipt_sha256: String,
    pub baseline_receipt_sha256: String,
}

impl QualificationContentOnlyEventEqualityV1 {
    fn validate(&self) -> Result<(), String> {
        for (hash, name) in [
            (
                &self.candidate_carrier_set_sha256,
                "candidate event carrier-set SHA-256",
            ),
            (
                &self.baseline_carrier_set_sha256,
                "baseline event carrier-set SHA-256",
            ),
            (
                &self.candidate_receipt_sha256,
                "candidate event receipt SHA-256",
            ),
            (
                &self.baseline_receipt_sha256,
                "baseline event receipt SHA-256",
            ),
        ] {
            validate_hex(hash, 64, name)?;
        }
        Ok(())
    }

    fn is_equal(&self) -> bool {
        self.candidate_carrier_set_sha256 == self.baseline_carrier_set_sha256
            && self.candidate_receipt_sha256 == self.baseline_receipt_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyAllocationRowV1 {
    pub state: QualificationContentOnlyInventoryStateV1,
    pub candidate_complete_profile_allocated_bytes: u64,
    pub baseline_complete_profile_allocated_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyApfsRunV1 {
    pub schema: String,
    pub identity: QualificationContentOnlyExecutionIdentityV1,
    pub run_index: u8,
    pub fresh_disposable_root: bool,
    pub native_execution: bool,
    pub event_equality: QualificationContentOnlyEventEqualityV1,
    pub allocations: Vec<QualificationContentOnlyAllocationRowV1>,
    pub evidence_sha256: String,
}

impl QualificationContentOnlyApfsRunV1 {
    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256_without(&self.evidence_sha256, |evidence_sha256| {
            let mut preimage = self.clone();
            preimage.evidence_sha256 = evidence_sha256;
            preimage
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != QUALIFICATION_CONTENT_ONLY_EVIDENCE_SCHEMA_V1 {
            return Err("unsupported content-only APFS evidence schema".to_owned());
        }
        self.identity.validate()?;
        if ![0, 1].contains(&self.run_index) {
            return Err("unsupported content-only APFS run index".to_owned());
        }
        if !self.fresh_disposable_root || !self.native_execution {
            return Err("content-only APFS evidence is not native on a fresh root".to_owned());
        }
        self.event_equality.validate()?;
        validate_rows_once(
            self.allocations.iter().map(|row| row.state),
            "content-only allocation state",
        )?;
        if self.allocations.iter().any(|row| {
            row.candidate_complete_profile_allocated_bytes == 0
                || row.baseline_complete_profile_allocated_bytes == 0
        }) {
            return Err("content-only allocation row is incomplete".to_owned());
        }
        validate_bound_hash(
            &self.evidence_sha256,
            &self.canonical_sha256()?,
            "content-only APFS evidence",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyPackageAdmissionRowV1 {
    pub target: QualificationContentOnlyPackageTargetV1,
    pub status: QualificationContentOnlyStatusV1,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyPackageAdmissionV1 {
    pub schema: String,
    pub identity: QualificationContentOnlyExecutionIdentityV1,
    pub rows: Vec<QualificationContentOnlyPackageAdmissionRowV1>,
    pub evidence_sha256: String,
}

impl QualificationContentOnlyPackageAdmissionV1 {
    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256_without(&self.evidence_sha256, |evidence_sha256| {
            let mut preimage = self.clone();
            preimage.evidence_sha256 = evidence_sha256;
            preimage
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != QUALIFICATION_CONTENT_ONLY_PACKAGE_ADMISSION_SCHEMA_V1 {
            return Err("unsupported content-only package-admission schema".to_owned());
        }
        self.identity.validate()?;
        validate_rows_once(
            self.rows.iter().map(|row| row.target),
            "content-only package target",
        )?;
        for row in &self.rows {
            validate_hex(&row.receipt_sha256, 64, "package receipt SHA-256")?;
        }
        validate_bound_hash(
            &self.evidence_sha256,
            &self.canonical_sha256()?,
            "content-only package-admission evidence",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyPackageV1 {
    pub schema: String,
    pub identity: QualificationContentOnlyExecutionIdentityV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub g0_receipt: Option<QualificationContentOnlyG0ReceiptV1>,
    pub apfs_runs: Vec<QualificationContentOnlyApfsRunV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_admission: Option<QualificationContentOnlyPackageAdmissionV1>,
    pub package_sha256: String,
}

impl QualificationContentOnlyPackageV1 {
    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256_without(&self.package_sha256, |package_sha256| {
            let mut preimage = self.clone();
            preimage.package_sha256 = package_sha256;
            preimage
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != QUALIFICATION_CONTENT_ONLY_PACKAGE_SCHEMA_V1 {
            return Err("unsupported content-only package schema".to_owned());
        }
        self.identity.validate()?;
        if let Some(receipt) = &self.g0_receipt {
            receipt.validate()?;
            require_same_identity(&self.identity, &receipt.identity, "G0 evidence")?;
        }
        validate_rows_once(
            self.apfs_runs.iter().map(|run| run.run_index),
            "content-only APFS run",
        )?;
        for run in &self.apfs_runs {
            run.validate()?;
            require_same_identity(&self.identity, &run.identity, "APFS evidence")?;
        }
        if let Some(admission) = &self.package_admission {
            admission.validate()?;
            require_same_identity(&self.identity, &admission.identity, "package admission")?;
        }
        validate_bound_hash(
            &self.package_sha256,
            &self.canonical_sha256()?,
            "content-only package",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyCriterionResultV1 {
    pub kind: QualificationContentOnlyCriterionKindV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub g0_criterion: Option<QualificationContentOnlyG0CriterionV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<QualificationContentOnlyInventoryStateV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_target: Option<QualificationContentOnlyPackageTargetV1>,
    pub status: QualificationContentOnlyStatusV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_allocated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_allocated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub savings_bps: Option<i64>,
    pub interpretation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationContentOnlyEvaluationV1 {
    pub schema: String,
    pub contract_sha256: String,
    pub package_sha256: String,
    pub criteria: Vec<QualificationContentOnlyCriterionResultV1>,
    pub status: QualificationContentOnlyStatusV1,
    pub evaluation_sha256: String,
}

impl QualificationContentOnlyEvaluationV1 {
    pub fn canonical_sha256(&self) -> Result<String, String> {
        canonical_sha256_without(&self.evaluation_sha256, |evaluation_sha256| {
            let mut preimage = self.clone();
            preimage.evaluation_sha256 = evaluation_sha256;
            preimage
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != QUALIFICATION_CONTENT_ONLY_EVALUATION_SCHEMA_V1
            || self.contract_sha256 != QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1
        {
            return Err("unsupported content-only evaluation identity".to_owned());
        }
        validate_hex(&self.package_sha256, 64, "evaluation package SHA-256")?;
        if aggregate_status(self.criteria.iter().map(|criterion| criterion.status)) != self.status {
            return Err("content-only evaluation aggregate does not match its criteria".to_owned());
        }
        validate_bound_hash(
            &self.evaluation_sha256,
            &self.canonical_sha256()?,
            "content-only evaluation",
        )
    }
}

pub fn evaluate_qualification_content_only_v1(
    package: &QualificationContentOnlyPackageV1,
) -> Result<QualificationContentOnlyEvaluationV1, String> {
    let contract = qualification_content_only_contract_v1();
    contract.validate()?;
    package.validate()?;

    let mut criteria = Vec::new();
    for required in &contract.g0_criteria {
        let status = package
            .g0_receipt
            .as_ref()
            .and_then(|receipt| receipt.rows.iter().find(|row| row.criterion == *required))
            .map_or(QualificationContentOnlyStatusV1::Unknown, |row| row.status);
        criteria.push(QualificationContentOnlyCriterionResultV1 {
            kind: QualificationContentOnlyCriterionKindV1::G0,
            g0_criterion: Some(*required),
            run_index: None,
            state: None,
            package_target: None,
            status,
            candidate_allocated_bytes: None,
            baseline_allocated_bytes: None,
            savings_bps: None,
            interpretation: format!("G0 {} gates independently", required.as_str()),
        });
    }

    for run_index in &contract.run_indices {
        let run = package
            .apfs_runs
            .iter()
            .find(|run| run.run_index == *run_index);
        let event_status = run.map_or(QualificationContentOnlyStatusV1::Unknown, |run| {
            if run.event_equality.is_equal() {
                QualificationContentOnlyStatusV1::Passed
            } else {
                QualificationContentOnlyStatusV1::Failed
            }
        });
        criteria.push(QualificationContentOnlyCriterionResultV1 {
            kind: QualificationContentOnlyCriterionKindV1::EventEquality,
            g0_criterion: None,
            run_index: Some(*run_index),
            state: None,
            package_target: None,
            status: event_status,
            candidate_allocated_bytes: None,
            baseline_allocated_bytes: None,
            savings_bps: run.map(|_| i64::from(contract.event_savings_bps)),
            interpretation:
                "event carriers and receipt must be byte-equal; 0 bps is informational only"
                    .to_owned(),
        });

        for state in &contract.inventory_states {
            let allocation = run.and_then(|run| {
                run.allocations
                    .iter()
                    .find(|allocation| allocation.state == *state)
            });
            let (status, candidate, baseline, savings_bps) = match allocation {
                Some(allocation) => {
                    let savings_bps = checked_savings_bps(
                        allocation.candidate_complete_profile_allocated_bytes,
                        allocation.baseline_complete_profile_allocated_bytes,
                    )?;
                    let status = if savings_bps
                        >= i64::from(contract.minimum_complete_profile_savings_bps)
                    {
                        QualificationContentOnlyStatusV1::Passed
                    } else {
                        QualificationContentOnlyStatusV1::Failed
                    };
                    (
                        status,
                        Some(allocation.candidate_complete_profile_allocated_bytes),
                        Some(allocation.baseline_complete_profile_allocated_bytes),
                        Some(savings_bps),
                    )
                }
                None => (QualificationContentOnlyStatusV1::Unknown, None, None, None),
            };
            criteria.push(QualificationContentOnlyCriterionResultV1 {
                kind: QualificationContentOnlyCriterionKindV1::CompleteProfileSavings,
                g0_criterion: None,
                run_index: Some(*run_index),
                state: Some(*state),
                package_target: None,
                status,
                candidate_allocated_bytes: candidate,
                baseline_allocated_bytes: baseline,
                savings_bps,
                interpretation: format!(
                    "run {run_index} {} complete-profile allocation gates independently at {} bps",
                    state.as_str(),
                    contract.minimum_complete_profile_savings_bps
                ),
            });
        }
    }

    for target in &contract.package_targets {
        let status = package
            .package_admission
            .as_ref()
            .and_then(|admission| admission.rows.iter().find(|row| row.target == *target))
            .map_or(QualificationContentOnlyStatusV1::Unknown, |row| row.status);
        criteria.push(QualificationContentOnlyCriterionResultV1 {
            kind: QualificationContentOnlyCriterionKindV1::PackageAdmission,
            g0_criterion: None,
            run_index: None,
            state: None,
            package_target: Some(*target),
            status,
            candidate_allocated_bytes: None,
            baseline_allocated_bytes: None,
            savings_bps: None,
            interpretation: format!("{} package and provenance admission", target.as_str()),
        });
    }

    let status = aggregate_status(criteria.iter().map(|criterion| criterion.status));
    let mut evaluation = QualificationContentOnlyEvaluationV1 {
        schema: QUALIFICATION_CONTENT_ONLY_EVALUATION_SCHEMA_V1.to_owned(),
        contract_sha256: QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1.to_owned(),
        package_sha256: package.package_sha256.clone(),
        criteria,
        status,
        evaluation_sha256: String::new(),
    };
    evaluation.evaluation_sha256 = evaluation.canonical_sha256()?;
    Ok(evaluation)
}

fn aggregate_status(
    statuses: impl IntoIterator<Item = QualificationContentOnlyStatusV1>,
) -> QualificationContentOnlyStatusV1 {
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses.contains(&QualificationContentOnlyStatusV1::Failed) {
        QualificationContentOnlyStatusV1::Failed
    } else if statuses.contains(&QualificationContentOnlyStatusV1::Unknown) {
        QualificationContentOnlyStatusV1::Unknown
    } else {
        QualificationContentOnlyStatusV1::Passed
    }
}

fn checked_savings_bps(candidate: u64, baseline: u64) -> Result<i64, String> {
    if baseline == 0 {
        return Err("complete-profile baseline allocation is zero".to_owned());
    }
    let difference = i128::from(baseline)
        .checked_sub(i128::from(candidate))
        .ok_or_else(|| "complete-profile allocation subtraction overflowed".to_owned())?;
    let scaled = difference
        .checked_mul(10_000)
        .ok_or_else(|| "complete-profile savings calculation overflowed".to_owned())?;
    let basis_points = scaled
        .checked_div(i128::from(baseline))
        .ok_or_else(|| "complete-profile savings division failed".to_owned())?;
    i64::try_from(basis_points)
        .map_err(|_| "complete-profile savings result is out of range".to_owned())
}

fn require_same_identity(
    package: &QualificationContentOnlyExecutionIdentityV1,
    evidence: &QualificationContentOnlyExecutionIdentityV1,
    kind: &str,
) -> Result<(), String> {
    if package != evidence {
        return Err(format!("{kind} has mixed execution identity"));
    }
    Ok(())
}

fn validate_rows_once<T>(rows: impl IntoIterator<Item = T>, name: &str) -> Result<(), String>
where
    T: Copy + Ord,
{
    let mut seen = BTreeSet::new();
    if rows.into_iter().any(|row| !seen.insert(row)) {
        return Err(format!("duplicate {name}"));
    }
    Ok(())
}

fn validate_bound_hash(provided: &str, expected: &str, name: &str) -> Result<(), String> {
    validate_hex(provided, 64, &format!("{name} SHA-256"))?;
    if provided != expected {
        return Err(format!("{name} hash mismatch"));
    }
    Ok(())
}

fn validate_hex(value: &str, length: usize, name: &str) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{name} must be {length} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    canonical_json_bytes(&value)
        .map(|bytes| sha256_bytes_hex(&bytes))
        .map_err(|error| error.to_string())
}

fn canonical_sha256_without<T: Serialize>(
    _current: &str,
    preimage: impl FnOnce(String) -> T,
) -> Result<String, String> {
    canonical_sha256(&preimage(String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench_support::foundation::{
        QUALIFICATION_PERFORMANCE_CONTRACT_SHA256_V2, QUALIFICATION_PROSPECTIVE_CONTRACT_SHA256_V1,
        qualification_performance_contract_v2_publication,
        qualification_prospective_contract_v1_publication,
    };

    fn digest(label: &str) -> String {
        sha256_bytes_hex(label.as_bytes())
    }

    fn identity() -> QualificationContentOnlyExecutionIdentityV1 {
        QualificationContentOnlyExecutionIdentityV1 {
            source_commit: QUALIFICATION_CONTENT_ONLY_DERIVATION_COMMIT_V1.to_owned(),
            source_tree: QUALIFICATION_CONTENT_ONLY_DERIVATION_TREE_V1.to_owned(),
            source_dirty: false,
            cargo_lock_sha256: QUALIFICATION_CONTENT_ONLY_DERIVATION_CARGO_LOCK_SHA256_V1
                .to_owned(),
            contract_schema: QUALIFICATION_CONTENT_ONLY_CONTRACT_SCHEMA_V1.to_owned(),
            contract_sha256: QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1.to_owned(),
            profile_id: QUALIFICATION_CONTENT_ONLY_PROFILE_ID_V1.to_owned(),
            physical_profile_id: QUALIFICATION_CONTENT_ONLY_PHYSICAL_PROFILE_ID_V1,
            logical_capability_epoch: QUALIFICATION_CONTENT_ONLY_LOGICAL_CAPABILITY_EPOCH_V1
                .to_owned(),
            profile_source_sha256: digest("content-only profile source"),
            codec_source_sha256: digest("PBRF codec source"),
            runner_source_sha256: digest("content-only runner source"),
            generator_schema: QUALIFICATION_GENERATOR_SCHEMA_V1.to_owned(),
            public_seed_hex: QUALIFICATION_PUBLIC_SEED_HEX_V1.to_owned(),
            g0_generator_spec_sha256: QUALIFICATION_G0_SPEC_SHA256_V1.to_owned(),
            g0_manifest_sha256: QUALIFICATION_G0_MANIFEST_SHA256_V1.to_owned(),
            g0_operation_schedule_sha256: QUALIFICATION_G0_SCHEDULE_SHA256_V1.to_owned(),
            g1_generator_spec_sha256: QUALIFICATION_G1_SPEC_SHA256_V1.to_owned(),
            g1_manifest_sha256: QUALIFICATION_G1_MANIFEST_SHA256_V1.to_owned(),
            g1_operation_schedule_sha256: QUALIFICATION_G1_SCHEDULE_SHA256_V1.to_owned(),
            operating_system: QUALIFICATION_CONTENT_ONLY_OPERATING_SYSTEM_V1.to_owned(),
            architecture: "aarch64".to_owned(),
            filesystem: QUALIFICATION_CONTENT_ONLY_FILESYSTEM_V1.to_owned(),
            allocation_api: QUALIFICATION_CONTENT_ONLY_ALLOCATION_API_V1.to_owned(),
            private_corpus_configured: false,
        }
    }

    fn g0_receipt(
        identity: &QualificationContentOnlyExecutionIdentityV1,
    ) -> QualificationContentOnlyG0ReceiptV1 {
        let mut receipt = QualificationContentOnlyG0ReceiptV1 {
            schema: QUALIFICATION_CONTENT_ONLY_EVIDENCE_SCHEMA_V1.to_owned(),
            identity: identity.clone(),
            rows: QualificationContentOnlyG0CriterionV1::ALL
                .into_iter()
                .map(|criterion| QualificationContentOnlyG0RowV1 {
                    criterion,
                    status: QualificationContentOnlyStatusV1::Passed,
                    receipt_sha256: digest(criterion.as_str()),
                })
                .collect(),
            evidence_sha256: String::new(),
        };
        receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 evidence hash");
        receipt
    }

    fn apfs_run(
        identity: &QualificationContentOnlyExecutionIdentityV1,
        run_index: u8,
    ) -> QualificationContentOnlyApfsRunV1 {
        let event_hash = digest(&format!("event-{run_index}"));
        let receipt_hash = digest(&format!("receipt-{run_index}"));
        let mut run = QualificationContentOnlyApfsRunV1 {
            schema: QUALIFICATION_CONTENT_ONLY_EVIDENCE_SCHEMA_V1.to_owned(),
            identity: identity.clone(),
            run_index,
            fresh_disposable_root: true,
            native_execution: true,
            event_equality: QualificationContentOnlyEventEqualityV1 {
                candidate_carrier_set_sha256: event_hash.clone(),
                baseline_carrier_set_sha256: event_hash,
                candidate_receipt_sha256: receipt_hash.clone(),
                baseline_receipt_sha256: receipt_hash,
            },
            allocations: QualificationContentOnlyInventoryStateV1::ALL
                .into_iter()
                .map(|state| QualificationContentOnlyAllocationRowV1 {
                    state,
                    candidate_complete_profile_allocated_bytes: 9_000,
                    baseline_complete_profile_allocated_bytes: 10_000,
                })
                .collect(),
            evidence_sha256: String::new(),
        };
        run.evidence_sha256 = run.canonical_sha256().expect("APFS evidence hash");
        run
    }

    fn package_admission(
        identity: &QualificationContentOnlyExecutionIdentityV1,
    ) -> QualificationContentOnlyPackageAdmissionV1 {
        let mut admission = QualificationContentOnlyPackageAdmissionV1 {
            schema: QUALIFICATION_CONTENT_ONLY_PACKAGE_ADMISSION_SCHEMA_V1.to_owned(),
            identity: identity.clone(),
            rows: QualificationContentOnlyPackageTargetV1::ALL
                .into_iter()
                .map(|target| QualificationContentOnlyPackageAdmissionRowV1 {
                    target,
                    status: QualificationContentOnlyStatusV1::Passed,
                    receipt_sha256: digest(target.as_str()),
                })
                .collect(),
            evidence_sha256: String::new(),
        };
        admission.evidence_sha256 = admission
            .canonical_sha256()
            .expect("package-admission evidence hash");
        admission
    }

    fn complete_package() -> QualificationContentOnlyPackageV1 {
        let identity = identity();
        let mut package = QualificationContentOnlyPackageV1 {
            schema: QUALIFICATION_CONTENT_ONLY_PACKAGE_SCHEMA_V1.to_owned(),
            identity: identity.clone(),
            g0_receipt: Some(g0_receipt(&identity)),
            apfs_runs: vec![apfs_run(&identity, 0), apfs_run(&identity, 1)],
            package_admission: Some(package_admission(&identity)),
            package_sha256: String::new(),
        };
        rehash_package(&mut package);
        package
    }

    fn rehash_package(package: &mut QualificationContentOnlyPackageV1) {
        package.package_sha256 = package.canonical_sha256().expect("package hash");
    }

    fn result_for(
        evaluation: &QualificationContentOnlyEvaluationV1,
        kind: QualificationContentOnlyCriterionKindV1,
        run_index: Option<u8>,
        state: Option<QualificationContentOnlyInventoryStateV1>,
    ) -> &QualificationContentOnlyCriterionResultV1 {
        evaluation
            .criteria
            .iter()
            .find(|criterion| {
                criterion.kind == kind
                    && criterion.run_index == run_index
                    && criterion.state == state
            })
            .expect("criterion result")
    }

    #[test]
    fn content_only_contract_v1_freezes_the_approved_profile_and_gate_shape() {
        let contract = qualification_content_only_contract_v1();

        assert_eq!(
            QUALIFICATION_CONTENT_ONLY_PROFILE_ID_V1,
            "qualification-loose-journal-pbrf-content-v1"
        );
        assert_eq!(QUALIFICATION_CONTENT_ONLY_PHYSICAL_PROFILE_ID_V1, 3);
        assert_eq!(
            QUALIFICATION_CONTENT_ONLY_MIN_COMPLETE_SAVINGS_BPS_V1,
            1_000
        );
        assert_eq!(
            contract.logical_capability_epoch,
            "pointbreak.foundation.v1"
        );
        assert_eq!(
            contract.derivation.pointbreak_commit,
            "8ac504bf318b73f3559dcc7c4e25fb827e452bcc"
        );
        assert_eq!(
            contract.derivation.pointbreak_tree,
            "9e422cee6dfd72c463dfce4f2d77db1a8af6c2f8"
        );
        assert_eq!(
            contract.derivation.cargo_lock_sha256,
            "27a080ac57d757d19d0f386fd9d48e00cae1866654beeb112a36d0f07f63327f"
        );
        assert_eq!(
            contract
                .workloads
                .iter()
                .map(|workload| workload.workload)
                .collect::<Vec<_>>(),
            [
                QualificationGeneratedWorkloadV1::G0,
                QualificationGeneratedWorkloadV1::G1
            ]
        );
        assert_eq!(contract.platform.operating_system, "macos");
        assert_eq!(contract.platform.filesystem, "apfs");
        assert_eq!(contract.platform.allocation_api, "stat_blocks_512");
        assert_eq!(contract.independent_runs, 2);
        assert_eq!(contract.run_indices, [0, 1]);
        assert_eq!(
            contract.inventory_states,
            QualificationContentOnlyInventoryStateV1::ALL
        );
        assert_eq!(contract.content_requirements.len(), 5);
        assert_eq!(contract.codec.header_bytes, 192);
        assert_eq!(contract.codec.adaptive_encoding, "raw_or_zstd");
        assert_eq!(contract.codec.zstd_level, 1);
        assert!(contract.codec.checksum_required);
        assert!(contract.codec.pledged_decoded_size_required);
        assert!(!contract.codec.dictionary_allowed);
        assert!(!contract.codec.trailing_bytes_allowed);
        assert_eq!(
            contract.publication.outcomes,
            [
                QualificationContentOnlyCreateOutcomeV1::Created,
                QualificationContentOnlyCreateOutcomeV1::AlreadyExists,
                QualificationContentOnlyCreateOutcomeV1::Conflict,
            ]
        );
        assert_eq!(contract.forbidden_profile_shapes.len(), 7);
        assert_eq!(contract.event_savings_bps, 0);
        assert!(!contract.timing_claim_authorized);
        assert!(!contract.recovery_speed_claim_authorized);
        assert!(!contract.physical_profile_selection_authorized);
        assert_eq!(
            contract.canonical_sha256().expect("contract hash"),
            QUALIFICATION_CONTENT_ONLY_CONTRACT_SHA256_V1
        );
        contract.validate().expect("frozen content-only contract");
    }

    #[test]
    fn content_only_g0_rows_fail_and_remain_unknown_independently() {
        for criterion in QualificationContentOnlyG0CriterionV1::ALL {
            let mut failed = complete_package();
            let receipt = failed.g0_receipt.as_mut().expect("G0 receipt");
            receipt
                .rows
                .iter_mut()
                .find(|row| row.criterion == criterion)
                .expect("G0 row")
                .status = QualificationContentOnlyStatusV1::Failed;
            receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 hash");
            rehash_package(&mut failed);
            let evaluation =
                evaluate_qualification_content_only_v1(&failed).expect("failed evaluation");
            assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Failed);
            assert_eq!(
                evaluation
                    .criteria
                    .iter()
                    .find(|row| row.g0_criterion == Some(criterion))
                    .expect("G0 evaluation row")
                    .status,
                QualificationContentOnlyStatusV1::Failed
            );

            let mut absent = complete_package();
            let receipt = absent.g0_receipt.as_mut().expect("G0 receipt");
            receipt.rows.retain(|row| row.criterion != criterion);
            receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 hash");
            rehash_package(&mut absent);
            let evaluation =
                evaluate_qualification_content_only_v1(&absent).expect("unknown evaluation");
            assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Unknown);
            assert_eq!(
                evaluation
                    .criteria
                    .iter()
                    .find(|row| row.g0_criterion == Some(criterion))
                    .expect("G0 evaluation row")
                    .status,
                QualificationContentOnlyStatusV1::Unknown
            );
        }
    }

    #[test]
    fn content_only_allocation_floor_gates_every_run_and_state_independently() {
        for run_index in [0, 1] {
            for state in QualificationContentOnlyInventoryStateV1::ALL {
                let passed = complete_package();
                let evaluation =
                    evaluate_qualification_content_only_v1(&passed).expect("passing evaluation");
                let row = result_for(
                    &evaluation,
                    QualificationContentOnlyCriterionKindV1::CompleteProfileSavings,
                    Some(run_index),
                    Some(state),
                );
                assert_eq!(row.status, QualificationContentOnlyStatusV1::Passed);
                assert_eq!(row.savings_bps, Some(1_000));

                let mut failed = complete_package();
                let run = failed
                    .apfs_runs
                    .iter_mut()
                    .find(|run| run.run_index == run_index)
                    .expect("APFS run");
                run.allocations
                    .iter_mut()
                    .find(|row| row.state == state)
                    .expect("allocation row")
                    .candidate_complete_profile_allocated_bytes = 9_001;
                run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
                rehash_package(&mut failed);
                let evaluation =
                    evaluate_qualification_content_only_v1(&failed).expect("failed evaluation");
                let row = result_for(
                    &evaluation,
                    QualificationContentOnlyCriterionKindV1::CompleteProfileSavings,
                    Some(run_index),
                    Some(state),
                );
                assert_eq!(row.status, QualificationContentOnlyStatusV1::Failed);
                assert_eq!(row.savings_bps, Some(999));
                assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Failed);
            }
        }
    }

    #[test]
    fn content_only_missing_runs_and_allocation_states_are_unknown() {
        let mut missing_run = complete_package();
        missing_run.apfs_runs.retain(|run| run.run_index != 1);
        rehash_package(&mut missing_run);
        let evaluation =
            evaluate_qualification_content_only_v1(&missing_run).expect("missing run evaluation");
        assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Unknown);
        assert_eq!(
            result_for(
                &evaluation,
                QualificationContentOnlyCriterionKindV1::EventEquality,
                Some(1),
                None
            )
            .status,
            QualificationContentOnlyStatusV1::Unknown
        );

        let mut missing_state = complete_package();
        let run = &mut missing_state.apfs_runs[0];
        run.allocations
            .retain(|row| row.state != QualificationContentOnlyInventoryStateV1::Reopened);
        run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
        rehash_package(&mut missing_state);
        let evaluation = evaluate_qualification_content_only_v1(&missing_state)
            .expect("missing state evaluation");
        assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Unknown);
    }

    #[test]
    fn content_only_event_equality_is_required_and_zero_savings_is_informational() {
        let package = complete_package();
        let evaluation =
            evaluate_qualification_content_only_v1(&package).expect("passing evaluation");
        for run_index in [0, 1] {
            let row = result_for(
                &evaluation,
                QualificationContentOnlyCriterionKindV1::EventEquality,
                Some(run_index),
                None,
            );
            assert_eq!(row.status, QualificationContentOnlyStatusV1::Passed);
            assert_eq!(row.savings_bps, Some(0));
            assert!(row.interpretation.contains("informational only"));
        }

        for receipt_mismatch in [false, true] {
            let mut mismatch = complete_package();
            let run = &mut mismatch.apfs_runs[0];
            if receipt_mismatch {
                run.event_equality.candidate_receipt_sha256 = digest("mismatched receipt");
            } else {
                run.event_equality.candidate_carrier_set_sha256 = digest("mismatched carriers");
            }
            run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
            rehash_package(&mut mismatch);
            let evaluation =
                evaluate_qualification_content_only_v1(&mismatch).expect("mismatch evaluation");
            assert_eq!(evaluation.status, QualificationContentOnlyStatusV1::Failed);
            assert_eq!(
                result_for(
                    &evaluation,
                    QualificationContentOnlyCriterionKindV1::EventEquality,
                    Some(0),
                    None
                )
                .status,
                QualificationContentOnlyStatusV1::Failed
            );
        }
    }

    #[test]
    fn content_only_mixed_or_stale_execution_identity_is_rejected_before_evaluation() {
        type IdentityMutation = fn(&mut QualificationContentOnlyExecutionIdentityV1);
        let mutations: Vec<IdentityMutation> = vec![
            |identity| identity.source_commit = "1".repeat(40),
            |identity| identity.source_tree = "2".repeat(40),
            |identity| identity.source_dirty = true,
            |identity| identity.cargo_lock_sha256 = "3".repeat(64),
            |identity| identity.contract_sha256 = "4".repeat(64),
            |identity| identity.profile_id.push_str("-stale"),
            |identity| identity.profile_source_sha256 = "5".repeat(64),
            |identity| identity.codec_source_sha256 = "6".repeat(64),
            |identity| identity.runner_source_sha256 = "7".repeat(64),
            |identity| identity.g0_manifest_sha256 = "8".repeat(64),
            |identity| identity.g1_operation_schedule_sha256 = "9".repeat(64),
            |identity| identity.filesystem = "ext4".to_owned(),
            |identity| identity.architecture = "hosted".to_owned(),
            |identity| identity.private_corpus_configured = true,
        ];
        for mutate in mutations {
            let mut package = complete_package();
            let receipt = package.g0_receipt.as_mut().expect("G0 receipt");
            mutate(&mut receipt.identity);
            receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 hash");
            rehash_package(&mut package);
            assert!(evaluate_qualification_content_only_v1(&package).is_err());
        }
    }

    #[test]
    fn content_only_duplicate_rows_are_rejected_before_evaluation() {
        let mut duplicate_g0 = complete_package();
        let receipt = duplicate_g0.g0_receipt.as_mut().expect("G0 receipt");
        receipt.rows.push(receipt.rows[0].clone());
        receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 hash");
        rehash_package(&mut duplicate_g0);
        assert!(evaluate_qualification_content_only_v1(&duplicate_g0).is_err());

        let mut duplicate_run = complete_package();
        duplicate_run
            .apfs_runs
            .push(duplicate_run.apfs_runs[0].clone());
        rehash_package(&mut duplicate_run);
        assert!(evaluate_qualification_content_only_v1(&duplicate_run).is_err());

        let mut unsupported_run = complete_package();
        unsupported_run.apfs_runs[0].run_index = 2;
        unsupported_run.apfs_runs[0].evidence_sha256 = unsupported_run.apfs_runs[0]
            .canonical_sha256()
            .expect("APFS hash");
        rehash_package(&mut unsupported_run);
        assert!(evaluate_qualification_content_only_v1(&unsupported_run).is_err());

        let mut duplicate_state = complete_package();
        let run = &mut duplicate_state.apfs_runs[0];
        run.allocations.push(run.allocations[0].clone());
        run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
        rehash_package(&mut duplicate_state);
        assert!(evaluate_qualification_content_only_v1(&duplicate_state).is_err());

        let mut incomplete_allocation = complete_package();
        let run = &mut incomplete_allocation.apfs_runs[0];
        run.allocations[0].candidate_complete_profile_allocated_bytes = 0;
        run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
        rehash_package(&mut incomplete_allocation);
        assert!(evaluate_qualification_content_only_v1(&incomplete_allocation).is_err());

        let mut duplicate_target = complete_package();
        let admission = duplicate_target
            .package_admission
            .as_mut()
            .expect("package admission");
        admission.rows.push(admission.rows[0].clone());
        admission.evidence_sha256 = admission.canonical_sha256().expect("admission hash");
        rehash_package(&mut duplicate_target);
        assert!(evaluate_qualification_content_only_v1(&duplicate_target).is_err());
    }

    #[test]
    fn content_only_package_admission_is_required_only_for_aggregate_pass() {
        let mut absent = complete_package();
        absent.package_admission = None;
        rehash_package(&mut absent);
        assert_eq!(
            evaluate_qualification_content_only_v1(&absent)
                .expect("absent package evaluation")
                .status,
            QualificationContentOnlyStatusV1::Unknown
        );

        let mut failed_g0 = absent.clone();
        let receipt = failed_g0.g0_receipt.as_mut().expect("G0 receipt");
        receipt.rows[0].status = QualificationContentOnlyStatusV1::Failed;
        receipt.evidence_sha256 = receipt.canonical_sha256().expect("G0 hash");
        rehash_package(&mut failed_g0);
        assert_eq!(
            evaluate_qualification_content_only_v1(&failed_g0)
                .expect("failed G0 evaluation")
                .status,
            QualificationContentOnlyStatusV1::Failed
        );

        let mut failed_allocation = complete_package();
        let run = &mut failed_allocation.apfs_runs[0];
        run.allocations[0].candidate_complete_profile_allocated_bytes = 9_001;
        run.evidence_sha256 = run.canonical_sha256().expect("APFS hash");
        rehash_package(&mut failed_allocation);
        assert_eq!(
            evaluate_qualification_content_only_v1(&failed_allocation)
                .expect("failed allocation evaluation")
                .status,
            QualificationContentOnlyStatusV1::Failed
        );

        let passed = complete_package();
        assert_eq!(
            evaluate_qualification_content_only_v1(&passed)
                .expect("passing evaluation")
                .status,
            QualificationContentOnlyStatusV1::Passed
        );
    }

    #[test]
    fn content_only_canonical_hashes_reject_mutation() {
        let mut contract = qualification_content_only_contract_v1();
        contract.event_savings_bps = 1;
        assert!(contract.validate().is_err());

        let mut evidence_mutation = complete_package();
        evidence_mutation
            .g0_receipt
            .as_mut()
            .expect("G0 receipt")
            .rows[0]
            .status = QualificationContentOnlyStatusV1::Failed;
        assert!(evidence_mutation.validate().is_err());

        let mut package_mutation = complete_package();
        package_mutation.package_admission = None;
        assert!(package_mutation.validate().is_err());

        let package = complete_package();
        let mut evaluation = evaluate_qualification_content_only_v1(&package).expect("evaluation");
        evaluation.status = QualificationContentOnlyStatusV1::Failed;
        assert!(evaluation.validate().is_err());
    }

    #[test]
    fn content_only_contract_v1_publication_fixture_and_docs_are_byte_stable() {
        let publication = qualification_content_only_contract_v1_publication();
        let json = format!(
            "{}\n",
            serde_json::to_string_pretty(&publication).expect("publication JSON")
        );
        assert_eq!(
            json,
            include_str!("../../../tests/fixtures/store-foundation/content-only-contract-v1.json")
        );

        let docs = include_str!("../../../docs/benchmarking.md");
        let table = docs
            .split_once("<!-- content-only-contract-v1:start -->\n")
            .expect("content-only docs start marker")
            .1
            .split_once("\n<!-- content-only-contract-v1:end -->")
            .expect("content-only docs end marker")
            .0;
        assert_eq!(table, publication.decision_table_markdown);

        let serialized = serde_json::to_string(&publication).expect("publication JSON");
        for forbidden in [
            "candidateAllocatedBytes",
            "baselineAllocatedBytes",
            "apfsRuns",
            "observedResults",
            "timingResult",
        ] {
            assert!(!serialized.contains(forbidden), "published {forbidden}");
        }
    }

    #[test]
    fn historical_contract_publications_and_fixtures_remain_unchanged() {
        assert_eq!(
            QUALIFICATION_PERFORMANCE_CONTRACT_SHA256_V2,
            "55c473f448c80ba26e5e0eeaf23ebdd7c7b3827954bfec78873d1fd839a54a36"
        );
        assert_eq!(
            qualification_performance_contract_v2_publication().contract_sha256,
            QUALIFICATION_PERFORMANCE_CONTRACT_SHA256_V2
        );
        assert_eq!(
            QUALIFICATION_PROSPECTIVE_CONTRACT_SHA256_V1,
            "8e9fb5bffef230d97d3f4abc8a70c79958e4372668af8bde19b3aa815382857d"
        );
        assert_eq!(
            qualification_prospective_contract_v1_publication().contract_sha256,
            QUALIFICATION_PROSPECTIVE_CONTRACT_SHA256_V1
        );
        assert_eq!(
            sha256_bytes_hex(include_bytes!(
                "../../../tests/fixtures/store-foundation/generated-workloads-v1.json"
            )),
            "04a1625c6b61979e788f08053a5c9d79e0d94e3d4d9de0a7dbd9fe775eedcea0"
        );
        assert_eq!(
            sha256_bytes_hex(include_bytes!(
                "../../../tests/fixtures/store-foundation/codec/golden.json"
            )),
            "56d85b7d57c72bf356a3d4a8317454e261581261cf503bffac8d7c651aed0101"
        );
    }
}
