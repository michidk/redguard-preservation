use super::{
    RaexRecord, RagrAnimGroup, RagrCommand, RgmFile, RgmSection,
    shared::{read_i32_le, read_script_name_9},
};
use std::collections::HashMap;

const OPCODE_NAMES: [&str; 16] = [
    "ShowFrame",
    "EndAnimation",
    "GoToPrevious",
    "GoToFuture",
    "PlaySound",
    "BreakPoint",
    "SetRotationXYZ",
    "SetRotationAxis",
    "SetPositionXYZ",
    "SetPositionAxis",
    "ChangeAnimGroup",
    "Rumble",
    "DelayCounter",
    "ConditionalDelay",
    "LoopControl",
    "Transition",
];

const ANIM_TYPE_NAMES: [&str; 3] = ["interruptible", "must_complete", "no_panic_revert"];
const RAHD_ITEM_SIZE: usize = 165;

impl RagrCommand {
    #[allow(clippy::cast_possible_truncation)] // Sign extension limits the value to the requested bit-width.
    const fn sign_extend(val: i32, bits: u32) -> i16 {
        let mask = (1i32 << bits) - 1;
        let sign_bit = 1i32 << (bits - 1);
        let raw = val & mask;
        if raw & sign_bit != 0 {
            (raw | !mask) as i16
        } else {
            raw as i16
        }
    }

    #[must_use]
    pub const fn param_10a(&self) -> i16 {
        Self::sign_extend((self.raw >> 4).cast_signed(), 10)
    }

    #[must_use]
    pub const fn param_10b(&self) -> i16 {
        Self::sign_extend((self.raw >> 14).cast_signed(), 10)
    }

    #[must_use]
    pub const fn param_6a(&self) -> i16 {
        Self::sign_extend((self.raw >> 4).cast_signed(), 6)
    }

    #[must_use]
    pub const fn param_6b(&self) -> i16 {
        Self::sign_extend((self.raw >> 10).cast_signed(), 6)
    }

    #[must_use]
    pub const fn param_6c(&self) -> i16 {
        Self::sign_extend((self.raw >> 16).cast_signed(), 6)
    }

    #[must_use]
    pub const fn param_2(&self) -> u8 {
        ((self.raw >> 4) & 0x3) as u8
    }

    #[must_use]
    pub const fn param_18(&self) -> i32 {
        let raw = ((self.raw >> 6) & 0x3FFFF).cast_signed();
        let sign_bit = 1i32 << 17;
        if raw & sign_bit != 0 {
            raw | !0x3FFFF
        } else {
            raw
        }
    }

    #[must_use]
    pub const fn param_6(&self) -> i16 {
        Self::sign_extend((self.raw >> 4).cast_signed(), 6)
    }

    #[must_use]
    pub const fn param_7a(&self) -> i16 {
        Self::sign_extend((self.raw >> 10).cast_signed(), 7)
    }

    #[must_use]
    pub const fn param_7b(&self) -> i16 {
        Self::sign_extend((self.raw >> 17).cast_signed(), 7)
    }

    #[must_use]
    pub const fn param_20(&self) -> i32 {
        let raw = ((self.raw >> 4) & 0xFFFFF).cast_signed();
        let sign_bit = 1i32 << 19;
        if raw & sign_bit != 0 {
            raw | !0xFFFFF
        } else {
            raw
        }
    }

    #[must_use]
    pub const fn handle_index(&self) -> i16 {
        self.param_10a()
    }

    #[must_use]
    pub const fn vertex_index(&self) -> i16 {
        self.param_10b()
    }

    #[must_use]
    pub const fn sets_attachment(&self) -> bool {
        self.opcode == 0
    }
}

#[allow(clippy::missing_const_for_fn)] // `u32::from` in const fn requires unstable const-trait support.
pub(super) fn decode_ragr_command(b0: u8, b1: u8, b2: u8) -> RagrCommand {
    let raw = u32::from(b0) | (u32::from(b1) << 8) | (u32::from(b2) << 16);
    RagrCommand {
        raw,
        opcode: (raw & 0xF) as u8,
    }
}

pub(super) fn parse_ragr_actor_groups_impl(
    ragr_data: &[u8],
    ragr_offset: usize,
) -> Vec<RagrAnimGroup> {
    let mut groups = Vec::new();
    let mut cursor = ragr_offset;

    loop {
        if cursor + 2 > ragr_data.len() {
            break;
        }
        let entry_size = u16::from_le_bytes([ragr_data[cursor], ragr_data[cursor + 1]]) as usize;
        if entry_size == 0 {
            break;
        }
        if cursor + 2 + entry_size > ragr_data.len() {
            break;
        }
        if entry_size < 8 {
            break;
        }

        let group_index = u16::from_le_bytes([ragr_data[cursor + 2], ragr_data[cursor + 3]]);
        let anim_id = u16::from_le_bytes([ragr_data[cursor + 4], ragr_data[cursor + 5]]);
        let flag = ragr_data[cursor + 6];
        let frame_count = u16::from_le_bytes([ragr_data[cursor + 8], ragr_data[cursor + 9]]);

        let cmd_bytes = (frame_count as usize) * 3;
        let cmd_start = cursor + 10;
        if cmd_start + cmd_bytes > ragr_data.len() {
            break;
        }

        let mut commands = Vec::with_capacity(frame_count as usize);
        for i in 0..frame_count as usize {
            let off = cmd_start + i * 3;
            commands.push(decode_ragr_command(
                ragr_data[off],
                ragr_data[off + 1],
                ragr_data[off + 2],
            ));
        }

        groups.push(RagrAnimGroup {
            group_index,
            anim_id,
            flag,
            frame_count,
            commands,
        });

        cursor += 2 + entry_size;
    }

    groups
}

pub(super) fn parse_rahd_ragr_index_impl(rahd_data: &[u8]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    if rahd_data.len() < 8 {
        return out;
    }

    let count =
        u32::from_le_bytes([rahd_data[0], rahd_data[1], rahd_data[2], rahd_data[3]]) as usize;
    let mut cursor = 8usize;
    for _ in 0..count {
        if cursor + RAHD_ITEM_SIZE > rahd_data.len() {
            break;
        }
        let item = &rahd_data[cursor..cursor + RAHD_ITEM_SIZE];

        let Some(script_name) = read_script_name_9(item, 4) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };
        let Some(ragr_offset) =
            read_i32_le(item, 0x31).map(|value| usize::try_from(value.max(0)).unwrap_or_default())
        else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };

        if !script_name.is_empty() {
            out.insert(script_name, ragr_offset);
        }

        cursor += RAHD_ITEM_SIZE;
    }

    out
}

fn command_to_json(cmd: RagrCommand) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("opcode".into(), cmd.opcode.into());
    obj.insert("name".into(), OPCODE_NAMES[cmd.opcode as usize].into());

    match cmd.opcode {
        0 => {
            obj.insert("handle_index".into(), cmd.param_10a().into());
            obj.insert("vertex_index".into(), cmd.param_10b().into());
        }
        4 => {
            obj.insert("sound_param".into(), cmd.param_10a().into());
            obj.insert("volume_shift".into(), cmd.param_10b().into());
        }
        10 => {
            obj.insert("target_group".into(), cmd.param_10a().into());
            obj.insert("target_frame".into(), cmd.param_10b().into());
        }
        6 | 8 => {
            obj.insert("x".into(), cmd.param_6a().into());
            obj.insert("y".into(), cmd.param_6b().into());
            obj.insert("z".into(), cmd.param_6c().into());
        }
        7 | 9 => {
            obj.insert("axis".into(), cmd.param_2().into());
            obj.insert("value".into(), cmd.param_18().into());
        }
        15 => {
            obj.insert("trigger_mask".into(), cmd.param_6().into());
            obj.insert("start_frame".into(), cmd.param_7a().into());
            obj.insert("target_group".into(), cmd.param_7b().into());
        }
        _ => {
            obj.insert("value".into(), cmd.param_20().into());
        }
    }

    serde_json::Value::Object(obj)
}

fn group_to_json(group: &RagrAnimGroup) -> serde_json::Value {
    let anim_type_name = ANIM_TYPE_NAMES
        .get(group.flag as usize)
        .unwrap_or(&"unknown");

    serde_json::json!({
        "group_index": group.group_index,
        "anim_id": group.anim_id,
        "anim_type": group.flag,
        "anim_type_name": anim_type_name,
        "frame_count": group.frame_count,
        "commands": group.commands.iter().copied().map(command_to_json).collect::<Vec<_>>()
    })
}

fn raex_to_json(rec: &RaexRecord) -> serde_json::Value {
    serde_json::json!({
        "grip0": rec.grip0,
        "grip1": rec.grip1,
        "scabbard0": rec.scabbard0,
        "scabbard1": rec.scabbard1,
        "unknown_08": rec.unknown_08,
        "texture_id": rec.texture_id,
        "v_vertex": rec.v_vertex,
        "v_size": rec.v_size,
        "taunt_id": rec.taunt_id,
        "unknown_12": rec.unknown_12,
        "unknown_14": rec.unknown_14,
        "unknown_16": rec.unknown_16,
        "range_min": rec.range_min,
        "range_ideal": rec.range_ideal,
        "range_max": rec.range_max,
    })
}

pub(super) fn export_rgm_metadata_json_impl(rgm: &RgmFile) -> serde_json::Value {
    let rahd_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Rahd(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let ragr_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Ragr(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let raex_records = rgm.sections.iter().find_map(|s| match s {
        RgmSection::RaexParsed(_, recs) => Some(recs.as_slice()),
        _ => None,
    });

    let Some(rahd) = rahd_data else {
        return serde_json::json!({ "actors": [] });
    };
    if rahd.len() < 8 {
        return serde_json::json!({ "actors": [] });
    }

    let count = u32::from_le_bytes([rahd[0], rahd[1], rahd[2], rahd[3]]) as usize;
    let mut actors = Vec::new();
    for i in 0..count {
        let rec_off = 8 + i * RAHD_ITEM_SIZE;
        if rec_off + RAHD_ITEM_SIZE > rahd.len() {
            break;
        }
        let item = &rahd[rec_off..rec_off + RAHD_ITEM_SIZE];

        let script_name = read_script_name_9(item, 4).unwrap_or_default();
        if script_name.is_empty() {
            continue;
        }

        let ragr_offset = read_i32_le(item, 0x31).unwrap_or(-1);
        let mut actor = serde_json::Map::new();
        actor.insert("index".into(), i.into());
        actor.insert("script_name".into(), script_name.clone().into());

        if let Some(ragr) = ragr_data
            && let Ok(ragr_offset) = usize::try_from(ragr_offset)
        {
            let groups = parse_ragr_actor_groups_impl(ragr, ragr_offset);
            if !groups.is_empty() {
                actor.insert(
                    "animation_groups".into(),
                    groups.iter().map(group_to_json).collect::<Vec<_>>().into(),
                );
            }
        }

        if let Some(raex) = raex_records
            && i < raex.len()
        {
            actor.insert("raex".into(), raex_to_json(&raex[i]));
        }

        actors.push(serde_json::Value::Object(actor));
    }

    serde_json::json!({ "actors": actors })
}
