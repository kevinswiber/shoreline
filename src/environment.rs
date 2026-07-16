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
