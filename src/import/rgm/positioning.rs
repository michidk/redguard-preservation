use super::{
    FlatSprite, MpobRecord, MprpRecord, MpsRecord, PositionedLight, PositionedModel, RgmFile,
    RgmSection,
    shared::{read_i16_le, read_i32_le, read_script_name_9},
};
use crate::import::rob::RobSegment;
use crate::{
    import::registry::Registry,
    model3d::{
        self, FaceData, FaceNormal, FaceVertex, FrameDataEntry, FrameType, Model3DFile,
        Model3DHeader, ModelVersion, TextureData, VertexCoord, VertexNormal,
    },
};
use glam::{EulerRot, Mat3, Quat, Vec3};
use log::{debug, info, warn};
use rayon::prelude::*;
use std::collections::HashMap;

const RGM_POSITION_SCALE: f32 = 1.0 / 5120.0;
const MPOB_ANGLE_TO_DEGREES: f32 = 180.0 / 1024.0;
const ROPE_LINK_Y_STEP: f32 = 0.8;
const MPSF_ITEM_SIZE: usize = 24;
const RAHD_ITEM_SIZE: usize = 165;
const MPRP_RECORD_SIZE: usize = 80;

#[derive(Debug, Clone)]
struct MpsfRecord {
    pos_x: i32,
    pos_y: i32,
    pos_z: u32,
    texture_id: u16,
    image_id: u8,
}

type RobSegmentIndex = HashMap<String, Vec<RobSegment>>;

fn build_rob_segment_index(
    registry: &Registry,
    preferred_rob_stem: Option<&str>,
) -> RobSegmentIndex {
    let mut rob_entries: Vec<_> = registry
        .files
        .values()
        .filter(|e| e.file_type == crate::import::FileType::Rob)
        .collect();
    let preferred_rob_stem = preferred_rob_stem.map(str::to_ascii_uppercase);
    rob_entries.sort_by_key(|entry| {
        let stem = entry
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_uppercase();
        let preferred_rank = if preferred_rob_stem.as_deref() == Some(stem.as_str()) {
            0usize
        } else {
            1usize
        };
        (preferred_rank, registry.source_rank_key(&entry.path))
    });

    let segment_batches: Vec<Vec<(String, RobSegment)>> = rob_entries
        .par_iter()
        .filter_map(|entry| {
            let data = read_entry_data(entry).ok()?;
            let rob_file = crate::import::rob::parse_rob_file(&data).ok()?;
            let segments: Vec<_> = rob_file
                .segments
                .into_iter()
                .filter(RobSegment::has_embedded_3d_data)
                .map(|seg| (seg.name().to_ascii_uppercase(), seg))
                .collect();
            Some(segments)
        })
        .collect();

    let mut index = RobSegmentIndex::new();
    for batch in segment_batches {
        for (name, segment) in batch {
            index.entry(name).or_default().push(segment);
        }
    }
    index
}

#[derive(Clone)]
struct LoadedModel {
    model: Model3DFile,
    render_attributes: Option<u8>,
}

impl LoadedModel {
    fn from_segment(segment: &RobSegment) -> Option<Self> {
        Some(Self {
            model: segment.parse_embedded_3d_data().ok()?,
            render_attributes: Some(segment.segment_attribs),
        })
    }
}

/// A placement omitted or emitted without geometry by the existing model loader.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlacementDiagnostic {
    /// Section, record index and field within the source RGM.
    pub field: String,
    /// Resolved asset reference, empty when resolution found no name.
    pub model: String,
    /// Number of missing mesh placements, including repeated rope links.
    pub count: usize,
    /// Loader failure; this does not change fallback or placement selection.
    pub reason: String,
}

#[allow(clippy::too_many_arguments)]
fn placement_model(
    name: &str,
    field: String,
    count: usize,
    registry: &Registry,
    cache: &mut HashMap<String, std::result::Result<LoadedModel, String>>,
    index: &RobSegmentIndex,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) -> Option<LoadedModel> {
    match cached_load_model(name, registry, cache, index) {
        Ok(model) => Some(model),
        Err(reason) => {
            diagnostics.push(PlacementDiagnostic {
                field,
                model: name.into(),
                count,
                reason,
            });
            None
        }
    }
}

/// Cache wrapper around `load_model_from_registry` that avoids redundant disk I/O.
/// Many MPOB records reference the same model (e.g. SHARKFIN ×30, WATERSND ×30).
/// Failures are cached with their reasons, preventing repeated ROB scans.
fn cached_load_model(
    model_name: &str,
    registry: &Registry,
    cache: &mut HashMap<String, std::result::Result<LoadedModel, String>>,
    rob_index: &RobSegmentIndex,
) -> std::result::Result<LoadedModel, String> {
    let key = model_name.trim_matches('\0').trim().to_ascii_uppercase();
    if let Some(cached) = cache.get(&key) {
        return cached.clone();
    }
    let result = load_model_from_registry(model_name, registry, rob_index);
    cache.insert(key, result.clone());
    result
}

pub(super) fn extract_positioned_models(
    input: &[u8],
    rgm_file: &RgmFile,
    registry: &Registry,
    preferred_rob_stem: Option<&str>,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) -> (Vec<PositionedModel>, Vec<PositionedLight>) {
    let mut positioned_models = Vec::new();
    let mut model_cache: HashMap<String, std::result::Result<LoadedModel, String>> = HashMap::new();
    let rob_index = build_rob_segment_index(registry, preferred_rob_stem);
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
            RgmSection::Mps(_, mps_records) => process_mps_section(
                mps_records,
                registry,
                &mut positioned_models,
                &mut model_cache,
                &rob_index,
                diagnostics,
            ),
            RgmSection::MpobParsed(_, mpob_records) => process_mpob_section(
                mpob_records,
                raan_data,
                &rahd_index,
                &rahd_texture_overrides,
                registry,
                &mut positioned_models,
                &mut model_cache,
                &rob_index,
                diagnostics,
            ),
            RgmSection::MprpParsed(_, records) => {
                info!("Found MPRP section with {} rope records", records.len());
                append_rope_records(
                    records,
                    registry,
                    &mut positioned_models,
                    &mut model_cache,
                    &rob_index,
                    diagnostics,
                );
            }
            RgmSection::Mprp(_, mprp_data) => {
                let records = parse_mprp_records(mprp_data);
                info!(
                    "Found raw MPRP section with {} decodable rope records",
                    records.len()
                );
                append_rope_records(
                    &records,
                    registry,
                    &mut positioned_models,
                    &mut model_cache,
                    &rob_index,
                    diagnostics,
                );
            }
            RgmSection::Mpf(_, mpf_data) => {
                append_mpf_models(mpf_data, rgm_file, &mut positioned_models, diagnostics);
            }
            RgmSection::MplParsed(_, records) if !records.is_empty() => {
                warn!(
                    "Authored local lights require engine shading; retained in RGM metadata, not converted to punctual lights"
                );
            }
            _ => {}
        }
    }

    info!(
        "Successfully parsed RGM file, found {} positioned models",
        positioned_models.len()
    );
    (positioned_models, Vec::new())
}

fn process_mps_section(
    mps_records: &[MpsRecord],
    registry: &Registry,
    positioned_models: &mut Vec<PositionedModel>,
    model_cache: &mut HashMap<String, std::result::Result<LoadedModel, String>>,
    rob_index: &RobSegmentIndex,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) {
    info!(
        "Found MPSO section with {} static objects",
        mps_records.len()
    );

    for (idx, record) in mps_records.iter().enumerate() {
        let model_name = record.model_name();
        if model_name.is_empty() {
            diagnostics.push(PlacementDiagnostic {
                field: format!("MPSO[{idx}].model_name"),
                model: model_name,
                count: 1,
                reason: "empty model name".into(),
            });
            continue;
        }

        info!("Looking for static model: '{model_name}'");
        if let Some(model) = placement_model(
            &model_name,
            format!("MPSO[{idx}].model_name"),
            1,
            registry,
            model_cache,
            rob_index,
            diagnostics,
        ) {
            positioned_models.push(PositionedModel {
                flat: None,
                model: model.model,
                render_attributes: model.render_attributes,
                transform: mps_transform(record),
                model_name: format!("S{idx:03}_{model_name}"),
                source_id: Some(model_name),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_mpob_section(
    mpob_records: &[MpobRecord],
    raan_data: Option<&[u8]>,
    rahd_index: &HashMap<String, (usize, usize)>,
    rahd_texture_overrides: &HashMap<String, u16>,
    registry: &Registry,
    positioned_models: &mut Vec<PositionedModel>,
    model_cache: &mut HashMap<String, std::result::Result<LoadedModel, String>>,
    rob_index: &RobSegmentIndex,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) {
    info!(
        "Found MPOB section with {} object records",
        mpob_records.len()
    );

    for (idx, record) in mpob_records.iter().enumerate() {
        let script_name = record.script_name();
        let resolved_name = resolve_mpob_model_name(record, &script_name, raan_data, rahd_index);
        if resolved_name.is_empty() {
            diagnostics.push(PlacementDiagnostic {
                field: format!("MPOB[{idx}].model_name"),
                model: resolved_name,
                count: 1,
                reason: "model resolution returned an empty name".into(),
            });
            continue;
        }

        info!(
            "Looking for positioned model: '{}' (script='{}', static={})",
            resolved_name, script_name, record.is_static
        );

        if let Some(model) = placement_model(
            &resolved_name,
            format!("MPOB[{idx}].model_name"),
            1,
            registry,
            model_cache,
            rob_index,
            diagnostics,
        ) {
            positioned_models.push(build_loaded_mpob_model(
                model,
                record,
                idx,
                &script_name,
                &resolved_name,
                rahd_texture_overrides.get(&script_name).copied(),
            ));
        } else {
            positioned_models.push(PositionedModel {
                flat: None,
                model: build_empty_model(),
                transform: mpob_transform(record),
                model_name: format!("B_{idx:03}_{script_name}"),
                source_id: None,
                render_attributes: None,
            });
        }
    }
}

fn resolve_mpob_model_name(
    record: &MpobRecord,
    script_name: &str,
    raan_data: Option<&[u8]>,
    rahd_index: &HashMap<String, (usize, usize)>,
) -> String {
    let model_name = record.model_name();
    let raan_fallback = match (raan_data, rahd_index.get(script_name)) {
        (Some(raan), Some((raan_offset, _raan_count))) => parse_raan_model_name(raan, *raan_offset),
        _ => None,
    };

    if model_name.is_empty()
        && let Some(fallback) = raan_fallback.as_ref()
    {
        info!(
            "Resolved empty MPOB model via RAAN fallback: script='{script_name}' -> '{fallback}'"
        );
    }

    if !model_name.is_empty() {
        model_name
    } else if let Some(name) = raan_fallback {
        name
    } else {
        script_name.to_owned()
    }
}

fn build_loaded_mpob_model(
    mut loaded: LoadedModel,
    record: &MpobRecord,
    idx: usize,
    script_name: &str,
    resolved_name: &str,
    texture_override: Option<u16>,
) -> PositionedModel {
    if let Some(tex_id) = texture_override {
        debug!("Applying RAHD texture override for script '{script_name}': TEXBSI.{tex_id:03}");
        apply_texture_override(&mut loaded.model, tex_id);
    }

    let source_id = texture_override.map_or_else(
        || resolved_name.to_owned(),
        |tex_id| format!("{resolved_name}:tex{tex_id}"),
    );

    PositionedModel {
        flat: None,
        model: loaded.model,
        render_attributes: loaded.render_attributes,
        transform: mpob_transform(record),
        model_name: format!("B_{idx:03}_{script_name}"),
        source_id: Some(source_id),
    }
}

fn append_mpf_models(
    mpf_data: &[u8],
    rgm: &RgmFile,
    positioned_models: &mut Vec<PositionedModel>,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) {
    let records = parse_mpsf_records(mpf_data);
    if !records.is_empty() {
        info!("Found MPSF section with {} flat records", records.len());
    }

    for (idx, record) in records.iter().enumerate() {
        if rgm
            .sections
            .iter()
            .any(|section| matches!(section, RgmSection::Flat(_, _)))
        {
            diagnostics.push(PlacementDiagnostic {
                field: format!("MPSF[{idx}].texture_data"),
                model: format!("TEXBSI.{:03}/{}", record.texture_id, record.image_id),
                count: 1,
                reason: "malformed FLAT texture attributes".into(),
            });
            continue;
        }
        positioned_models.push(PositionedModel {
            flat: Some(FlatSprite {
                texture_id: record.texture_id,
                image_id: record.image_id,
                attributes: rgm
                    .flat_texture_attributes((record.texture_id << 7) | u16::from(record.image_id))
                    .copied(),
            }),
            model: build_flat_model(record.texture_id, record.image_id),
            transform: translation_matrix(decode_position(
                record.pos_x,
                record.pos_y,
                record.pos_z,
            )),
            model_name: format!("F{idx:03}_{}/{}", record.texture_id, record.image_id),
            source_id: None,
            render_attributes: None,
        });
    }
}

fn append_rope_records(
    records: &[MprpRecord],
    registry: &Registry,
    positioned_models: &mut Vec<PositionedModel>,
    model_cache: &mut HashMap<String, std::result::Result<LoadedModel, String>>,
    rob_index: &RobSegmentIndex,
    diagnostics: &mut Vec<PlacementDiagnostic>,
) {
    for (i, record) in records.iter().enumerate() {
        let mut pos = decode_position(record.pos_x, record.pos_y, record.pos_z);
        let link_count = usize::try_from(record.length.max(0)).unwrap_or_default();
        if !record.rope_model.is_empty()
            && let Some(model) = placement_model(
                &record.rope_model,
                format!("MPRP[{i}].rope_model"),
                link_count,
                registry,
                model_cache,
                rob_index,
                diagnostics,
            )
        {
            for j in 0..link_count {
                pos[1] -= ROPE_LINK_Y_STEP;
                positioned_models.push(PositionedModel {
                    flat: None,
                    model: model.model.clone(),
                    render_attributes: model.render_attributes,
                    transform: translation_matrix(pos),
                    model_name: format!("R{i:03}_{j:03}_{}", record.rope_model),
                    source_id: Some(record.rope_model.clone()),
                });
            }
        }
        if !record.static_model.is_empty()
            && let Some(model) = placement_model(
                &record.static_model,
                format!("MPRP[{i}].static_model"),
                1,
                registry,
                model_cache,
                rob_index,
                diagnostics,
            )
        {
            pos[1] -= ROPE_LINK_Y_STEP;
            positioned_models.push(PositionedModel {
                flat: None,
                model: model.model,
                render_attributes: model.render_attributes,
                transform: translation_matrix(pos),
                model_name: format!("R{i:03}_{link_count:03}_{}", record.static_model),
                source_id: Some(record.static_model.clone()),
            });
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
// Fixed-point scene units are intentionally converted to f32 transforms.
pub(super) fn decode_position(pos_x: i32, pos_y: i32, pos_z: u32) -> [f32; 3] {
    let x = -(f64::from(pos_x) * 256.0) as f32 * RGM_POSITION_SCALE;
    let y = -(f64::from(pos_y) * 256.0) as f32 * RGM_POSITION_SCALE;
    let z24 = f64::from(pos_z & 0x00FF_FFFF);
    let z_full_scale = f64::from(0x00FF_FFFF_u32);
    let z = -((z_full_scale - (z24 * 256.0)) as f32) * RGM_POSITION_SCALE;
    [x, y, z]
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

fn parse_mpsf_records(data: &[u8]) -> Vec<MpsfRecord> {
    if data.len() < 4 {
        return Vec::new();
    }

    let count = usize::try_from(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
        .unwrap_or_default();
    let mut records = Vec::with_capacity(count);
    let mut cursor = 4usize;
    for _ in 0..count {
        if cursor + MPSF_ITEM_SIZE > data.len() {
            break;
        }

        let item = &data[cursor..cursor + MPSF_ITEM_SIZE];
        let Some(pos_x) = read_i24_le_signed(item, 8) else {
            cursor += MPSF_ITEM_SIZE;
            continue;
        };
        let Some(pos_y) = read_i24_le_signed(item, 12) else {
            cursor += MPSF_ITEM_SIZE;
            continue;
        };
        let pos_z = u32::from_le_bytes([item[16], item[17], item[18], 0]);
        let texture_data = u16::from_le_bytes([item[20], item[21]]);
        let texture_id = texture_data >> 7;
        let image_id = u8::try_from(texture_data & 0x7F).unwrap_or_default();

        records.push(MpsfRecord {
            pos_x,
            pos_y,
            pos_z,
            texture_id,
            image_id,
        });

        cursor += MPSF_ITEM_SIZE;
    }

    records
}

fn build_flat_model(texture_id: u16, image_id: u8) -> Model3DFile {
    Model3DFile {
        header: model_header(4, 1, 4),
        version: ModelVersion::V27,
        frame_data: vec![FrameDataEntry {
            vertex_offset: 0,
            normal_offset: 0,
            reserved: 0,
            frame_type: FrameType::Static3D,
        }],
        face_data: vec![flat_model_face(texture_id, image_id)],
        vertex_coords: flat_model_vertices(),
        face_normals: vec![FaceNormal {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        }],
        normal_indices: vec![],
        vertex_normals: flat_model_normals(),
        frame_vertex_data: vec![],
        frame_normal_data: vec![],
    }
}

const fn model_header(
    num_vertices: u32,
    num_faces: u32,
    total_face_vertices: u32,
) -> Model3DHeader {
    Model3DHeader {
        version: *b"v2.7",
        num_vertices,
        num_faces,
        radius: 0,
        num_frames: 0,
        offset_frame_data: 0,
        total_face_vertices,
        offset_section4: 0,
        section4_count: 0,
        _unused_24: 0,
        offset_normal_indices: 0,
        offset_vertex_normals: 0,
        offset_vertex_coords: 0,
        offset_face_normals: 0,
        total_face_vertices_dup: total_face_vertices,
        offset_face_data: 0,
    }
}

fn flat_model_face(texture_id: u16, image_id: u8) -> FaceData {
    FaceData {
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
    }
}

fn flat_model_vertices() -> Vec<VertexCoord> {
    vec![
        VertexCoord {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VertexCoord {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
        VertexCoord {
            x: 20.0,
            y: -20.0,
            z: 0.0,
        },
        VertexCoord {
            x: 0.0,
            y: -20.0,
            z: 0.0,
        },
    ]
}

fn flat_model_normals() -> Vec<VertexNormal> {
    vec![
        VertexNormal {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        VertexNormal {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        VertexNormal {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        VertexNormal {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
    ]
}

#[allow(clippy::missing_const_for_fn)] // Contains heap allocations (`Vec`), so this cannot be const.
fn build_empty_model() -> Model3DFile {
    Model3DFile {
        header: model_header(0, 0, 0),
        version: ModelVersion::V27,
        frame_data: vec![],
        face_data: vec![],
        vertex_coords: vec![],
        face_normals: vec![],
        normal_indices: vec![],
        vertex_normals: vec![],
        frame_vertex_data: vec![],
        frame_normal_data: vec![],
    }
}

fn parse_rahd_raan_index(rahd_data: &[u8]) -> HashMap<String, (usize, usize)> {
    let mut out = HashMap::new();
    if rahd_data.len() < 8 {
        return out;
    }

    let count = usize::try_from(u32::from_le_bytes([
        rahd_data[0],
        rahd_data[1],
        rahd_data[2],
        rahd_data[3],
    ]))
    .unwrap_or_default();
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
        let Some(raan_count) =
            read_i32_le(item, 0x21).map(|value| usize::try_from(value.max(0)).unwrap_or_default())
        else {
            cursor += RAHD_ITEM_SIZE;
            continue;
        };
        let Some(raan_offset) =
            read_i32_le(item, 0x29).map(|value| usize::try_from(value.max(0)).unwrap_or_default())
        else {
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

    let count = usize::try_from(u32::from_le_bytes([
        rahd_data[0],
        rahd_data[1],
        rahd_data[2],
        rahd_data[3],
    ]))
    .unwrap_or_default();
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
        let Some(texture_id) =
            read_i16_le(item, 0x9B).map(|value| u16::try_from(value.max(0)).unwrap_or_default())
        else {
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

/// Applies an actor texture set to a model, as placement does for RAHD/RAEX
/// `texture_id`. Use it for every multi-frame model drawn by that actor, including
/// RAAN frame sources; geometry, frames and normals are unchanged.
pub fn apply_texture_override(model: &mut Model3DFile, texture_override: u16) {
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
    let count = usize::try_from(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
        .unwrap_or_default();
    let mut records = Vec::new();
    let mut cursor = 4usize;

    for _ in 0..count {
        if cursor + MPRP_RECORD_SIZE > data.len() {
            break;
        }
        let rec = &data[cursor..cursor + MPRP_RECORD_SIZE];

        let id = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]);
        let unknown0 = rec[4];
        let encoded_pos_x = u32::from_le_bytes([rec[5], rec[6], rec[7], 0]);
        let pad_x = rec[8];
        let encoded_pos_y = u32::from_le_bytes([rec[9], rec[10], rec[11], 0]);
        let pad_y = rec[12];
        let pos_z = u32::from_le_bytes([rec[13], rec[14], rec[15], 0]);
        let angle_y = i32::from_le_bytes([rec[16], rec[17], rec[18], rec[19]]);
        let rope_type = i32::from_le_bytes([rec[20], rec[21], rec[22], rec[23]]);
        let swing = i32::from_le_bytes([rec[24], rec[25], rec[26], rec[27]]);
        let speed = i32::from_le_bytes([rec[28], rec[29], rec[30], rec[31]]);
        let pos_x = sign_extend_u24(encoded_pos_x);
        let pos_y = sign_extend_u24(encoded_pos_y);
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

const fn translation_matrix(translation: [f32; 3]) -> [f32; 16] {
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

const fn sign_extend_u24(value: u32) -> i32 {
    let mut bytes = value.to_le_bytes();
    if (value & 0x0080_0000) != 0 {
        bytes[3] = 0xFF;
    }
    i32::from_le_bytes(bytes)
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
        let length = usize::try_from(u32::from_be_bytes([
            input[cursor + 4],
            input[cursor + 5],
            input[cursor + 6],
            input[cursor + 7],
        ]))
        .unwrap_or_default();
        cursor += 8;

        if name == *b"END " {
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

#[allow(clippy::cast_precision_loss)] // Q4.28 fixed-point values are intentionally narrowed for transforms.
fn q4_28_to_f32(value: i32) -> f32 {
    value as f32 / 268_435_456.0
}

const fn build_column_major_matrix(
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
    let angle_x = u16::try_from(record.angle_x % 2048).unwrap_or_default();
    let angle_y = u16::try_from(record.angle_y % 2048).unwrap_or_default();
    let angle_z = u16::try_from(record.angle_z % 2048).unwrap_or_default();

    let ex = f32::from(angle_x) * MPOB_ANGLE_TO_DEGREES;
    let ey = f32::from(angle_y) * MPOB_ANGLE_TO_DEGREES;
    let ez = -(f32::from(angle_z) * MPOB_ANGLE_TO_DEGREES);

    let rx = ex.to_radians();
    let ry = ey.to_radians();
    let rz = ez.to_radians();
    let (sx, cx) = libm::sincosf(rx);
    let (sy, cy) = libm::sincosf(ry);
    let (sz, cz) = libm::sincosf(rz);

    let yaw_matrix = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let pitch_matrix = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let roll_matrix = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];

    let temp = multiply_3x3(yaw_matrix, pitch_matrix);
    let rotation = multiply_3x3(temp, roll_matrix);

    build_column_major_matrix(
        rotation,
        decode_position(record.pos_x, record.pos_y, record.pos_z),
    )
}

fn multiply_3x3(lhs: [[f32; 3]; 3], rhs: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0_f32; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            out[row][col] = lhs[row][0].mul_add(
                rhs[0][col],
                lhs[row][1].mul_add(rhs[1][col], lhs[row][2] * rhs[2][col]),
            );
        }
    }
    out
}

fn read_entry_data(entry: &crate::import::registry::FileEntry) -> std::io::Result<Vec<u8>> {
    if let Some(data) = &entry.data {
        return Ok(data.clone());
    }
    std::fs::read(&entry.path)
}

const fn model_file_rank(file_type: crate::import::FileType) -> Option<usize> {
    match file_type {
        crate::import::FileType::Model3dc => Some(0),
        crate::import::FileType::Model3d => Some(1),
        crate::import::FileType::Rob => Some(2),
        _ => None,
    }
}

fn load_model_from_registry(
    model_name: &str,
    registry: &Registry,
    rob_index: &RobSegmentIndex,
) -> std::result::Result<LoadedModel, String> {
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

    let file_entry = candidates.iter().find_map(|candidate| {
        registry
            .files
            .iter()
            .filter(|(key, _)| *key == candidate || key.starts_with(&format!("{candidate}#")))
            .filter_map(|(_, entry)| {
                model_file_rank(entry.file_type).map(|type_rank| {
                    let source_rank = registry.source_rank_key(&entry.path);
                    (type_rank, source_rank, entry)
                })
            })
            .min_by_key(|(type_rank, source_rank, _)| (*type_rank, source_rank.clone()))
            .map(|(_, _, entry)| entry)
    });

    if let Some(file_entry) = file_entry {
        info!(
            "Found model '{}' at path: {}",
            file_entry.name,
            file_entry.path.display()
        );

        match file_entry.file_type {
            crate::import::FileType::Model3d | crate::import::FileType::Model3dc => {
                match read_entry_data(file_entry) {
                    Ok(data) => match model3d::parse_3d_file(&data) {
                        Ok(model) => {
                            info!("Successfully loaded model '{model_name}'");
                            Ok(LoadedModel {
                                model,
                                render_attributes: None,
                            })
                        }
                        Err(e) => {
                            warn!("Failed to parse model '{model_name}': {e}");
                            Err(format!("{}: decode: {e}", file_entry.path.display()))
                        }
                    },
                    Err(e) => {
                        warn!(
                            "Failed to read model file '{}': {}",
                            file_entry.path.display(),
                            e
                        );
                        Err(format!("{}: read: {e}", file_entry.path.display()))
                    }
                }
            }
            crate::import::FileType::Rob => match read_entry_data(file_entry) {
                Ok(data) => match crate::import::rob::parse_rob_file(&data) {
                    Ok(archive) => archive
                        .segments
                        .iter()
                        .find_map(LoadedModel::from_segment)
                        .ok_or_else(|| {
                            format!("{}: no decodable model segment", file_entry.path.display())
                        }),
                    Err(e) => {
                        warn!("Failed to parse ROB file '{model_name}': {e}");
                        Err(format!("{}: decode: {e}", file_entry.path.display()))
                    }
                },
                Err(e) => {
                    warn!(
                        "Failed to read ROB file '{}': {}",
                        file_entry.path.display(),
                        e
                    );
                    Err(format!("{}: read: {e}", file_entry.path.display()))
                }
            },
            _ => {
                warn!(
                    "Unsupported file type for model '{}': {:?}",
                    model_name, file_entry.file_type
                );
                Err(format!(
                    "{}: unsupported model file type",
                    file_entry.path.display()
                ))
            }
        }
    } else {
        for candidate in &candidates {
            if let Some(model) = rob_index
                .get(&candidate.to_ascii_uppercase())
                .and_then(|segments| segments.iter().find_map(LoadedModel::from_segment))
            {
                info!(
                    "Resolved model '{model_name}' via ROB segment index (candidate '{candidate}')"
                );
                return Ok(model);
            }
        }

        warn!("Model '{model_name}' not found in registry or ROB index (tried {candidates:?})");
        Err(format!(
            "model {model_name}: no decodable model in registry or ROB index"
        ))
    }
}

fn strip_extension(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlacementType {
    Mesh = 0,
    FlatSprite = 1,
    RopeLink = 2,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub model_name: String,
    pub source_id: String,
    pub transform: [f32; 16],
    pub object_type: PlacementType,
    pub texture_id: u16,
    pub image_id: u8,
}

pub(super) fn extract_placements(
    _input: &[u8],
    rgm_file: &RgmFile,
) -> (Vec<Placement>, Vec<PositionedLight>) {
    let mut placements = Vec::new();

    for section in &rgm_file.sections {
        match section {
            RgmSection::Mps(_, mps_records) => {
                for (idx, record) in mps_records.iter().enumerate() {
                    let file_name = record.model_name();
                    if file_name.is_empty() {
                        continue;
                    }
                    let stem = strip_extension(&file_name).to_owned();
                    placements.push(Placement {
                        model_name: stem.clone(),
                        source_id: format!("S{idx:03}_{stem}"),
                        transform: mps_transform(record),
                        object_type: PlacementType::Mesh,
                        texture_id: 0,
                        image_id: 0,
                    });
                }
            }
            RgmSection::MprpParsed(_, records) => {
                append_rope_placements(records, &mut placements);
            }
            RgmSection::Mprp(_, mprp_data) => {
                let records = parse_mprp_records(mprp_data);
                append_rope_placements(&records, &mut placements);
            }
            RgmSection::Mpf(_, mpf_data) => {
                let records = parse_mpsf_records(mpf_data);
                for (idx, record) in records.iter().enumerate() {
                    placements.push(Placement {
                        model_name: String::new(),
                        source_id: format!("F{idx:03}"),
                        transform: translation_matrix(decode_position(
                            record.pos_x,
                            record.pos_y,
                            record.pos_z,
                        )),
                        object_type: PlacementType::FlatSprite,
                        texture_id: record.texture_id,
                        image_id: record.image_id,
                    });
                }
            }
            _ => {}
        }
    }

    (placements, Vec::new())
}

fn append_rope_placements(records: &[MprpRecord], placements: &mut Vec<Placement>) {
    for (i, record) in records.iter().enumerate() {
        let mut pos = decode_position(record.pos_x, record.pos_y, record.pos_z);
        let link_count = usize::try_from(record.length.max(0)).unwrap_or_default();
        let rope_stem = strip_extension(&record.rope_model);
        let static_stem = strip_extension(&record.static_model);
        if !rope_stem.is_empty() {
            for j in 0..link_count {
                pos[1] -= ROPE_LINK_Y_STEP;
                placements.push(Placement {
                    model_name: rope_stem.to_owned(),
                    source_id: format!("R{i:03}_{j:03}_{rope_stem}"),
                    transform: translation_matrix(pos),
                    object_type: PlacementType::RopeLink,
                    texture_id: 0,
                    image_id: 0,
                });
            }
        }
        if !static_stem.is_empty() {
            pos[1] -= ROPE_LINK_Y_STEP;
            placements.push(Placement {
                model_name: static_stem.to_owned(),
                source_id: format!("R{i:03}_{link_count:03}_{static_stem}"),
                transform: translation_matrix(pos),
                object_type: PlacementType::Mesh,
                texture_id: 0,
                image_id: 0,
            });
        }
    }
}

#[cfg(test)]
mod lazy_archive_tests {
    use super::*;

    fn segment(bytes: &[u8]) -> RobSegment {
        let mut segment =
            crate::import::rob::parse_rob_file(include_bytes!("../../../tests/fixtures/scene.rob"))
                .unwrap()
                .segments
                .remove(0);
        segment.segment_type = 0;
        segment.data_size = u32::try_from(bytes.len()).unwrap();
        segment.data = bytes.to_vec();
        segment
    }

    #[test]
    fn skipped_placements_retain_record_and_loader_reason() {
        let record = MpsRecord {
            id: 1,
            model_name: *b"ABSENT      ",
            pos_x: 0,
            pos_y: 0,
            pos_z: 0,
            rotation_matrix: [0; 9],
            trailing: [0; 2],
        };
        let registry = Registry::new(std::path::PathBuf::new());
        let mut models = Vec::new();
        let mut cache = HashMap::new();
        let index = HashMap::new();
        let mut diagnostics = Vec::new();
        process_mps_section(
            &[record.clone(), record],
            &registry,
            &mut models,
            &mut cache,
            &index,
            &mut diagnostics,
        );
        assert!(models.is_empty());
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].field, "MPSO[0].model_name");
        assert_eq!(diagnostics[1].field, "MPSO[1].model_name");
        assert_eq!(diagnostics[0].count, 1);
        assert_eq!(diagnostics[0].reason, diagnostics[1].reason);
        assert!(diagnostics[0].reason.contains("no decodable model"));
    }

    #[test]
    fn archive_lookup_keeps_priority_and_skips_invalid_candidates() {
        let registry = Registry::new(Default::default());
        let mut first = segment(include_bytes!("../../../tests/fixtures/model_v26.3d"));
        first.segment_attribs = 65;
        let mut second = segment(include_bytes!("../../../tests/fixtures/model_v40.3dc"));
        second.segment_attribs = 2;
        let mut index = RobSegmentIndex::from([("SEGMENT1".into(), vec![first, second])]);
        assert_eq!(
            load_model_from_registry("segment1", &registry, &index)
                .unwrap()
                .model
                .version,
            ModelVersion::V26
        );
        let mut cache = HashMap::new();
        assert_eq!(
            cached_load_model("segment1", &registry, &mut cache, &index)
                .unwrap()
                .render_attributes,
            Some(65)
        );
        index.get_mut("SEGMENT1").unwrap()[0].data.clear();
        assert_eq!(
            load_model_from_registry("segment1", &registry, &index)
                .unwrap()
                .model
                .version,
            ModelVersion::V40
        );
        assert_eq!(
            load_model_from_registry("segment1", &registry, &index)
                .unwrap()
                .render_attributes,
            Some(2)
        );
        assert_eq!(
            cached_load_model("segment1", &registry, &mut cache, &index)
                .unwrap()
                .render_attributes,
            Some(65)
        );
        index.get_mut("SEGMENT1").unwrap()[1].data.clear();
        assert!(load_model_from_registry("segment1", &registry, &index).is_err());
        assert!(load_model_from_registry("absent", &registry, &index).is_err());
    }
}
