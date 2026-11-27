//! Read-only packed screen images retained by original save containers.
use super::SaveGame;
use crate::{Result, error::Error};

/// The two fixed-size image chunks in version 1.31 saves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveImageKind {
    /// Full screen used behind progress while loading a saved game.
    Screen,
    /// Book-page save preview.
    Thumbnail,
}

impl SaveImageKind {
    /// Stored row width, in pixels.
    pub const fn width(self) -> usize {
        match self {
            Self::Screen => 640,
            Self::Thumbnail => 256,
        }
    }

    /// Number of stored rows.
    pub const fn height(self) -> usize {
        match self {
            Self::Screen => 480,
            Self::Thumbnail => 192,
        }
    }

    const fn chunk(self) -> &'static str {
        match self {
            Self::Screen => "SCR3",
            Self::Thumbnail => "THMB",
        }
    }
}

/// Packed 16-bit framebuffer words in stored row order, without a palette lookup.
/// The original payload remains in the save for lossless serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveImage {
    kind: SaveImageKind,
    pixels: Vec<u16>,
}

impl SaveImage {
    /// Image role and its fixed dimensions.
    pub fn kind(&self) -> SaveImageKind {
        self.kind
    }

    /// One word per pixel, row-major, decoded from little-endian storage.
    pub fn pixels(&self) -> &[u16] {
        &self.pixels
    }
}

impl SaveGame {
    /// Decode an optional original image without opening the metadata bitmap path.
    /// Missing and empty chunks contain no image. Duplicate chunks and nonempty
    /// payloads with a different fixed size are rejected with world/chunk context.
    pub fn image(&self, kind: SaveImageKind) -> Result<Option<SaveImage>> {
        let mut sections = self
            .sections
            .iter()
            .filter(|s| s.name == kind.chunk().as_bytes());
        let Some(section) = sections.next() else {
            return Ok(None);
        };
        let invalid = |message| {
            Error::Parse(format!(
                "save game world {} {}: {message}",
                self.metadata.world_id,
                kind.chunk()
            ))
        };
        if sections.next().is_some() {
            return Err(invalid("duplicate image chunk".to_string()));
        }
        if section.data.is_empty() {
            return Ok(None);
        }
        let expected = kind.width() * kind.height() * 2;
        if section.data.len() != expected {
            return Err(invalid(format!(
                "expected {expected} bytes, got {}",
                section.data.len()
            )));
        }
        let (words, _) = section.data.as_chunks::<2>();
        let pixels = words.iter().copied().map(u16::from_le_bytes).collect();
        Ok(Some(SaveImage { kind, pixels }))
    }
}
