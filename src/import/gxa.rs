use crate::{Result, error::Error};
use delharc::decode::{Decoder, Lh1Decoder};

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

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_u32 out of bounds at offset {offset}"
        )));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
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
    for (i, &px) in pixels[..pixel_count].iter().enumerate() {
        let idx = usize::from(px);
        let o = i * 4;
        if idx == 0 {
            rgba[o..o + 4].copy_from_slice(&[0, 0, 0, 0]);
        } else {
            let c = palette[idx];
            rgba[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }

    Ok(rgba)
}

fn decompress_rle(compressed: &[u8], pixel_count: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(pixel_count);
    let mut cur = 0usize;

    while out.len() < pixel_count {
        if cur >= compressed.len() {
            return Err(Error::Parse(
                "GXA RLE compressed stream truncated".to_string(),
            ));
        }
        let ctrl = compressed[cur];
        cur += 1;

        if ctrl & 0x80 != 0 {
            if cur >= compressed.len() {
                return Err(Error::Parse(
                    "GXA RLE repeat run missing value byte".to_string(),
                ));
            }
            let value = compressed[cur];
            cur += 1;
            let run_len = usize::from(ctrl & 0x7F) + 1;
            for _ in 0..run_len {
                if out.len() == pixel_count {
                    break;
                }
                out.push(value);
            }
        } else {
            let run_len = usize::from(ctrl) + 1;
            if cur + run_len > compressed.len() {
                return Err(Error::Parse(
                    "GXA RLE literal run exceeds compressed stream".to_string(),
                ));
            }
            let remaining = pixel_count - out.len();
            let copy_len = run_len.min(remaining);
            out.extend_from_slice(&compressed[cur..cur + copy_len]);
            cur += run_len;
        }
    }

    Ok(out)
}

fn decompress_lzhuf(compressed: &[u8], pixel_count: usize) -> Result<Vec<u8>> {
    let mut decoder = Lh1Decoder::new(compressed);
    let mut output = vec![0u8; pixel_count];
    decoder
        .fill_buffer(&mut output)
        .map_err(|e| Error::Parse(format!("GXA LZHUF decompression failed: {e:?}")))?;
    Ok(output)
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

        let section_end = if cursor + section_len > data.len() {
            if tag_bytes == *b"BBMP" {
                data.len()
            } else {
                return Err(Error::Parse(format!(
                    "GXA section out of bounds at offset {cursor} with length {section_len}"
                )));
            }
        } else {
            cursor + section_len
        };

        let section = &data[cursor..section_end];
        cursor = section_end;

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
                // BPAL stores 6-bit VGA values (0–63); shift left by 2 for 8-bit RGB
                for (i, color) in palette.iter_mut().enumerate() {
                    let off = i * 3;
                    *color = [
                        section[off] << 2,
                        section[off + 1] << 2,
                        section[off + 2] << 2,
                    ];
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

                    let compression = read_i16_le(section, sec_cur + 10)?;

                    let pixels = match compression {
                        0 => {
                            let data_start = sec_cur + 18;
                            let data_end = data_start + pixel_count;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} raw payload out of bounds"
                                )));
                            }
                            sec_cur = data_end;
                            section[data_start..data_end].to_vec()
                        }
                        1 => {
                            if sec_cur + 22 > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} RLE header out of bounds"
                                )));
                            }
                            let comp_size = usize::try_from(read_u32_le(section, sec_cur + 18)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA RLE compressed size conversion failed".to_string(),
                                    )
                                })?;
                            let data_start = sec_cur + 22;
                            let data_end = data_start + comp_size;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} RLE payload out of bounds"
                                )));
                            }
                            let decompressed =
                                decompress_rle(&section[data_start..data_end], pixel_count)?;
                            sec_cur = data_end;
                            decompressed
                        }
                        2 => {
                            if sec_cur + 26 > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} LZHUF header out of bounds"
                                )));
                            }
                            let comp_size = usize::try_from(read_u32_le(section, sec_cur + 18)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA LZHUF compressed size conversion failed".to_string(),
                                    )
                                })?;
                            let _uncomp_size = usize::try_from(read_u32_le(section, sec_cur + 22)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA LZHUF uncompressed size conversion failed".to_string(),
                                    )
                                })?;
                            let data_start = sec_cur + 26;
                            let data_end = data_start + comp_size;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} LZHUF payload out of bounds"
                                )));
                            }
                            let decompressed =
                                decompress_lzhuf(&section[data_start..data_end], pixel_count)?;
                            sec_cur = data_end;
                            decompressed
                        }
                        _ => {
                            return Err(Error::Parse(format!(
                                "GXA BBMP frame {frame_idx} unsupported compression type {compression}"
                            )));
                        }
                    };

                    let rgba = decode_frame_rgba(width, height, &pixels, &palette)?;
                    decoded_frames.push(GxaFrame {
                        width: u16::try_from(width)
                            .map_err(|_| Error::Parse("GXA width > u16::MAX".to_string()))?,
                        height: u16::try_from(height)
                            .map_err(|_| Error::Parse("GXA height > u16::MAX".to_string()))?,
                        rgba,
                    });
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
