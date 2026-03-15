use super::palette::Palette;
use crate::{Result, error::Error};

#[derive(Debug, Clone)]
/// Parsed image entry from a BSI or TEXBSI file.
pub struct BsiImage {
    pub name: String,
    pub image_index: u16,
    pub width: u16,
    pub height: u16,
    pub x_offset: i16,
    pub y_offset: i16,
    pub frame_count: u16,
    /// Animation frame duration in milliseconds (BHDR offset 16).
    /// Converted at runtime to DOS PIT timer ticks: round(ms × 18.2 / 1000).
    /// Clamped to minimum 1. Range 0–500, typically 71.
    pub anim_delay: i16,
    /// 8.8 fixed-point texture coordinate scale factor (BHDR offsets 22–23, LE u16).
    /// scale = tex_scale / 256.0. Default 0x0100 (1.0) substituted when zero.
    pub tex_scale: u16,
    /// Pixel data encoding mode (BHDR offset 24).
    /// 0 = raw uncompressed, 4 = animated offset table.
    /// Values 1–3 are engine-supported but absent from shipped files.
    pub data_encoding: i16,
    pub is_animated: bool,
    pub palette: Option<Vec<[u8; 3]>>,
    pub pixel_data: Vec<u8>,
    /// Additional animation frames beyond frame 0 (pixel data per frame).
    /// Empty for static images. Frame 0 is always in `pixel_data`.
    pub extra_frames: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
/// Parsed BSI file containing one or more image entries.
pub struct BsiFile {
    pub images: Vec<BsiImage>,
}

impl BsiImage {
    /// Decodes indexed pixels to RGBA using an external or embedded palette.
    /// Decodes frame 0 indexed pixels to RGBA using an external or embedded palette.
    pub fn decode_rgba(&self, palette: Option<&Palette>) -> Vec<u8> {
        self.decode_pixels_rgba(&self.pixel_data, palette)
    }

    /// Decodes a specific animation frame to RGBA.
    /// Frame 0 uses `pixel_data`; frames 1..N use `extra_frames`.
    /// Returns `None` if the frame index is out of range.
    pub fn decode_frame_rgba(&self, frame: usize, palette: Option<&Palette>) -> Option<Vec<u8>> {
        let pixels = if frame == 0 {
            &self.pixel_data
        } else {
            self.extra_frames.get(frame - 1)?
        };
        Some(self.decode_pixels_rgba(pixels, palette))
    }

    fn decode_pixels_rgba(&self, pixels: &[u8], palette: Option<&Palette>) -> Vec<u8> {
        let embedded_palette = self.palette.as_ref();
        let w = self.width as usize;
        let h = self.height as usize;
        let mut rgba = vec![0u8; w * h * 4];

        for i in 0..w * h {
            let idx = if i < pixels.len() { pixels[i] } else { 0 };

            let offset = i * 4;
            if idx == 0 {
                rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else if let Some(palette) = palette {
                let c = palette.colors[idx as usize];
                rgba[offset..offset + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            } else if let Some(embedded) = embedded_palette {
                let c = embedded[idx as usize];
                rgba[offset..offset + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            } else {
                rgba[offset..offset + 4].copy_from_slice(&[idx, idx, idx, 255]);
            }
        }

        rgba
    }
}

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_le_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_le_i16(data: &[u8], offset: usize) -> Result<i16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(i16::from_le_bytes(bytes))
}

fn read_le_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u16::from_le_bytes(bytes))
}

/// Parses a BSI/TEXBSI byte slice into image records.
pub fn parse_bsi_file(data: &[u8]) -> Result<BsiFile> {
    let mut images = Vec::new();
    let mut pos = 0;

    while pos + 13 <= data.len() {
        let name_bytes = &data[pos..pos + 9];
        if name_bytes.iter().all(|&b| b == 0) {
            break;
        }

        let name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();
        let record_size = read_le_u32(data, pos + 9)? as usize;
        let record_start = pos + 13;
        let record_end = record_start + record_size;

        if record_end > data.len() {
            return Err(Error::Parse(format!(
                "BSI record '{name}' extends past end of data (need {record_end}, have {})",
                data.len()
            )));
        }

        let image_index: u16 = if name.len() >= 6 {
            name[3..].parse().unwrap_or(0)
        } else {
            0
        };

        let mut is_animated = false;
        let mut bhdr_data: Option<&[u8]> = None;
        let mut cmap_data: Option<&[u8]> = None;
        let mut pixel_data: Vec<u8> = Vec::new();

        let mut sub_pos = record_start;
        while sub_pos + 4 <= record_end {
            let tag = &data[sub_pos..sub_pos + 4];

            if tag == b"END " {
                break;
            }

            if sub_pos + 8 > data.len() {
                break;
            }

            let payload_size = read_be_u32(data, sub_pos + 4)? as usize;
            let payload_start = sub_pos + 8;
            let payload_end = payload_start + payload_size;

            match tag {
                b"BSIF" => {}
                b"IFHD" => {
                    is_animated = true;
                }
                b"BHDR" => {
                    if payload_end <= data.len() {
                        bhdr_data = Some(&data[payload_start..payload_end]);
                    }
                }
                b"CMAP" => {
                    if payload_end <= data.len() {
                        cmap_data = Some(&data[payload_start..payload_end]);
                    }
                }
                b"DATA" => {
                    if payload_end <= data.len() {
                        pixel_data = data[payload_start..payload_end].to_vec();
                    }
                }
                _ => {}
            }

            sub_pos = payload_end;
        }

        let (
            x_offset,
            y_offset,
            width,
            height,
            frame_count,
            anim_delay,
            effect_word,
            data_encoding,
        ) = if let Some(bhdr) = bhdr_data {
            if bhdr.len() >= 26 {
                (
                    read_le_i16(bhdr, 0)?,
                    read_le_i16(bhdr, 2)?,
                    read_le_i16(bhdr, 4)?.max(0) as u16,
                    read_le_i16(bhdr, 6)?.max(0) as u16,
                    read_le_i16(bhdr, 14)?.max(1) as u16,
                    read_le_i16(bhdr, 16)?,
                    read_le_u16(bhdr, 22)?,
                    read_le_i16(bhdr, 24)?,
                )
            } else if bhdr.len() >= 16 {
                (
                    read_le_i16(bhdr, 0)?,
                    read_le_i16(bhdr, 2)?,
                    read_le_i16(bhdr, 4)?.max(0) as u16,
                    read_le_i16(bhdr, 6)?.max(0) as u16,
                    read_le_i16(bhdr, 14)?.max(1) as u16,
                    0,
                    0,
                    0,
                )
            } else {
                (0, 0, 0, 0, 1, 0, 0, 0)
            }
        } else {
            (0, 0, 0, 0, 1, 0, 0, 0)
        };

        let palette = cmap_data.and_then(|d| {
            if d.len() >= 768 {
                let mut pal = vec![[0u8; 3]; 256];
                for (i, entry) in pal.iter_mut().enumerate() {
                    entry[0] = d[i * 3];
                    entry[1] = d[i * 3 + 1];
                    entry[2] = d[i * 3 + 2];
                }
                Some(pal)
            } else {
                None
            }
        });

        let (decoded_pixel_data, extra_frames) = if frame_count <= 1 {
            (pixel_data, Vec::new())
        } else {
            let w = width as usize;
            let h = height as usize;
            let fc = frame_count as usize;
            let table_entries = h.saturating_mul(fc);
            let table_bytes = table_entries.saturating_mul(4);

            if w == 0 || h == 0 || pixel_data.len() < table_bytes {
                (Vec::new(), Vec::new())
            } else {
                let mut offsets = Vec::with_capacity(table_entries);
                for i in 0..table_entries {
                    let start = i * 4;
                    let end = start + 4;
                    let bytes: [u8; 4] = pixel_data
                        .get(start..end)
                        .and_then(|s| s.try_into().ok())
                        .ok_or_else(|| {
                            Error::Parse(format!("BSI: truncated frame offset table at index {i}"))
                        })?;
                    let offset = i32::from_le_bytes(bytes).max(0) as usize;
                    offsets.push(offset);
                }

                let decode_frame = |frame_idx: usize| -> Vec<u8> {
                    let mut buf = vec![0u8; w * h];
                    for y in 0..h {
                        let offset_index = frame_idx * h + y;
                        if offset_index >= offsets.len() {
                            break;
                        }
                        let row_offset = offsets[offset_index];
                        let row_end = row_offset.saturating_add(w);
                        if row_end <= pixel_data.len() {
                            let dst = y * w;
                            buf[dst..dst + w].copy_from_slice(&pixel_data[row_offset..row_end]);
                        }
                    }
                    buf
                };

                let frame0 = decode_frame(0);
                let extra: Vec<Vec<u8>> = (1..fc).map(decode_frame).collect();
                (frame0, extra)
            }
        };

        images.push(BsiImage {
            name,
            image_index,
            width,
            height,
            x_offset,
            y_offset,
            frame_count,
            anim_delay,
            tex_scale: effect_word,
            data_encoding,
            is_animated,
            palette,
            pixel_data: decoded_pixel_data,
            extra_frames,
        });

        pos = record_end;
        if pos + 8 <= data.len() && &data[pos..pos + 4] == b"END " {
            pos += 8;
        }
    }

    Ok(BsiFile { images })
}
