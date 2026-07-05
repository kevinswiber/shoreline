mod support;
use support::shore;

#[test]
fn help_hides_legacy_surfaces_but_keeps_them_functional() {
    let help = shore(["--help"]);
    assert!(help.status.success());
    let stdout = String::from_utf8(help.stdout).expect("stdout is utf-8");
    let commands_section = stdout
        .split("Commands:")
        .nth(1)
        .expect("--help lists a Commands: section");

    for hidden in ["dump", "show", "notes"] {
        assert!(
            !commands_section
                .lines()
                .any(|line| line.split_whitespace().next() == Some(hidden)),
            "shore --help still lists hidden command {hidden:?}:\n{stdout}"
        );
    }

    for leaf in [
        vec!["dump", "--help"],
        vec!["show", "--help"],
        vec!["notes", "apply", "--help"],
    ] {
        let output = shore(leaf.clone());
        assert!(output.status.success(), "{leaf:?} should still exit 0");
    }
}
