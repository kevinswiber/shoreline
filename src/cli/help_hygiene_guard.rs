//! Guard: `--track` help-text hygiene (#380). Every occurrence of the `track`
//! argument across the CLI must carry non-empty long-form help, and `store
//! remove`'s about must state the claim/erase distinction rather than only
//! its exactly-one-selector mechanics.

use clap::CommandFactory;

fn walk_track_args(cmd: &clap::Command, prefix: &mut Vec<String>, out: &mut Vec<String>) {
    if let Some(arg) = cmd.get_arguments().find(|a| a.get_id().as_str() == "track") {
        let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
        if help.is_empty() {
            out.push(prefix.join(" "));
        }
    }
    for sub in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
        prefix.push(sub.get_name().to_owned());
        walk_track_args(sub, prefix, out);
        prefix.pop();
    }
}

#[test]
fn every_track_flag_has_help_text() {
    let cmd = super::Cli::command();
    let mut offenders = Vec::new();
    walk_track_args(&cmd, &mut Vec::new(), &mut offenders);
    assert!(
        offenders.is_empty(),
        "leaves with an undocumented --track flag:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn store_remove_about_names_the_claim_not_erase_distinction() {
    let cmd = super::Cli::command();
    let store = cmd
        .find_subcommand("store")
        .expect("store is a registered command");
    let remove = store
        .find_subcommand("remove")
        .expect("store remove is a registered command");
    let about = remove
        .get_about()
        .map(|a| a.to_string())
        .unwrap_or_default();
    assert!(
        about.contains("does not erase") || about.contains("no bytes"),
        "store remove's about should state it only records a removal claim, not erase bytes: {about:?}"
    );
    assert!(
        about.contains("store compact"),
        "store remove's about should point at `store compact` for the actual erasure: {about:?}"
    );
}
