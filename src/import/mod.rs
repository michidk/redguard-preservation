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
/// Parser for GXA bitmap archives.
pub mod gxa;
/// Parser for 3D and 3DC model files.
pub mod model3d;
/// Parser for COL palette files.
pub mod palette;
/// Export helpers for COL palette assets.
pub mod palette_export;
/// Shared PNG save helper with compression control.
pub mod png;
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
    Gxa,      // .gxa
    Sfx,      // .sfx
    Rtx,      // .rtx
}

const ALL_FILE_TYPES: [FileType; 13] = [
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
    FileType::Gxa,
    FileType::Sfx,
    FileType::Rtx,
];

impl FileType {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &ALL_FILE_TYPES
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Bsi => "Texture BSI",
            Self::Cht => "Cheat States",
            Self::Col => "Palette",
            Self::Model3d => "Model",
            Self::Model3dc => "Animated Model",
            Self::Rob => "ROB World",
            Self::Rgm => "Map Data",
            Self::Pvo => "PVO Data",
            Self::Wld => "World Geometry",
            Self::Fnt => "Font",
            Self::Gxa => "GXA Bitmap",
            Self::Sfx => "Sound Effects",
            Self::Rtx => "Dialogue Audio",
        }
    }

    #[must_use]
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Bsi => &[".bsi"],
            Self::Cht => &[".cht"],
            Self::Col => &[".col"],
            Self::Model3d => &[".3d"],
            Self::Model3dc => &[".3dc"],
            Self::Rob => &[".rob"],
            Self::Rgm => &[".rgm"],
            Self::Pvo => &[".pvo"],
            Self::Wld => &[".wld"],
            Self::Fnt => &[".fnt"],
            Self::Gxa => &[".gxa"],
            Self::Sfx => &[".sfx"],
            Self::Rtx => &[".rtx"],
        }
    }

    /// Matches a bare extension string (case-insensitive, without leading dot).
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        let lower = ext.to_ascii_lowercase();
        Self::all().iter().copied().find(|ft| {
            ft.extensions()
                .iter()
                .any(|e| e.trim_start_matches('.') == lower)
        })
    }

    /// Detects a supported Redguard file type from path extension or filename stem.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let p = path.as_ref();
        if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && let Some(ft) = Self::from_extension(ext)
        {
            return Some(ft);
        }
        let stem = p.file_stem()?.to_str()?.to_ascii_uppercase();
        if stem == "TEXBSI" {
            return Some(Self::Bsi);
        }
        None
    }
}

impl std::str::FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_extension(s.trim()).ok_or_else(|| {
            let supported = Self::all()
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
