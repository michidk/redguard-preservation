use crate::import::{
    bsi::{self, BsiFile, BsiImage},
    palette::Palette,
};
use log::warn;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct AllFramesInfo {
    pub width: u16,
    pub height: u16,
    pub frame_count: u16,
    pub frames: Vec<Option<Vec<u8>>>,
}

pub(super) struct AnimatedTexture {
    pub name: String,
    pub delay_ms: i16,
    pub frames: Vec<Vec<u8>>,
    pub has_alpha: bool,
}

pub struct TextureCache {
    palette: Option<Palette>,
    bsi_files: HashMap<u16, BsiFile>,
    texbsi_paths: HashMap<u16, PathBuf>,
    software: bool,
    opaque_images: HashSet<(u16, u16)>,
    missing_texture_requests: BTreeMap<(u16, u8), String>,
    remapped_texture_requests: HashSet<(u16, u16)>,
}

impl TextureCache {
    /// Builds a texture cache by indexing `TEXBSI.*` files under `asset_dir`.
    #[must_use]
    pub fn new(asset_dir: PathBuf, palette: Option<Palette>) -> Self {
        let software = Self::detect_software(&asset_dir);
        Self::new_for_edition(asset_dir, palette, software)
    }

    pub(crate) fn detect_software(asset_dir: &std::path::Path) -> bool {
        crate::import::registry::detect_software_assets(asset_dir)
    }

    /// Selects software TEXTURE banks or Glide TEXBSI banks explicitly.
    #[must_use]
    pub fn new_for_edition(asset_dir: PathBuf, palette: Option<Palette>, software: bool) -> Self {
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
            let prefix = if software { "TEXTURE." } else { "TEXBSI." };
            if !upper.starts_with(prefix) {
                continue;
            }

            let Some(id_part) = upper.strip_prefix(prefix) else {
                continue;
            };

            if software && id_part.len() != 3 {
                continue;
            }
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
            software,
            opaque_images: HashSet::new(),
            missing_texture_requests: BTreeMap::new(),
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
                    // Populate texbsi_paths so resolve_texture_id() can remap other IDs
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
            software: false,
            opaque_images: HashSet::new(),
            missing_texture_requests: BTreeMap::new(),
            remapped_texture_requests: HashSet::new(),
        }
    }

    /// Builds a software cache from bank bytes without filesystem access.
    pub fn from_software_data(
        data: HashMap<u16, Vec<u8>>,
        palette: Option<Palette>,
    ) -> crate::Result<Self> {
        let mut cache = Self {
            palette,
            bsi_files: HashMap::new(),
            texbsi_paths: HashMap::new(),
            software: true,
            opaque_images: HashSet::new(),
            missing_texture_requests: BTreeMap::new(),
            remapped_texture_requests: HashSet::new(),
        };
        for (id, bytes) in data {
            let bank = crate::import::software_texture::parse_texture_bank(&bytes)
                .map_err(|e| crate::error::Error::Parse(format!("TEXTURE.{id:03}: {e}")))?;
            cache.insert_software_bank(id, bank);
            cache.texbsi_paths.insert(id, PathBuf::new());
        }
        Ok(cache)
    }

    fn insert_software_bank(
        &mut self,
        id: u16,
        bank: crate::import::software_texture::SoftwareTextureBank,
    ) {
        use crate::import::{bsi::BsiImage, software_texture::SoftwareTextureEntry};
        let images = bank
            .entries
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let image_index = u16::try_from(index).ok()?;
                let (width, height, x_offset, y_offset, delay, flags, mut frames) = match entry {
                    SoftwareTextureEntry::SolidColor(color) => {
                        (1, 1, 0, 0, 0, 0, vec![vec![color]])
                    }
                    SoftwareTextureEntry::Image(image) => (
                        image.width,
                        image.height,
                        image.x_offset,
                        image.y_offset,
                        image.frame_delay,
                        image.flags,
                        image.frames,
                    ),
                };
                if frames.is_empty() {
                    return None;
                }
                if flags & 0x100 == 0 {
                    self.opaque_images.insert((id, image_index));
                }
                Some(BsiImage {
                    name: format!("{index:03}"),
                    image_index,
                    width,
                    height,
                    x_offset,
                    y_offset,
                    frame_count: u16::try_from(frames.len()).ok()?,
                    anim_delay: delay as i16,
                    tex_scale: 256,
                    data_encoding: 0,
                    is_animated: frames.len() > 1,
                    palette: None,
                    pixel_data: frames.remove(0),
                    extra_frames: frames,
                })
            })
            .collect();
        self.bsi_files.insert(id, BsiFile { images });
    }

    #[must_use]
    pub const fn palette(&self) -> Option<&Palette> {
        self.palette.as_ref()
    }

    /// Referenced texture/image failures retained by the existing fallback path.
    #[must_use]
    pub const fn missing_requests(&self) -> &BTreeMap<(u16, u8), String> {
        &self.missing_texture_requests
    }

    fn warn_missing_once(&mut self, texture_id: u16, image_id: u8, reason: &str) {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            self.missing_texture_requests.entry((texture_id, image_id))
        {
            entry.insert(reason.into());
            let prefix = if self.software { "TEXTURE" } else { "TEXBSI" };
            warn!(
                "Texture lookup failed for {prefix}.{texture_id:03} image {image_id}: {reason}; using white fallback material"
            );
        }
    }

    /// Resolves an authored texture bank against available IDs using the exporter's remapping rules.
    pub fn resolve_texture_id(texture_id: u16, contains: impl Fn(u16) -> bool) -> u16 {
        if contains(texture_id) {
            return texture_id;
        }

        let folded_9bit = texture_id & 0x01FF;
        if contains(folded_9bit) {
            return folded_9bit;
        }

        if texture_id > 500 {
            let wrapped = texture_id % 500;
            if contains(wrapped) {
                return wrapped;
            }
        }

        texture_id
    }

    fn find_texbsi_path(&self, texture_id: u16) -> Option<(u16, PathBuf)> {
        let resolved_id = if self.software {
            texture_id
        } else {
            Self::resolve_texture_id(texture_id, |id| self.texbsi_paths.contains_key(&id))
        };
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
        let resolved_id = if self.software {
            texture_id
        } else {
            Self::resolve_texture_id(texture_id, |id| self.texbsi_paths.contains_key(&id))
        };
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

        if self.software {
            match crate::import::software_texture::parse_texture_bank(&data) {
                Ok(bank) => {
                    self.insert_software_bank(texture_id, bank);
                    return true;
                }
                Err(_) => {
                    self.warn_missing_once(texture_id, image_id, "failed to parse TEXTURE bank");
                    return false;
                }
            }
        }
        let Ok(bsi) = bsi::parse_bsi_file(&data) else {
            self.warn_missing_once(texture_id, image_id, "failed to parse TEXBSI file");
            return false;
        };

        self.bsi_files.insert(texture_id, bsi.clone());
        self.bsi_files.entry(resolved_texture_id).or_insert(bsi);
        true
    }

    pub fn ensure_bsi_available(&mut self, texture_id: u16) {
        self.ensure_bsi_loaded(texture_id, 0);
    }

    #[must_use]
    pub fn image_count(&self, texture_id: u16) -> Option<usize> {
        self.bsi_files.get(&texture_id).map(|bsi| bsi.images.len())
    }

    fn image_index(&mut self, texture_id: u16, image_id: u8) -> Option<usize> {
        if !self.ensure_bsi_loaded(texture_id, image_id) {
            return None;
        }
        let index = self
            .bsi_files
            .get(&texture_id)?
            .images
            .iter()
            .position(|entry| entry.image_index == u16::from(image_id));
        if index.is_none() {
            self.warn_missing_once(texture_id, image_id, "image id not present in texture bank");
        }
        index
    }

    fn decode_cached_frame(
        &self,
        texture_id: u16,
        image: &BsiImage,
        frame: usize,
    ) -> Option<Vec<u8>> {
        let mut rgba = flip_rows_vertical(
            image.decode_frame_rgba(frame, self.palette.as_ref())?,
            image.width,
        );
        if self
            .opaque_images
            .contains(&(texture_id, image.image_index))
        {
            for pixel in rgba.as_chunks_mut::<4>().0 {
                if pixel[3] == 0 {
                    pixel[..3]
                        .copy_from_slice(&self.palette.as_ref().map_or([0; 3], |p| p.colors[0]));
                }
                pixel[3] = 255;
            }
        }
        Some(rgba)
    }

    pub(super) fn get_image_chroma_key(
        &mut self,
        texture_id: u16,
        image_id: u8,
    ) -> Option<[u8; 3]> {
        let index = self.image_index(texture_id, image_id)?;
        Some(self.bsi_files.get(&texture_id)?.images[index].palette_rgb(0, self.palette.as_ref()))
    }

    pub fn get_image_rgba(&mut self, texture_id: u16, image_id: u8) -> Option<(Vec<u8>, u16, u16)> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];
        Some((
            self.decode_cached_frame(texture_id, image, 0)?,
            image.width,
            image.height,
        ))
    }

    pub fn get_image_rgba_with_frame_count(
        &mut self,
        texture_id: u16,
        image_id: u8,
    ) -> Option<(Vec<u8>, u16, u16, u16)> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];
        Some((
            self.decode_cached_frame(texture_id, image, 0)?,
            image.width,
            image.height,
            image.frame_count,
        ))
    }

    pub fn get_all_frames_by_image_id(
        &mut self,
        texture_id: u16,
        image_id: u8,
    ) -> Option<AllFramesInfo> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];
        let frames = (0..usize::from(image.frame_count))
            .map(|frame| self.decode_cached_frame(texture_id, image, frame))
            .collect();
        Some(AllFramesInfo {
            width: image.width,
            height: image.height,
            frame_count: image.frame_count,
            frames,
        })
    }

    fn terrain_frame_rgba(&self, image: &BsiImage, frame: usize) -> Option<Vec<u8>> {
        let mut rgba = flip_rows_vertical(
            image.decode_frame_rgba(frame, self.palette.as_ref())?,
            image.width,
        );
        let zero = self
            .palette
            .as_ref()
            .map(|palette| palette.colors[0])
            .or_else(|| image.palette.as_ref().map(|palette| palette[0]))
            .unwrap_or([0; 3]);
        for pixel in rgba.as_chunks_mut::<4>().0 {
            if pixel[3] == 0 {
                pixel[..3].copy_from_slice(&zero);
            }
            pixel[3] = 255;
        }
        Some(rgba)
    }

    /// Terrain disables chroma rejection, retaining palette-zero RGB and opaque alpha.
    pub(super) fn terrain_image_png(
        &mut self,
        texture_id: u16,
        image_id: u8,
        compress: bool,
    ) -> Option<(Vec<u8>, u16, u16, bool)> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];
        let rgba = self.terrain_frame_rgba(image, 0)?;
        let png = encode_rgba_png(image.width.into(), image.height.into(), &rgba, compress)?;
        Some((png, image.width, image.height, false))
    }

    /// Authored animated image identity, delay in milliseconds, and PNG frames after frame zero.
    pub(super) fn animation_pngs(
        &mut self,
        texture_id: u16,
        image_id: u8,
        compress: bool,
        terrain: bool,
    ) -> Option<AnimatedTexture> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];
        if image.frame_count <= 1 {
            return None;
        }
        let frames = (1..usize::from(image.frame_count))
            .map(|frame| {
                let rgba = if terrain {
                    self.terrain_frame_rgba(image, frame)?
                } else {
                    flip_rows_vertical(
                        image.decode_frame_rgba(frame, self.palette.as_ref())?,
                        image.width,
                    )
                };
                encode_rgba_png(image.width.into(), image.height.into(), &rgba, compress)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(AnimatedTexture {
            name: image.name.clone(),
            delay_ms: image.anim_delay,
            frames,
            has_alpha: !terrain && image.extra_frames.iter().any(|pixels| pixels.contains(&0)),
        })
    }

    /// Returns dimensions for a texture/image pair without decoding PNG bytes.
    pub fn get_image_dimensions(&mut self, texture_id: u16, image_id: u8) -> Option<(u16, u16)> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];

        Some((image.width, image.height))
    }

    /// Returns the effective `tex_scale` factor for a texture/image pair.
    /// The raw BHDR `tex_scale` is 8.8 fixed-point; `0` is treated as `0x0100` (1.0).
    pub fn get_image_tex_scale(&mut self, texture_id: u16, image_id: u8) -> Option<f32> {
        let index = self.image_index(texture_id, image_id)?;
        let image = &self.bsi_files.get(&texture_id)?.images[index];

        let raw = if image.tex_scale == 0 {
            0x0100
        } else {
            image.tex_scale
        };
        Some(f32::from(raw) / 256.0)
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
        let has_alpha = rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 0);
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

/// Flips RGBA pixel rows vertically. BSI pixel data is stored top-to-bottom
/// but the engine's texture origin is bottom-left; flipping aligns the image
/// with glTF/Unity's top-left UV origin after the V-flip (1 − v).
fn flip_rows_vertical(rgba: Vec<u8>, width: u16) -> Vec<u8> {
    let stride = usize::from(width) * 4;
    if stride == 0 {
        return rgba;
    }
    let mut flipped = Vec::with_capacity(rgba.len());
    for row in rgba.chunks_exact(stride).rev() {
        flipped.extend_from_slice(row);
    }
    flipped
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

#[cfg(test)]
mod resolution_tests {
    use super::TextureCache;

    #[test]
    fn bank_resolution_preserves_exact_then_folded_then_wrapped_priority() {
        let resolve =
            |available: &[u16]| TextureCache::resolve_texture_id(700, |id| available.contains(&id));
        assert_eq!(resolve(&[700, 188, 200]), 700);
        assert_eq!(resolve(&[188, 200]), 188);
        assert_eq!(resolve(&[200]), 200);
        assert_eq!(resolve(&[]), 700);
    }
}
