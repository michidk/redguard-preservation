//! World handle API — Unity opens a world context, native owns all internals.
//!
//! This abstraction hides WORLD.INI parsing, palette resolution, and texture
//! caching from the FFI consumer. Unity only needs to know the world ID.

use crate::gltf::TextureCache;
use crate::import::palette::Palette;
use crate::import::world_ini::{self, WorldEntry, WorldIni};
use std::fmt;
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

/// Manual `Debug` implementation that surfaces useful metadata only.
///
/// A derived `Debug` would recurse into `TextureCache` (which holds decoded
/// TEXBSI byte buffers) and the cached `rgm_bytes`/`wld_bytes`, producing
/// many MB of formatter output. This impl shows the world identity and
/// load-state and elides the heavy fields with `finish_non_exhaustive`.
impl fmt::Debug for WorldHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldHandle")
            .field("assets_dir", &self.assets_dir)
            .field("entry", &self.entry)
            .field("rgm_bytes_loaded", &self.rgm_bytes.is_some())
            .field("wld_bytes_loaded", &self.wld_bytes.is_some())
            .finish_non_exhaustive()
    }
}

impl WorldHandle {
    /// Opens a world by its index from WORLD.INI.
    ///
    /// Resolves the palette and initializes the texture cache. Returns an error
    /// if the world index doesn't exist or the palette cannot be loaded.
    pub fn open(assets_dir: PathBuf, world_id: u32) -> crate::Result<Self> {
        let ini_path = world_ini::find_world_ini(&assets_dir).ok_or_else(|| {
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

        let palette_path = world_ini::find_palette_on_disk(&assets_dir, &entry.palette)
            .ok_or_else(|| {
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

    /// Opens a world using caller-supplied asset paths instead of a `WORLD.INI` entry.
    ///
    /// `palette_path` must be an absolute path to an existing file; `rgm_path`
    /// and `wld_path` are optional — pass `None` for palette-only contexts
    /// (e.g. the ModelViewer needs a palette to decode 3D model textures but
    /// has no scene) or for worlds without a terrain layer. When supplied,
    /// the paths must refer to existing files; the function does not perform
    /// extension fallback, case-insensitive directory scans, or any other
    /// lookup heuristics. For WORLD.INI-relative path resolution, use
    /// [`WorldHandle::open`](Self::open) instead.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`](crate::error::Error::Parse) if any supplied
    /// path is missing on disk, or if the palette bytes fail to parse.
    pub fn open_explicit(
        assets_dir: PathBuf,
        rgm_path: Option<String>,
        wld_path: Option<String>,
        palette_path: String,
    ) -> crate::Result<Self> {
        let palette_disk_path = require_existing_file(&palette_path, "palette_path")?;
        if let Some(ref rgm) = rgm_path {
            require_existing_file(rgm, "rgm_path")?;
        }
        if let Some(ref wld) = wld_path {
            require_existing_file(wld, "wld_path")?;
        }

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
                map: rgm_path.unwrap_or_default(),
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

    /// Resolves the RGM file path on disk. Returns `None` for palette-only
    /// handles (those opened with no `rgm_path`).
    pub fn resolve_rgm_path(&self) -> Option<PathBuf> {
        if self.entry.map.is_empty() {
            return None;
        }
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
    /// Errors when the handle was opened in palette-only mode (no `rgm_path`
    /// supplied), since there is no scene data to return.
    pub fn rgm_bytes(&mut self) -> crate::Result<&[u8]> {
        if self.rgm_bytes.is_none() {
            if self.entry.map.is_empty() {
                return Err(crate::error::Error::Parse(
                    "world handle has no RGM (palette-only context)".to_string(),
                ));
            }
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

/// Validates that `path` names an existing file on disk and returns the
/// resolved [`PathBuf`]. Used by [`WorldHandle::open_explicit`] to enforce the
/// "caller passes absolute paths" contract — no fuzzy lookup, no extension
/// guessing, no asset-tree walking.
fn require_existing_file(path: &str, label: &str) -> crate::Result<PathBuf> {
    let candidate = PathBuf::from(path);
    if !candidate.is_file() {
        return Err(crate::error::Error::Parse(format!(
            "{label}: not an existing file: {path}"
        )));
    }
    Ok(candidate)
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
    use super::WorldHandle;
    use std::path::{Path, PathBuf};

    #[test]
    fn ini_path_backslash_normalization() {
        let path = "MAPS\\ISLAND.RGM";
        let normalized = path.replace('\\', "/");
        assert_eq!(normalized, "MAPS/ISLAND.RGM");
    }

    #[test]
    fn open_explicit_rejects_missing_palette() {
        let tmp = tempdir();
        let rgm = touch(&tmp, "scene.RGM");
        let err = WorldHandle::open_explicit(
            tmp.clone(),
            Some(rgm.to_string_lossy().into_owned()),
            None,
            tmp.join("does-not-exist.COL")
                .to_string_lossy()
                .into_owned(),
        )
        .expect_err("missing palette path must fail");
        let message = err.to_string();
        assert!(
            message.contains("palette_path"),
            "error should reference palette_path argument, got: {message}"
        );
    }

    #[test]
    fn open_explicit_rejects_missing_rgm_when_supplied() {
        let tmp = tempdir();
        let palette = touch_palette(&tmp);
        let err = WorldHandle::open_explicit(
            tmp.clone(),
            Some(tmp.join("missing.RGM").to_string_lossy().into_owned()),
            None,
            palette.to_string_lossy().into_owned(),
        )
        .expect_err("missing rgm path must fail when supplied");
        assert!(err.to_string().contains("rgm_path"));
    }

    #[test]
    fn open_explicit_rejects_missing_wld_when_supplied() {
        let tmp = tempdir();
        let palette = touch_palette(&tmp);
        let rgm = touch(&tmp, "scene.RGM");
        let err = WorldHandle::open_explicit(
            tmp.clone(),
            Some(rgm.to_string_lossy().into_owned()),
            Some(tmp.join("missing.WLD").to_string_lossy().into_owned()),
            palette.to_string_lossy().into_owned(),
        )
        .expect_err("missing wld path must fail when supplied");
        assert!(err.to_string().contains("wld_path"));
    }

    #[test]
    fn open_explicit_accepts_palette_only() {
        // ModelViewer's OpenPaletteContext: no scene, no terrain, just a
        // palette so 3D model textures can be decoded.
        let tmp = tempdir();
        let palette = touch_palette(&tmp);
        let handle = WorldHandle::open_explicit(
            tmp.clone(),
            None,
            None,
            palette.to_string_lossy().into_owned(),
        )
        .expect("palette-only open should succeed");
        assert_eq!(handle.rgm_path_raw(), "");
        assert!(handle.wld_path_raw().is_none());
        assert!(handle.resolve_rgm_path().is_none());
    }

    #[test]
    fn rgm_bytes_errors_on_palette_only_handle() {
        let tmp = tempdir();
        let palette = touch_palette(&tmp);
        let mut handle = WorldHandle::open_explicit(
            tmp.clone(),
            None,
            None,
            palette.to_string_lossy().into_owned(),
        )
        .expect("palette-only open should succeed");
        let err = handle
            .rgm_bytes()
            .expect_err("rgm_bytes must error on palette-only handle");
        assert!(err.to_string().contains("palette-only"));
    }

    /// Creates a unique temp directory under the OS temp root. Avoids pulling
    /// the `tempfile` crate dependency for two tests.
    fn tempdir() -> PathBuf {
        let unique = format!(
            "rgpre-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("temp dir creation should succeed");
        dir
    }

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"").expect("touch should succeed");
        path
    }

    /// Writes the smallest byte sequence accepted by `Palette::parse` so that
    /// "good palette" paths exist for negative-path tests without depending on
    /// a real COL file.
    fn touch_palette(dir: &Path) -> PathBuf {
        // Palette::parse expects an 8-byte header + 256 RGB triplets = 776
        // bytes. The header content is ignored; only the size matters.
        let path = dir.join("dummy.COL");
        std::fs::write(&path, vec![0u8; 776]).expect("palette write should succeed");
        path
    }
}
