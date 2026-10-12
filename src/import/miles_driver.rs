//! Stored Miles Sound System DOS driver headers. See docs/formats/MILES-DRIVER.md.

use crate::{Result, error::Error};
use serde::Serialize;

/// Hardware interface selected by the image signature, independent of its filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DriverKind {
    /// Digital sample playback (.DIG).
    Digital,
    /// MIDI output or synthesis (.MDI).
    Midi,
}

/// Sound-card I/O configuration. A value of -1 means unspecified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IoParameters {
    pub io_address: i16,
    pub irq: i16,
    pub dma_8_bit: i16,
    pub dma_16_bit: i16,
    pub reserved: [i32; 4],
}

/// On-disk VDI header and the original image, without installing or executing it.
///
/// Pointer fields are packed DOS segment:offset values, not file offsets. Drivers
/// populate their configuration pointers and service rate during initialization.
#[derive(Debug, Clone, Serialize)]
pub struct MilesDriver<'a> {
    pub kind: DriverKind,
    /// Raw driver revision, distinct from the host Miles library version.
    pub driver_version: u32,
    pub common_io_configurations: u32,
    pub num_io_configurations: u16,
    pub environment_string: u32,
    pub io: IoParameters,
    /// Background service frequency in Hz; non-positive values disable the timer.
    pub service_rate: i16,
    pub busy: u16,
    pub driver_number: u16,
    /// File-relative offset of the INT 66h dispatcher.
    pub dispatcher_offset: u16,
    pub previous_isr: u32,
    /// Older headers omit the 80-byte device name block.
    pub device_name: Option<&'a str>,
    #[serde(skip)]
    pub image: &'a [u8],
}

/// Parse a complete .DIG or .MDI image, retaining its code and data as opaque bytes.
///
/// Rejects truncated headers, invalid signatures, out-of-image dispatchers, and
/// malformed device names. Does not dereference runtime pointers or execute code.
pub fn parse_miles_driver(data: &[u8]) -> Result<MilesDriver<'_>> {
    if data.len() < 0xba {
        return Err(Error::Parse("Miles driver: truncated VDI header".into()));
    }
    let kind = if data[..7].eq_ignore_ascii_case(b"AIL3DIG") {
        DriverKind::Digital
    } else if data[..7].eq_ignore_ascii_case(b"AIL3MDI") {
        DriverKind::Midi
    } else {
        return Err(Error::Parse("Miles driver: invalid signature".into()));
    };
    if data[7] != 0x1a {
        return Err(Error::Parse(
            "Miles driver: missing signature terminator".into(),
        ));
    }

    let word = |offset| u16::from_le_bytes([data[offset], data[offset + 1]]);
    let dword = |offset| {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    };
    let driver_version = dword(8);
    // ESS AudioDrive stores revision zero but includes the newer name block.
    let header_size = if driver_version == 0 || driver_version >= 0x112 {
        0x10a
    } else {
        0xba
    };
    if data.len() < header_size {
        return Err(Error::Parse(
            "Miles driver: truncated device name block".into(),
        ));
    }
    let dispatcher_offset = word(0x34);
    if usize::from(dispatcher_offset) < header_size || usize::from(dispatcher_offset) >= data.len()
    {
        return Err(Error::Parse(format!(
            "Miles driver: dispatcher offset {dispatcher_offset:#x} is outside the driver body"
        )));
    }
    let device_name = if header_size == 0x10a {
        let bytes = &data[0xba..0x10a];
        let end = bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| Error::Parse("Miles driver: unterminated device name".into()))?;
        let name = &bytes[..end];
        if !name.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            return Err(Error::Parse("Miles driver: non-ASCII device name".into()));
        }
        Some(std::str::from_utf8(name).map_err(|e| Error::Parse(e.to_string()))?)
    } else {
        None
    };

    Ok(MilesDriver {
        kind,
        driver_version,
        common_io_configurations: dword(0x0c),
        num_io_configurations: word(0x10),
        environment_string: dword(0x12),
        io: IoParameters {
            io_address: word(0x16) as i16,
            irq: word(0x18) as i16,
            dma_8_bit: word(0x1a) as i16,
            dma_16_bit: word(0x1c) as i16,
            reserved: std::array::from_fn(|i| dword(0x1e + i * 4) as i32),
        },
        service_rate: word(0x2e) as i16,
        busy: word(0x30),
        driver_number: word(0x32),
        dispatcher_offset,
        previous_isr: dword(0x36),
        device_name,
        image: data,
    })
}
