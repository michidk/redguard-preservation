//! Lossless save containers and read-only save-slot directories.

use crate::{Result, error::Error};
use std::collections::BTreeMap;
use std::path::Path;

/// Slot metadata from the version 1.31 SVGM payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMetadata {
    pub version: [u8; 4],
    pub unknown_04: u32,
    pub unknown_08: u32,
    /// NUL-terminated display text, decoded with UTF-8 replacement characters.
    pub description: String,
    pub world_id: u32,
    /// Stored bitmap path; the loader never opens this path.
    pub bitmap_filename: String,
}

/// One chunk, including its original payload without interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSection {
    pub name: [u8; 4],
    pub data: Vec<u8>,
}

/// A SAV or TSG container. Sections retain their order and original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveGame {
    pub metadata: SaveMetadata,
    /// Includes SVGM; excludes the payload-free END terminator.
    pub sections: Vec<SaveSection>,
}

/// A loaded slot, without modifying installed files or restoring engine state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlot {
    pub game: SaveGame,
    /// Per-map snapshots keyed by uppercase filename, including `.TSG`.
    pub maps: BTreeMap<String, SaveGame>,
    /// Optional LOGBOOK.TXT bytes, retaining its original encoding and line endings.
    pub logbook: Option<Vec<u8>>,
}

/// Parse version 1.31 SAVEGAME.SAV or a per-map TSG snapshot.
/// Validates the container and metadata, not the opaque runtime records.
pub fn parse_savegame_file(mut bytes: &[u8]) -> Result<SaveGame> {
    let invalid = |message: &str| Error::Parse(format!("save game: {message}"));
    let mut metadata = None;
    let mut sections = Vec::new();
    loop {
        if bytes.starts_with(b"END ") {
            if bytes.len() != 4 {
                return Err(invalid("trailing bytes after END"));
            }
            return Ok(SaveGame {
                metadata: metadata.ok_or_else(|| invalid("missing SVGM metadata"))?,
                sections,
            });
        }
        let header = bytes
            .get(..8)
            .ok_or_else(|| invalid("truncated chunk header or missing END"))?;
        let name: [u8; 4] = header[..4].try_into().unwrap();
        let size = usize::try_from(u32::from_be_bytes(header[4..8].try_into().unwrap()))
            .map_err(|_| invalid("chunk length exceeds address space"))?;
        bytes = &bytes[8..];
        let data = bytes
            .get(..size)
            .ok_or_else(|| invalid("chunk payload exceeds remaining file bytes"))?;
        if sections.is_empty() && name != *b"SVGM" {
            return Err(invalid("first chunk must be SVGM"));
        }
        if name == *b"SVGM" {
            if metadata.is_some() {
                return Err(invalid("duplicate SVGM metadata"));
            }
            metadata = Some(parse_metadata(data)?);
        }
        sections.push(SaveSection {
            name,
            data: data.to_vec(),
        });
        bytes = &bytes[size..];
    }
}

/// Load a SAVEGAME.NNN directory, matching resource names case-insensitively.
/// Requires SAVEGAME.SAV; TSG files and LOGBOOK.TXT are optional. Unrelated files
/// are ignored. Duplicate resource names and non-regular resources are rejected.
pub fn load_save_slot(path: impl AsRef<Path>) -> Result<SaveSlot> {
    let path = path.as_ref();
    let file_error = |path: &Path, message: String| Error::File {
        path: path.to_path_buf(),
        message,
    };
    let entries = std::fs::read_dir(path).map_err(|e| file_error(path, e.to_string()))?;
    let mut resources = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|e| file_error(path, e.to_string()))?;
        let Some(name) = entry.file_name().to_str().map(str::to_ascii_uppercase) else {
            continue;
        };
        if name != "SAVEGAME.SAV" && name != "LOGBOOK.TXT" && !name.ends_with(".TSG") {
            continue;
        }
        let resource_path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| file_error(&resource_path, e.to_string()))?;
        if !kind.is_file() {
            return Err(file_error(&resource_path, "expected a regular file".into()));
        }
        if resources.insert(name, resource_path.clone()).is_some() {
            return Err(file_error(
                &resource_path,
                "duplicate resource filename".into(),
            ));
        }
    }
    let read = |path: &Path| std::fs::read(path).map_err(|e| file_error(path, e.to_string()));
    let parse = |path: &Path| {
        parse_savegame_file(&read(path)?).map_err(|e| file_error(path, e.to_string()))
    };
    let game_path = resources
        .remove("SAVEGAME.SAV")
        .ok_or_else(|| file_error(path, "missing SAVEGAME.SAV".into()))?;
    let game = parse(&game_path)?;
    let logbook = resources
        .remove("LOGBOOK.TXT")
        .map(|path| read(&path))
        .transpose()?;
    let maps = resources
        .into_iter()
        .map(|(name, path)| Ok((name, parse(&path)?)))
        .collect::<Result<_>>()?;
    Ok(SaveSlot {
        game,
        maps,
        logbook,
    })
}

/// Encode SAV or TSG sections without changing payload bytes, order, or duplicate
/// non-metadata chunks. Pass `SaveGame::sections` for a lossless round trip.
/// The sections include SVGM and exclude the END terminator. Runtime payloads
/// remain opaque; encoding does not establish that the engine can restore them.
pub fn encode_savegame_file(sections: &[SaveSection]) -> Result<Vec<u8>> {
    let invalid = |message: &str| Error::Parse(format!("save game: {message}"));
    let first = sections
        .first()
        .ok_or_else(|| invalid("missing SVGM metadata"))?;
    if first.name != *b"SVGM" {
        return Err(invalid("first chunk must be SVGM"));
    }
    parse_metadata(&first.data)?;
    let mut length = 4usize;
    for (index, section) in sections.iter().enumerate() {
        if section.name == *b"END " {
            return Err(invalid("END is a terminator, not a payload section"));
        }
        if index != 0 && section.name == *b"SVGM" {
            return Err(invalid("duplicate SVGM metadata"));
        }
        u32::try_from(section.data.len()).map_err(|_| invalid("chunk length exceeds u32"))?;
        length = length
            .checked_add(8)
            .and_then(|n| n.checked_add(section.data.len()))
            .ok_or_else(|| invalid("container length exceeds address space"))?;
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|e| Error::Conversion(format!("save game allocation: {e}")))?;
    for section in sections {
        bytes.extend_from_slice(&section.name);
        bytes.extend_from_slice(&(section.data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&section.data);
    }
    bytes.extend_from_slice(b"END ");
    Ok(bytes)
}

fn parse_metadata(data: &[u8]) -> Result<SaveMetadata> {
    let invalid = |message: &str| Error::Parse(format!("save game: {message}"));
    if data.len() != 272 {
        return Err(invalid("SVGM payload must be 272 bytes"));
    }
    if &data[..4] != b"1.31" {
        return Err(invalid("unsupported save version; expected 1.31"));
    }
    let text = |raw: &[u8]| {
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..end]).into_owned()
    };
    let number = |offset| u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    Ok(SaveMetadata {
        version: data[..4].try_into().unwrap(),
        unknown_04: number(4),
        unknown_08: number(8),
        description: text(&data[12..140]),
        world_id: number(140),
        bitmap_filename: text(&data[144..272]),
    })
}

/// SVGF values followed by the native flag-state bytes in installed flag order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFlags {
    pub values: Vec<i32>,
    /// Retained as bytes to preserve the original representation exactly.
    pub states: Vec<u8>,
}

/// Decode one SVGF payload. The count must account for every remaining byte.
pub fn parse_save_flags(data: &[u8]) -> Result<SaveFlags> {
    let invalid = || Error::Parse("save game SVGF: invalid count or payload length".into());
    let count = u32::from_le_bytes(data.get(..4).ok_or_else(invalid)?.try_into().unwrap());
    let count = usize::try_from(count).map_err(|_| invalid())?;
    let expected = count
        .checked_mul(5)
        .and_then(|n| n.checked_add(4))
        .ok_or_else(invalid)?;
    if data.len() != expected {
        return Err(invalid());
    }
    let end = 4 + count * 4;
    Ok(SaveFlags {
        values: data[4..end]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|value| i32::from_le_bytes(*value))
            .collect(),
        states: data[end..].to_vec(),
    })
}

/// Encode one SVGF payload, requiring one state byte for each flag value.
pub fn encode_save_flags(flags: &SaveFlags) -> Result<Vec<u8>> {
    if flags.values.len() != flags.states.len() {
        return Err(Error::Parse(
            "save game SVGF: flag and state counts differ".into(),
        ));
    }
    let count = u32::try_from(flags.values.len())
        .map_err(|_| Error::Parse("save game SVGF: count exceeds u32".into()))?;
    let length = flags
        .values
        .len()
        .checked_mul(5)
        .and_then(|n| n.checked_add(4))
        .ok_or_else(|| Error::Parse("save game SVGF: payload exceeds address space".into()))?;
    let mut data = Vec::new();
    data.try_reserve_exact(length)
        .map_err(|e| Error::Conversion(format!("save game SVGF allocation: {e}")))?;
    data.extend_from_slice(&count.to_le_bytes());
    for value in &flags.values {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&flags.states);
    Ok(data)
}
