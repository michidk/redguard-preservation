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
    /// Returns `true` when this cheat is active (nonzero value).
    pub fn is_on(&self) -> bool {
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
    pub fn named_cheats(&self) -> &[CheatEntry] {
        &self.entries[..CHT_CHEAT_COUNT]
    }

    /// Returns cheat entries for unused slots that have nonzero values (unexpected).
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
