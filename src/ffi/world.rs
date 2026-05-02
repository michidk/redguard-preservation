//! World handle API — Unity opens a world context, native owns all internals.
//!
//! This abstraction hides WORLD.INI parsing, palette resolution, and texture
//! caching from the FFI consumer. Unity only needs to know the world ID.

use crate::gltf::TextureCache;
use crate::import::palette::Palette;
use crate::import::world_ini::{WorldEntry, WorldIni};
use std::path::{Path, PathBuf};

/// Opaque world handle exposed to FFI consumers.
///
/// Owns the resolved palette, texture cache, and cached file data for a
/// specific world from WORLD.INI.
pub struct WorldHandle {
    /// Root asset directory (contains WORLD.INI, fxart/, maps/, etc.).
    pub assets_dir: PathBuf,
    /// The resolved world entry from WORLD.INI.
    pub entry: WorldEntry,
    /// Loaded palette for this world.
    pub palette: Palette,
    /// Texture cache initialized with this world's palette.
    pub texture_cache: TextureCache,
    /// Cached RGM file bytes (loaded on first access).
    rgm_bytes: Option<Vec<u8>>,
    /// Cached WLD file bytes (loaded on first access).
    wld_bytes: Option<Vec<u8>>,
}

impl WorldHandle {
    /// Opens a world by its index from WORLD.INI.
    ///
    /// Resolves the palette and initializes the texture cache. Returns an error
    /// if the world index doesn't exist or the palette cannot be loaded.
    pub fn open(assets_dir: PathBuf, world_id: u32) -> crate::Result<Self> {
        let ini_path = super::find_world_ini(&assets_dir).ok_or_else(|| {
            crate::error::Error::Parse(format!(
                "WORLD.INI not found in assets_dir: {}",
                assets_dir.display()
            ))
        })?;

        let ini_content = std::fs::read_to_string(&ini_path)?;
        let world_ini = WorldIni::parse(&ini_content);

        let entry = world_ini
            .entries
            .into_iter()
            .find(|e| e.index == world_id)
            .ok_or_else(|| {
                crate::error::Error::Parse(format!("world_id {} not found in WORLD.INI", world_id))
            })?;

        let palette_path =
            super::find_palette_on_disk(&assets_dir, &entry.palette).ok_or_else(|| {
                crate::error::Error::Parse(format!(
                    "palette not found for world {}: {}",
                    world_id, entry.palette
                ))
            })?;

        let palette_bytes = std::fs::read(&palette_path)?;
        let palette = Palette::parse(&palette_bytes)?;

        let texture_cache = TextureCache::new(
            assets_dir.clone(),
            Some(Palette {
                colors: palette.colors,
            }),
        );

        Ok(Self {
            assets_dir,
            entry,
            palette,
            texture_cache,
            rgm_bytes: None,
            wld_bytes: None,
        })
    }

    pub fn open_explicit(
        assets_dir: PathBuf,
        rgm_path: String,
        wld_path: Option<String>,
        palette_path: String,
    ) -> crate::Result<Self> {
        let palette_disk_path = super::find_palette_on_disk(&assets_dir, &palette_path)
            .or_else(|| {
                let candidate = PathBuf::from(&palette_path);
                candidate.is_file().then_some(candidate)
            })
            .ok_or_else(|| {
                crate::error::Error::Parse(format!("palette not found: {palette_path}"))
            })?;

        let palette_bytes = std::fs::read(&palette_disk_path)?;
        let palette = Palette::parse(&palette_bytes)?;

        let texture_cache = TextureCache::new(
            assets_dir.clone(),
            Some(Palette {
                colors: palette.colors,
            }),
        );

        Ok(Self {
            assets_dir,
            entry: WorldEntry {
                index: u32::MAX,
                map: rgm_path,
                world: wld_path,
                palette: palette_path,
            },
            palette,
            texture_cache,
            rgm_bytes: None,
            wld_bytes: None,
        })
    }

    /// Returns the world index.
    #[must_use]
    pub fn world_id(&self) -> u32 {
        self.entry.index
    }

    /// Returns the RGM scene path (relative, as stored in WORLD.INI).
    #[must_use]
    pub fn rgm_path_raw(&self) -> &str {
        &self.entry.map
    }

    /// Returns the WLD terrain path if present.
    #[must_use]
    pub fn wld_path_raw(&self) -> Option<&str> {
        self.entry.world.as_deref()
    }

    /// Returns the palette path (relative, as stored in WORLD.INI).
    #[must_use]
    pub fn palette_path_raw(&self) -> &str {
        &self.entry.palette
    }

    /// Resolves the RGM file path on disk.
    pub fn resolve_rgm_path(&self) -> Option<PathBuf> {
        resolve_ini_path(&self.assets_dir, &self.entry.map)
    }

    /// Resolves the WLD file path on disk.
    pub fn resolve_wld_path(&self) -> Option<PathBuf> {
        self.entry
            .world
            .as_ref()
            .and_then(|w| resolve_ini_path(&self.assets_dir, w))
    }

    /// Returns the cached RGM bytes, loading from disk on first access.
    pub fn rgm_bytes(&mut self) -> crate::Result<&[u8]> {
        if self.rgm_bytes.is_none() {
            let path = self.resolve_rgm_path().ok_or_else(|| {
                crate::error::Error::Parse(format!("RGM file not found: {}", self.entry.map))
            })?;
            self.rgm_bytes = Some(std::fs::read(path)?);
        }
        Ok(self.rgm_bytes.as_ref().unwrap())
    }

    /// Returns the cached WLD bytes, loading from disk on first access.
    pub fn wld_bytes(&mut self) -> crate::Result<&[u8]> {
        if self.wld_bytes.is_none() {
            let wld_ini_path = self.entry.world.as_ref().ok_or_else(|| {
                crate::error::Error::Parse(format!("world {} has no WLD terrain", self.entry.index))
            })?;
            let path = resolve_ini_path(&self.assets_dir, wld_ini_path).ok_or_else(|| {
                crate::error::Error::Parse(format!("WLD file not found: {}", wld_ini_path))
            })?;
            self.wld_bytes = Some(std::fs::read(path)?);
        }
        Ok(self.wld_bytes.as_ref().unwrap())
    }

    /// Returns a reference to the palette.
    #[must_use]
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns a mutable reference to the texture cache.
    pub fn texture_cache_mut(&mut self) -> &mut TextureCache {
        &mut self.texture_cache
    }
}

fn resolve_ini_path(assets_dir: &Path, ini_path: &str) -> Option<PathBuf> {
    let normalized = ini_path.replace('\\', "/");
    let absolute = PathBuf::from(&normalized);
    if absolute.is_file() {
        return Some(absolute);
    }
    let components: Vec<&str> = normalized.split('/').collect();

    let exact = assets_dir.join(&normalized);
    if exact.is_file() {
        return Some(exact);
    }

    let mut current = assets_dir.to_path_buf();
    for component in &components {
        let component_lower = component.to_ascii_lowercase();
        let entries = std::fs::read_dir(&current).ok()?;

        let mut found = None;
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            if name.to_string_lossy().to_ascii_lowercase() == component_lower {
                found = Some(entry.path());
                break;
            }
        }

        current = found?;
    }

    if current.is_file() {
        Some(current)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ini_path_backslash_normalization() {
        let path = "MAPS\\ISLAND.RGM";
        let normalized = path.replace('\\', "/");
        assert_eq!(normalized, "MAPS/ISLAND.RGM");
    }
}
