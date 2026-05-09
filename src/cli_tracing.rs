use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::sync::Mutex;

use clap::{Args, ValueEnum};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

#[derive(Clone, Debug, Args)]
pub(crate) struct TracingArgs {
    #[arg(long, global = true, value_name = "FILTER")]
    pub(crate) log: Option<String>,

    #[arg(long, global = true, value_enum, default_value_t = LogFormatArg::Compact)]
    pub(crate) log_format: LogFormatArg,

    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) log_file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum LogFormatArg {
    Compact,
    Pretty,
    Json,
}

pub(crate) fn tracing_enabled(args: &TracingArgs) -> bool {
    resolve_log_filter(args).is_some()
}

pub(crate) fn init_tracing(args: &TracingArgs) -> Result<(), Box<dyn std::error::Error>> {
    let Some(filter) = resolve_log_filter(args) else {
        return Ok(());
    };
    let filter = EnvFilter::try_new(&filter)
        .map_err(|error| invalid_input(format!("invalid log filter: {error}")))?;
    let (writer, ansi) = writer(args.log_file.as_ref())?;

    init_tracing_with_writer(filter, args.log_format, writer, ansi)
}

pub(crate) fn init_tracing_with_writer(
    filter: EnvFilter,
    format: LogFormatArg,
    writer: BoxMakeWriter,
    ansi: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(ansi);

    match format {
        LogFormatArg::Compact => builder
            .compact()
            .try_init()
            .map_err(|error| io::Error::other(error.to_string()))?,
        LogFormatArg::Pretty => builder
            .pretty()
            .try_init()
            .map_err(|error| io::Error::other(error.to_string()))?,
        LogFormatArg::Json => builder
            .json()
            .try_init()
            .map_err(|error| io::Error::other(error.to_string()))?,
    }

    Ok(())
}

fn resolve_log_filter(args: &TracingArgs) -> Option<String> {
    if let Some(filter) = non_empty(args.log.as_deref()) {
        return Some(filter.to_owned());
    }

    ["SHORE_LOG", "RUST_LOG"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|value| non_empty(Some(&value)).map(str::to_owned))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn writer(log_file: Option<&PathBuf>) -> io::Result<(BoxMakeWriter, bool)> {
    match log_file {
        Some(path) => {
            let file = append_file(path)?;
            Ok((BoxMakeWriter::new(Mutex::new(file)), false))
        }
        None => Ok((BoxMakeWriter::new(io::stderr), io::stderr().is_terminal())),
    }
}

fn append_file(path: &PathBuf) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new().create(true).append(true).open(path)
}

fn invalid_input(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
