//! IFF PBM chunky bitmaps. See docs/formats/LBM.md.

use super::indexed_image::{IndexedImage, pixel_count};
use crate::{Result, error::Error};

#[derive(Debug, Clone)]
pub struct LbmChunk<'a> {
    pub tag: [u8; 4],
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct LbmFile<'a> {
    pub compression: u8,
    pub masking: u8,
    pub image: IndexedImage,
    pub chunks: Vec<LbmChunk<'a>>,
}

/// Decode an 8-bit PBM FORM with raw or ByteRun1 rows, retaining unknown chunks.
/// Planar ILBM and mask planes are rejected. Images are limited to 16 Mi pixels.
pub fn parse_lbm_file(data: &[u8]) -> Result<LbmFile<'_>> {
    if data.len() < 12 || &data[..4] != b"FORM" || &data[8..12] != b"PBM " {
        return Err(Error::Parse("LBM: expected IFF PBM FORM".into()));
    }
    let form_size = u32::from_be_bytes(data[4..8].try_into().unwrap());
    if u64::from(form_size) + 8 != data.len() as u64 {
        return Err(Error::Parse("LBM: FORM size does not match file".into()));
    }
    let mut chunks = Vec::new();
    let mut cursor = 12;
    while cursor < data.len() {
        let header = data
            .get(cursor..cursor + 8)
            .ok_or_else(|| Error::Parse("LBM: truncated chunk header".into()))?;
        let size = usize::try_from(u32::from_be_bytes(header[4..8].try_into().unwrap()))
            .map_err(|_| Error::Parse("LBM: chunk size overflow".into()))?;
        cursor += 8;
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| Error::Parse("LBM: chunk range overflow".into()))?;
        let payload = data
            .get(cursor..end)
            .ok_or_else(|| Error::Parse("LBM: truncated chunk".into()))?;
        chunks.push(LbmChunk {
            tag: header[..4].try_into().unwrap(),
            data: payload,
        });
        cursor = end
            .checked_add(size & 1)
            .filter(|&n| n <= data.len())
            .ok_or_else(|| Error::Parse("LBM: missing chunk padding".into()))?;
    }
    let required = |tag: &[u8; 4]| -> Result<&[u8]> {
        let mut matches = chunks.iter().filter(|chunk| &chunk.tag == tag);
        let payload = matches
            .next()
            .ok_or_else(|| Error::Parse(format!("LBM: missing {}", String::from_utf8_lossy(tag))))?
            .data;
        if matches.next().is_some() {
            return Err(Error::Parse("LBM: duplicate bitmap chunk".into()));
        }
        Ok(payload)
    };
    let header = required(b"BMHD")?;
    if header.len() != 20 || header[8] != 8 || !matches!(header[9], 0 | 2) || header[10] > 1 {
        return Err(Error::Parse(
            "LBM: unsupported or truncated bitmap header".into(),
        ));
    }
    let width = u16::from_be_bytes([header[0], header[1]]);
    let height = u16::from_be_bytes([header[2], header[3]]);
    let count = pixel_count(width, height)?;
    let transparent = u16::from_be_bytes([header[12], header[13]]);
    let transparent_index = if header[9] == 2 {
        Some(
            u8::try_from(transparent)
                .map_err(|_| Error::Parse("LBM: invalid transparent index".into()))?,
        )
    } else {
        None
    };
    let colors = required(b"CMAP")?;
    if colors.is_empty() || colors.len() > 768 || !colors.len().is_multiple_of(3) {
        return Err(Error::Parse("LBM: invalid color map".into()));
    }
    let mut palette = [[0; 3]; 256];
    for (color, bytes) in palette.iter_mut().zip(colors.as_chunks::<3>().0) {
        color.copy_from_slice(bytes);
    }
    let body = required(b"BODY")?;
    let stride = (usize::from(width) + 1) & !1;
    let mut cursor = 0;
    let mut pixels = Vec::with_capacity(count);
    for _ in 0..height {
        let mut row = Vec::with_capacity(stride);
        while row.len() < stride {
            let control = *body
                .get(cursor)
                .ok_or_else(|| Error::Parse("LBM: truncated BODY".into()))?;
            cursor += 1;
            if header[10] == 0 {
                row.push(control);
                continue;
            }
            if control == 128 {
                continue;
            }
            let run = if control < 128 {
                usize::from(control) + 1
            } else {
                257 - usize::from(control)
            };
            if row.len() + run > stride {
                return Err(Error::Parse("LBM: ByteRun1 crosses row boundary".into()));
            }
            if control < 128 {
                let bytes = body
                    .get(cursor..cursor + run)
                    .ok_or_else(|| Error::Parse("LBM: truncated literal run".into()))?;
                row.extend_from_slice(bytes);
                cursor += run;
            } else {
                let value = *body
                    .get(cursor)
                    .ok_or_else(|| Error::Parse("LBM: truncated repeat run".into()))?;
                cursor += 1;
                row.resize(row.len() + run, value);
            }
        }
        pixels.extend_from_slice(&row[..usize::from(width)]);
    }
    if body[cursor..].iter().any(|&b| header[10] != 1 || b != 128) {
        return Err(Error::Parse("LBM: trailing BODY data".into()));
    }
    Ok(LbmFile {
        compression: header[10],
        masking: header[9],
        image: IndexedImage {
            width,
            height,
            palette,
            pixels,
            transparent_index,
        },
        chunks,
    })
}
