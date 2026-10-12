//! Eight-bit, single-plane PCX images. See docs/formats/PCX.md.

use super::indexed_image::{IndexedImage, pixel_count};
use crate::{Result, error::Error};

#[derive(Debug, Clone)]
pub struct PcxFile {
    pub version: u8,
    pub encoding: u8,
    pub x_min: u16,
    pub y_min: u16,
    pub bytes_per_line: u16,
    pub image: IndexedImage,
}

/// Decode raw or RLE 8-bit PCX with a trailing 256-color palette.
/// Other bit depths and plane counts are rejected. Images are limited to 16 Mi pixels.
pub fn parse_pcx_file(data: &[u8]) -> Result<PcxFile> {
    if data.len() < 128 + 769 || data[0] != 0x0a {
        return Err(Error::Parse(
            "PCX: invalid signature or truncated file".into(),
        ));
    }
    if data[3] != 8 || data[65] != 1 || data[2] > 1 {
        return Err(Error::Parse(
            "PCX: expected raw or RLE 8-bit single-plane image".into(),
        ));
    }
    let word = |i| u16::from_le_bytes([data[i], data[i + 1]]);
    let x_min = word(4);
    let y_min = word(6);
    let width = word(8)
        .checked_sub(x_min)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| Error::Parse("PCX: invalid horizontal bounds".into()))?;
    let height = word(10)
        .checked_sub(y_min)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| Error::Parse("PCX: invalid vertical bounds".into()))?;
    let count = pixel_count(width, height)?;
    let bytes_per_line = word(66);
    if bytes_per_line < width {
        return Err(Error::Parse(
            "PCX: scanline stride smaller than width".into(),
        ));
    }
    let palette_start = data.len() - 769;
    if data[palette_start] != 0x0c {
        return Err(Error::Parse("PCX: missing 256-color palette marker".into()));
    }
    let mut palette = [[0; 3]; 256];
    for (color, bytes) in palette
        .iter_mut()
        .zip(data[palette_start + 1..].as_chunks::<3>().0)
    {
        color.copy_from_slice(bytes);
    }
    let encoded = &data[128..palette_start];
    let mut cursor = 0;
    let mut pixels = Vec::with_capacity(count);
    for _ in 0..height {
        let mut row = Vec::with_capacity(usize::from(bytes_per_line));
        while row.len() < usize::from(bytes_per_line) {
            let control = *encoded
                .get(cursor)
                .ok_or_else(|| Error::Parse("PCX: truncated scanline".into()))?;
            cursor += 1;
            let (run, value) = if data[2] == 1 && control & 0xc0 == 0xc0 {
                let value = *encoded
                    .get(cursor)
                    .ok_or_else(|| Error::Parse("PCX: truncated RLE run".into()))?;
                cursor += 1;
                (usize::from(control & 0x3f), value)
            } else {
                (1, control)
            };
            if run == 0 || row.len() + run > usize::from(bytes_per_line) {
                return Err(Error::Parse(
                    "PCX: invalid run across scanline boundary".into(),
                ));
            }
            row.resize(row.len() + run, value);
        }
        pixels.extend_from_slice(&row[..usize::from(width)]);
    }
    if cursor != encoded.len() {
        return Err(Error::Parse(
            "PCX: trailing image data before palette".into(),
        ));
    }
    Ok(PcxFile {
        version: data[1],
        encoding: data[2],
        x_min,
        y_min,
        bytes_per_line,
        image: IndexedImage {
            width,
            height,
            palette,
            pixels,
            transparent_index: None,
        },
    })
}
