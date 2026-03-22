//! Command-line options and argument parsing
//!
//! This module defines the command-line interface using clap.

use clap::{Args, Parser, Subcommand};
use rgpre::import::FileType;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum FontOutputMode {
    Bitmap,
    Ttf,
}

/// Main CLI arguments
#[derive(Parser, Debug)]
#[command(
    name = "redguard-preservation",
    about = "A CLI tool to parse and analyze ROB and 3D model files from Redguard.",
    author,
    version,
    about
)]
pub struct Opts {
    /// The verbosity of the output
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    /// The command to run
    #[command(subcommand)]
    pub command: Commands,
}

/// Arguments for reading files
#[derive(Args, Debug, Clone)]
pub struct ReadArgs {
    /// The file to read
    #[arg(value_parser)]
    pub file: PathBuf,

    /// Optional file type override (otherwise inferred from extension)
    #[arg(short, long, value_parser = clap::value_parser!(FileType))]
    pub filetype: Option<FileType>,
}

/// Arguments for converting files to GLTF
#[derive(Args, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
// CLI flags mirror user-facing switches; bool options are intentional here.
pub struct ConvertArgs {
    /// The file to convert
    #[arg(value_parser)]
    pub file: PathBuf,

    /// Optional file type override (otherwise inferred from extension)
    #[arg(short, long, value_parser = clap::value_parser!(FileType))]
    pub filetype: Option<FileType>,

    /// Output file path (defaults to input file with .gltf extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Unified asset root for model lookup and TEXBSI texture files
    #[arg(long, env)]
    pub assets: Option<PathBuf>,

    /// Legacy alias for --assets (kept for compatibility)
    #[arg(long, env, hide = true)]
    pub asset_path: Option<PathBuf>,

    /// COL palette file for solid-color face lookup
    #[arg(long, env)]
    pub palette: Option<PathBuf>,

    /// Legacy alias for --assets (kept for compatibility)
    #[arg(long, hide = true)]
    pub asset_dir: Option<PathBuf>,

    #[arg(long, value_enum, help = "FNT conversion output mode: bitmap or ttf")]
    pub font_output: Option<FontOutputMode>,

    /// For WLD->GLB conversion, export only terrain (skip companion RGM placement merge)
    #[arg(long, default_value_t = false)]
    pub terrain_only: bool,

    /// For WLD->GLB conversion, enable/disable terrain texturing
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub terrain_textures: bool,

    /// Apply lossless deflate compression to embedded PNG textures
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,

    /// For TEXBSI conversion, export all animation frames (default: frame 0 only)
    #[arg(long, default_value_t = false)]
    pub all_frames: bool,
}

/// Arguments for scanning a directory
#[derive(Args, Debug, Clone)]
pub struct ScanArgs {
    /// The directory to scan
    #[arg(value_parser)]
    pub dir: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read and parse a ROB or 3D model file
    #[clap(alias = "r")]
    Read(ReadArgs),
    /// Convert a ROB or 3D model file to GLTF format
    #[clap(alias = "c")]
    Convert(ConvertArgs),
    /// Scan a directory for files
    #[clap(alias = "s")]
    Scan(ScanArgs),
}
