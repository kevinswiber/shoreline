//! The public sensitivity-scan vocabulary: the finding kinds, severities, and the
//! combined-outcome lattice reported by `store_status`'s worktree scan.
//!
//! Findings cross tool boundaries — shoreline's clone-local scanner emits them and
//! downstream gates (for example a relay's egress classification gate) consume
//! them — so the vocabulary is a typed public contract rather than inline string
//! literals. Wire strings are the snake_case forms; serde, `as_str`, and `Display`
//! always agree.
//!
//! Variant declaration order is load-bearing: the derived `Ord` gives
//! `Medium < High` and `Allow < Warn < Block`.

use serde::{Deserialize, Serialize};

/// A sensitivity finding class reported by the worktree scanner.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityKind {
    KnownToken,
    PrivateKey,
    HighEntropy,
    SensitiveFilename,
    GeneratedPath,
}

/// How severe a sensitivity finding class is: `medium` < `high`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivitySeverity {
    Medium,
    High,
}

/// The advisory policy outcome lattice: `allow` < `warn` < `block`. A repository's
/// combined outcome is the maximum of its findings' outcomes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityPolicyOutcome {
    Allow,
    Warn,
    Block,
}

impl SensitivityKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::KnownToken,
        Self::PrivateKey,
        Self::HighEntropy,
        Self::SensitiveFilename,
        Self::GeneratedPath,
    ];

    /// The canonical kind name — exactly the serde snake_case wire form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KnownToken => "known_token",
            Self::PrivateKey => "private_key",
            Self::HighEntropy => "high_entropy",
            Self::SensitiveFilename => "sensitive_filename",
            Self::GeneratedPath => "generated_path",
        }
    }

    /// Parse the exact wire form; `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// The severity the scanner assigns this kind.
    pub fn severity(&self) -> SensitivitySeverity {
        match self {
            Self::KnownToken | Self::PrivateKey => SensitivitySeverity::High,
            Self::HighEntropy | Self::SensitiveFilename | Self::GeneratedPath => {
                SensitivitySeverity::Medium
            }
        }
    }

    /// The per-finding policy outcome the scanner assigns this kind.
    pub fn policy_outcome(&self) -> SensitivityPolicyOutcome {
        match self {
            Self::KnownToken | Self::PrivateKey => SensitivityPolicyOutcome::Block,
            Self::HighEntropy | Self::SensitiveFilename | Self::GeneratedPath => {
                SensitivityPolicyOutcome::Warn
            }
        }
    }
}

impl std::fmt::Display for SensitivityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SensitivitySeverity {
    /// Every severity, in ascending order.
    pub const ALL: [Self; 2] = [Self::Medium, Self::High];

    /// The canonical severity name — exactly the serde snake_case wire form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Parse the exact wire form; `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|severity| severity.as_str() == value)
    }
}

impl std::fmt::Display for SensitivitySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SensitivityPolicyOutcome {
    /// Every outcome, in ascending (dominance) order.
    pub const ALL: [Self; 3] = [Self::Allow, Self::Warn, Self::Block];

    /// The canonical outcome name — exactly the serde snake_case wire form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Block => "block",
        }
    }

    /// Parse the exact wire form; `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.as_str() == value)
    }

    /// Fold finding outcomes into the combined repository outcome: the maximum
    /// under `allow` < `warn` < `block`, with `allow` as the empty-scan identity.
    pub fn combine(outcomes: impl IntoIterator<Item = Self>) -> Self {
        outcomes.into_iter().fold(Self::Allow, Ord::max)
    }
}

impl std::fmt::Display for SensitivityPolicyOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
