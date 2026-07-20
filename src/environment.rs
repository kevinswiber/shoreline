//! Canonical names for Pointbreak-owned environment inputs.
//!
//! Callers retain their selector-specific parsing, precedence, empty-value, and
//! non-Unicode behavior. This module owns names only; it deliberately has no
//! compatibility table or paired-variable resolver.

pub const HOME: &str = "POINTBREAK_HOME";
pub const ACTOR_ID: &str = "POINTBREAK_ACTOR_ID";
pub const SIGNING: &str = "POINTBREAK_SIGNING";
pub const SIGNING_KEY: &str = "POINTBREAK_SIGNING_KEY";
pub const FORMAT: &str = "POINTBREAK_FORMAT";
pub const THEME: &str = "POINTBREAK_THEME";
pub const LOG: &str = "POINTBREAK_LOG";
pub const BACKEND: &str = "POINTBREAK_BACKEND";
pub const PERF: &str = "POINTBREAK_PERF";

pub const RUNTIME_VARIABLES: [&str; 9] = [
    HOME,
    ACTOR_ID,
    SIGNING,
    SIGNING_KEY,
    FORMAT,
    THEME,
    LOG,
    BACKEND,
    PERF,
];

pub const BENCH_FIXTURE: &str = "POINTBREAK_BENCH_FIXTURE";
pub const BENCH_REPO: &str = "POINTBREAK_BENCH_REPO";

pub const BENCH_VARIABLES: [&str; 2] = [BENCH_FIXTURE, BENCH_REPO];

/// Test-only override plumbing for `POINTBREAK_HOME`. Under a runner with
/// process-per-test isolation (cargo-nextest, the `just test` / CI runner) the
/// env mutation is hermetic and needs no synchronization. Plain `cargo test`
/// runs every test as a thread of one process, where unsynchronized set/remove
/// calls race and tests resolve paths under each other's homes — there, all
/// overrides serialize on one crate-wide lock. Best-effort in that mode: the
/// lock covers every override writer, but a concurrent test that merely READS
/// the environment can still observe a transient override; only process
/// isolation closes that.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    /// The capability probe: true when the runner has documented that this test
    /// owns its process. cargo-nextest's env contract sets
    /// `NEXTEST_EXECUTION_MODE=process-per-test` in every test it spawns; the
    /// VALUE is checked (not mere presence of a nextest marker) so a future
    /// non-isolated nextest mode falls back to the lock.
    fn process_is_isolated_per_test() -> bool {
        std::env::var("NEXTEST_EXECUTION_MODE").as_deref() == Ok("process-per-test")
    }

    /// Set `POINTBREAK_HOME` for the duration of `f`. Skips the lock under a
    /// process-isolated runner; otherwise holds the crate-wide lock. The
    /// variable is removed even when `f` panics, so one failing test cannot
    /// leak its home into the next serialized one.
    pub(crate) fn with_home_override<T>(home: &Path, f: impl FnOnce() -> T) -> T {
        let _guard: Option<MutexGuard<'_, ()>> = if process_is_isolated_per_test() {
            None
        } else {
            Some(
                HOME_LOCK
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            )
        };
        struct ClearOnDrop;
        impl Drop for ClearOnDrop {
            fn drop(&mut self) {
                // SAFETY: under HOME_LOCK (still held while unwinding) or in an
                // isolated process — see with_home_override.
                unsafe { std::env::remove_var(super::HOME) };
            }
        }
        // SAFETY: either this test owns its process (nextest isolation) or
        // HOME_LOCK serializes every in-process mutation of this variable.
        unsafe { std::env::set_var(super::HOME, home) };
        let _clear = ClearOnDrop;
        f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_variable_table_is_canonical_and_complete() {
        assert_eq!(
            RUNTIME_VARIABLES,
            [
                "POINTBREAK_HOME",
                "POINTBREAK_ACTOR_ID",
                "POINTBREAK_SIGNING",
                "POINTBREAK_SIGNING_KEY",
                "POINTBREAK_FORMAT",
                "POINTBREAK_THEME",
                "POINTBREAK_LOG",
                "POINTBREAK_BACKEND",
                "POINTBREAK_PERF",
            ]
        );
        assert!(
            RUNTIME_VARIABLES
                .iter()
                .all(|name| name.starts_with("POINTBREAK_"))
        );
    }

    #[test]
    fn benchmark_variable_table_is_canonical_and_complete() {
        assert_eq!(
            BENCH_VARIABLES,
            ["POINTBREAK_BENCH_FIXTURE", "POINTBREAK_BENCH_REPO"]
        );
    }
}
