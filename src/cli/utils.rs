//! Shared utilities for CLI commands

use color_eyre::Result;
use color_eyre::eyre::eyre;
use log::{info, warn};
use rgpre::import::FileType;
use rgpre::import::palette::Palette;
use rgpre::import::world_ini::WorldIni;
use std::path::{Path, PathBuf};

const KNOWN_GAME_SUBDIRS: [&str; 4] = ["3dart", "fxart", "input", "maps"];

fn is_known_game_subdir(name: &str) -> bool {
    KNOWN_GAME_SUBDIRS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn count_known_subdirs(path: &Path) -> usize {
    KNOWN_GAME_SUBDIRS
        .iter()
        .filter(|name| path.join(name).is_dir())
        .count()
}

pub fn resolve_asset_root_from_input(input_file: &Path) -> PathBuf {
    let parent = input_file.parent().unwrap_or_else(|| Path::new("."));

    for ancestor in parent.ancestors() {
        if count_known_subdirs(ancestor) >= 2 {
            return ancestor.to_path_buf();
        }
    }

    if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
        && is_known_game_subdir(parent_name)
    {
        return parent
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
    }

    parent.to_path_buf()
}

pub fn resolve_filetype(path: &Path) -> Result<FileType> {
    FileType::from_path(path).ok_or_else(|| {
        eyre!(
            "Could not infer file type for '{}'. Supported extensions: .3d, .3dc, .rob, .rgm, .wld, .fnt, .col, .pvo, .sfx, .rtx, .gxa, .cht, and TEXBSI.* by stem.",
            path.display()
        )
    })
}

pub fn parse_file(
    file_path: &std::path::Path,
    filetype: Option<rgpre::import::FileType>,
    registry: Option<&rgpre::import::registry::Registry>,
) -> Result<Vec<rgpre::import::model3d::Model3DFile>> {
    let filetype = filetype.map_or_else(|| resolve_filetype(file_path), Ok)?;

    let file_content = std::fs::read(file_path)?;

    match filetype {
        rgpre::import::FileType::Rob => {
            let (_rob_file, models) = rgpre::import::rob::parse_rob_with_models(&file_content)?;
            info!(
                "Successfully parsed ROB file, found {} models",
                models.len()
            );
            Ok(models)
        }
        rgpre::import::FileType::Rgm => {
            let registry = registry
                .ok_or_else(|| eyre!("internal error: registry is required for RGM files"))?;
            let (_rgm_file, positioned_models, _lights) =
                rgpre::import::rgm::parse_rgm_with_models(&file_content, registry)?;
            info!(
                "Successfully parsed RGM file, found {} positioned models",
                positioned_models.len()
            );
            // Extract the models from PositionedModel structures
            let models: Vec<_> = positioned_models.into_iter().map(|pm| pm.model).collect();
            Ok(models)
        }
        rgpre::import::FileType::Model3d | rgpre::import::FileType::Model3dc => {
            let model = rgpre::import::model3d::parse_3d_file(&file_content)?;
            info!("Successfully parsed 3D model file");
            Ok(vec![model])
        }
        _ => Ok(vec![]),
    }
}

const WORLD_INI_NAMES: [&str; 2] = ["WORLD.INI", "world.ini"];

fn find_world_ini(asset_root: &Path) -> Option<PathBuf> {
    for name in &WORLD_INI_NAMES {
        let path = asset_root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn find_palette_on_disk(asset_root: &Path, ini_palette_path: &str) -> Option<PathBuf> {
    let filename = ini_palette_path
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_else(|| ini_palette_path.trim());
    let filename_lower = filename.to_ascii_lowercase();

    for dir_name in &["fxart", "3dart", "FXART", "3DART"] {
        let dir = asset_root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        let exact = dir.join(filename);
        if exact.is_file() {
            return Some(exact);
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                if entry.file_name().to_string_lossy().to_ascii_lowercase() == filename_lower {
                    return Some(entry.path());
                }
            }
        }
    }
    None
}

pub fn auto_resolve_palette(
    asset_root: &Path,
    input_file: &Path,
    filetype: FileType,
) -> Result<Option<Palette>> {
    if !matches!(filetype, FileType::Rgm | FileType::Wld) {
        return Ok(None);
    }

    let Some(ini_path) = find_world_ini(asset_root) else {
        return Ok(None);
    };

    let content = std::fs::read_to_string(&ini_path)?;
    let world_ini = WorldIni::parse(&content);

    let file_stem = input_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let matches = match filetype {
        FileType::Rgm => world_ini.find_by_map_stem(file_stem),
        FileType::Wld => world_ini.find_by_world_stem(file_stem),
        _ => return Ok(None),
    };

    if matches.is_empty() {
        return Ok(None);
    }

    if matches.len() > 1 {
        let palettes: Vec<_> = matches
            .iter()
            .map(|e| format!("  world {}: {}", e.index, e.palette))
            .collect();
        warn!(
            "Multiple WORLD.INI entries match '{}'; using world {} palette. Alternatives:\n{}\nUse --palette to override.",
            file_stem,
            matches[0].index,
            palettes[1..].join("\n")
        );
    }

    let Some(palette_path) = find_palette_on_disk(asset_root, &matches[0].palette) else {
        warn!(
            "WORLD.INI specifies palette '{}' for world {}, but file not found under {}",
            matches[0].palette,
            matches[0].index,
            asset_root.display()
        );
        return Ok(None);
    };

    info!(
        "Auto-resolved palette from WORLD.INI (world {}): {}",
        matches[0].index,
        palette_path.display()
    );
    let data = std::fs::read(&palette_path)?;
    let palette = Palette::parse(&data).map_err(|e| eyre!("{e}"))?;
    Ok(Some(palette))
}
