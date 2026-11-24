//! Retained session globals in version 1.31 save payloads.
use crate::{Result, error::Error};

fn check(bytes: &[u8], size: usize, chunk: &str) -> Result<()> {
    if bytes.len() != size {
        return Err(Error::Parse(format!(
            "save game {chunk}: expected {size} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(())
}
fn word(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

/// The two retained HUD bitmap slots; `-1` frame indices denote empty slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavePowerUps {
    /// Per-slot retained frame values, in slot order.
    pub frames: [i32; 2],
    /// Frame indices used to rebuild each slot after a world load.
    pub frame_indices: [i32; 2],
    /// Alternation word selecting the next slot when both are occupied.
    pub alternation: i32,
}

/// Additional full-session state. World-only saves do not contain this payload.
/// Retained words are decoded without normalization, including selection switches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveSessionGlobals {
    /// Published player health, copied into both player records by full load.
    pub player_health: i32,
    pub compass_display: i32,
    /// Selected camera record flags retained by full load.
    pub camera_flags: u16,
    pub retained_block: [u8; 12],
    /// Journal tags in table order; zero denotes an empty entry.
    pub journal_tags: [u32; 400],
    /// Zero selects CYRUS, nonzero selects GREMLIN.
    pub selected_player_switch: i32,
    pub power_ups: SavePowerUps,
    /// Four retained words assigned when loading a world.
    pub world_state: [i32; 4],
    /// Health candle display duration/state word.
    pub health_candle_display: i32,
}
impl SaveSessionGlobals {
    /// Decode the complete fixed-size SVG2 payload.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        check(bytes, 1666, "SVG2")?;
        Ok(Self {
            player_health: word(bytes, 0),
            compass_display: word(bytes, 4),
            camera_flags: u16::from_le_bytes(bytes[8..10].try_into().unwrap()),
            retained_block: bytes[10..22].try_into().unwrap(),
            journal_tags: std::array::from_fn(|i| word(bytes, 22 + i * 4) as u32),
            selected_player_switch: word(bytes, 1622),
            power_ups: SavePowerUps {
                frames: [word(bytes, 1626), word(bytes, 1630)],
                frame_indices: [word(bytes, 1634), word(bytes, 1638)],
                alternation: word(bytes, 1642),
            },
            world_state: std::array::from_fn(|i| word(bytes, 1646 + i * 4)),
            health_candle_display: word(bytes, 1662),
        })
    }
    /// Re-encode every retained field without normalization.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1666);
        bytes.extend_from_slice(&self.player_health.to_le_bytes());
        bytes.extend_from_slice(&self.compass_display.to_le_bytes());
        bytes.extend_from_slice(&self.camera_flags.to_le_bytes());
        bytes.extend_from_slice(&self.retained_block);
        for tag in self.journal_tags {
            bytes.extend_from_slice(&tag.to_le_bytes());
        }
        bytes.extend_from_slice(&self.selected_player_switch.to_le_bytes());
        for value in self
            .power_ups
            .frames
            .into_iter()
            .chain(self.power_ups.frame_indices)
            .chain([self.power_ups.alternation])
            .chain(self.world_state)
            .chain([self.health_candle_display])
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

/// Serialized selected-player controller state, before world-loop entry resets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavePlayerControllerGlobals {
    pub hit_inhibited: i32,
    pub death_animation_override: i32,
    /// Whole nominal ticks accumulated by death execution.
    pub death_progress: u32,
    pub death_mode: i32,
    /// Captured local position in signed 24.8 map units.
    pub death_position_q8: [i32; 3],
    /// Unconditional preliminary countdown, in whole nominal ticks.
    pub action_ticks: u32,
    /// World-loop falling timeout, in whole nominal ticks.
    pub falling_elapsed: u32,
    /// Ordinary idle timer, in BIOS-clock measurement windows.
    pub idle_counter: i32,
}

/// Known gameplay fields from SVGV. Unlisted fields belong to other owners or
/// have no established runtime interpretation here. A save does not contain
/// every engine global; decoding this structure cannot establish a new session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveGameplayGlobals {
    pub controller: SavePlayerControllerGlobals,
    /// Successful activation consumes the action input until release.
    pub activation_consumed_use: i32,
    /// Retained body-response contact count used by the sliding-state test.
    pub body_response_contacts: u32,
    pub lineup_result: i32,
    pub rope_jump_latch: i32,
    pub world_loop_frame: u32,
    pub lockout: u16,
    pub health_candle_level: i32,
    pub health_candle_frame: i32,
    pub health_candle_position: [i32; 2],
}
impl SaveGameplayGlobals {
    /// Decode known gameplay fields from the exact version 1.31 SVGV payload.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        check(bytes, 1542, "SVGV")?;
        Ok(Self {
            controller: SavePlayerControllerGlobals {
                hit_inhibited: word(bytes, 0x30),
                death_animation_override: word(bytes, 0x34),
                death_progress: word(bytes, 0x38) as u32,
                death_mode: word(bytes, 0x40),
                death_position_q8: std::array::from_fn(|i| word(bytes, 0x48 + i * 4)),
                action_ticks: word(bytes, 0x60) as u32,
                falling_elapsed: word(bytes, 0x5c) as u32,
                idle_counter: word(bytes, 0x10c),
            },
            activation_consumed_use: word(bytes, 0x10),
            body_response_contacts: word(bytes, 0x130) as u32,
            lineup_result: word(bytes, 0x64),
            rope_jump_latch: word(bytes, 0x6c),
            world_loop_frame: word(bytes, 0x90) as u32,
            lockout: u16::from_le_bytes(bytes[0x224..0x226].try_into().unwrap()),
            health_candle_level: word(bytes, 0x5ea),
            health_candle_frame: word(bytes, 0x5ee),
            health_candle_position: [word(bytes, 0x5f2), word(bytes, 0x5f6)],
        })
    }

    /// Apply the selected-player and iteration resets performed on world-loop
    /// entry after a full load. Other saved controller fields remain restored.
    pub fn enter_world_loop(&mut self) {
        self.controller.hit_inhibited = 0;
        self.controller.death_animation_override = 0;
        self.controller.death_progress = 0;
        self.world_loop_frame = 0;
    }
}

/// Controller ownership that a full save load retains from the running session.
/// These fields have no SVGV or SVG2 record. Callers must supply established
/// prior-session values; there is deliberately no default constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadRetainedPlayerGlobals {
    pub maximum_health: i32,
    /// Script menu mode; the existing menu entries are rebuilt after full load.
    pub menu_state: u16,
    /// Conditional countdown, in whole nominal ticks.
    pub conditional_ticks: u32,
    /// The second unconditional countdown, in whole nominal ticks.
    pub secondary_ticks: u32,
    pub sound_maintenance_inhibited: bool,
    /// SetPotion/GetPotion counters; the full loader has no saved potion table.
    pub potions: [i32; 4],
    /// Movie pending for the next world load; -1 means no pending movie.
    pub queued_movie: i32,
    /// Retained return value of the selected-player ledge-state helper.
    pub selected_ledge_result: i32,
}
