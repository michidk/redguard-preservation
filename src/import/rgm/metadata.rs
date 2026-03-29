use super::{
    MpobRecord, MprpRecord, MpsRecord, MpslRecord, RaexRecord, RagrAnimGroup, RagrCommand,
    RavcRecord, RgmFile, RgmSection, positioning,
    shared::{read_i16_le, read_i32_le, read_script_name_9},
};
use crate::import::soup_def::SoupDef;
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

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn mps_to_json(record: &MpsRecord) -> serde_json::Value {
    serde_json::json!({
        "id": record.id,
        "model_name": record.model_name(),
        "pos_x": record.pos_x,
        "pos_y": record.pos_y,
        "pos_z": record.pos_z,
        "position": positioning::decode_position(record.pos_x, record.pos_y, record.pos_z),
        "rotation_matrix": record.rotation_matrix,
        "trailing": record.trailing,
    })
}

fn mpob_to_json(index: usize, record: &MpobRecord) -> serde_json::Value {
    serde_json::json!({
        "index": index,
        "id": record.id,
        "object_type": record.object_type,
        "is_active": record.is_active,
        "is_static": record.is_static,
        "script_name": record.script_name(),
        "model_name": record.model_name(),
        "texture_id": record.texture_id,
        "image_id": record.image_id,
        "position": positioning::decode_position(record.pos_x, record.pos_y, record.pos_z),
        "angle_x": record.angle_x,
        "angle_y": record.angle_y,
        "angle_z": record.angle_z,
        "intensity": record.intensity,
        "radius": record.radius,
        "model_id": record.model_id,
        "world_id": record.world_id,
        "red": record.red,
        "green": record.green,
        "blue": record.blue,
    })
}

fn mpsl_to_json(record: &MpslRecord) -> serde_json::Value {
    serde_json::json!({
        "color_r": record.color_r,
        "color_g": record.color_g,
        "color_b": record.color_b,
        "color_flags": record.color_flags,
        "unknown0": record.unknown0,
        "position": positioning::decode_position(record.pos_x, record.pos_y, record.pos_z),
        "param0": record.param0,
        "param1": record.param1,
        "unknown1_hex": hex_encode(&record.unknown1),
    })
}

fn mprp_to_json(record: &MprpRecord) -> serde_json::Value {
    serde_json::json!({
        "id": record.id,
        "unknown0": record.unknown0,
        "position": positioning::decode_position(record.pos_x, record.pos_y, record.pos_z),
        "angle_y": record.angle_y,
        "rope_type": record.rope_type,
        "swing": record.swing,
        "speed": record.speed,
        "length": record.length,
        "static_model": record.static_model,
        "rope_model": record.rope_model,
        "unknown1": record.unknown1,
    })
}

fn ravc_to_json(record: &RavcRecord) -> serde_json::Value {
    serde_json::json!({
        "offset_x": record.offset_x,
        "offset_y": record.offset_y,
        "offset_z": record.offset_z,
        "vertex": record.vertex,
        "radius": record.radius,
    })
}

fn raw_section_to_json(name: &str, data: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "size": data.len(),
        "data_hex": hex_encode(data),
    })
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let chunk: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(chunk))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let chunk: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(chunk))
}

fn read_u8(bytes: &[u8], offset: usize) -> Option<u8> {
    bytes.get(offset).copied()
}

fn read_i24_le_signed(bytes: &[u8], offset: usize) -> Option<i32> {
    let b0 = i32::from(*bytes.get(offset)?);
    let b1 = i32::from(*bytes.get(offset + 1)?) << 8;
    let b2 = i32::from(*bytes.get(offset + 2)?) << 16;
    let mut value = b0 | b1 | b2;
    if (value & 0x0080_0000) != 0 {
        value |= !0x00FF_FFFF;
    }
    Some(value)
}

fn read_u24_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let b0 = u32::from(*bytes.get(offset)?);
    let b1 = u32::from(*bytes.get(offset + 1)?) << 8;
    let b2 = u32::from(*bytes.get(offset + 2)?) << 16;
    Some(b0 | b1 | b2)
}

fn parse_raan_entries(data: &[u8], offset: usize, count: usize) -> Vec<serde_json::Value> {
    let mut entries = Vec::new();
    let mut cursor = offset;
    for _ in 0..count {
        if cursor + 6 > data.len() {
            break;
        }

        let Some(frame_count) = read_u8(data, cursor + 4) else {
            break;
        };
        let Some(model_type) = read_u8(data, cursor + 5) else {
            break;
        };

        let path_start = cursor + 6;
        let Some(path_len) = data[path_start..].iter().position(|&b| b == 0) else {
            break;
        };
        let path_end = path_start + path_len;
        let file_path = String::from_utf8_lossy(&data[path_start..path_end]).to_string();

        entries.push(serde_json::json!({
            "frame_count": frame_count,
            "model_type": model_type,
            "file_path": file_path,
        }));

        cursor = path_end + 1;
    }
    entries
}

fn parse_mpf_records(data: &[u8]) -> Vec<serde_json::Value> {
    let Some(count_u32) = read_u32_le(data, 0) else {
        return Vec::new();
    };
    let count = usize::try_from(count_u32).unwrap_or_default();
    let mut out = Vec::with_capacity(count);
    let mut cursor = 4usize;

    for _ in 0..count {
        if cursor + 24 > data.len() {
            break;
        }

        let Some(id) = read_u32_le(data, cursor) else {
            break;
        };
        let Some(pos_x) = read_i24_le_signed(data, cursor + 8) else {
            break;
        };
        let Some(pos_y) = read_i24_le_signed(data, cursor + 12) else {
            break;
        };
        let Some(pos_z) = read_u24_le(data, cursor + 16) else {
            break;
        };
        let Some(texture_data) = read_u16_le(data, cursor + 20) else {
            break;
        };

        out.push(serde_json::json!({
            "id": id,
            "position": positioning::decode_position(pos_x, pos_y, pos_z),
            "texture_id": texture_data >> 7,
            "image_id": texture_data & 0x7F,
        }));

        cursor += 24;
    }

    out
}

fn parse_mpm_records(data: &[u8]) -> Vec<serde_json::Value> {
    let Some(count_u32) = read_u32_le(data, 0) else {
        return Vec::new();
    };
    let count = usize::try_from(count_u32).unwrap_or_default();
    let mut out = Vec::with_capacity(count);
    let mut cursor = 4usize;

    for _ in 0..count {
        if cursor + 13 > data.len() {
            break;
        }

        let Some(pos_x) = read_i24_le_signed(data, cursor) else {
            break;
        };
        let Some(pos_y) = read_i24_le_signed(data, cursor + 4) else {
            break;
        };
        let Some(pos_z_signed) = read_i24_le_signed(data, cursor + 8) else {
            break;
        };
        let pos_z = (pos_z_signed as u32) & 0x00FF_FFFF;
        let reserved = read_u8(data, cursor + 12).unwrap_or_default();

        out.push(serde_json::json!({
            "position": positioning::decode_position(pos_x, pos_y, pos_z),
            "reserved": reserved,
        }));

        cursor += 13;
    }

    out
}

fn parse_wdnm_maps(data: &[u8]) -> Vec<serde_json::Value> {
    let Some(map_count_u32) = read_u32_le(data, 0) else {
        return Vec::new();
    };
    let map_count = usize::try_from(map_count_u32).unwrap_or_default();
    let mut maps = Vec::with_capacity(map_count);
    let mut cursor = 4usize;

    for _ in 0..map_count {
        if cursor + 0x1C > data.len() {
            break;
        }

        let map_start = cursor;
        let Some(map_length_u32) = read_u32_le(data, map_start) else {
            break;
        };
        let map_length = usize::try_from(map_length_u32).unwrap_or_default();
        let Some(node_count_u32) = read_u32_le(data, map_start + 4) else {
            break;
        };
        let node_count = usize::try_from(node_count_u32).unwrap_or_default();
        let node_count_dup = read_u32_le(data, map_start + 8).unwrap_or_default();
        let Some(map_pos_x) = read_i24_le_signed(data, map_start + 0x0C) else {
            break;
        };
        let Some(map_pos_y) = read_i24_le_signed(data, map_start + 0x10) else {
            break;
        };
        let Some(map_pos_z_signed) = read_i24_le_signed(data, map_start + 0x14) else {
            break;
        };
        let map_pos_z = (map_pos_z_signed as u32) & 0x00FF_FFFF;
        let radius = read_u32_le(data, map_start + 0x18).unwrap_or_default();

        cursor = map_start + 0x1C;

        let mut nodes = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            if cursor + 0x0C > data.len() {
                break;
            }

            let node_start = cursor;
            let node_length = read_u32_le(data, node_start).unwrap_or_default();
            let node_pos_x = read_u16_le(data, node_start + 4).unwrap_or_default();
            let node_pos_y = read_i16_le(data, node_start + 6).unwrap_or_default();
            let node_pos_z = read_u16_le(data, node_start + 8).unwrap_or_default();
            let reserved = read_u8(data, node_start + 0x0A).unwrap_or_default();
            let route_count = usize::from(read_u8(data, node_start + 0x0B).unwrap_or_default());

            cursor = node_start + 0x0C;
            let mut routes = Vec::with_capacity(route_count);
            for _ in 0..route_count {
                if cursor + 4 > data.len() {
                    break;
                }
                let target_node_id = read_u16_le(data, cursor).unwrap_or_default();
                let cost = read_u16_le(data, cursor + 2).unwrap_or_default();
                routes.push(serde_json::json!({
                    "target_node_id": target_node_id,
                    "cost": cost,
                }));
                cursor += 4;
            }

            if let Ok(node_len) = usize::try_from(node_length)
                && node_len >= 0x0C
                && node_start + node_len <= data.len()
                && node_start + node_len > cursor
            {
                cursor = node_start + node_len;
            }

            nodes.push(serde_json::json!({
                "node_length": node_length,
                "node_pos_x": node_pos_x,
                "node_pos_y": node_pos_y,
                "node_pos_z": node_pos_z,
                "reserved": reserved,
                "route_count": route_count,
                "routes": routes,
            }));
        }

        if map_length >= 0x1C
            && map_start + map_length <= data.len()
            && map_start + map_length > cursor
        {
            cursor = map_start + map_length;
        }

        maps.push(serde_json::json!({
            "map_length": map_length_u32,
            "node_count": node_count_u32,
            "node_count_dup": node_count_dup,
            "position": positioning::decode_position(map_pos_x, map_pos_y, map_pos_z),
            "radius": radius,
            "walk_nodes": nodes,
        }));
    }

    maps
}

fn parse_rafs_entries(data: &[u8]) -> Vec<serde_json::Value> {
    if data.len() <= 10 {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor + 11 <= data.len() {
        out.push(serde_json::json!({
            "data_hex": hex_encode(&data[cursor..cursor + 11]),
        }));
        cursor += 11;
    }
    out
}

fn parse_mpsz_entries(data: &[u8]) -> Vec<serde_json::Value> {
    const RECORD_SIZE: usize = 49;
    if data.len() < RECORD_SIZE {
        return Vec::new();
    }

    let count = data.len() / RECORD_SIZE;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * RECORD_SIZE;
        let rec = &data[off..off + RECORD_SIZE];
        let total_x = read_i32_le(rec, 0x00).unwrap_or_default();
        let total_y = read_i32_le(rec, 0x04).unwrap_or_default();
        let total_z = read_i32_le(rec, 0x08).unwrap_or_default();
        let center_x = read_i32_le(rec, 0x0C).unwrap_or_default();
        let center_y = read_i32_le(rec, 0x10).unwrap_or_default();
        let center_z = read_i32_le(rec, 0x14).unwrap_or_default();
        let neg_x = read_i32_le(rec, 0x18).unwrap_or_default();
        let neg_y = read_i32_le(rec, 0x1C).unwrap_or_default();
        let neg_z = read_i32_le(rec, 0x20).unwrap_or_default();
        let pos_x = read_i32_le(rec, 0x24).unwrap_or_default();
        let pos_y = read_i32_le(rec, 0x28).unwrap_or_default();
        let pos_z = read_i32_le(rec, 0x2C).unwrap_or_default();
        let flags = rec.get(0x30).copied().unwrap_or_default();
        out.push(serde_json::json!({
            "index": i,
            "total_extent": [total_x, total_y, total_z],
            "center_offset": [center_x, center_y, center_z],
            "neg_extent": [neg_x, neg_y, neg_z],
            "pos_extent": [pos_x, pos_y, pos_z],
            "flags": flags,
        }));
    }
    out
}

pub(super) fn export_rgm_metadata_json_impl(
    rgm: &RgmFile,
    soup_def: Option<&SoupDef>,
) -> serde_json::Value {
    let rahd_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Rahd(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let ragr_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Ragr(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let raat_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Raat(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let raan_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Raan(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let rahk_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Rahk(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let ranm_data = rgm.sections.iter().find_map(|s| match s {
        RgmSection::Ranm(_, data) => Some(data.as_slice()),
        _ => None,
    });
    let raex_records = rgm.sections.iter().find_map(|s| match s {
        RgmSection::RaexParsed(_, recs) => Some(recs.as_slice()),
        _ => None,
    });

    let mut actors = Vec::new();
    if let Some(rahd) = rahd_data
        && rahd.len() >= 8
    {
        let count = u32::from_le_bytes([rahd[0], rahd[1], rahd[2], rahd[3]]) as usize;
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

            if let Some(raat) = raat_data {
                let start = i.saturating_mul(256);
                if let Some(slice) = raat.get(start..start + 256) {
                    let mut attrs = serde_json::Map::new();
                    for (attr_idx, value) in slice.iter().copied().enumerate() {
                        if value == 0 {
                            continue;
                        }
                        let key = soup_def
                            .and_then(|def| def.attributes.get(attr_idx))
                            .filter(|name| !name.is_empty())
                            .cloned()
                            .unwrap_or_else(|| format!("attr_{attr_idx}"));
                        attrs.insert(key, serde_json::Value::from(value));
                    }
                    if !attrs.is_empty() {
                        actor.insert("attributes".into(), serde_json::Value::Object(attrs));
                    }
                }
            }

            if let Some(raan) = raan_data {
                let raan_count = read_i32_le(item, 0x21)
                    .map(|value| usize::try_from(value.max(0)).unwrap_or_default())
                    .unwrap_or_default();
                let raan_offset = read_i32_le(item, 0x29)
                    .map(|value| usize::try_from(value.max(0)).unwrap_or_default())
                    .unwrap_or_default();
                if raan_count > 0 {
                    let entries = parse_raan_entries(raan, raan_offset, raan_count);
                    if !entries.is_empty() {
                        actor.insert("raan_entries".into(), serde_json::Value::Array(entries));
                    }
                }
            }

            if let Some(rahk) = rahk_data {
                let rahk_offset = read_i32_le(item, 0x5D)
                    .map(|v| usize::try_from(v.max(0)).unwrap_or_default())
                    .unwrap_or_default();
                if rahk_offset > 0 && rahk_offset < rahk.len() {
                    let mut hooks = Vec::new();
                    let mut cursor = rahk_offset;
                    while cursor + 4 <= rahk.len() {
                        let val = read_u32_le(rahk, cursor).unwrap_or_default();
                        if val == 0 && cursor > rahk_offset {
                            break;
                        }
                        hooks.push(serde_json::Value::from(val));
                        cursor += 4;
                    }
                    if !hooks.is_empty() {
                        actor.insert("rahk_hooks".into(), serde_json::Value::Array(hooks));
                    }
                }
            }

            if let Some(ranm) = ranm_data {
                let ranm_offset = read_i32_le(item, 0x19)
                    .map(|v| usize::try_from(v.max(0)).unwrap_or_default())
                    .unwrap_or_default();
                if ranm_offset < ranm.len()
                    && let Some(end) = ranm[ranm_offset..].iter().position(|&b| b == 0)
                {
                    let s =
                        String::from_utf8_lossy(&ranm[ranm_offset..ranm_offset + end]).to_string();
                    if !s.is_empty() {
                        actor.insert("ranm_name".into(), serde_json::Value::String(s));
                    }
                }
            }

            let mpsz_index_0 = read_i32_le(item, 0x8D).unwrap_or(-1);
            let mpsz_index_1 = read_i32_le(item, 0x91).unwrap_or(-1);
            if mpsz_index_0 >= 0 {
                actor.insert("mpsz_bounds_0".into(), mpsz_index_0.into());
            }
            if mpsz_index_1 >= 0 {
                actor.insert("mpsz_bounds_1".into(), mpsz_index_1.into());
            }

            actors.push(serde_json::Value::Object(actor));
        }
    }

    let mut mps_placements = Vec::new();
    let mut mpob_objects = Vec::new();
    let mut lights = Vec::new();
    let mut ropes = Vec::new();
    let mut collision_volumes = Vec::new();
    let mut flat_sprites = Vec::new();
    let mut markers = Vec::new();
    let mut walk_node_maps = Vec::new();
    let mut ralc_locations = Vec::new();
    let mut rafs_entries = Vec::new();
    let mut mpsz_entries = Vec::new();
    let mut raw_sections = Vec::new();

    for section in &rgm.sections {
        match section {
            RgmSection::Mps(_, records) => {
                mps_placements.extend(records.iter().map(mps_to_json));
            }
            RgmSection::MpobParsed(_, records) => {
                mpob_objects.extend(
                    records
                        .iter()
                        .enumerate()
                        .map(|(index, record)| mpob_to_json(index, record)),
                );
            }
            RgmSection::MplParsed(_, records) => {
                lights.extend(records.iter().map(mpsl_to_json));
            }
            RgmSection::MprpParsed(_, records) => {
                ropes.extend(records.iter().map(mprp_to_json));
            }
            RgmSection::RavcParsed(_, records) => {
                collision_volumes.extend(records.iter().map(ravc_to_json));
            }
            RgmSection::Wdnm(_, data) => {
                walk_node_maps.extend(parse_wdnm_maps(data));
            }
            RgmSection::Rafs(_, data) => {
                rafs_entries.extend(parse_rafs_entries(data));
            }
            RgmSection::Rast(_, _)
            | RgmSection::Rasb(_, _)
            | RgmSection::Rava(_, _)
            | RgmSection::Rasc(_, _)
            | RgmSection::Rahk(_, _)
            | RgmSection::Ranm(_, _) => {}
            RgmSection::Ralc(_, data) => {
                let mut cursor = 0usize;
                while cursor + 12 <= data.len() {
                    let offset_x = read_i32_le(data, cursor).unwrap_or_default();
                    let offset_y = read_i32_le(data, cursor + 4).unwrap_or_default();
                    let offset_z = read_i32_le(data, cursor + 8).unwrap_or_default();
                    ralc_locations.push(serde_json::json!({
                        "offset_x": offset_x,
                        "offset_y": offset_y,
                        "offset_z": offset_z,
                    }));
                    cursor += 12;
                }
            }
            RgmSection::Raat(_, _) => {}
            RgmSection::Raan(_, _) => {}
            RgmSection::Mpf(_, data) => {
                flat_sprites.extend(parse_mpf_records(data));
            }
            RgmSection::Mpm(_, data) => {
                markers.extend(parse_mpm_records(data));
            }
            RgmSection::Mpsz(_, data) => {
                mpsz_entries.extend(parse_mpsz_entries(data));
            }
            RgmSection::Flat(_, data) => raw_sections.push(raw_section_to_json("FLAT", data)),
            RgmSection::Raex(_, data) => raw_sections.push(raw_section_to_json("RAEX", data)),
            RgmSection::Ravc(_, data) => raw_sections.push(raw_section_to_json("RAVC", data)),
            RgmSection::Mpob(_, data) => raw_sections.push(raw_section_to_json("MPOB", data)),
            RgmSection::Mprp(_, data) => raw_sections.push(raw_section_to_json("MPRP", data)),
            RgmSection::Mpl(_, data) => raw_sections.push(raw_section_to_json("MPL ", data)),
            RgmSection::Rahd(_, _)
            | RgmSection::Ragr(_, _)
            | RgmSection::RaexParsed(_, _)
            | RgmSection::End(_) => {}
        }
    }

    serde_json::json!({
        "actors": actors,
        "mps_placements": mps_placements,
        "mpob_objects": mpob_objects,
        "lights": lights,
        "ropes": ropes,
        "collision_volumes": collision_volumes,
        "flat_sprites": flat_sprites,
        "markers": markers,
        "walk_node_maps": walk_node_maps,
        "ralc_locations": ralc_locations,
        "rafs_entries": rafs_entries,
        "mpsz_entries": mpsz_entries,
        "raw_sections": raw_sections,
    })
}
