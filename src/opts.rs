//! Command-line options and argument parsing
//!
//! This module defines the command-line interface using clap.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// Single PNG per image, frame 0 only
    Png,
    /// All animation frames as separate PNGs
    Frames,
    /// Animated GIF for multi-frame images, PNG for single-frame (TEXBSI/GXA default)
    Gif,
    /// JSON metadata only (COL palette)
    Json,
    /// Bitmap font atlas: PNG + BMFont + JSON (FNT default)
    Bitmap,
    /// TrueType font
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
}

/// Arguments for converting files to GLTF
#[derive(Args, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
// CLI flags mirror user-facing switches; bool options are intentional here.
pub struct ConvertArgs {
    /// The file to convert
    #[arg(value_parser)]
    pub file: PathBuf,

    /// Output file or directory; when a directory, the filename is derived automatically
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

    /// Output format override (default depends on file type)
    #[arg(long, value_enum)]
    pub format: Option<OutputFormat>,

    /// For WLD->GLB conversion, export only terrain (skip companion RGM placement merge)
    #[arg(long, default_value_t = false)]
    pub terrain_only: bool,

    /// For WLD->GLB conversion, enable/disable terrain texturing
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub terrain_textures: bool,

    /// Apply higher PNG compression to exported images and embedded GLB textures
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,

    /// For RTX conversion, use resolved text names as filenames instead of 4-char tags
    #[arg(long, default_value_t = false)]
    pub resolve_names: bool,
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
    /// Read and parse a supported file, printing its decoded structure
    #[clap(alias = "r")]
    Read(ReadArgs),
    /// Convert a supported file to its output format (GLB, JSON, PNG, WAV, etc.)
    #[clap(alias = "c")]
    Convert(ConvertArgs),
    /// Scan a directory for files
    #[clap(alias = "s")]
    Scan(ScanArgs),
}
