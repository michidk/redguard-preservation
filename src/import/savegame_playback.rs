//! Read-only playback views of retained original save records.
use crate::{Result, error::Error};

/// Retained clock words from version 1.31 SVGV, following formats/SAVEGAME.md.
/// Exposed timing and computed timing are separate. Host epochs and saved
/// accumulators must be rebound by the consumer; this does not advance time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveClock {
    pub measurement_baseline: u32,
    pub measurement_remainder: i32,
    pub measured_fps: u32,
    pub next_scale: u32,
    pub nominal_increment: u32,
    pub fine_increment: u32,
    pub warmup: u32,
    pub bios_sample: u32,
    pub measurement_frames: u32,
    pub smoothed_fps: u32,
    pub frame_scale: u32,
    pub exposed_nominal_ticks: u32,
    pub nominal_total: u32,
    pub nominal_scaled_total: u32,
    pub nominal_scaled_increment: u32,
    pub exposed_fine_ticks: u32,
    pub fine_total: u32,
    pub fine_scaled_total: u32,
    pub fine_scaled_increment: u32,
    pub min_frame_rate: u32,
    pub max_frame_rate: u32,
    pub use_smooth_fps: u32,
    pub use_smooth_divisor: u32,
    pub normal_rate_scaled: u32,
}

/// Decode the documented clock words, leaving the other SVGV bytes untouched.
/// The caller retains the original payload for lossless editing.
pub fn parse_save_clock(bytes: &[u8]) -> Result<SaveClock> {
    if bytes.len() != 1542 {
        return Err(Error::Parse(format!(
            "save game SVGV: expected 1542 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(SaveClock {
        measurement_baseline: u32::from_le_bytes(bytes[148..152].try_into().unwrap()),
        measurement_remainder: i32::from_le_bytes(bytes[152..156].try_into().unwrap()),
        measured_fps: u32::from_le_bytes(bytes[156..160].try_into().unwrap()),
        next_scale: u32::from_le_bytes(bytes[160..164].try_into().unwrap()),
        nominal_increment: u32::from_le_bytes(bytes[164..168].try_into().unwrap()),
        fine_increment: u32::from_le_bytes(bytes[168..172].try_into().unwrap()),
        warmup: u32::from_le_bytes(bytes[172..176].try_into().unwrap()),
        bios_sample: u32::from_le_bytes(bytes[180..184].try_into().unwrap()),
        measurement_frames: u32::from_le_bytes(bytes[184..188].try_into().unwrap()),
        smoothed_fps: u32::from_le_bytes(bytes[192..196].try_into().unwrap()),
        frame_scale: u32::from_le_bytes(bytes[196..200].try_into().unwrap()),
        exposed_nominal_ticks: u32::from_le_bytes(bytes[200..204].try_into().unwrap()),
        nominal_total: u32::from_le_bytes(bytes[204..208].try_into().unwrap()),
        nominal_scaled_total: u32::from_le_bytes(bytes[208..212].try_into().unwrap()),
        nominal_scaled_increment: u32::from_le_bytes(bytes[212..216].try_into().unwrap()),
        exposed_fine_ticks: u32::from_le_bytes(bytes[216..220].try_into().unwrap()),
        fine_total: u32::from_le_bytes(bytes[220..224].try_into().unwrap()),
        fine_scaled_total: u32::from_le_bytes(bytes[224..228].try_into().unwrap()),
        fine_scaled_increment: u32::from_le_bytes(bytes[228..232].try_into().unwrap()),
        min_frame_rate: u32::from_le_bytes(bytes[232..236].try_into().unwrap()),
        max_frame_rate: u32::from_le_bytes(bytes[236..240].try_into().unwrap()),
        use_smooth_fps: u32::from_le_bytes(bytes[240..244].try_into().unwrap()),
        use_smooth_divisor: u32::from_le_bytes(bytes[244..248].try_into().unwrap()),
        normal_rate_scaled: u32::from_le_bytes(bytes[248..252].try_into().unwrap()),
    })
}

/// Mutable 37-byte animation playback header from SMOB.
/// Group definitions and model allocations remain installed resources.
/// Byte nine is opaque and retained verbatim. A saved sound handle is not a
/// host audio handle and requires separate binding before playback resumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveAnimationPlayback {
    pub current: i16,
    pub next: i16,
    pub next_speed: u8,
    pub frame: i16,
    pub speed: u8,
    pub speed_count: u8,
    pub opaque_9: u8,
    pub stop_flag: u8,
    pub halt_flag: u8,
    pub hold_flag: u8,
    pub rep_count: u8,
    pub rewind_flag: u8,
    pub loop_count: u8,
    pub sound_handle: u8,
    pub real_frame: u8,
    pub max_group: u8,
    pub position: [i16; 3],
    pub rotation: [i16; 3],
    pub last_rotation: [i16; 3],
}
impl From<&[u8; 37]> for SaveAnimationPlayback {
    fn from(bytes: &[u8; 37]) -> Self {
        let word = |offset| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        Self {
            current: word(0),
            next: word(2),
            next_speed: bytes[4],
            frame: word(5),
            speed: bytes[7],
            speed_count: bytes[8],
            opaque_9: bytes[9],
            stop_flag: bytes[10],
            halt_flag: bytes[11],
            hold_flag: bytes[12],
            rep_count: bytes[13],
            rewind_flag: bytes[14],
            loop_count: bytes[15],
            sound_handle: bytes[16],
            real_frame: bytes[17],
            max_group: bytes[18],
            position: std::array::from_fn(|axis| word(19 + axis * 2)),
            rotation: std::array::from_fn(|axis| word(25 + axis * 2)),
            last_rotation: std::array::from_fn(|axis| word(31 + axis * 2)),
        }
    }
}
