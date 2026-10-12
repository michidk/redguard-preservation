//! Windows Program Information Files. See docs/formats/PIF.md.

use crate::{Result, error::Error};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pif386Settings {
    pub memory_max_kb: u16,
    pub memory_min_kb: u16,
    pub foreground_priority: u16,
    pub background_priority: u16,
    pub ems_max_kb: u16,
    pub ems_min_kb: u16,
    pub xms_max_kb: u16,
    pub xms_min_kb: u16,
    pub option_flags: u16,
    pub memory_flags: u16,
    pub video_flags: u16,
    pub hotkey: [u16; 9],
    pub parameters: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PifExtension<'a> {
    pub header_offset: u16,
    pub signature: String,
    pub next_offset: u16,
    pub data_offset: u16,
    pub data: &'a [u8],
    pub settings_386: Option<Pif386Settings>,
}

/// Base launch settings and the linked extension records, including PIFEX.
#[derive(Debug, Clone, Serialize)]
pub struct PifFile<'a> {
    pub unknown_00: [u8; 2],
    pub title: String,
    pub memory_max_kb: u16,
    pub memory_min_kb: u16,
    pub program: String,
    pub flags_63: u8,
    pub start_drive: u8,
    pub start_directory: String,
    pub parameters: String,
    pub video_mode: u8,
    pub irq_low: u8,
    pub irq_high: u8,
    pub rows: u8,
    pub columns: u8,
    pub window_y: u8,
    pub window_x: u8,
    pub flags_16f: u8,
    pub flags_170: u8,
    pub extensions: Vec<PifExtension<'a>>,
    #[serde(skip)]
    pub image: &'a [u8],
}

fn text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| char::from(b))
        .collect()
}

fn word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Parse a 369-byte base PIF, optionally followed by a PIFEX extension chain.
///
/// Rejects truncated records, invalid root signatures, out-of-file ranges and
/// cycles. Unknown extension payloads remain raw bytes. Text bytes map directly
/// to U+0000..U+00FF, without assuming an OEM or Windows code page.
pub fn parse_pif_file(data: &[u8]) -> Result<PifFile<'_>> {
    if data.len() < 369 {
        return Err(Error::Parse("PIF: truncated base header".into()));
    }
    let mut extensions = Vec::new();
    if data.len() > 369 {
        let mut offset = 369u16;
        let mut visited = BTreeSet::new();
        while offset != 0xffff {
            if offset < 369 || !visited.insert(offset) {
                return Err(Error::Parse("PIF: invalid or cyclic extension link".into()));
            }
            let start = usize::from(offset);
            let header = data
                .get(start..start + 22)
                .ok_or_else(|| Error::Parse("PIF: truncated extension header".into()))?;
            let next_offset = word(header, 16);
            let data_offset = word(header, 18);
            let size = usize::from(word(header, 20));
            if offset == 369
                && (&header[..16] != b"MICROSOFT PIFEX\0" || data_offset != 0 || size != 369)
            {
                return Err(Error::Parse("PIF: invalid PIFEX root".into()));
            }
            let payload = data
                .get(usize::from(data_offset)..usize::from(data_offset) + size)
                .ok_or_else(|| Error::Parse("PIF: extension data outside file".into()))?;
            let signature = text(&header[..16]);
            let settings_386 = if signature == "WINDOWS 386 3.0" {
                if payload.len() < 104 {
                    return Err(Error::Parse("PIF: truncated Windows 386 settings".into()));
                }
                Some(Pif386Settings {
                    memory_max_kb: word(payload, 0),
                    memory_min_kb: word(payload, 2),
                    foreground_priority: word(payload, 4),
                    background_priority: word(payload, 6),
                    ems_max_kb: word(payload, 8),
                    ems_min_kb: word(payload, 10),
                    xms_max_kb: word(payload, 12),
                    xms_min_kb: word(payload, 14),
                    option_flags: word(payload, 16),
                    memory_flags: word(payload, 18),
                    video_flags: word(payload, 20),
                    hotkey: std::array::from_fn(|i| word(payload, 22 + i * 2)),
                    parameters: text(&payload[40..104]),
                })
            } else {
                None
            };
            extensions.push(PifExtension {
                header_offset: offset,
                signature,
                next_offset,
                data_offset,
                data: payload,
                settings_386,
            });
            offset = next_offset;
        }
    }
    Ok(PifFile {
        unknown_00: [data[0], data[1]],
        title: text(&data[2..32]).trim_end_matches(' ').to_owned(),
        memory_max_kb: word(data, 0x20),
        memory_min_kb: word(data, 0x22),
        program: text(&data[0x24..0x63]),
        flags_63: data[0x63],
        start_drive: data[0x64],
        start_directory: text(&data[0x65..0xa5]),
        parameters: text(&data[0xa5..0xe5]),
        video_mode: data[0xe5],
        irq_low: data[0xe7],
        irq_high: data[0xe8],
        rows: data[0xe9],
        columns: data[0xea],
        window_y: data[0xeb],
        window_x: data[0xec],
        flags_16f: data[0x16f],
        flags_170: data[0x170],
        extensions,
        image: data,
    })
}
