//! Microsoft SideWinder version 3 profiles. See docs/formats/SIDEWINDER.md.

use crate::{Result, error::Error};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SidewinderControl<'a> {
    pub name: String,
    pub field_40: u32,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Serialize)]
pub struct SidewinderProfile<'a> {
    pub version: u32,
    pub device_id: [u8; 16],
    pub header: &'a [u8],
    pub controls: Vec<SidewinderControl<'a>>,
    #[serde(skip)]
    pub image: &'a [u8],
}

/// Read the fixed header and control records, retaining all opaque profile data.
pub fn parse_sidewinder_file(data: &[u8]) -> Result<SidewinderProfile<'_>> {
    const HEADER_SIZE: usize = 142;
    const RECORD_SIZE: usize = 684;
    if data.len() < HEADER_SIZE + RECORD_SIZE {
        return Err(Error::Parse("SideWinder: truncated profile".into()));
    }
    let word = |offset| u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    if usize::try_from(word(0)).ok() != Some(data.len()) {
        return Err(Error::Parse("SideWinder: file size mismatch".into()));
    }
    if &data[12..22] != b"SideWinder" || word(4) != 3 {
        return Err(Error::Parse(
            "SideWinder: expected version 3 profile".into(),
        ));
    }
    if !(data.len() - HEADER_SIZE).is_multiple_of(RECORD_SIZE) {
        return Err(Error::Parse("SideWinder: incomplete control record".into()));
    }
    let controls = data[HEADER_SIZE..]
        .as_chunks::<RECORD_SIZE>()
        .0
        .iter()
        .map(|record| {
            let end = record[..64].iter().position(|&b| b == 0).unwrap_or(64);
            SidewinderControl {
                name: record[..end].iter().map(|&b| char::from(b)).collect(),
                field_40: u32::from_le_bytes(record[64..68].try_into().unwrap()),
                data: record.as_slice(),
            }
        })
        .collect();
    Ok(SidewinderProfile {
        version: word(4),
        device_id: data[22..38].try_into().unwrap(),
        header: &data[..HEADER_SIZE],
        controls,
        image: data,
    })
}
