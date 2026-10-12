//! Deluxe Paint LPF animation pages and RunSkipDump records. See docs/formats/ANM.md.

use super::indexed_image::{IndexedImage, pixel_count};
use crate::{Result, error::Error};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AnmPage {
    pub base_record: u16,
    pub record_count: u16,
    pub data_size: u16,
}

#[derive(Debug, Clone)]
pub struct AnmFile<'a> {
    pub declared_frame_count: u32,
    pub frames_per_second: u16,
    pub last_delta_valid: bool,
    pub pages: Vec<AnmPage>,
    pub records: Vec<&'a [u8]>,
    pub loop_delta: Option<&'a [u8]>,
    pub frames: Vec<IndexedImage>,
}

fn word(data: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([data[i], data[i + 1]])
}

fn dword(data: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
}

/// Decode single-bitmap, 8-bit LPF animations. Split records and other codecs
/// are rejected. Frames are limited to 16 Mi pixels and 256 MiB of indexed pixels in total.
pub fn parse_anm_file(data: &[u8]) -> Result<AnmFile<'_>> {
    if data.len() < 1280 || &data[..4] != b"LPF " || &data[16..20] != b"ANIM" {
        return Err(Error::Parse("ANM: invalid or truncated LPF header".into()));
    }
    let page_count = usize::from(word(data, 6));
    if word(data, 4) != 256
        || page_count == 0
        || page_count > 256
        || data[24] != 0
        || data[26] > 1
        || data[27] > 1
        || data[28] != 0
        || data[29] != 1
        || data[30] != 0
        || data[31] != 1
    {
        return Err(Error::Parse("ANM: unsupported LPF layout or codec".into()));
    }
    let width = word(data, 20);
    let height = word(data, 22);
    let count = pixel_count(width, height)?;
    let record_count = usize::try_from(dword(data, 8))
        .map_err(|_| Error::Parse("ANM: record count overflow".into()))?;
    let frame_count = record_count
        .checked_sub(usize::from(data[26]))
        .filter(|&n| n > 0 && n <= data.len() / 7 && n <= (256 * 1024 * 1024) / count)
        .ok_or_else(|| Error::Parse("ANM: invalid or excessive frame count".into()))?;
    let fps = word(data, 68);
    if fps == 0 {
        return Err(Error::Parse("ANM: zero frame rate".into()));
    }
    let table_offset = usize::from(word(data, 14));
    if table_offset < 1280 {
        return Err(Error::Parse("ANM: page table overlaps palette".into()));
    }
    let table = data
        .get(table_offset..table_offset + 256 * 6)
        .ok_or_else(|| Error::Parse("ANM: truncated page table".into()))?;
    let mut pages = Vec::with_capacity(page_count);
    let mut records = vec![None; record_count];
    for i in 0..page_count {
        let descriptor = &table[i * 6..i * 6 + 6];
        let page = AnmPage {
            base_record: word(descriptor, 0),
            record_count: word(descriptor, 2),
            data_size: word(descriptor, 4),
        };
        if page.record_count & 0xc000 != 0 || page.record_count == 0 {
            return Err(Error::Parse("ANM: empty page or split record".into()));
        }
        let n = usize::from(page.record_count);
        let base = usize::from(page.base_record);
        if base + n > record_count {
            return Err(Error::Parse("ANM: page record range outside index".into()));
        }
        let start = table_offset + 256 * 6 + i * 65536;
        let end = start + 8 + n * 2 + usize::from(page.data_size);
        if end > start + 65536 {
            return Err(Error::Parse("ANM: page exceeds 64 KiB".into()));
        }
        let payload = data
            .get(start..end)
            .ok_or_else(|| Error::Parse("ANM: truncated page".into()))?;
        if &payload[..6] != descriptor {
            return Err(Error::Parse("ANM: page header disagrees with table".into()));
        }
        let mut cursor = 8 + n * 2;
        for j in 0..n {
            let size = usize::from(word(payload, 8 + j * 2));
            let record = payload
                .get(cursor..cursor + size)
                .filter(|r| r.len() >= 7)
                .ok_or_else(|| Error::Parse("ANM: truncated bitmap record".into()))?;
            if records[base + j].replace(record).is_some() {
                return Err(Error::Parse("ANM: overlapping record ranges".into()));
            }
            cursor += size;
        }
        if cursor != payload.len() {
            return Err(Error::Parse(
                "ANM: page size disagrees with record sizes".into(),
            ));
        }
        pages.push(page);
    }
    let records: Vec<&[u8]> = records
        .into_iter()
        .map(|r| r.ok_or_else(|| Error::Parse("ANM: missing record".into())))
        .collect::<Result<_>>()?;
    let palette = std::array::from_fn(|i| {
        [
            data[256 + i * 4 + 2],
            data[256 + i * 4 + 1],
            data[256 + i * 4],
        ]
    });
    let mut pixels = vec![0; count];
    let mut frames = Vec::with_capacity(frame_count);
    for record in &records[..frame_count] {
        decode_record(record, &mut pixels)?;
        frames.push(IndexedImage {
            width,
            height,
            palette,
            pixels: pixels.clone(),
            transparent_index: None,
        });
    }
    let loop_delta = if data[26] != 0 {
        records.last().copied()
    } else {
        None
    };
    if data[27] != 0
        && let Some(record) = loop_delta
    {
        decode_record(record, &mut pixels)?;
    }
    Ok(AnmFile {
        declared_frame_count: dword(data, 64),
        frames_per_second: fps,
        last_delta_valid: data[27] != 0,
        pages,
        records,
        loop_delta,
        frames,
    })
}

fn decode_record(record: &[u8], pixels: &mut [u8]) -> Result<()> {
    if record.len() < 7 || record[0] != b'B' || record[1] != 0 {
        return Err(Error::Parse("ANM: unsupported bitmap record".into()));
    }
    let mut input = 4;
    let mut output = 0;
    loop {
        let control = *record
            .get(input)
            .ok_or_else(|| Error::Parse("ANM: missing record terminator".into()))?;
        input += 1;
        let (kind, count) = match control {
            0 => {
                let count = *record
                    .get(input)
                    .ok_or_else(|| Error::Parse("ANM: truncated short run".into()))?;
                input += 1;
                (3, usize::from(count))
            }
            1..=127 => (2, usize::from(control)),
            129..=255 => (0, usize::from(control & 0x7f)),
            128 => {
                let bytes = record
                    .get(input..input + 2)
                    .ok_or_else(|| Error::Parse("ANM: truncated long operation".into()))?;
                input += 2;
                let value = u16::from_le_bytes([bytes[0], bytes[1]]);
                if value == 0 {
                    return Ok(());
                }
                let kind = value >> 14;
                let count = usize::from(value & 0x3fff);
                if kind == 2 && count == 0 {
                    return Err(Error::Parse("ANM: invalid long operation".into()));
                }
                if kind != 0 && count == 0 {
                    continue;
                }
                (kind, count + if kind == 1 { 0x4000 } else { 0 })
            }
        };
        let end = output + count;
        let target = pixels
            .get_mut(output..end)
            .ok_or_else(|| Error::Parse("ANM: operation exceeds frame bounds".into()))?;
        if kind == 2 {
            let source = record
                .get(input..input + count)
                .ok_or_else(|| Error::Parse("ANM: truncated literal data".into()))?;
            target.copy_from_slice(source);
            input += count;
        } else if kind == 3 {
            let value = *record
                .get(input)
                .ok_or_else(|| Error::Parse("ANM: missing run value".into()))?;
            input += 1;
            target.fill(value);
        }
        output = end;
    }
}
