use serde::{Deserialize, Serialize};

/// Bounded vocabulary naming the local import seam that stamped an event.
///
/// ADR-0009: the binding predicate reads presence only; `via` and
/// `receivedAt` are operator-facing detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IngestVia {
    IngestEvents,
    BundleApply,
}

/// Local importer bookkeeping stamped on every event that enters the store
/// through a foreign-event seam. Trustworthy to this store under the
/// single-writer contract; never a signed fact, never trustworthy to a third
/// party reading a mirrored or copied store (ADR-0009).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestProvenance {
    pub via: IngestVia,
    pub received_at: String,
}
