//! Software-renderer TEXTURE banks and palette lookup tables.

use crate::{Result, error::Error};

/// A software image record. Frames contain row-major palette indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareTexture {
    pub x_offset: i16,
    pub y_offset: i16,
    pub width: u16,
    pub height: u16,
    pub flags: u16,
    pub frame_delay: u16,
    pub scale_x: i16,
    pub scale_y: i16,
    pub frames: Vec<Vec<u8>>,
}

impl SoftwareTexture {
    /// Decodes an indexed frame. Bit 8 makes palette index zero transparent.
    pub fn rgba(&self, frame: usize, palette: Option<&super::palette::Palette>) -> Result<Vec<u8>> {
        let pixels = self
            .frames
            .get(frame)
            .ok_or_else(|| Error::Parse(format!("TEXTURE: frame {frame} outside image")))?;
        Ok(pixels
            .iter()
            .flat_map(|&index| {
                let [r, g, b] =
                    palette.map_or([index; 3], |palette| palette.colors[usize::from(index)]);
                [
                    r,
                    g,
                    b,
                    if index == 0 && self.flags & 0x100 != 0 {
                        0
                    } else {
                        255
                    },
                ]
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftwareTextureEntry {
    SolidColor(u8),
    Image(SoftwareTexture),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareTextureBank {
    pub name: String,
    pub entries: Vec<SoftwareTextureEntry>,
}

fn bytes(input: &[u8], offset: usize, count: usize) -> Result<&[u8]> {
    offset
        .checked_add(count)
        .and_then(|end| input.get(offset..end))
        .ok_or_else(|| {
            Error::Parse(format!(
                "TEXTURE: truncated data at byte {offset}, need {count} bytes"
            ))
        })
}
fn word(input: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes(input, offset, 2)?.try_into().unwrap(),
    ))
}
fn dword(input: &[u8], offset: usize) -> Result<usize> {
    usize::try_from(u32::from_le_bytes(
        bytes(input, offset, 4)?.try_into().unwrap(),
    ))
    .map_err(|_| Error::Parse("TEXTURE: offset exceeds address space".into()))
}
fn relative(input: &[u8], base: usize, offset: usize) -> Result<usize> {
    let result = base
        .checked_add(offset)
        .ok_or_else(|| Error::Parse("TEXTURE: offset overflow".into()))?;
    bytes(input, result, 0)?;
    Ok(result)
}

/// Parses every record and animation frame, rejecting incomplete pixel data.
pub fn parse_texture_bank(input: &[u8]) -> Result<SoftwareTextureBank> {
    let count = usize::from(word(input, 0)?);
    let name_bytes = bytes(input, 2, 24)?;
    let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(24);
    let name = String::from_utf8_lossy(&name_bytes[..name_end])
        .trim()
        .to_owned();
    bytes(input, 26, count * 20)?;
    let mut entries = Vec::with_capacity(count);
    let mut total_pixels = 0usize;
    for index in 0..count {
        let directory = 26 + index * 20;
        let offset = dword(input, directory + 2)?;
        let entry = if offset == 0 {
            SoftwareTextureEntry::SolidColor(bytes(input, directory + 1, 1)?[0])
        } else {
            let pixels =
                usize::from(word(input, offset + 4)?) * usize::from(word(input, offset + 6)?);
            let decoded = pixels
                .checked_mul(usize::from(word(input, offset + 20)?))
                .ok_or_else(|| Error::Parse("TEXTURE: decoded pixel count overflow".into()))?;
            total_pixels = total_pixels
                .checked_add(decoded)
                .ok_or_else(|| Error::Parse("TEXTURE: decoded pixel count overflow".into()))?;
            if total_pixels > 16 * 1024 * 1024 {
                return Err(Error::Parse(
                    "TEXTURE: bank exceeds 16777216 decoded pixels".into(),
                ));
            }
            parse_image(input, offset)
                .map(SoftwareTextureEntry::Image)
                .map_err(|e| Error::Parse(format!("TEXTURE record {index}: {e}")))?
        };
        entries.push(entry);
    }
    Ok(SoftwareTextureBank { name, entries })
}

fn parse_image(input: &[u8], offset: usize) -> Result<SoftwareTexture> {
    bytes(input, offset, 28)?;
    let width = word(input, offset + 4)?;
    let height = word(input, offset + 6)?;
    let flags = word(input, offset + 8)?;
    let frame_count = usize::from(word(input, offset + 20)?);
    let mut frames = Vec::with_capacity(frame_count);
    if frame_count != 0 {
        super::indexed_image::pixel_count(width, height)?;
        let data = relative(input, offset, dword(input, offset + 14)?)?;
        if frame_count == 1 {
            let stride = if flags & 0x0800 != 0 {
                256
            } else {
                usize::from(width)
            };
            if usize::from(width) > stride {
                return Err(Error::Parse("TEXTURE: width exceeds row stride".into()));
            }
            let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height));
            for row in 0..usize::from(height) {
                pixels.extend_from_slice(bytes(input, data + row * stride, usize::from(width))?);
            }
            frames.push(pixels);
        } else {
            bytes(input, data, frame_count * 4)?;
            for frame in 0..frame_count {
                let start = relative(input, data, dword(input, data + frame * 4)?)?;
                if word(input, start)? != width || word(input, start + 2)? != height {
                    return Err(Error::Parse(format!(
                        "TEXTURE frame {frame}: inconsistent dimensions"
                    )));
                }
                let mut cursor = start + 4;
                let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height));
                for _ in 0..height {
                    let mut x = 0;
                    while x < usize::from(width) {
                        let run = bytes(input, cursor, 2)?;
                        cursor += 2;
                        let transparent = usize::from(run[0]);
                        let literal = usize::from(run[1]);
                        if transparent + literal == 0
                            || x + transparent + literal > usize::from(width)
                        {
                            return Err(Error::Parse(format!(
                                "TEXTURE frame {frame}: invalid pixel run"
                            )));
                        }
                        pixels.resize(pixels.len() + transparent, 0);
                        pixels.extend_from_slice(bytes(input, cursor, literal)?);
                        cursor += literal;
                        x += transparent + literal;
                    }
                }
                frames.push(pixels);
            }
        }
    }
    Ok(SoftwareTexture {
        x_offset: word(input, offset)? as i16,
        y_offset: word(input, offset + 2)? as i16,
        width,
        height,
        flags,
        frame_delay: word(input, offset + 22)?,
        scale_x: word(input, offset + 24)? as i16,
        scale_y: word(input, offset + 26)? as i16,
        frames,
    })
}

/// A SHADE or HAZE file's 64 rows of 256 palette-index mappings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteLookupTable {
    pub rows: Vec<[u8; 256]>,
}

impl PaletteLookupTable {
    pub fn parse(input: &[u8]) -> Result<Self> {
        if input.len() != 16384 {
            return Err(Error::Parse(format!(
                "SHADE/HAZE: expected 16384 bytes, got {}",
                input.len()
            )));
        }
        Ok(Self {
            rows: input.as_chunks::<256>().0.to_vec(),
        })
    }

    #[must_use]
    pub fn map(&self, row: usize, index: u8) -> Option<u8> {
        self.rows.get(row).map(|values| values[usize::from(index)])
    }
}
