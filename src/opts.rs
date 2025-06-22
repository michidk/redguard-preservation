//! Command-line options and argument parsing
//!
//! This module defines the command-line interface using clap.

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

/// Redguard Preservation - A tool for parsing ROB and 3D model files
#[derive(Parser, Debug)]
#[command(name = "redguard-preservation")]
#[command(about = "Parse and analyze ROB files and embedded 3D models")]
#[command(version)]
pub struct Opts {
    /// Input ROB file to parse
    #[arg(short, long)]
    pub input: String,

    /// Output directory for extracted models (optional)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Extract embedded 3D models to separate files
    #[arg(short, long)]
    pub extract: bool,

    /// Verbosity level
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Parse and display information about a ROB file
    Parse {
        /// Show detailed segment information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Extract embedded 3D models from a ROB file
    Extract {
        /// Output format for extracted models
        #[arg(short, long, default_value = "obj")]
        format: String,
    },
    /// Analyze 3D model statistics
    Analyze {
        /// Include bounding box information
        #[arg(short, long)]
        bounds: bool,
    },
}
