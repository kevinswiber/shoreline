use std::io::Write;

use clap::{Args, Subcommand};

pub(super) mod revisions;
pub(super) mod show;

#[derive(Debug, Args)]
pub(super) struct ReviewArgs {
    #[command(subcommand)]
    command: ReviewCommand,
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    Revisions(revisions::RevisionsArgs),
    Show(show::ShowArgs),
}

pub(super) fn run(
    args: ReviewArgs,
    stdout: &mut dyn Write,
    _stderr: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        ReviewCommand::Revisions(args) => revisions::run(args, stdout),
        ReviewCommand::Show(args) => show::run(args, stdout),
    }
}
