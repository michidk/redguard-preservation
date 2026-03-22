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
    /// `8.8` fixed-point texture coordinate scale factor (BHDR offsets 22–23, LE u16).
    /// `scale` = `tex_scale` / `256.0`. Default `0x0100` (`1.0`) substituted when zero.
    pub tex_scale: u16,
    /// Pixel data encoding mode (BHDR offset 24).
    /// `0` = raw uncompressed, `4` = animated offset table.
    /// Values `1`–`3` are engine-supported but absent from shipped files.
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
    /// Decodes frame 0 indexed pixels to RGBA using an external or embedded palette.
    #[must_use]
    pub fn decode_rgba(&self, palette: Option<&Palette>) -> Vec<u8> {
        self.decode_pixels_rgba(&self.pixel_data, palette)
    }

    /// Decodes a specific animation frame to RGBA.
    /// Frame 0 uses `pixel_data`; frames 1..N use `extra_frames`.
    /// Returns `None` if the frame index is out of range.
    #[must_use]
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
        let width = usize::from(self.width);
        let height = usize::from(self.height);
        let mut rgba = vec![0_u8; width * height * 4];

        for i in 0..width * height {
            let idx = pixels.get(i).copied().unwrap_or(0);
            let offset = i * 4;
            if idx == 0 {
                rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else if let Some(palette) = palette {
                let color = palette.colors[usize::from(idx)];
                rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            } else if let Some(embedded) = embedded_palette {
                let color = embedded[usize::from(idx)];
                rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            } else {
                rgba[offset..offset + 4].copy_from_slice(&[idx, idx, idx, 255]);
            }
        }

        rgba
    }
}

#[derive(Default)]
struct RecordChunks<'a> {
    is_animated: bool,
    bhdr_data: Option<&'a [u8]>,
    cmap_data: Option<&'a [u8]>,
    pixel_data: Vec<u8>,
}

type BhdrFields = (i16, i16, u16, u16, u16, i16, u16, i16);

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_le_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_le_i16(data: &[u8], offset: usize) -> Result<i16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(i16::from_le_bytes(bytes))
}

fn read_le_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| Error::Parse(format!("BSI: truncated data at offset 0x{offset:X}")))?;
    Ok(u16::from_le_bytes(bytes))
}

fn parse_record_header(data: &[u8], pos: usize) -> Result<Option<(String, usize, usize)>> {
    if pos + 13 > data.len() {
        return Ok(None);
    }

    let name_bytes = &data[pos..pos + 9];
    if name_bytes.iter().all(|&b| b == 0) {
        return Ok(None);
    }

    let name = String::from_utf8_lossy(name_bytes)
        .trim_matches('\0')
        .to_string();
    let record_size = usize::try_from(read_le_u32(data, pos + 9)?)
        .map_err(|e| Error::Parse(format!("BSI record '{name}' size does not fit usize: {e}")))?;
    let record_start = pos + 13;
    let record_end = record_start
        .checked_add(record_size)
        .ok_or_else(|| Error::Parse(format!("BSI record '{name}' end offset overflow")))?;

    if record_end > data.len() {
        return Err(Error::Parse(format!(
            "BSI record '{name}' extends past end of data (need {record_end}, have {})",
            data.len()
        )));
    }

    Ok(Some((name, record_start, record_end)))
}

fn parse_record_chunks(
    data: &[u8],
    record_start: usize,
    record_end: usize,
) -> Result<RecordChunks<'_>> {
    let mut chunks = RecordChunks::default();
    let mut sub_pos = record_start;

    while sub_pos + 4 <= record_end {
        let tag = &data[sub_pos..sub_pos + 4];
        if tag == b"END " {
            break;
        }
        if sub_pos + 8 > data.len() {
            break;
        }

        let payload_size = usize::try_from(read_be_u32(data, sub_pos + 4)?)
            .map_err(|e| Error::Parse(format!("BSI subchunk size does not fit usize: {e}")))?;
        let payload_start = sub_pos + 8;
        let payload_end = payload_start
            .checked_add(payload_size)
            .ok_or_else(|| Error::Parse("BSI subchunk payload end overflow".to_string()))?;

        if payload_end <= data.len() {
            match tag {
                b"IFHD" => chunks.is_animated = true,
                b"BHDR" => chunks.bhdr_data = Some(&data[payload_start..payload_end]),
                b"CMAP" => chunks.cmap_data = Some(&data[payload_start..payload_end]),
                b"DATA" => chunks.pixel_data = data[payload_start..payload_end].to_vec(),
                _ => {}
            }
        }

        sub_pos = payload_end;
    }

    Ok(chunks)
}

fn non_negative_i16_to_u16(value: i16) -> u16 {
    u16::try_from(value.max(0)).unwrap_or(0)
}

fn parse_bhdr_fields(bhdr_data: Option<&[u8]>) -> Result<BhdrFields> {
    if let Some(bhdr) = bhdr_data {
        if bhdr.len() >= 26 {
            return Ok((
                read_le_i16(bhdr, 0)?,
                read_le_i16(bhdr, 2)?,
                non_negative_i16_to_u16(read_le_i16(bhdr, 4)?),
                non_negative_i16_to_u16(read_le_i16(bhdr, 6)?),
                non_negative_i16_to_u16(read_le_i16(bhdr, 14)?.max(1)),
                read_le_i16(bhdr, 16)?,
                read_le_u16(bhdr, 22)?,
                read_le_i16(bhdr, 24)?,
            ));
        }
        if bhdr.len() >= 16 {
            return Ok((
                read_le_i16(bhdr, 0)?,
                read_le_i16(bhdr, 2)?,
                non_negative_i16_to_u16(read_le_i16(bhdr, 4)?),
                non_negative_i16_to_u16(read_le_i16(bhdr, 6)?),
                non_negative_i16_to_u16(read_le_i16(bhdr, 14)?.max(1)),
                0,
                0,
                0,
            ));
        }
    }

    Ok((0, 0, 0, 0, 1, 0, 0, 0))
}

fn parse_embedded_palette(cmap_data: Option<&[u8]>) -> Option<Vec<[u8; 3]>> {
    cmap_data.and_then(|data| {
        if data.len() < 768 {
            return None;
        }

        let mut palette = vec![[0_u8; 3]; 256];
        for (i, entry) in palette.iter_mut().enumerate() {
            entry[0] = data[i * 3];
            entry[1] = data[i * 3 + 1];
            entry[2] = data[i * 3 + 2];
        }
        Some(palette)
    })
}

fn decode_animated_frames(
    width: u16,
    height: u16,
    frame_count: u16,
    pixel_data: Vec<u8>,
) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
    if frame_count <= 1 {
        return Ok((pixel_data, Vec::new()));
    }

    let width_usize = usize::from(width);
    let height_usize = usize::from(height);
    let frame_count_usize = usize::from(frame_count);
    let table_entries = height_usize.saturating_mul(frame_count_usize);
    let table_bytes = table_entries.saturating_mul(4);

    if width_usize == 0 || height_usize == 0 || pixel_data.len() < table_bytes {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut offsets = Vec::with_capacity(table_entries);
    for i in 0..table_entries {
        let start = i * 4;
        let end = start + 4;
        let bytes: [u8; 4] = pixel_data
            .get(start..end)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                Error::Parse(format!("BSI: truncated frame offset table at index {i}"))
            })?;
        let offset_i32 = i32::from_le_bytes(bytes).max(0);
        let offset = usize::try_from(offset_i32)
            .map_err(|e| Error::Parse(format!("BSI frame row offset conversion failed: {e}")))?;
        offsets.push(offset);
    }

    let decode_frame = |frame_idx: usize| -> Vec<u8> {
        let mut buffer = vec![0_u8; width_usize * height_usize];
        for y in 0..height_usize {
            let offset_index = frame_idx.saturating_mul(height_usize).saturating_add(y);
            if offset_index >= offsets.len() {
                break;
            }
            let row_offset = offsets[offset_index];
            let row_end = row_offset.saturating_add(width_usize);
            if row_end <= pixel_data.len() {
                let dst = y.saturating_mul(width_usize);
                buffer[dst..dst + width_usize].copy_from_slice(&pixel_data[row_offset..row_end]);
            }
        }
        buffer
    };

    let frame0 = decode_frame(0);
    let extra_frames: Vec<Vec<u8>> = (1..frame_count_usize).map(decode_frame).collect();
    Ok((frame0, extra_frames))
}

fn parse_image_record(data: &[u8], pos: usize) -> Result<Option<(BsiImage, usize)>> {
    let Some((name, record_start, record_end)) = parse_record_header(data, pos)? else {
        return Ok(None);
    };

    let image_index = if name.len() >= 6 {
        name[3..].parse().unwrap_or(0)
    } else {
        0
    };

    let chunks = parse_record_chunks(data, record_start, record_end)?;
    let (x_offset, y_offset, width, height, frame_count, anim_delay, tex_scale, data_encoding) =
        parse_bhdr_fields(chunks.bhdr_data)?;
    let palette = parse_embedded_palette(chunks.cmap_data);
    let (pixel_data, extra_frames) =
        decode_animated_frames(width, height, frame_count, chunks.pixel_data)?;

    let image = BsiImage {
        name,
        image_index,
        width,
        height,
        x_offset,
        y_offset,
        frame_count,
        anim_delay,
        tex_scale,
        data_encoding,
        is_animated: chunks.is_animated,
        palette,
        pixel_data,
        extra_frames,
    };

    Ok(Some((image, record_end)))
}

/// Parses a BSI/TEXBSI byte slice into image records.
#[allow(clippy::missing_errors_doc)]
pub fn parse_bsi_file(data: &[u8]) -> Result<BsiFile> {
    let mut images = Vec::new();
    let mut pos = 0_usize;

    while let Some((image, record_end)) = parse_image_record(data, pos)? {
        images.push(image);
        pos = record_end;
        if pos + 8 <= data.len() && &data[pos..pos + 4] == b"END " {
            pos += 8;
        }
    }

    Ok(BsiFile { images })
}
