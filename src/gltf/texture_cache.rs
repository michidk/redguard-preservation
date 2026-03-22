use crate::import::{
    bsi::{self, BsiFile},
    palette::Palette,
};
use log::warn;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Texture and palette cache used during GLTF material/image generation.
pub struct TextureCache {
    palette: Option<Palette>,
    bsi_files: HashMap<u16, BsiFile>,
    texbsi_paths: HashMap<u16, PathBuf>,
    missing_texture_requests: HashSet<(u16, u8)>,
    remapped_texture_requests: HashSet<(u16, u16)>,
}

impl TextureCache {
    /// Builds a texture cache by indexing `TEXBSI.*` files under `asset_dir`.
    #[must_use]
    pub fn new(asset_dir: PathBuf, palette: Option<Palette>) -> Self {
        let mut texbsi_paths = HashMap::new();
        for entry in WalkDir::new(asset_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let Some(file_name) = entry.path().file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let upper = file_name.to_ascii_uppercase();
            if !upper.starts_with("TEXBSI.") {
                continue;
            }

            let Some(id_part) = upper.strip_prefix("TEXBSI.") else {
                continue;
            };

            if let Ok(texture_id) = id_part.parse::<u16>() {
                texbsi_paths
                    .entry(texture_id)
                    .or_insert_with(|| entry.path().to_path_buf());
            }
        }

        Self {
            palette,
            bsi_files: HashMap::new(),
            texbsi_paths,
            missing_texture_requests: HashSet::new(),
            remapped_texture_requests: HashSet::new(),
        }
    }

    /// Builds a texture cache from pre-loaded TEXBSI data (no filesystem access).
    ///
    /// `texbsi_data` maps TEXBSI IDs (e.g. 302) to raw file bytes.
    /// Used by the FFI layer where the caller provides all data in memory.
    #[must_use]
    pub fn from_data(texbsi_data: HashMap<u16, Vec<u8>>, palette: Option<Palette>) -> Self {
        let mut bsi_files = HashMap::new();
        let mut texbsi_paths = HashMap::new();

        for (id, data) in texbsi_data {
            match bsi::parse_bsi_file(&data) {
                Ok(parsed) => {
                    bsi_files.insert(id, parsed);
                    // Populate texbsi_paths so resolve_texbsi_id() can remap other IDs
                    // to this one. The PathBuf is never used for I/O since bsi_files
                    // is already populated.
                    texbsi_paths.insert(id, PathBuf::new());
                }
                Err(e) => {
                    warn!("Failed to parse pre-loaded TEXBSI.{id:03}: {e}");
                }
            }
        }

        Self {
            palette,
            bsi_files,
            texbsi_paths,
            missing_texture_requests: HashSet::new(),
            remapped_texture_requests: HashSet::new(),
        }
    }

    #[must_use]
    pub const fn palette(&self) -> Option<&Palette> {
        self.palette.as_ref()
    }

    fn warn_missing_once(&mut self, texture_id: u16, image_id: u8, reason: &str) {
        if self.missing_texture_requests.insert((texture_id, image_id)) {
            warn!(
                "Texture lookup failed for TEXBSI.{texture_id:03} image {image_id}: {reason}; using white fallback material"
            );
        }
    }

    fn resolve_texbsi_id(&self, texture_id: u16) -> u16 {
        if self.texbsi_paths.contains_key(&texture_id) {
            return texture_id;
        }

        let folded_9bit = texture_id & 0x01FF;
        if self.texbsi_paths.contains_key(&folded_9bit) {
            return folded_9bit;
        }

        if texture_id > 500 {
            let wrapped = texture_id % 500;
            if self.texbsi_paths.contains_key(&wrapped) {
                return wrapped;
            }
        }

        texture_id
    }

    fn find_texbsi_path(&self, texture_id: u16) -> Option<(u16, PathBuf)> {
        let resolved_id = self.resolve_texbsi_id(texture_id);
        self.texbsi_paths
            .get(&resolved_id)
            .cloned()
            .map(|path| (resolved_id, path))
    }

    fn warn_remapped_once(&mut self, requested_texture_id: u16, resolved_texture_id: u16) {
        if requested_texture_id == resolved_texture_id {
            return;
        }

        if self
            .remapped_texture_requests
            .insert((requested_texture_id, resolved_texture_id))
        {
            warn!(
                "Remapped TEXBSI id {requested_texture_id} -> {resolved_texture_id} for texture lookup"
            );
        }
    }

    fn ensure_bsi_loaded(&mut self, texture_id: u16, image_id: u8) -> bool {
        if self.bsi_files.contains_key(&texture_id) {
            return true;
        }

        // Resolve remapped ID and check if it's already loaded (covers from_data pre-loads)
        let resolved_id = self.resolve_texbsi_id(texture_id);
        if resolved_id != texture_id
            && let Some(bsi) = self.bsi_files.get(&resolved_id).cloned()
        {
            self.warn_remapped_once(texture_id, resolved_id);
            self.bsi_files.insert(texture_id, bsi);
            return true;
        }

        let Some((resolved_texture_id, path)) = self.find_texbsi_path(texture_id) else {
            self.warn_missing_once(texture_id, image_id, "file not found or unreadable");
            return false;
        };

        self.warn_remapped_once(texture_id, resolved_texture_id);

        let Ok(data) = std::fs::read(&path) else {
            self.warn_missing_once(texture_id, image_id, "file not found or unreadable");
            return false;
        };

        let Ok(bsi) = bsi::parse_bsi_file(&data) else {
            self.warn_missing_once(texture_id, image_id, "failed to parse TEXBSI file");
            return false;
        };

        self.bsi_files.insert(texture_id, bsi.clone());
        self.bsi_files.entry(resolved_texture_id).or_insert(bsi);
        true
    }

    /// Returns RGBA pixels and dimensions for a texture/image pair.
    pub fn get_image_rgba(&mut self, texture_id: u16, image_id: u8) -> Option<(Vec<u8>, u16, u16)> {
        if !self.ensure_bsi_loaded(texture_id, image_id) {
            return None;
        }

        let bsi = self.bsi_files.get(&texture_id)?;
        let Some(image) = bsi
            .images
            .iter()
            .find(|entry| entry.image_index == u16::from(image_id))
        else {
            self.warn_missing_once(texture_id, image_id, "image id not present in TEXBSI file");
            return None;
        };
        let rgba = image.decode_rgba(self.palette.as_ref());
        Some((rgba, image.width, image.height))
    }

    /// Returns dimensions for a texture/image pair without decoding PNG bytes.
    pub fn get_image_dimensions(&mut self, texture_id: u16, image_id: u8) -> Option<(u16, u16)> {
        if !self.ensure_bsi_loaded(texture_id, image_id) {
            return None;
        }

        let bsi = self.bsi_files.get(&texture_id)?;
        let Some(image) = bsi
            .images
            .iter()
            .find(|entry| entry.image_index == u16::from(image_id))
        else {
            self.warn_missing_once(texture_id, image_id, "image id not present in TEXBSI file");
            return None;
        };

        Some((image.width, image.height))
    }

    /// Returns PNG-encoded image bytes, dimensions, and whether the image
    /// contains any transparent pixels (palette index 0) for a texture/image pair.
    pub fn get_image_png(
        &mut self,
        texture_id: u16,
        image_id: u8,
        compress: bool,
    ) -> Option<(Vec<u8>, u16, u16, bool)> {
        let (rgba, width, height) = self.get_image_rgba(texture_id, image_id)?;
        let has_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] == 0);
        let Some(png_bytes) = encode_rgba_png(u32::from(width), u32::from(height), &rgba, compress)
        else {
            self.warn_missing_once(
                texture_id,
                image_id,
                "failed to encode PNG (invalid image dimensions or pixel data)",
            );
            return None;
        };
        Some((png_bytes, width, height, has_alpha))
    }
}

fn encode_rgba_png(width: u32, height: u32, rgba: &[u8], compress: bool) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};

    if width == 0 || height == 0 {
        return None;
    }

    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return None;
    }

    let mut buf = Vec::new();
    let (compression, filter) = if compress {
        (CompressionType::Default, FilterType::Adaptive)
    } else {
        (CompressionType::Fast, FilterType::NoFilter)
    };
    let encoder = PngEncoder::new_with_quality(&mut buf, compression, filter);
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(buf)
}

pub(super) fn create_palette_color_png(rgb: [u8; 3], compress: bool) -> Option<Vec<u8>> {
    let width = 8u32;
    let height = 8u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    encode_rgba_png(width, height, &rgba, compress)
}
