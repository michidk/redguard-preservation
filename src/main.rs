//! Command-line interface for Redguard Preservation
//!
//! This binary provides a command-line interface for parsing TES Redguard files.

mod cli;
mod opts;

use clap::Parser;
use color_eyre::Result;
use log::trace;
use std::io::Write;

use crate::opts::{Commands, Opts};

fn main() -> Result<()> {
    color_eyre::install()?;

    let opts = Opts::parse();
    let opts_dbg = format!("{opts:#?}");

    env_logger::Builder::from_default_env()
        .filter_level(opts.verbose.log_level_filter())
        .format(move |buf, record| log_format(buf, record, opts.verbose.log_level_filter()))
        .init();

    trace!("Parsed Opts:\n{opts_dbg}");

    match opts.command {
        Commands::Read(args) => cli::handle_read_command(args)?,
        Commands::Convert(args) => cli::handle_convert_command(args)?,
        Commands::Scan(args) => cli::handle_scan_command(args)?,
    }

    Ok(())
}

/// Formats log messages with colored single-char level prefix.
///
/// At default verbosity (Info), info messages are printed without a prefix.
/// At higher verbosity, all messages get a colored level prefix.
fn log_format(
    buf: &mut env_logger::fmt::Formatter,
    record: &log::Record,
    filter: log::LevelFilter,
) -> std::io::Result<()> {
    let level = record.level();
    let colored = match level {
        log::Level::Trace => "\x1b[37mT\x1b[0m",
        log::Level::Debug => "\x1b[36mD\x1b[0m",
        log::Level::Info => "\x1b[32mI\x1b[0m",
        log::Level::Warn => "\x1b[33mW\x1b[0m",
        log::Level::Error => "\x1b[31mE\x1b[0m",
    };

    if level == log::Level::Info && filter == log::LevelFilter::Info {
        writeln!(buf, "{}", record.args())
    } else {
        writeln!(buf, "{}: {}", colored, record.args())
    }
}
