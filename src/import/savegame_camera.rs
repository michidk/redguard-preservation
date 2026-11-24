//! Saved logical camera records and their load-time actor binding.
use super::SaveActorOwners;
use crate::{Result, error::Error};

/// Logical values in one camera record. Positions retain native Q8 values;
/// angles retain signed words in 2048 units per turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveCameraValues {
    pub position_offset: [i32; 3],
    pub angle_offset: [i32; 3],
    pub request: [i32; 3],
    pub basis: [[i32; 3]; 2],
    pub bound_position_history: [i32; 3],
    pub target_offset: [i32; 3],
    /// Stored distance, before the candidate stage shifts it left eight.
    pub distance: i32,
    pub minimum_distance: i32,
    /// Complete stored word; camera stages consume flag bits in its low byte.
    pub flags: u32,
    pub maximum_movement: u32,
}

/// One saved record. Pointer-shaped words remain private and are never bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveCameraRecord {
    bytes: [u8; 112],
}

/// SVCM stores nine records and the selected record's slot. It does not store
/// the separate selected-index word, published pose, render pose or glide marks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveCameras {
    pub records: [SaveCameraRecord; 9],
    pub selected_record: usize,
}

/// A record whose owner is an independently initialized actor. Loading always
/// binds that actor's local position and local angles, irrespective of the saved
/// pointer words or the actor attribute used by ordinary camera binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundSaveCamera {
    pub actor_receiver: usize,
    pub values: SaveCameraValues,
}

impl SaveCameras {
    /// Prepare the whole population before changing live camera state. Each
    /// initialized angle offset must belong to the corresponding camera slot.
    pub fn bind(
        &self,
        owners: &SaveActorOwners,
        initialized_angle_offsets: [[i32; 3]; 9],
    ) -> Result<[Option<BoundSaveCamera>; 9]> {
        if self.selected_record >= self.records.len() {
            return Err(Error::Parse(
                "save game SVCM: selected record outside camera population".into(),
            ));
        }
        let mut result = std::array::from_fn(|_| None);
        for (index, record) in self.records.iter().enumerate() {
            result[index] = record.bind(owners, initialized_angle_offsets[index])?;
        }
        if result[self.selected_record].is_none() {
            return Err(Error::Parse(format!(
                "save game SVCM: selected record {} has no owner",
                self.selected_record
            )));
        }
        Ok(result)
    }
}

impl SaveCameraRecord {
    /// The saved actor identity; zero and minus one represent no owner.
    pub fn owner(&self) -> Option<i32> {
        let identity = self.word(0x50) as i32;
        (!matches!(identity, -1 | 0)).then_some(identity)
    }

    pub fn values(&self) -> SaveCameraValues {
        let vector = |offset| std::array::from_fn(|axis| self.word(offset + axis * 4) as i32);
        SaveCameraValues {
            position_offset: vector(0),
            angle_offset: vector(12),
            request: vector(24),
            basis: [vector(36), vector(48)],
            bound_position_history: vector(64),
            target_offset: vector(84),
            distance: self.word(96) as i32,
            minimum_distance: self.word(100) as i32,
            flags: self.word(104),
            maximum_movement: self.word(108),
        }
    }

    /// Reconstruct a bound record without replaying a camera native. The load
    /// retains this slot's initialized angle offset and copies every other
    /// logical value from the save. Null records remain unbound; callers must
    /// reject an unbound selected record before publishing any restored state.
    pub fn bind(
        &self,
        owners: &SaveActorOwners,
        initialized_angle_offset: [i32; 3],
    ) -> Result<Option<BoundSaveCamera>> {
        let Some(owner) = self.owner() else {
            return Ok(None);
        };
        let actor_receiver = owners
            .receiver(owner)
            .map_err(|error| Error::Parse(format!("save game SVCM owner {owner}: {error}")))?;
        let mut values = self.values();
        values.angle_offset = initialized_angle_offset;
        Ok(Some(BoundSaveCamera {
            actor_receiver,
            values,
        }))
    }

    fn word(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.bytes[offset..offset + 4].try_into().unwrap())
    }
}

/// Decode the complete fixed SVCM payload, rejecting unsupported counts and
/// out-of-range selection before exposing any records.
pub fn parse_save_cameras(data: &[u8]) -> Result<SaveCameras> {
    let invalid = |message| Error::Parse(format!("save game SVCM: {message}"));
    if data.len() != 1016 {
        return Err(invalid(
            "expected nine records and selected slot (1016 bytes)",
        ));
    }
    if u32::from_le_bytes(data[..4].try_into().unwrap()) != 9 {
        return Err(invalid("expected nine camera records"));
    }
    let selected_record = u32::from_le_bytes(data[1012..].try_into().unwrap()) as usize;
    if selected_record >= 9 {
        return Err(invalid("selected record is outside the camera population"));
    }
    Ok(SaveCameras {
        records: std::array::from_fn(|index| SaveCameraRecord {
            bytes: data[4 + index * 112..4 + (index + 1) * 112]
                .try_into()
                .unwrap(),
        }),
        selected_record,
    })
}

/// Losslessly encode the stored records, including private pointer words. These
/// words are archival bytes and must never be dereferenced by a native loader.
pub fn encode_save_cameras(cameras: &SaveCameras) -> Result<Vec<u8>> {
    if cameras.selected_record >= 9 {
        return Err(Error::Parse(
            "save game SVCM: selected record outside camera population".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(1016);
    bytes.extend_from_slice(&9u32.to_le_bytes());
    for record in &cameras.records {
        bytes.extend_from_slice(&record.bytes);
    }
    bytes.extend_from_slice(&(cameras.selected_record as u32).to_le_bytes());
    Ok(bytes)
}

/// Camera globals retained separately from SVCM. Loading selects `index` again,
/// applies `selected_flags`, clears transition/reselection state and requests a
/// render cut. Render positions are stored as rounded whole map units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveCameraGlobals {
    pub previous_position: [i32; 3],
    pub current_position: [i32; 3],
    pub current_angles: [i32; 3],
    pub previous_angles: [i32; 3],
    pub transformed_basis: [[i32; 3]; 2],
    pub index: u8,
    pub scripted_active: u16,
    pub scripted_nesting: u32,
    pub render_position: [i32; 3],
    pub selected_flags: u16,
    pub obstacle_size_q8: i32,
    pub maximum_acceleration: i32,
    /// Native stored negative vertical-velocity limit.
    pub maximum_vertical_velocity: i32,
    pub static_rope_threshold: i32,
    pub static_rope_aim: [[i32; 3]; 2],
}

/// Decode camera globals from the complete version 1.31 global payloads.
/// The load path reads only the low byte of the saved index word.
pub fn parse_save_camera_globals(svgv: &[u8], svg2: &[u8]) -> Result<SaveCameraGlobals> {
    if svgv.len() != 1542 || svg2.len() != 1666 {
        return Err(Error::Parse(
            "save game camera globals: invalid SVGV or SVG2 length".into(),
        ));
    }
    let word = |offset| u32::from_le_bytes(svgv[offset..offset + 4].try_into().unwrap());
    let vector = |offset| std::array::from_fn(|axis| word(offset + axis * 4) as i32);
    let index = svgv[0x24e];
    if index >= 9 {
        return Err(Error::Parse(format!(
            "save game camera globals: selected index {index} outside nine records"
        )));
    }
    Ok(SaveCameraGlobals {
        previous_position: vector(0x17c),
        current_position: vector(0x18c),
        current_angles: vector(0x198),
        previous_angles: vector(0x1a4),
        transformed_basis: [vector(0x1b0), vector(0x1bc)],
        index,
        scripted_active: u16::from_le_bytes(svgv[0x22e..0x230].try_into().unwrap()),
        scripted_nesting: word(0x230),
        render_position: vector(0x5fa),
        selected_flags: u16::from_le_bytes(svg2[8..10].try_into().unwrap()),
        obstacle_size_q8: word(0x1c8) as i32,
        maximum_acceleration: word(0x1cc) as i32,
        maximum_vertical_velocity: word(0x24a) as i32,
        static_rope_threshold: word(0x276) as i32,
        static_rope_aim: [vector(0x25e), vector(0x26a)],
    })
}
