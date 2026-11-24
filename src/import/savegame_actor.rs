//! Semantic retained actor values and saved-identity binding, without execution.
use super::SaveObject;
use crate::{Result, error::Error};
use std::collections::BTreeMap;

/// Local and family pose histories. Positions use signed Q8 map units; angles
/// retain complete signed words, with 2048 units per turn. Decoding does not wrap
/// script-written angles or refresh the family/model matrices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorSpatial {
    pub position_q8: [i32; 3],
    pub attached_position_q8: [i32; 3],
    pub previous_attached_position_q8: [i32; 3],
    pub initial_position_q8: [i32; 3],
    pub previous_position_q8: [i32; 3],
    /// Separate end-of-update position retained for nonplayer water rollback.
    pub water_rollback_position_q8: [i32; 3],
    /// Preserve the byte as well as its nonzero validity interpretation.
    pub stable_validity: u8,
    pub stored_stable_position_q8: [i32; 3],
    pub stable_position_q8: Option<[i32; 3]>,
    pub angles: [i32; 3],
    pub attached_angles: [i32; 3],
    pub previous_attached_angles: [i32; 3],
    pub initial_angles: [i32; 3],
    pub previous_angles: [i32; 3],
}

/// Receiver-owned controller/property state copied by full object load. Bytes
/// retain their stored values; decoding neither enters a motion nor requests an
/// animation. Native consumers validate their supported motion/AI variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorController {
    pub script_control: u8,
    pub heading_change: i16,
    pub rope_wait: u8,
    pub sliding_state: u8,
    pub falling_state: u8,
    pub queued_motion: u8,
    pub living_state: u8,
    pub extra_time: u8,
    pub pending_action: u32,
    pub accepted_action: u32,
    pub action_time: u8,
    pub arms_time: u8,
    pub jump_time: u8,
    pub jump_height: u8,
    pub fall_origin_q8: i32,
    pub fall_timer: u8,
    pub accumulated_displacement_q8: i32,
    pub last_direction: u8,
    pub movement_counter: u8,
    pub collision_response: u32,
    pub collision_age: u8,
    pub overlap_identifier: u16,
    pub surface: u8,
    pub surface_time: u8,
    pub ai_type: u8,
    pub ai_mode: u8,
    pub ai_delay: u8,
    pub ai_variables: [i16; 2],
    pub ai_length: i32,
    pub weapon_drawn: u8,
    pub torch_timer: u8,
    /// Burning counter retained by dousing and consumed by the next lighting transition.
    pub extinguished_torch_timer: u8,
    pub held_item: u8,
    pub motion_q8: [i32; 3],
    pub ledge_position_q8: [i32; 3],
    pub hand_position_q8: [i32; 3],
    pub hand_angles: [i32; 3],
    pub grip: [i16; 2],
    pub scabbard: [i16; 2],
    pub active: u8,
    pub hidden: u8,
    pub dead: u8,
    pub invisible: u8,
    pub message_index: u8,
    pub broadcast_index: u8,
    pub anchor: u8,
}

/// Saved object references are identities, never host pointers. The null saved
/// identity is minus one, independently of the unsigned runtime identity word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorReferences {
    pub parent: Option<i32>,
    pub linked: Option<i32>,
    pub support: Option<i32>,
    pub position_link_owner: Option<i32>,
}

/// Stored embedded model-instance transform, separate from logical Q8 position
/// and angles. IEEE single-precision bits retain the exact saved matrix history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorTransform {
    /// Map-unit translation, with no renderer-axis conversion.
    pub translation_bits: [u32; 3],
    /// Row-major single-precision orientation matrix.
    pub matrix_bits: [[u32; 3]; 3],
}

impl SaveActorTransform {
    pub fn translation(self) -> [f32; 3] {
        self.translation_bits.map(f32::from_bits)
    }
    pub fn matrix(self) -> [[f32; 3]; 3] {
        self.matrix_bits.map(|row| row.map(f32::from_bits))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorState {
    pub model_transform: SaveActorTransform,
    pub spatial: SaveActorSpatial,
    pub controller: SaveActorController,
    pub references: SaveActorReferences,
}

impl SaveObject {
    /// Decode retained actor state without substituting initialized values.
    /// Missing conditional link storage is rejected before owner binding.
    pub fn actor_state(&self) -> Result<SaveActorState> {
        let word = |offset| u32::from_le_bytes(self.body[offset..offset + 4].try_into().unwrap());
        let short = |offset| i16::from_le_bytes(self.body[offset..offset + 2].try_into().unwrap());
        let triple = |offset| std::array::from_fn(|axis| word(offset + axis * 4) as i32);
        let reference = |value| (value != -1).then_some(value);
        let stable = triple(0x108);
        let link = match (word(0x21), &self.opaque_60) {
            (0, None) => None,
            (1, Some(record)) => reference(i32::from_le_bytes(record[..4].try_into().unwrap())),
            _ => {
                return Err(Error::Parse(format!(
                    "save game SMOB actor {}: position-link presence differs from retained allocation",
                    self.object_id
                )));
            }
        };
        Ok(SaveActorState {
            model_transform: SaveActorTransform {
                translation_bits: std::array::from_fn(|axis| word(0x39 + axis * 4)),
                matrix_bits: std::array::from_fn(|row| {
                    std::array::from_fn(|column| word(0x51 + (row * 3 + column) * 4))
                }),
            },
            spatial: SaveActorSpatial {
                position_q8: triple(0xcb),
                attached_position_q8: triple(0xd7),
                previous_attached_position_q8: triple(0xe3),
                initial_position_q8: triple(0xef),
                previous_position_q8: triple(0xfb),
                water_rollback_position_q8: triple(0x114),
                stable_validity: self.body[0x107],
                stored_stable_position_q8: stable,
                stable_position_q8: (self.body[0x107] != 0).then_some(stable),
                angles: triple(0x120),
                attached_angles: triple(0x12c),
                previous_attached_angles: triple(0x138),
                initial_angles: triple(0x144),
                previous_angles: triple(0x150),
            },
            controller: SaveActorController {
                script_control: self.body[0x1b],
                heading_change: short(0x1a9),
                rope_wait: self.body[0x1b0],
                sliding_state: self.body[0x1b1],
                falling_state: self.body[0x1b2],
                queued_motion: self.body[0x1b3],
                living_state: self.body[0x1b4],
                extra_time: self.body[0x1b5],
                pending_action: word(0x1b6),
                accepted_action: word(0x1ba),
                action_time: self.body[0x1be],
                arms_time: self.body[0x1bf],
                jump_time: self.body[0x1c0],
                jump_height: self.body[0x1c1],
                fall_origin_q8: word(0x1c2) as i32,
                fall_timer: self.body[0x1c6],
                accumulated_displacement_q8: word(0x1c7) as i32,
                last_direction: self.body[0x1cb],
                movement_counter: self.body[0x1cc],
                collision_response: word(0x1cd),
                collision_age: self.body[0x1d1],
                overlap_identifier: short(0x1d2) as u16,
                surface: self.body[0x1dc],
                surface_time: self.body[0x1dd],
                ai_type: self.body[0x1de],
                ai_mode: self.body[0x1df],
                ai_delay: self.body[0x1e0],
                ai_variables: [short(0x1e1), short(0x1e3)],
                ai_length: word(0x1e5) as i32,
                weapon_drawn: self.body[0x1fd],
                torch_timer: self.body[0x1fe],
                extinguished_torch_timer: self.body[0x1ff],
                held_item: self.body[0x200],
                motion_q8: triple(0x15c),
                ledge_position_q8: triple(0x168),
                hand_position_q8: triple(0x201),
                hand_angles: triple(0x20d),
                grip: [short(0x219), short(0x21b)],
                scabbard: [short(0x21d), short(0x21f)],
                active: self.body[0x237],
                hidden: self.body[0x238],
                dead: self.body[0x239],
                invisible: self.body[0x23a],
                message_index: self.body[0x259],
                broadcast_index: self.body[0x25a],
                anchor: self.body[0x25b],
            },
            references: SaveActorReferences {
                parent: reference(word(8) as i32),
                linked: reference(word(12) as i32),
                support: self.support_object_id(),
                position_link_owner: link,
            },
        })
    }
}

/// One independently initialized actor allocation. Special player identities are
/// distinct from ordinary runtime identifiers, including runtime identifier zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorBinding {
    pub object_id: i32,
    pub runtime_identifier: u16,
}

/// Complete saved-to-initialized owner population. Indices are into the supplied
/// initialized bindings; no live state is touched during construction.
#[derive(Debug)]
pub struct SaveActorOwners {
    receivers: BTreeMap<i32, usize>,
}

impl SaveActorOwners {
    /// Validate the complete census, identity words and every decoded reference.
    /// Missing, duplicate or foreign ownership rejects the complete population.
    pub fn bind(objects: &[SaveObject], initialized: &[SaveActorBinding]) -> Result<Self> {
        let invalid =
            |message: String| Error::Parse(format!("save game SMOB ownership: {message}"));
        if objects.len() != initialized.len() {
            return Err(invalid("initialized actor census differs".into()));
        }
        let mut receivers = BTreeMap::new();
        for (index, owner) in initialized.iter().enumerate() {
            if owner.object_id == -1 || receivers.insert(owner.object_id, index).is_some() {
                return Err(invalid(format!(
                    "initialized actor identity {} is null or duplicated",
                    owner.object_id
                )));
            }
        }
        let result = Self { receivers };
        let mut saved = std::collections::BTreeSet::new();
        for object in objects {
            let index = result.receiver(object.object_id)?;
            if !saved.insert(object.object_id)
                || initialized[index].runtime_identifier != object.runtime_identifier()
            {
                return Err(invalid(format!(
                    "actor {} saved identity duplicated or runtime binding differs",
                    object.object_id
                )));
            }
            let references = object.actor_state()?.references;
            for (field, identity) in [
                ("parent", references.parent),
                ("linked", references.linked),
                ("support", references.support),
                ("position link", references.position_link_owner),
            ] {
                if let Some(identity) = identity {
                    result.receiver(identity).map_err(|_| {
                        invalid(format!(
                            "actor {} {field}: owner {identity} absent",
                            object.object_id
                        ))
                    })?;
                }
            }
        }
        Ok(result)
    }

    /// Resolve one saved identity into the caller's native allocation order.
    pub fn receiver(&self, object_id: i32) -> Result<usize> {
        self.receivers.get(&object_id).copied().ok_or_else(|| {
            Error::Parse(format!(
                "save game SMOB ownership: actor {object_id} is not initialized"
            ))
        })
    }
}
