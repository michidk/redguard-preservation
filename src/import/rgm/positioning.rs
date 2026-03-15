use super::{
    MpobRecord, MprpRecord, MpsRecord, PositionedLight, PositionedModel, RgmFile, RgmSection,
    shared::{read_i16_le, read_i32_le, read_script_name_9},
};
use crate::{
    import::registry::Registry,
    model3d::{
        self, FaceData, FaceNormal, FaceVertex, FrameDataEntry, FrameType, Model3DFile,
        Model3DHeader, ModelVersion, TextureData, VertexCoord, VertexNormal,
    },
};
use glam::{EulerRot, Mat3, Quat, Vec3};
use log::{debug, info, warn};
use std::collections::HashMap;

const RGM_POSITION_SCALE: f32 = 1.0 / 5120.0;
const MPOB_ANGLE_TO_DEGREES: f32 = 180.0 / 1024.0;
const ROPE_LINK_Y_STEP: f32 = 0.8;

#[derive(Debug, Clone)]
struct MpsfRecord {
    pos_x: i32,
    pos_y: i32,
    pos_z: u32,
    texture_id: u16,
    image_id: u8,
}

pub(super) fn extract_positioned_models(
    input: &[u8],
    rgm_file: &RgmFile,
    registry: &Registry,
) -> (Vec<PositionedModel>, Vec<PositionedLight>) {
    let mut positioned_models = Vec::new();
    let mut positioned_lights = Vec::new();
    let rahd_data = first_section_payload(input, *b"RAHD");
    let raan_data = first_section_payload(input, *b"RAAN");

    let rahd_index = rahd_data.map(parse_rahd_raan_index).unwrap_or_default();
    let rahd_texture_overrides = rahd_data
        .map(parse_rahd_texture_overrides)
        .unwrap_or_default();

    info!(
        "Processing {} RGM sections for models",
        rgm_file.sections.len()
    );

    for (i, section) in rgm_file.sections.iter().enumerate() {
        info!("Processing section {}: {}", i + 1, section.header().name());

        match section {
            RgmSection::Mps(_, mps_records) => {
                info!(
                    "Found MPSO section with {} static objects",
                    mps_records.len()
                );

                for (idx, record) in mps_records.iter().enumerate() {
                    let model_name = record.model_name();
                    if !model_name.is_empty() {
                        info!("Looking for static model: '{model_name}'");
                        if let Some(model) = load_model_from_registry(&model_name, registry) {
                            positioned_models.push(PositionedModel {
                                model,
                                transform: mps_transform(record),
                                model_name: format!("S{idx:03}_{model_name}"),
                                source_id: Some(model_name.clone()),
                            });
                        }
                    }
                }
            }
            RgmSection::MpobParsed(_, mpob_records) => {
                info!(
                    "Found MPOB section with {} object records",
                    mpob_records.len()
                );

                for (idx, record) in mpob_records.iter().enumerate() {
                    let model_name = record.model_name();
                    let script_name = record.script_name();
                    let raan_fallback = if let (Some(raan), Some((raan_offset, _raan_count))) =
                        (raan_data, rahd_index.get(&script_name))
                    {
                        parse_raan_model_name(raan, *raan_offset)
                    } else {
                        None
                    };

                    if model_name.is_empty()
                        && let Some(fallback) = raan_fallback.as_ref()
                    {
                        info!(
                            "Resolved empty MPOB model via RAAN fallback: script='{script_name}' -> '{fallback}'"
                        );
                    }

                    let resolved_name = if !model_name.is_empty() {
                        model_name
                    } else if let Some(name) = raan_fallback {
                        name
                    } else {
                        script_name.clone()
                    };

                    if !resolved_name.is_empty() {
                        info!(
                            "Looking for positioned model: '{}' (script='{}', static={})",
                            resolved_name, script_name, record.is_static
                        );
                        if let Some(model) = load_model_from_registry(&resolved_name, registry) {
                            let mut model = model;
                            let texture_override = rahd_texture_overrides.get(&script_name);
                            if let Some(tex_id) = texture_override {
                                debug!(
                                    "Applying RAHD texture override for script '{script_name}': TEXBSI.{tex_id:03}"
                                );
                                apply_texture_override(&mut model, *tex_id);
                            }
                            let source_id = match texture_override {
                                Some(tex_id) => format!("{resolved_name}:tex{tex_id}"),
                                None => resolved_name.clone(),
                            };
                            positioned_models.push(PositionedModel {
                                model,
                                transform: mpob_transform(record),
                                model_name: format!("B_{idx:03}_{script_name}"),
                                source_id: Some(source_id),
                            });
                        } else {
                            positioned_models.push(PositionedModel {
                                model: build_empty_model(),
                                transform: mpob_transform(record),
                                model_name: format!("B_{idx:03}_{script_name}"),
                                source_id: None,
                            });
                        }
                    }
                }
            }
            RgmSection::MprpParsed(_, records) => {
                info!("Found MPRP section with {} rope records", records.len());
                append_rope_records(records, registry, &mut positioned_models);
            }
            RgmSection::Mprp(_, mprp_data) => {
                let records = parse_mprp_records(mprp_data);
                info!(
                    "Found raw MPRP section with {} decodable rope records",
                    records.len()
                );
                append_rope_records(&records, registry, &mut positioned_models);
            }
            RgmSection::Mpf(_, mpf_data) => {
                let records = parse_mpsf_records(mpf_data);
                if !records.is_empty() {
                    info!("Found MPSF section with {} flat records", records.len());
                }

                for (i, record) in records.iter().enumerate() {
                    positioned_models.push(PositionedModel {
                        model: build_flat_model(record.texture_id, record.image_id),
                        transform: translation_matrix(decode_position(
                            record.pos_x,
                            record.pos_y,
                            record.pos_z,
                        )),
                        model_name: format!("F{i:03}_{}/{}", record.texture_id, record.image_id),
                        source_id: None,
                    });
                }
            }
            RgmSection::MplParsed(_, records) => {
                if !records.is_empty() {
                    info!("Found MPSL section with {} lights", records.len());
                }

                for (idx, record) in records.iter().enumerate() {
                    positioned_lights.push(PositionedLight {
                        color: [
                            decode_mpsl_color(record.color_r),
                            decode_mpsl_color(record.color_g),
                            decode_mpsl_color(record.color_b),
                        ],
                        position: decode_position(record.pos_x, record.pos_y, record.pos_z),
                        range: decode_mpsl_range(record.param0, record.param1),
                        name: format!("L{idx:03}"),
                    });
                }
            }
            _ => {}
        }
    }

    info!(
        "Successfully parsed RGM file, found {} positioned models and {} positioned lights",
        positioned_models.len(),
        positioned_lights.len()
    );
    (positioned_models, positioned_lights)
}

fn append_rope_records(
    records: &[MprpRecord],
    registry: &Registry,
    positioned_models: &mut Vec<PositionedModel>,
) {
    for (i, record) in records.iter().enumerate() {
        let mut pos = decode_position(record.pos_x, record.pos_y, record.pos_z);
        let link_count = record.length.max(0) as usize;

        if !record.rope_model.is_empty()
            && let Some(model) = load_model_from_registry(&record.rope_model, registry)
        {
            for j in 0..link_count {
                pos[1] -= ROPE_LINK_Y_STEP;
                positioned_models.push(PositionedModel {
                    model: model.clone(),
                    transform: translation_matrix(pos),
                    model_name: format!("R{i:03}_{j:03}_{}", record.rope_model),
                    source_id: Some(record.rope_model.clone()),
                });
            }
        }

        if !record.static_model.is_empty()
            && let Some(model) = load_model_from_registry(&record.static_model, registry)
        {
            pos[1] -= ROPE_LINK_Y_STEP;
            positioned_models.push(PositionedModel {
                model,
                transform: translation_matrix(pos),
                model_name: format!("R{i:03}_{link_count:03}_{}", record.static_model),
                source_id: Some(record.static_model.clone()),
            });
        }
    }
}

pub(super) fn decode_position(pos_x: i32, pos_y: i32, pos_z: u32) -> [f32; 3] {
    let x = -(((pos_x as i64) * 256) as f32) * RGM_POSITION_SCALE;
    let y = -(((pos_y as i64) * 256) as f32) * RGM_POSITION_SCALE;
    let z24 = (pos_z & 0x00FF_FFFF) as i64;
    let z = -((0x00FF_FFFF_i64 - (z24 * 256)) as f32) * RGM_POSITION_SCALE;
    [x, y, z]
}

fn decode_mpsl_color(color_byte: u8) -> f32 {
    (color_byte as f32 - 31.0) / 193.0
}

fn decode_mpsl_range(param0: i16, param1: i16) -> f32 {
    let raw = if param0 != 0 { param0 } else { param1 };
    (raw as f32).abs() * 0.05
}

fn read_i24_le_signed(bytes: &[u8], offset: usize) -> Option<i32> {
    let b0 = *bytes.get(offset)? as i32;
    let b1 = (*bytes.get(offset + 1)? as i32) << 8;
    let b2 = (*bytes.get(offset + 2)? as i32) << 16;
    let mut value = b0 | b1 | b2;
    if (value & 0x0080_0000) != 0 {
        value |= !0x00FF_FFFF;
    }
    Some(value)
}

fn parse_mpsf_records(data: &[u8]) -> Vec<MpsfRecord> {
    if data.len() < 4 {
        return Vec::new();
    }

    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    const ITEM_SIZE: usize = 24;
    let mut records = Vec::with_capacity(count);
    let mut cursor = 4usize;
    for _ in 0..count {
        if cursor + ITEM_SIZE > data.len() {
            break;
        }

        let item = &data[cursor..cursor + ITEM_SIZE];
        let Some(pos_x) = read_i24_le_signed(item, 8) else {
            cursor += ITEM_SIZE;
            continue;
        };
        let Some(pos_y) = read_i24_le_signed(item, 12) else {
            cursor += ITEM_SIZE;
            continue;
        };
        let pos_z = u32::from_le_bytes([item[16], item[17], item[18], 0]);
        let texture_data = u16::from_le_bytes([item[20], item[21]]);
        let texture_id = texture_data >> 7;
        let image_id = (texture_data & 0x7F) as u8;

        records.push(MpsfRecord {
            pos_x,
            pos_y,
            pos_z,
            texture_id,
            image_id,
        });

        cursor += ITEM_SIZE;
    }

    records
}

fn build_flat_model(texture_id: u16, image_id: u8) -> Model3DFile {
    let header = Model3DHeader {
        version: *b"v2.7",
        num_vertices: 4,
        num_faces: 1,
        radius: 0,
        num_frames: 0,
        offset_frame_data: 0,
        total_face_vertices: 4,
        offset_section4: 0,
        section4_count: 0,
        _unused_24: 0,
        offset_normal_indices: 0,
        offset_vertex_normals: 0,
        offset_vertex_coords: 0,
        offset_face_normals: 0,
        total_face_vertices_dup: 4,
        offset_face_data: 0,
    };

    let face_data = FaceData {
        vertex_count: 4,
        tex_hi: 0,
        texture_data: TextureData::Texture {
            texture_id,
            image_id,
        },
        face_vertices: vec![
            FaceVertex {
                vertex_index: 0,
                u: 0,
                v: 0,
            },
            FaceVertex {
                vertex_index: 1,
                u: 256,
                v: 0,
            },
            FaceVertex {
                vertex_index: 2,
                u: 256,
                v: 256,
            },
            FaceVertex {
                vertex_index: 3,
                u: 0,
                v: 256,
            },
        ],
    };

    Model3DFile {
        header,
        version: ModelVersion::V27,
        frame_data: vec![FrameDataEntry {
            vertex_offset: 0,
            normal_offset: 0,
            reserved: 0,
            frame_type: FrameType::Static3D,
        }],
        face_data: vec![face_data],
        vertex_coords: vec![
            VertexCoord {
                x: -0.5,
                y: 0.0,
                z: 0.5,
            },
            VertexCoord {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            VertexCoord {
                x: 0.5,
                y: 0.0,
                z: -0.5,
            },
            VertexCoord {
                x: -0.5,
                y: 0.0,
                z: -0.5,
            },
        ],
        face_normals: vec![FaceNormal {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }],
        normal_indices: vec![],
        vertex_normals: vec![
            VertexNormal {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            VertexNormal {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            VertexNormal {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            VertexNormal {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ],
    }
}

fn build_empty_model() -> Model3DFile {
    let header = Model3DHeader {
        version: *b"v2.7",
        num_vertices: 0,
        num_faces: 0,
        radius: 0,
        num_frames: 0,
        offset_frame_data: 0,
        total_face_vertices: 0,
        offset_section4: 0,
        section4_count: 0,
        _unused_24: 0,
        offset_normal_indices: 0,
        offset_vertex_normals: 0,
        offset_vertex_coords: 0,
        offset_face_normals: 0,
        total_face_vertices_dup: 0,
        offset_face_data: 0,
    };

    Model3DFile {
        header,
        version: ModelVersion::V27,
        frame_data: vec![],
        face_data: vec![],
        vertex_coords: vec![],
        face_normals: vec![],
        normal_indices: vec![],
        vertex_normals: vec![],
    }
}

fn parse_rahd_raan_index(rahd_data: &[u8]) -> HashMap<String, (usize, usize)> {
    let mut out = HashMap::new();
    if rahd_data.len() < 8 {
        return out;
    }

    let count =
        u32::from_le_bytes([rahd_data[0], rahd_data[1], rahd_data[2], rahd_data[3]]) as usize;
    let mut cursor = 8usize;
    const RAHD_ITEM_SIZE: usize = 165;

    for _ in 0..count {
        if cursor + RAHD_ITEM_SIZE > rahd_data.len() {
            break;
        }
        let item = &rahd_data[cursor..cursor + RAHD_ITEM_SIZE];

        let Some(script_name) = read_script_name_9(item, 4) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };
        let Some(raan_count) = read_i32_le(item, 0x21).map(|v| v.max(0) as usize) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };
        let Some(raan_offset) = read_i32_le(item, 0x29).map(|v| v.max(0) as usize) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };

        if !script_name.is_empty() && raan_count > 0 {
            out.insert(script_name, (raan_offset, raan_count));
        }

        cursor += RAHD_ITEM_SIZE;
    }

    out
}

fn parse_rahd_texture_overrides(rahd_data: &[u8]) -> HashMap<String, u16> {
    let mut out = HashMap::new();
    if rahd_data.len() < 8 {
        return out;
    }

    let count =
        u32::from_le_bytes([rahd_data[0], rahd_data[1], rahd_data[2], rahd_data[3]]) as usize;
    let mut cursor = 8usize;
    const RAHD_ITEM_SIZE: usize = 165;

    for _ in 0..count {
        if cursor + RAHD_ITEM_SIZE > rahd_data.len() {
            break;
        }
        let item = &rahd_data[cursor..cursor + RAHD_ITEM_SIZE];

        let Some(script_name) = read_script_name_9(item, 4) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };
        let Some(texture_id) = read_i16_le(item, 0x9B).map(|v| v.max(0) as u16) else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };

        if !script_name.is_empty() && texture_id > 0 {
            out.insert(script_name, texture_id);
        }

        cursor += RAHD_ITEM_SIZE;
    }

    out
}

fn apply_texture_override(model: &mut Model3DFile, texture_override: u16) {
    let mut solid_before = 0usize;
    let mut textured_before = 0usize;
    for face in &model.face_data {
        match face.texture_data {
            TextureData::SolidColor(_) => solid_before += 1,
            TextureData::Texture { .. } => textured_before += 1,
        }
    }

    for face in &mut model.face_data {
        match &mut face.texture_data {
            TextureData::Texture {
                texture_id,
                image_id: _,
            } => {
                if *texture_id >= 2 {
                    *texture_id = texture_override;
                }
            }
            TextureData::SolidColor(_) => {
                face.texture_data = TextureData::Texture {
                    texture_id: texture_override,
                    image_id: 0,
                };
            }
        }
    }

    let mut solid_after = 0usize;
    let mut textured_after = 0usize;
    for face in &model.face_data {
        match face.texture_data {
            TextureData::SolidColor(_) => solid_after += 1,
            TextureData::Texture { .. } => textured_after += 1,
        }
    }
    debug!(
        "Applied texture override TEXBSI.{texture_override:03}: faces solid {solid_before}->{solid_after}, textured {textured_before}->{textured_after}"
    );
}

fn parse_raan_model_name(raan_data: &[u8], mut offset: usize) -> Option<String> {
    if offset + 6 > raan_data.len() {
        return None;
    }

    offset += 6;

    let mut end = offset;
    while end < raan_data.len() && raan_data[end] != 0 {
        end += 1;
    }
    if end >= raan_data.len() || end == offset {
        return None;
    }

    let raw = String::from_utf8_lossy(&raan_data[offset..end]).to_string();
    let normalized = raw.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    let stem = file_name.split('.').next().unwrap_or(file_name).trim();
    if stem.is_empty() {
        None
    } else {
        Some(stem.to_ascii_uppercase())
    }
}

fn parse_mprp_records(data: &[u8]) -> Vec<MprpRecord> {
    if data.len() < 4 {
        return Vec::new();
    }
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut records = Vec::new();
    let mut cursor = 4usize;
    const MPRP_RECORD_SIZE: usize = 80;

    for _ in 0..count {
        if cursor + MPRP_RECORD_SIZE > data.len() {
            break;
        }
        let rec = &data[cursor..cursor + MPRP_RECORD_SIZE];

        let id = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]);
        let unknown0 = rec[4];
        let pos_x_u = u32::from_le_bytes([rec[5], rec[6], rec[7], 0]);
        let pad_x = rec[8];
        let pos_y_u = u32::from_le_bytes([rec[9], rec[10], rec[11], 0]);
        let pad_y = rec[12];
        let pos_z = u32::from_le_bytes([rec[13], rec[14], rec[15], 0]);
        let angle_y = i32::from_le_bytes([rec[16], rec[17], rec[18], rec[19]]);
        let rope_type = i32::from_le_bytes([rec[20], rec[21], rec[22], rec[23]]);
        let swing = i32::from_le_bytes([rec[24], rec[25], rec[26], rec[27]]);
        let speed = i32::from_le_bytes([rec[28], rec[29], rec[30], rec[31]]);
        let pos_x = if pos_x_u & 0x0080_0000 != 0 {
            (pos_x_u | 0xFF00_0000) as i32
        } else {
            pos_x_u as i32
        };
        let pos_y = if pos_y_u & 0x0080_0000 != 0 {
            (pos_y_u | 0xFF00_0000) as i32
        } else {
            pos_y_u as i32
        };
        let length = i16::from_le_bytes([rec[32], rec[33]]);
        let static_model = String::from_utf8_lossy(&rec[34..43])
            .trim_matches('\0')
            .to_string();
        let rope_model = String::from_utf8_lossy(&rec[43..52])
            .trim_matches('\0')
            .to_string();
        let mut unknown1 = [0_i32; 7];
        for (idx, field) in unknown1.iter_mut().enumerate() {
            let start = 52 + idx * 4;
            *field =
                i32::from_le_bytes([rec[start], rec[start + 1], rec[start + 2], rec[start + 3]]);
        }

        records.push(MprpRecord {
            id,
            unknown0,
            pos_x,
            pad_x,
            pos_y,
            pad_y,
            pos_z,
            angle_y,
            rope_type,
            swing,
            speed,
            length,
            static_model,
            rope_model,
            unknown1,
        });

        cursor += MPRP_RECORD_SIZE;
    }

    records
}

fn translation_matrix(translation: [f32; 3]) -> [f32; 16] {
    [
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

pub(super) fn first_section_payload(input: &[u8], target: [u8; 4]) -> Option<&[u8]> {
    let mut cursor = 0usize;
    while cursor + 8 <= input.len() {
        let name = [
            input[cursor],
            input[cursor + 1],
            input[cursor + 2],
            input[cursor + 3],
        ];
        let length = u32::from_be_bytes([
            input[cursor + 4],
            input[cursor + 5],
            input[cursor + 6],
            input[cursor + 7],
        ]) as usize;
        cursor += 8;

        if name == [b'E', b'N', b'D', b' '] {
            break;
        }
        if cursor + length > input.len() {
            break;
        }

        let payload = &input[cursor..cursor + length];
        if name == target {
            return Some(payload);
        }

        cursor += length;
    }
    None
}

fn q4_28_to_f32(value: i32) -> f32 {
    value as f32 / 268_435_456.0
}

fn build_column_major_matrix(
    rotation_row_major: [[f32; 3]; 3],
    translation: [f32; 3],
) -> [f32; 16] {
    [
        rotation_row_major[0][0],
        rotation_row_major[1][0],
        rotation_row_major[2][0],
        0.0,
        rotation_row_major[0][1],
        rotation_row_major[1][1],
        rotation_row_major[2][1],
        0.0,
        rotation_row_major[0][2],
        rotation_row_major[1][2],
        rotation_row_major[2][2],
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

fn mps_transform(record: &MpsRecord) -> [f32; 16] {
    let raw_rotation = Mat3::from_cols(
        Vec3::new(
            q4_28_to_f32(record.rotation_matrix[0]),
            q4_28_to_f32(record.rotation_matrix[3]),
            q4_28_to_f32(record.rotation_matrix[6]),
        ),
        Vec3::new(
            q4_28_to_f32(record.rotation_matrix[1]),
            q4_28_to_f32(record.rotation_matrix[4]),
            q4_28_to_f32(record.rotation_matrix[7]),
        ),
        Vec3::new(
            q4_28_to_f32(record.rotation_matrix[2]),
            q4_28_to_f32(record.rotation_matrix[5]),
            q4_28_to_f32(record.rotation_matrix[8]),
        ),
    );

    let base_quat = Quat::from_mat3(&raw_rotation);
    let (ey, ez, ex) = base_quat.to_euler(EulerRot::YZX);
    let fixed_quat = Quat::from_euler(EulerRot::YZX, -ey, ez, -ex);
    let fixed_rotation = Mat3::from_quat(fixed_quat);

    let rotation = [
        [
            fixed_rotation.x_axis.x,
            fixed_rotation.y_axis.x,
            fixed_rotation.z_axis.x,
        ],
        [
            fixed_rotation.x_axis.y,
            fixed_rotation.y_axis.y,
            fixed_rotation.z_axis.y,
        ],
        [
            fixed_rotation.x_axis.z,
            fixed_rotation.y_axis.z,
            fixed_rotation.z_axis.z,
        ],
    ];

    build_column_major_matrix(
        rotation,
        decode_position(record.pos_x, record.pos_y, record.pos_z),
    )
}

fn mpob_transform(record: &MpobRecord) -> [f32; 16] {
    let ex = (record.angle_x % 2048) as f32 * MPOB_ANGLE_TO_DEGREES;
    let ey = (record.angle_y % 2048) as f32 * MPOB_ANGLE_TO_DEGREES;
    let ez = -((record.angle_z % 2048) as f32 * MPOB_ANGLE_TO_DEGREES);

    let rx = ex.to_radians();
    let ry = ey.to_radians();
    let rz = ez.to_radians();
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();

    let ry_m = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx_m = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let rz_m = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];

    let mut temp = [[0.0_f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            temp[i][j] =
                ry_m[i][0] * rx_m[0][j] + ry_m[i][1] * rx_m[1][j] + ry_m[i][2] * rx_m[2][j];
        }
    }

    let mut rotation = [[0.0_f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            rotation[i][j] =
                temp[i][0] * rz_m[0][j] + temp[i][1] * rz_m[1][j] + temp[i][2] * rz_m[2][j];
        }
    }

    build_column_major_matrix(
        rotation,
        decode_position(record.pos_x, record.pos_y, record.pos_z),
    )
}

fn load_model_from_registry(model_name: &str, registry: &Registry) -> Option<Model3DFile> {
    let mut candidates = Vec::new();
    let trimmed = model_name.trim_matches('\0').trim();
    if !trimmed.is_empty() {
        candidates.push(trimmed.to_string());
        candidates.push(trimmed.to_ascii_uppercase());

        if let Some((base, _)) = trimmed.split_once('.') {
            candidates.push(base.to_string());
            candidates.push(base.to_ascii_uppercase());
        }
    }

    candidates.sort();
    candidates.dedup();

    let file_entry = candidates
        .iter()
        .find_map(|candidate| registry.get_file_by_name(candidate));

    if let Some(file_entry) = file_entry {
        info!(
            "Found model '{}' at path: {}",
            file_entry.name,
            file_entry.path.display()
        );

        match file_entry.file_type {
            crate::import::FileType::Model3d | crate::import::FileType::Model3dc => {
                match std::fs::read(&file_entry.path) {
                    Ok(data) => match model3d::parse_3d_file(&data) {
                        Ok(model) => {
                            info!("Successfully loaded model '{model_name}'");
                            Some(model)
                        }
                        Err(e) => {
                            warn!("Failed to parse model '{model_name}': {e}");
                            None
                        }
                    },
                    Err(e) => {
                        warn!(
                            "Failed to read model file '{}': {}",
                            file_entry.path.display(),
                            e
                        );
                        None
                    }
                }
            }
            crate::import::FileType::Rob => match std::fs::read(&file_entry.path) {
                Ok(data) => match crate::import::rob::parse_rob_with_models(&data) {
                    Ok((_, rob_models)) => {
                        info!(
                            "Successfully loaded {} models from ROB file '{}'",
                            rob_models.len(),
                            model_name
                        );
                        rob_models.into_iter().next()
                    }
                    Err(e) => {
                        warn!("Failed to parse ROB file '{model_name}': {e}");
                        None
                    }
                },
                Err(e) => {
                    warn!(
                        "Failed to read ROB file '{}': {}",
                        file_entry.path.display(),
                        e
                    );
                    None
                }
            },
            _ => {
                warn!(
                    "Unsupported file type for model '{}': {:?}",
                    model_name, file_entry.file_type
                );
                None
            }
        }
    } else {
        let mut rob_files: Vec<_> = registry
            .files
            .values()
            .filter(|entry| entry.file_type == crate::import::FileType::Rob)
            .collect();

        rob_files.sort_by_key(|entry| registry.source_rank_key(&entry.path));

        for file_entry in rob_files {
            if file_entry.file_type != crate::import::FileType::Rob {
                continue;
            }

            let data = match std::fs::read(&file_entry.path) {
                Ok(data) => data,
                Err(_) => continue,
            };

            let rob_file = match crate::import::rob::parse_rob_file(&data) {
                Ok(rob) => rob,
                Err(_) => continue,
            };

            for segment in &rob_file.segments {
                let segment_name = segment.name();
                let is_match = candidates
                    .iter()
                    .any(|candidate| segment_name.eq_ignore_ascii_case(candidate));
                if !is_match {
                    continue;
                }

                if segment.has_embedded_3d_data() {
                    match segment.parse_embedded_3d_data() {
                        Ok(model) => {
                            info!(
                                "Resolved model '{}' via ROB segment '{}' from {}",
                                model_name,
                                segment_name,
                                file_entry.path.display()
                            );
                            return Some(model);
                        }
                        Err(e) => {
                            warn!(
                                "Failed parsing ROB segment '{}' in {}: {}",
                                segment_name,
                                file_entry.path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        warn!("Model '{model_name}' not found in registry (tried {candidates:?})");
        None
    }
}
