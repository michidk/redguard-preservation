//! Saved spatial-list and family tables. Actor references remain save identities.
use crate::{Result, error::Error};

/// One populated spatial list's first serialized actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSpatialList {
    /// Runtime spatial-list identity, distinct from an actor identity.
    pub list_id: u32,
    /// First serialized actor; following actors are linked through SMOB.
    pub first_actor_id: i32,
}

/// SMGR spatial-list heads followed by the complete 128 by 128 cell table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSpatialGrid {
    pub lists: Vec<SaveSpatialList>,
    /// Row-major cell-to-list identities. Zero denotes no list.
    pub cells: Vec<u16>,
}

/// Parse SMGR framing without accepting spatial-list identities as pointers.
pub fn parse_save_spatial_grid(bytes: &[u8]) -> Result<SaveSpatialGrid> {
    let invalid = |message| Error::Parse(format!("save game SMGR: {message}"));
    let count = bytes
        .get(..4)
        .ok_or_else(|| invalid("missing list count"))?;
    let count = u32::from_le_bytes(count.try_into().unwrap()) as usize;
    let size = count
        .checked_mul(8)
        .and_then(|n| n.checked_add(4 + 32768))
        .ok_or_else(|| invalid("list count exceeds address space"))?;
    if bytes.len() != size {
        return Err(invalid(
            "list count and cell table do not match payload length",
        ));
    }
    let lists = bytes[4..4 + count * 8]
        .as_chunks::<8>()
        .0
        .iter()
        .map(|record| SaveSpatialList {
            list_id: u32::from_le_bytes(record[..4].try_into().unwrap()),
            first_actor_id: i32::from_le_bytes(record[4..].try_into().unwrap()),
        })
        .collect();
    let cells = bytes[4 + count * 8..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|word| u16::from_le_bytes(*word))
        .collect();
    Ok(SaveSpatialGrid { lists, cells })
}

/// A family slot's actor and two saved vector blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveFamily {
    /// Saved actor identity. Minus one and zero resolve to no actor; minus two
    /// and minus three select the two player owners.
    pub actor_id: i32,
    /// Accumulated group position in Q8 map units.
    pub position_q8: [i32; 3],
    /// Accumulated group angles, retaining all signed stored words.
    pub angles: [i32; 3],
}

/// Decode all 256 SMFM family slots, including vectors in absent slots.
pub fn parse_save_families(bytes: &[u8]) -> Result<Vec<SaveFamily>> {
    if bytes.len() != 256 * 28 {
        return Err(Error::Parse(format!(
            "save game SMFM: expected 7168 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes
        .as_chunks::<28>()
        .0
        .iter()
        .map(|record| {
            let word = |i| i32::from_le_bytes(record[i..i + 4].try_into().unwrap());
            SaveFamily {
                actor_id: word(0),
                position_q8: std::array::from_fn(|i| word(4 + i * 4)),
                angles: std::array::from_fn(|i| word(16 + i * 4)),
            }
        })
        .collect())
}

/// One SMRP owner. The retained body is not a native-memory image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRope {
    pub rope_id: u32,
    /// Saved list references and rope state. Native array, attachment-list and
    /// end-model pointers require reconstruction and must never be dereferenced.
    pub body: [u8; 138],
}

impl SaveRope {
    /// Three rope list references resolved by rope identity by the loader.
    pub fn list_references(&self) -> [u32; 3] {
        std::array::from_fn(|i| u32::from_le_bytes(self.body[i * 4..i * 4 + 4].try_into().unwrap()))
    }
}

/// Parse SMRP's fixed-size records; record count is owned by the world globals.
pub fn parse_save_ropes(bytes: &[u8]) -> Result<Vec<SaveRope>> {
    if !bytes.len().is_multiple_of(142) {
        return Err(Error::Parse(format!(
            "save game SMRP: length {} is not a multiple of 142",
            bytes.len()
        )));
    }
    Ok(bytes
        .as_chunks::<142>()
        .0
        .iter()
        .map(|record| SaveRope {
            rope_id: u32::from_le_bytes(record[..4].try_into().unwrap()),
            body: record[4..].try_into().unwrap(),
        })
        .collect())
}

/// Complete retained 41-byte actor light record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorLight {
    pub locked: bool,
    pub enabled: bool,
    pub inactive_ticks: u8,
    pub offset: [i16; 3],
    pub starting_intensity: i16,
    pub intensity: i16,
    pub starting_radius: i16,
    pub radius: i16,
    pub flicker: [u16; 3],
    pub size: [u16; 3],
    pub rgb: [i16; 3],
    pub starting_rgb: [i16; 3],
}

/// Retained flat playback. The saved cache slot is replaced by a texture lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorFlat {
    pub enabled: bool,
    pub pending: u8,
    pub offset: [u16; 3],
    pub texture: u16,
    pub last_frame: u16,
    pub counter: u16,
    pub frame: u16,
    pub animate: [u8; 2],
}

impl super::SaveObject {
    /// Decode retained light controls without allocation defaults or script calls.
    pub fn light(&self) -> Option<SaveActorLight> {
        self.opaque_41.as_ref().map(|bytes| {
            let word = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
            SaveActorLight {
                locked: bytes[0] != 0,
                enabled: bytes[1] != 0,
                inactive_ticks: bytes[2],
                offset: std::array::from_fn(|i| word(3 + 2 * i) as i16),
                starting_intensity: word(9) as i16,
                intensity: word(11) as i16,
                starting_radius: word(13) as i16,
                radius: word(15) as i16,
                flicker: std::array::from_fn(|i| word(17 + 2 * i)),
                size: std::array::from_fn(|i| word(23 + 2 * i)),
                rgb: std::array::from_fn(|i| word(29 + 2 * i) as i16),
                starting_rgb: std::array::from_fn(|i| word(35 + 2 * i) as i16),
            }
        })
    }

    /// Decode flat playback, replacing the native bitmap cache slot separately.
    pub fn flat(&self) -> Option<SaveActorFlat> {
        self.opaque_20.as_ref().map(|bytes| {
            let word = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
            SaveActorFlat {
                enabled: bytes[0] != 0,
                pending: bytes[1],
                offset: std::array::from_fn(|i| word(2 + i * 2)),
                texture: word(8),
                last_frame: word(12),
                counter: word(14),
                frame: word(16),
                animate: [bytes[18], bytes[19]],
            }
        })
    }
}

/// Retained rope oscillator and amplitude-service state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveRopeSwing {
    pub amplitude: i32,
    pub phase: i32,
    pub step: i32,
    pub push: i32,
    pub direction: u8,
    pub previous_sum: i32,
}

impl SaveRope {
    /// Saved oscillator values, copied independently of initialized link arrays.
    pub fn swing(&self) -> SaveRopeSwing {
        let word = |offset| i32::from_le_bytes(self.body[offset..offset + 4].try_into().unwrap());
        SaveRopeSwing {
            amplitude: word(0x56),
            phase: word(0x6a),
            step: word(0x66),
            push: word(0x62),
            direction: self.body[0x38],
            previous_sum: word(0x5e),
        }
    }
}

impl SaveRope {
    /// Rope identity retained in its runtime body, independently of the record prefix.
    pub fn runtime_identifier(&self) -> u8 {
        self.body[12]
    }
    /// Saved rope type flags, including turn and swing behavior flags.
    pub fn kind(&self) -> u8 {
        self.body[13]
    }
    /// Base position in signed 24.8 map units.
    pub fn position_q8(&self) -> [i32; 3] {
        std::array::from_fn(|i| {
            i32::from_le_bytes(self.body[18 + i * 4..22 + i * 4].try_into().unwrap())
        })
    }
    /// Heading in engine angle units (2048 per turn).
    pub fn heading(&self) -> i32 {
        i32::from_le_bytes(self.body[30..34].try_into().unwrap())
    }
    /// Maximum amplitude retained by the oscillator.
    pub fn maximum(&self) -> i32 {
        i32::from_le_bytes(self.body[34..38].try_into().unwrap())
    }
    /// Number of links whose allocation must already exist.
    pub fn link_count(&self) -> i32 {
        i32::from_le_bytes(self.body[62..66].try_into().unwrap())
    }
    /// Rope-system active word; zero disables updates.
    pub fn active(&self) -> i32 {
        i32::from_le_bytes(self.body[78..82].try_into().unwrap())
    }
    /// Current phase advance rate.
    pub fn speed(&self) -> i32 {
        i32::from_le_bytes(self.body[90..94].try_into().unwrap())
    }
}
