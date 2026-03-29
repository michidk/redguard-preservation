//! Command-line options and argument parsing
//!
//! This module defines the command-line interface using clap.
//! The `convert` command uses a subcommand-per-format pattern so each format
//! only exposes its valid flags. `convert <FILE>` auto-detects the format
//! and uses defaults; `convert <FORMAT> <FILE>` enables format-specific options.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

// ── Per-format output format enums ──────────────────────────────────────────

/// Output format for TEXBSI texture bank conversion
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum TexbsiFormat {
    /// Animated GIF for multi-frame images, PNG for single-frame (default)
    Gif,
    /// Single PNG per image, frame 0 only
    Png,
    /// All animation frames as separate PNGs
    Frames,
}

/// Output format for GXA bitmap archive conversion
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum GxaFormat {
    /// Animated GIF (default)
    Gif,
    /// Individual frame PNGs
    Png,
}

/// Output format for FNT font conversion
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum FntFormat {
    /// PNG atlas + BMFont descriptor + JSON glyph metadata (default)
    Bitmap,
    /// TrueType font (bitmap-traced vector outlines)
    Ttf,
}

/// Output format for COL palette conversion
#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum ColFormat {
    /// Swatch PNG only
    Png,
    /// Palette metadata JSON only
    Json,
}

// ── Shared arg structs ──────────────────────────────────────────────────────

/// Input file and optional output path, shared by all convert subcommands.
#[derive(Args, Debug, Clone)]
pub struct ConvertFileArgs {
    /// The file to convert
    #[arg(value_parser)]
    pub file: PathBuf,

    /// Output file or directory; when a directory, the filename is derived automatically
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

// ── Auto-detect mode ────────────────────────────────────────────────────────

/// Args for `convert <FILE>` (auto-detect format, default settings).
///
/// `file` is `Option` because clap needs a default when a subcommand is present
/// (`args_conflicts_with_subcommands`). In auto-detect mode we validate it exists.
#[derive(Args, Debug, Clone, Default)]
pub struct AutoConvertArgs {
    /// The file to convert (format is auto-detected)
    #[arg(value_parser)]
    pub file: Option<PathBuf>,

    /// Output file or directory
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Apply higher PNG compression to exported images and embedded GLB textures
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

// ── Per-format subcommand arg structs ───────────────────────────────────────

/// TEXBSI texture bank → images
#[derive(Args, Debug, Clone)]
pub struct TexbsiArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Output format for images
    #[arg(long, value_enum, default_value_t = TexbsiFormat::Gif)]
    pub format: TexbsiFormat,

    /// COL palette file for color decoding
    #[arg(long, env)]
    pub palette: Option<PathBuf>,

    /// Apply higher PNG compression
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// GXA bitmap archive → images
#[derive(Args, Debug, Clone)]
pub struct GxaArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Output format for images
    #[arg(long, value_enum, default_value_t = GxaFormat::Gif)]
    pub format: GxaFormat,

    /// Apply higher PNG compression
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// FNT font → bitmap atlas or TrueType
#[derive(Args, Debug, Clone)]
pub struct FntArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Output format
    #[arg(long, value_enum, default_value_t = FntFormat::Bitmap)]
    pub format: FntFormat,

    /// Apply higher PNG compression (bitmap mode only)
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// COL palette → PNG swatch and/or JSON metadata
#[derive(Args, Debug, Clone)]
pub struct ColArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Export only PNG swatch or only JSON metadata (default: both)
    #[arg(long, value_enum)]
    pub format: Option<ColFormat>,

    /// Apply higher PNG compression
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// WLD world → GLB terrain scene or map PNGs
#[derive(Args, Debug, Clone)]
pub struct WldArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Unified asset root for model lookup and TEXBSI texture files
    #[arg(long, env)]
    pub assets: Option<PathBuf>,

    /// COL palette file for terrain color lookup
    #[arg(long, env)]
    pub palette: Option<PathBuf>,

    /// Export only terrain, skip companion RGM placement merge
    #[arg(long, default_value_t = false)]
    pub terrain_only: bool,

    /// Enable/disable terrain texturing
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub terrain_textures: bool,

    /// Apply higher PNG compression
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// 3D / 3DC / ROB model → GLB
#[derive(Args, Debug, Clone)]
pub struct ModelArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Unified asset root for texture lookup
    #[arg(long, env)]
    pub assets: Option<PathBuf>,

    /// COL palette file for solid-color face lookup
    #[arg(long, env)]
    pub palette: Option<PathBuf>,

    /// Apply higher PNG compression to embedded GLB textures
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// RGM scene → GLB with actor metadata
#[derive(Args, Debug, Clone)]
pub struct RgmArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Unified asset root for model lookup and TEXBSI texture files
    #[arg(long, env)]
    pub assets: Option<PathBuf>,

    /// COL palette file for solid-color face lookup
    #[arg(long, env)]
    pub palette: Option<PathBuf>,

    /// Apply higher PNG compression to embedded GLB textures
    #[arg(long, default_value_t = false)]
    pub compress_textures: bool,
}

/// RTX dialogue audio → WAV files
#[derive(Args, Debug, Clone)]
pub struct RtxArgs {
    #[command(flatten)]
    pub io: ConvertFileArgs,

    /// Use resolved text names as filenames instead of 4-char tags
    #[arg(long, default_value_t = false)]
    pub resolve_names: bool,
}

// ── Convert subcommand enum ─────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum ConvertCommand {
    /// Convert TEXBSI texture bank (TEXBSI.###) to images
    Texbsi(TexbsiArgs),
    /// Convert GXA bitmap archive to images
    Gxa(GxaArgs),
    /// Convert FNT font to bitmap atlas or TrueType
    Fnt(FntArgs),
    /// Convert COL palette to PNG swatch and/or JSON metadata
    Col(ColArgs),
    /// Convert WLD world to GLB terrain scene or map PNGs
    Wld(WldArgs),
    /// Convert 3D/3DC/ROB model(s) to GLB
    Model(ModelArgs),
    /// Convert RGM scene to GLB with actor metadata
    Rgm(RgmArgs),
    /// Convert RTX dialogue audio to WAV files
    Rtx(RtxArgs),
    /// Convert SFX sound bank to WAV files
    Sfx(ConvertFileArgs),
    /// Convert CHT cheat states to JSON
    Cht(ConvertFileArgs),
    /// Convert PVO visibility octree to JSON
    Pvo(ConvertFileArgs),
}

// ── Top-level convert args (git-stash pattern) ─────────────────────────────

/// Convert a supported file to its output format (GLB, JSON, PNG, WAV, etc.)
#[derive(Args, Debug, Clone)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub command: Option<ConvertCommand>,

    #[command(flatten)]
    pub auto: AutoConvertArgs,
}

// ── Other top-level commands ────────────────────────────────────────────────

/// Arguments for reading files
#[derive(Args, Debug, Clone)]
pub struct ReadArgs {
    /// The file to read
    #[arg(value_parser)]
    pub file: PathBuf,
}

/// Arguments for scanning a directory
#[derive(Args, Debug, Clone)]
pub struct ScanArgs {
    /// The directory to scan
    #[arg(value_parser)]
    pub dir: PathBuf,
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
