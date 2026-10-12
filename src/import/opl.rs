//! Miles two-operator FM instrument banks. See docs/formats/OPL.md.

use crate::{Result, error::Error};
use serde::Serialize;

/// Register values for one FM operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OplOperator {
    pub reg_20: u8,
    pub reg_40: u8,
    pub reg_60: u8,
    pub reg_80: u8,
    pub reg_e0: u8,
}

/// An indexed 14-byte timbre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OplInstrument {
    pub patch: u8,
    pub bank: u8,
    pub offset: u32,
    pub transposition: i8,
    pub modulator: OplOperator,
    pub reg_c0: u8,
    pub carrier: OplOperator,
}

#[derive(Debug, Clone, Serialize)]
pub struct OplFile<'a> {
    pub instruments: Vec<OplInstrument>,
    #[serde(skip)]
    pub image: &'a [u8],
}

/// Parse a Miles bank with 14-byte, two-operator timbres.
///
/// Offsets are file-relative and may be shared or unordered. Other timbre sizes
/// are rejected. The two-byte FF FF terminator has no offset field.
pub fn parse_opl_file(data: &[u8]) -> Result<OplFile<'_>> {
    let mut cursor = 0;
    let mut index = Vec::new();
    loop {
        let ids = data.get(cursor..cursor + 2).ok_or_else(|| {
            Error::Parse("OPL: missing index terminator or truncated entry".into())
        })?;
        if ids == [0xff, 0xff] {
            cursor += 2;
            break;
        }
        let offset = data
            .get(cursor + 2..cursor + 6)
            .ok_or_else(|| Error::Parse("OPL: truncated index offset".into()))?;
        index.push((
            ids[0],
            ids[1],
            u32::from_le_bytes(offset.try_into().unwrap()),
        ));
        cursor += 6;
    }

    let mut instruments = Vec::with_capacity(index.len());
    for (patch, bank, offset) in index {
        let start = usize::try_from(offset)
            .ok()
            .filter(|&start| start >= cursor && start <= data.len().saturating_sub(14))
            .ok_or_else(|| Error::Parse(format!("OPL: invalid timbre offset {offset:#x}")))?;
        let record = data
            .get(start..start + 14)
            .ok_or_else(|| Error::Parse("OPL: truncated timbre".into()))?;
        let size = u16::from_le_bytes([record[0], record[1]]);
        if size != 14 {
            return Err(Error::Parse(format!("OPL: unsupported timbre size {size}")));
        }
        let operator = |i: usize| OplOperator {
            reg_20: record[i],
            reg_40: record[i + 1],
            reg_60: record[i + 2],
            reg_80: record[i + 3],
            reg_e0: record[i + 4],
        };
        instruments.push(OplInstrument {
            patch,
            bank,
            offset,
            transposition: record[2] as i8,
            modulator: operator(3),
            reg_c0: record[8],
            carrier: operator(9),
        });
    }
    Ok(OplFile {
        instruments,
        image: data,
    })
}
