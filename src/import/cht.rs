//! Parser for REDGUARD.CHT cheat persistence files.
//!
//! The CHT file is a raw 256-byte dump of the engine's cheat state array:
//! 64 little-endian `u32` values. The first 13 entries correspond to the
//! built-in cheat codes; the remaining 51 slots are unused (always zero).

use crate::{Result, error::Error};

/// Total file size in bytes (64 × 4).
pub const CHT_FILE_SIZE: usize = 256;

/// Number of u32 slots in the cheat state array.
pub const CHT_SLOT_COUNT: usize = 64;

/// Number of named cheats recognised by the engine.
pub const CHT_CHEAT_COUNT: usize = 13;

/// Cheat names in index order, matching the engine's XOR-decoded name table.
pub const CHEAT_NAMES: [&str; CHT_CHEAT_COUNT] = [
    "oracle",
    "nodemarker",
    "moonraker",
    "task",
    "animation",
    "magiccarpet",
    "savecheats",
    "drevil",
    "drno",
    "goldfinger",
    "neversaydie",
    "oddjob",
    "yeahbaby",
];

/// A single cheat entry with its index, name, and raw state value.
#[derive(Debug, Clone)]
pub struct CheatEntry {
    /// Slot index (0–12 for named cheats, 13–63 for unused slots).
    pub index: usize,
    /// Cheat name, or `None` for unnamed slots beyond the first 13.
    pub name: Option<&'static str>,
    /// Raw state value. 0 = off, 1 = on (the engine also supports
    /// arbitrary integer values set via `cheat = N` console syntax).
    pub value: u32,
}

impl CheatEntry {
    /// Returns whether the stored value is nonzero.
    /// This does not establish a gameplay effect; some engine checks require exactly one.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.value != 0
    }
}

/// Parsed CHT file.
#[derive(Debug, Clone)]
pub struct ChtFile {
    /// All 64 cheat state entries.
    pub entries: Vec<CheatEntry>,
}

impl ChtFile {
    /// Returns only the 13 named cheat entries.
    #[must_use]
    pub fn named_cheats(&self) -> &[CheatEntry] {
        &self.entries[..CHT_CHEAT_COUNT]
    }

    /// Returns cheat entries for unused slots that have nonzero values (unexpected).
    #[must_use]
    pub fn nonzero_unnamed(&self) -> Vec<&CheatEntry> {
        self.entries[CHT_CHEAT_COUNT..]
            .iter()
            .filter(|e| e.value != 0)
            .collect()
    }
}

/// Parse a REDGUARD.CHT file from raw bytes.
pub fn parse_cht_file(data: &[u8]) -> Result<ChtFile> {
    if data.len() != CHT_FILE_SIZE {
        return Err(Error::Parse(format!(
            "CHT file must be exactly {} bytes, got {}",
            CHT_FILE_SIZE,
            data.len()
        )));
    }

    let entries = (0..CHT_SLOT_COUNT)
        .map(|i| {
            let offset = i * 4;
            let value = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            CheatEntry {
                index: i,
                name: CHEAT_NAMES.get(i).copied(),
                value,
            }
        })
        .collect();

    Ok(ChtFile { entries })
}

/// Encode all 64 raw state slots as a compatible REDGUARD.CHT file.
///
/// Values, including unnamed slots and integers other than zero and one, are
/// preserved without interpreting their gameplay meaning. See `docs/formats/CHT.md`.
#[must_use]
pub fn encode_cht_file(values: &[u32; CHT_SLOT_COUNT]) -> [u8; CHT_FILE_SIZE] {
    let mut bytes = [0; CHT_FILE_SIZE];
    for (chunk, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(values) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_raw_values_in_named_and_unnamed_slots() {
        let mut values = [0; CHT_SLOT_COUNT];
        values[10] = 0x1234_5678;
        values[63] = u32::MAX;
        let bytes = encode_cht_file(&values);
        let mut expected = [0; 256];
        expected[40..44].copy_from_slice(&[0x78, 0x56, 0x34, 0x12]);
        expected[252..256].copy_from_slice(&[255, 255, 255, 255]);
        assert_eq!(bytes, expected);
        let parsed = parse_cht_file(&bytes).unwrap();
        assert_eq!(parsed.named_cheats()[10].name, Some("neversaydie"));
        assert_eq!(parsed.named_cheats()[10].value, 0x1234_5678);
        assert_eq!(parsed.nonzero_unnamed()[0].index, 63);
        assert_eq!(parsed.nonzero_unnamed()[0].value, u32::MAX);
    }

    #[test]
    fn rejects_truncated_and_extended_files() {
        for length in [0, 255, 257] {
            assert!(parse_cht_file(&vec![0; length]).is_err());
        }
    }
}
