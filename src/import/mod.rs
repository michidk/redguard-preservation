/// Parser for BSI/TEXBSI image archives.
pub mod bsi;
/// Parser for REDGUARD.CHT cheat persistence files.
pub mod cht;
/// Parser for FNT bitmap font files.
pub mod fnt;
/// Export helpers for FNT assets.
pub mod fnt_export;
/// FNT to TrueType font conversion.
pub mod fnt_ttf;
/// Parser for 3D and 3DC model files.
pub mod model3d;
/// Parser for COL palette files.
pub mod palette;
/// Export helpers for COL palette assets.
pub mod palette_export;
/// Parser for PVO octree collision files.
pub mod pvo;
/// Asset registry and directory scanning helpers.
pub mod registry;
/// Parser and model extraction for RGM scene files.
pub mod rgm;
/// Parser and model extraction for ROB archives.
pub mod rob;
/// Parser for RTX dialogue/audio container files.
pub mod rtx;
/// Parser for SFX sound effect files.
pub mod sfx;
/// Parser and export helpers for WLD world files.
pub mod wld;
/// Parser for WORLD.INI world/level database.
pub mod world_ini;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
/// Supported Redguard file types recognized by this crate.
pub enum FileType {
    Bsi,      // .bsi
    Cht,      // .cht
    Col,      // .col
    Model3d,  // .3d
    Model3dc, // .3dc
    Rob,      // .rob
    Rgm,      // .rgm
    Pvo,      // .pvo
    Wld,      // .wld
    Fnt,      // .fnt
    Sfx,      // .sfx
    Rtx,      // .rtx
}

const ALL_FILE_TYPES: [FileType; 12] = [
    FileType::Bsi,
    FileType::Cht,
    FileType::Col,
    FileType::Model3d,
    FileType::Model3dc,
    FileType::Rob,
    FileType::Rgm,
    FileType::Pvo,
    FileType::Wld,
    FileType::Fnt,
    FileType::Sfx,
    FileType::Rtx,
];

impl FileType {
    pub const fn all() -> &'static [FileType] {
        &ALL_FILE_TYPES
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            FileType::Bsi => "Texture BSI",
            FileType::Cht => "Cheat States",
            FileType::Col => "Palette",
            FileType::Model3d => "Model",
            FileType::Model3dc => "Animated Model",
            FileType::Rob => "ROB World",
            FileType::Rgm => "Map Data",
            FileType::Pvo => "PVO Data",
            FileType::Wld => "World Geometry",
            FileType::Fnt => "Font",
            FileType::Sfx => "Sound Effects",
            FileType::Rtx => "Dialogue Audio",
        }
    }

    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            FileType::Bsi => &[".bsi"],
            FileType::Cht => &[".cht"],
            FileType::Col => &[".col"],
            FileType::Model3d => &[".3d"],
            FileType::Model3dc => &[".3dc"],
            FileType::Rob => &[".rob"],
            FileType::Rgm => &[".rgm"],
            FileType::Pvo => &[".pvo"],
            FileType::Wld => &[".wld"],
            FileType::Fnt => &[".fnt"],
            FileType::Sfx => &[".sfx"],
            FileType::Rtx => &[".rtx"],
        }
    }

    /// Matches a bare extension string (case-insensitive, without leading dot).
    pub fn from_extension(ext: &str) -> Option<Self> {
        let lower = ext.to_lowercase();
        FileType::all().iter().copied().find(|ft| {
            ft.extensions()
                .iter()
                .any(|e| e.trim_start_matches('.') == lower)
        })
    }

    /// Detects a supported Redguard file type from a path extension.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let extension = path.as_ref().extension()?.to_str()?;
        Self::from_extension(extension)
    }
}

impl std::str::FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_extension(s.trim()).ok_or_else(|| {
            let supported = FileType::all()
                .iter()
                .flat_map(|ft| {
                    ft.extensions()
                        .iter()
                        .map(|ext| ext.trim_start_matches('.').to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Unknown file type: {s}. Supported types: {supported}")
        })
    }
}
