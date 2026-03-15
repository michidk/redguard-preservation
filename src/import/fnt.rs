use crate::error::Error;

#[derive(Debug, Clone)]
/// Parsed FNT file with chunk order, metadata, palette, and glyph bitmap data.
pub struct FntFile {
    pub chunk_order: Vec<String>,
    pub header: FntHeader,
    pub palette: FntPalette,
    pub glyphs: Vec<FntGlyph>,
    pub rdat: Option<Vec<u8>>,
    pub trailing_padding: Vec<u8>,
}

#[derive(Debug, Clone)]
/// Parsed `FNHD` chunk fields from a Redguard FNT file.
pub struct FntHeader {
    pub description_raw: [u8; 32],
    pub description_text: String,
    pub unknown_24: u16,
    pub has_rdat: u16,
    pub unknown_28: u16,
    pub unknown_2a: u16,
    pub unknown_2c: u16,
    pub max_width: u16,
    pub line_height: u16,
    pub character_start: u16,
    pub character_count: u16,
    pub unknown_36: u16,
    pub unknown_38: u16,
    pub has_palette: u16,
}

#[derive(Debug, Clone)]
/// Parsed palette chunk (`BPAL` or `FPAL`) from a FNT file.
pub struct FntPalette {
    pub tag: [u8; 4],
    pub colors: Vec<[u8; 3]>,
}

#[derive(Debug, Clone)]
/// Parsed glyph bitmap record from the `FBMP` chunk.
pub struct FntGlyph {
    pub enabled: u16,
    pub offset_left: i16,
    pub offset_top: i16,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

fn read_u16_le(bytes: &[u8], off: usize) -> Result<u16, Error> {
    let end = off
        .checked_add(2)
        .ok_or_else(|| Error::Parse("u16 offset overflow".to_string()))?;
    let slice = bytes
        .get(off..end)
        .ok_or_else(|| Error::Parse("unexpected EOF while reading u16".to_string()))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_i16_le(bytes: &[u8], off: usize) -> Result<i16, Error> {
    let end = off
        .checked_add(2)
        .ok_or_else(|| Error::Parse("i16 offset overflow".to_string()))?;
    let slice = bytes
        .get(off..end)
        .ok_or_else(|| Error::Parse("unexpected EOF while reading i16".to_string()))?;
    Ok(i16::from_le_bytes([slice[0], slice[1]]))
}

fn parse_fnhd(data: &[u8]) -> Result<FntHeader, Error> {
    if data.len() != 56 {
        return Err(Error::Parse(format!(
            "invalid FNHD length: expected 56, got {}",
            data.len()
        )));
    }

    let mut description_raw = [0_u8; 32];
    description_raw.copy_from_slice(&data[..32]);
    let nul = description_raw
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(description_raw.len());
    let description_text = String::from_utf8_lossy(&description_raw[..nul]).to_string();

    Ok(FntHeader {
        description_raw,
        description_text,
        unknown_24: read_u16_le(data, 32)?,
        has_rdat: read_u16_le(data, 34)?,
        unknown_28: read_u16_le(data, 36)?,
        unknown_2a: read_u16_le(data, 38)?,
        unknown_2c: read_u16_le(data, 40)?,
        max_width: read_u16_le(data, 42)?,
        line_height: read_u16_le(data, 44)?,
        character_start: read_u16_le(data, 46)?,
        character_count: read_u16_le(data, 48)?,
        unknown_36: read_u16_le(data, 50)?,
        unknown_38: read_u16_le(data, 52)?,
        has_palette: read_u16_le(data, 54)?,
    })
}

fn parse_palette(tag: [u8; 4], data: &[u8]) -> Result<FntPalette, Error> {
    if !data.len().is_multiple_of(3) {
        return Err(Error::Parse(format!(
            "invalid palette length {}; must be divisible by 3",
            data.len()
        )));
    }

    let colors = data
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect::<Vec<_>>();

    Ok(FntPalette { tag, colors })
}

fn parse_fbmp(data: &[u8], glyph_count: u16) -> Result<Vec<FntGlyph>, Error> {
    let mut offset = 0_usize;
    let mut glyphs = Vec::with_capacity(glyph_count as usize);

    for _ in 0..glyph_count {
        let header_end = offset
            .checked_add(10)
            .ok_or_else(|| Error::Parse("glyph header overflow".to_string()))?;
        if header_end > data.len() {
            return Err(Error::Parse("truncated FBMP glyph header".to_string()));
        }

        let enabled = read_u16_le(data, offset)?;
        let offset_left = read_i16_le(data, offset + 2)?;
        let offset_top = read_i16_le(data, offset + 4)?;
        let width = read_u16_le(data, offset + 6)?;
        let height = read_u16_le(data, offset + 8)?;

        offset = header_end;

        let pixels_len = usize::from(width)
            .checked_mul(usize::from(height))
            .ok_or_else(|| Error::Parse("glyph pixel length overflow".to_string()))?;
        let pixel_end = offset
            .checked_add(pixels_len)
            .ok_or_else(|| Error::Parse("glyph pixel offset overflow".to_string()))?;

        if pixel_end > data.len() {
            return Err(Error::Parse("truncated FBMP glyph pixels".to_string()));
        }

        let pixels = data[offset..pixel_end].to_vec();
        offset = pixel_end;

        glyphs.push(FntGlyph {
            enabled,
            offset_left,
            offset_top,
            width,
            height,
            pixels,
        });
    }

    if offset != data.len() {
        return Err(Error::Parse(format!(
            "FBMP has {} trailing bytes after parsing {} glyphs",
            data.len() - offset,
            glyph_count
        )));
    }

    Ok(glyphs)
}

/// Parses a Redguard FNT file from raw bytes.
pub fn parse_fnt(bytes: &[u8]) -> Result<FntFile, Error> {
    let mut pos = 0_usize;

    let mut chunk_order = Vec::new();
    let mut header: Option<FntHeader> = None;
    let mut palette: Option<FntPalette> = None;
    let mut glyphs: Option<Vec<FntGlyph>> = None;
    let mut rdat: Option<Vec<u8>> = None;

    while pos < bytes.len() {
        let tag_end = pos
            .checked_add(4)
            .ok_or_else(|| Error::Parse("tag offset overflow".to_string()))?;
        if tag_end > bytes.len() {
            return Err(Error::Parse("truncated chunk tag".to_string()));
        }

        let mut tag = [0_u8; 4];
        tag.copy_from_slice(&bytes[pos..tag_end]);

        if tag == *b"END " {
            chunk_order.push("END".to_string());
            pos = tag_end;
            break;
        }

        let len_off = tag_end;
        let len_end = len_off
            .checked_add(4)
            .ok_or_else(|| Error::Parse("length offset overflow".to_string()))?;
        if len_end > bytes.len() {
            return Err(Error::Parse("truncated chunk length".to_string()));
        }

        let chunk_len = u32::from_be_bytes([
            bytes[len_off],
            bytes[len_off + 1],
            bytes[len_off + 2],
            bytes[len_off + 3],
        ]) as usize;

        let data_start = len_end;
        let data_end = data_start
            .checked_add(chunk_len)
            .ok_or_else(|| Error::Parse("chunk data offset overflow".to_string()))?;
        if data_end > bytes.len() {
            return Err(Error::Parse(format!(
                "chunk '{}' length {} exceeds file size",
                String::from_utf8_lossy(&tag),
                chunk_len
            )));
        }

        let data = &bytes[data_start..data_end];
        chunk_order.push(String::from_utf8_lossy(&tag).to_string());

        match &tag {
            b"FNHD" => {
                if header.is_some() {
                    return Err(Error::Parse("duplicate FNHD chunk".to_string()));
                }
                header = Some(parse_fnhd(data)?);
            }
            b"BPAL" | b"FPAL" => {
                if palette.is_some() {
                    return Err(Error::Parse("duplicate palette chunk".to_string()));
                }
                palette = Some(parse_palette(tag, data)?);
            }
            b"FBMP" => {
                if glyphs.is_some() {
                    return Err(Error::Parse("duplicate FBMP chunk".to_string()));
                }
                let hdr = header
                    .as_ref()
                    .ok_or_else(|| Error::Parse("FBMP encountered before FNHD".to_string()))?;
                glyphs = Some(parse_fbmp(data, hdr.character_count)?);
            }
            b"RDAT" => {
                if rdat.is_some() {
                    return Err(Error::Parse("duplicate RDAT chunk".to_string()));
                }
                rdat = Some(data.to_vec());
            }
            _ => {
                return Err(Error::Parse(format!(
                    "unsupported FNT chunk tag: '{}'",
                    String::from_utf8_lossy(&tag)
                )));
            }
        }

        pos = data_end;
    }

    let header = header.ok_or_else(|| Error::Parse("missing FNHD chunk".to_string()))?;
    let palette = palette.ok_or_else(|| Error::Parse("missing BPAL/FPAL chunk".to_string()))?;
    let glyphs = glyphs.ok_or_else(|| Error::Parse("missing FBMP chunk".to_string()))?;

    if header.has_rdat == 1 && rdat.is_none() {
        return Err(Error::Parse(
            "FNHD.HasRDAT is 1 but RDAT chunk is missing".to_string(),
        ));
    }
    if header.has_rdat == 0 && rdat.is_some() {
        return Err(Error::Parse(
            "FNHD.HasRDAT is 0 but RDAT chunk is present".to_string(),
        ));
    }

    let trailing_padding = if pos < bytes.len() {
        let tail = bytes[pos..].to_vec();
        if !tail.iter().all(|b| *b == 0) {
            return Err(Error::Parse(
                "non-zero trailing data after END marker".to_string(),
            ));
        }
        tail
    } else {
        Vec::new()
    };

    Ok(FntFile {
        chunk_order,
        header,
        palette,
        glyphs,
        rdat,
        trailing_padding,
    })
}
