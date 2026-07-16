//! Smoke check: the shipped top-level `pointbreak --help` tree matches the
//! review-surface grammar ADR — flat families present, retired names absent.

mod support;
use support::pointbreak;

#[test]
fn top_level_tree_matches_the_shipped_grammar() {
    let help = String::from_utf8(pointbreak(["--help"]).stdout).expect("help is utf-8");
    for present in [
        "revision",
        "observation",
        "assessment",
        "validation",
        "input-request",
        "association",
        "store",
        "key",
        "identity",
        "capture",
        "diff",
        "history",
        "endorse",
        "inspect",
    ] {
        assert!(
            help.contains(present),
            "pointbreak --help missing {present}: {help}"
        );
    }
    for retired in ["review", "keys"] {
        assert!(
            !help
                .lines()
                .any(|line| line.trim_start().starts_with(retired)),
            "pointbreak --help still lists retired top-level family {retired}: {help}"
        );
    }
}
