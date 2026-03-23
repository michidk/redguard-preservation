use crate::{Result, error::Error};

#[derive(Debug, Clone)]
pub struct GxaFrame {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GxaFile {
    pub title: String,
    pub frame_count: u16,
    pub frames: Vec<GxaFrame>,
}

fn read_i16_le(data: &[u8], offset: usize) -> Result<i16> {
    if offset + 2 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_i16 out of bounds at offset {offset}"
        )));
    }
    Ok(i16::from_le_bytes([data[offset], data[offset + 1]]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_u32 out of bounds at offset {offset}"
        )));
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn decode_frame_rgba(
    width: usize,
    height: usize,
    pixels: &[u8],
    palette: &[[u8; 3]; 256],
) -> Result<Vec<u8>> {
    let pixel_count = width
        .checked_mul(height)
        .ok_or_else(|| Error::Parse("GXA width*height overflow".to_string()))?;
    if pixels.len() < pixel_count {
        return Err(Error::Parse(format!(
            "GXA frame pixel data too small: got {}, need {pixel_count}",
            pixels.len()
        )));
    }

    let mut rgba = vec![0u8; pixel_count * 4];
    for y in 1..=height {
        for x in 0..width {
            let src = x + (y - 1) * width;
            let dst = x + (height - y) * width;
            let idx = usize::from(pixels[src]);
            let o = dst * 4;
            if idx == 0 {
                rgba[o..o + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                let c = palette[idx];
                rgba[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }

    Ok(rgba)
}

pub fn parse_gxa_file(data: &[u8]) -> Result<GxaFile> {
    let mut cursor = 0usize;

    let mut title = String::new();
    let mut frame_count = 0u16;
    let mut palette = [[0u8; 3]; 256];
    let mut has_palette = false;
    let mut decoded_frames: Vec<GxaFrame> = Vec::new();

    while cursor + 4 <= data.len() {
        let tag_bytes: [u8; 4] = data[cursor..cursor + 4]
            .try_into()
            .map_err(|_| Error::Parse("GXA section tag read failed".to_string()))?;
        cursor += 4;

        if tag_bytes == *b"END " {
            break;
        }

        let section_len = usize::try_from(read_u32_be(data, cursor)?)
            .map_err(|_| Error::Parse("GXA section length does not fit usize".to_string()))?;
        cursor += 4;

        if cursor + section_len > data.len() {
            return Err(Error::Parse(format!(
                "GXA section out of bounds at offset {cursor} with length {section_len}"
            )));
        }

        let section = &data[cursor..cursor + section_len];
        cursor += section_len;

        match &tag_bytes {
            b"BMHD" => {
                if section.len() < 34 {
                    return Err(Error::Parse(format!(
                        "GXA BMHD too small: {} bytes (need >= 34)",
                        section.len()
                    )));
                }
                let raw_title = &section[..22];
                let end = raw_title
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(raw_title.len());
                title = String::from_utf8_lossy(&raw_title[..end]).to_string();
                let count = read_i16_le(section, 32)?;
                if count < 0 {
                    return Err(Error::Parse(format!(
                        "GXA BMHD negative frame count: {count}"
                    )));
                }
                frame_count = u16::try_from(count)
                    .map_err(|_| Error::Parse("GXA frame count conversion failed".to_string()))?;
            }
            b"BPAL" => {
                if section.len() < 256 * 3 {
                    return Err(Error::Parse(format!(
                        "GXA BPAL too small: {} bytes (need >= 768)",
                        section.len()
                    )));
                }
                for (i, color) in palette.iter_mut().enumerate() {
                    let off = i * 3;
                    *color = [section[off], section[off + 1], section[off + 2]];
                }
                has_palette = true;
            }
            b"BBMP" => {
                if frame_count == 0 {
                    return Err(Error::Parse(
                        "GXA BBMP encountered before BMHD with valid frame count".to_string(),
                    ));
                }
                if !has_palette {
                    return Err(Error::Parse(
                        "GXA BBMP encountered before BPAL palette".to_string(),
                    ));
                }

                let mut sec_cur = 0usize;
                decoded_frames.clear();
                decoded_frames.reserve(usize::from(frame_count));

                for frame_idx in 0..usize::from(frame_count) {
                    if sec_cur + 18 > section.len() {
                        return Err(Error::Parse(format!(
                            "GXA BBMP frame {frame_idx} header out of bounds"
                        )));
                    }

                    let width_i16 = read_i16_le(section, sec_cur + 2)?;
                    let height_i16 = read_i16_le(section, sec_cur + 4)?;
                    if width_i16 <= 0 || height_i16 <= 0 {
                        return Err(Error::Parse(format!(
                            "GXA BBMP invalid dimensions for frame {frame_idx}: {width_i16}x{height_i16}"
                        )));
                    }
                    let width = usize::try_from(width_i16)
                        .map_err(|_| Error::Parse("GXA width conversion failed".to_string()))?;
                    let height = usize::try_from(height_i16)
                        .map_err(|_| Error::Parse("GXA height conversion failed".to_string()))?;

                    let pixel_count = width.checked_mul(height).ok_or_else(|| {
                        Error::Parse(format!(
                            "GXA BBMP frame {frame_idx} dimensions overflow: {width}x{height}"
                        ))
                    })?;

                    let data_start = sec_cur + 18;
                    let data_end = data_start + pixel_count;
                    if data_end > section.len() {
                        return Err(Error::Parse(format!(
                            "GXA BBMP frame {frame_idx} pixel payload out of bounds"
                        )));
                    }

                    let rgba =
                        decode_frame_rgba(width, height, &section[data_start..data_end], &palette)?;
                    decoded_frames.push(GxaFrame {
                        width: u16::try_from(width)
                            .map_err(|_| Error::Parse("GXA width > u16::MAX".to_string()))?,
                        height: u16::try_from(height)
                            .map_err(|_| Error::Parse("GXA height > u16::MAX".to_string()))?,
                        rgba,
                    });

                    sec_cur = data_end;
                }
            }
            _ => {}
        }
    }

    if decoded_frames.is_empty() {
        return Err(Error::Parse("GXA contains no decoded frames".to_string()));
    }

    Ok(GxaFile {
        title,
        frame_count,
        frames: decoded_frames,
    })
}
