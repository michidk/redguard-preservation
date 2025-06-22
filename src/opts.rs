//! Command-line options and argument parsing
//!
//! This module defines the command-line interface using clap.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI arguments
#[derive(Parser, Debug)]
#[command(
    name = "redguard-preservation",
    about = "A CLI tool to parse and analyze ROB and 3D model files from Redguard.",
    author,
    version,
    about
)]
pub(crate) struct Opts {
    /// The verbosity of the output
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    /// The command to run
    #[command(subcommand)]
    pub command: Commands,
}

/// Arguments for reading files
#[derive(Args, Debug, Clone)]
pub(crate) struct ReadArgs {
    /// The file to read
    #[arg(value_parser)]
    pub file: PathBuf,

    /// The type of file to read (rob or 3dc)
    #[arg(short, long)]
    pub filetype: Option<FileType>,
}

/// Arguments for converting files to GLTF
#[derive(Args, Debug, Clone)]
pub(crate) struct ConvertArgs {
    /// The file to convert
    #[arg(value_parser)]
    pub file: PathBuf,

    /// The type of file to convert (rob or 3dc)
    #[arg(short, long)]
    pub filetype: Option<FileType>,

    /// Output file path (defaults to input file with .gltf extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Supported file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileType {
    Rob,
    Model3D,
}

impl std::str::FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rob" => Ok(FileType::Rob),
            "3dc" | "3d" => Ok(FileType::Model3D),
            _ => Err(format!(
                "Unknown file type: {}. Supported types: rob, 3dc",
                s
            )),
        }
    }
}

impl FileType {
    /// Infer file type from file extension
    pub fn from_extension(path: &PathBuf) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "rob" => Some(FileType::Rob),
                "3dc" | "3d" => Some(FileType::Model3D),
                _ => None,
            })
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Read and parse a ROB or 3D model file
    Read {
        #[command(flatten)]
        args: ReadArgs,
    },
    /// Convert a ROB or 3D model file to GLTF format
    Convert {
        #[command(flatten)]
        args: ConvertArgs,
    },
    /// Dump vertices and faces for quick debugging
    Dump {
        #[command(flatten)]
        args: ConvertArgs,
        /// Optional model index (0-based) to dump.  If omitted, dumps all models.
        #[arg(short, long)]
        model: Option<usize>,
        /// Limit the number of faces printed per model
        #[arg(short = 'n', long, default_value_t = 20)]
        max_faces: usize,
    },
}
