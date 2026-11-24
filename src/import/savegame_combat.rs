//! Retained actor combat values, distinct from the object's SOUP task state.
use super::SaveObject;

/// Actor-owned combat state from the optional 422-byte SMOB combat allocation.
///
/// Installed attack/defence bindings and model allocations belong to the
/// initialized world. Saved pointer words never become native pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveActorCombat {
    /// Next combat-list owner identity, rebound separately from actor references.
    pub next_combat_id: u32,
    pub owner_id: i32,
    pub target_id: i32,
    pub allocation_index: i32,
    pub retained_target_position_q8: [i32; 3],
    pub action: i32,
    /// Near, far and averaged midpoint spacing ranges, in runtime Q8 units.
    pub spacing_ranges_q8: [i32; 3],
    pub retained_action_indices: [i32; 2],
    pub strength: i32,
    pub armor: i32,
    pub contact_vertex: i16,
    pub contact_radius_q8: i32,
    pub aggression: i32,
    pub death_default_group: i32,
    pub close_distance_passes: i32,
    pub attacker_selection_state: i32,
    pub executor_state: i32,
    pub move_selector: i32,
    pub cooldown: u32,
    pub attack_pacing: i32,
    pub defence_pacing: i32,
    pub defence_counter: i32,
    pub taunt_active: i32,
    /// Dialogue slot identity retained independently of the discarded RTX pointer.
    pub taunt_dialogue: i32,
    pub taunt_period: i32,
    pub spacing_period: i32,
    /// Saved flag bits. The loader clears bit 0x40 when applying this record.
    pub flags: u8,
    pub avoid_side: i32,
    pub avoid_mask: u32,
    pub avoid_hold: i32,
    pub lost_sight_ticks: i32,
    pub lost_sight_routing: i32,
}

impl SaveActorCombat {
    /// Flags copied by the original loader after clearing transient presentation.
    pub fn restored_flags(self) -> u8 {
        self.flags & !0x40
    }
}

impl SaveObject {
    /// Decode retained combat values without running the combat constructor.
    /// The `script_state` framing field holds this combat allocation; authored
    /// SOUP state and task continuations remain in the object body and SVRG.
    pub fn combat(&self) -> Option<SaveActorCombat> {
        self.script_state.as_ref().map(|bytes| {
            let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let signed = |offset| word(offset) as i32;
            SaveActorCombat {
                next_combat_id: word(0),
                owner_id: signed(4),
                target_id: signed(8),
                allocation_index: signed(12),
                retained_target_position_q8: std::array::from_fn(|i| signed(0x28 + i * 4)),
                action: signed(0x38),
                spacing_ranges_q8: std::array::from_fn(|i| signed(0x3c + i * 4)),
                retained_action_indices: [signed(0x48), signed(0x4c)],
                strength: signed(0x58),
                armor: signed(0x5c),
                contact_vertex: i16::from_le_bytes(bytes[0x60..0x62].try_into().unwrap()),
                contact_radius_q8: signed(0x62),
                aggression: signed(0x139),
                death_default_group: signed(0x13d),
                close_distance_passes: signed(0x141),
                attacker_selection_state: signed(0x145),
                executor_state: signed(0x14d),
                move_selector: signed(0x151),
                cooldown: word(0x159),
                attack_pacing: signed(0x161),
                defence_pacing: signed(0x165),
                defence_counter: signed(0x169),
                taunt_active: signed(0x179),
                taunt_dialogue: signed(0x17d),
                taunt_period: signed(0x185),
                spacing_period: signed(0x189),
                flags: bytes[0x18d],
                avoid_side: signed(0x18e),
                avoid_mask: word(0x192),
                avoid_hold: signed(0x196),
                lost_sight_ticks: signed(0x19a),
                lost_sight_routing: signed(0x19e),
            }
        })
    }
}
