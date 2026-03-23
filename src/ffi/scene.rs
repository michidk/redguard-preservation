use super::buffer::*;
use super::{i32_to_usize, read_bytes};
use crate::gltf::{MaterialKey, TextureCache, build_wld_unrolled_primitives};
use crate::import::rtx::RtxEntry;
use crate::import::{
    cht, fnt, fnt_ttf, gxa, palette::Palette, pvo, rgm, rob, rtx, sfx, wld, world_ini,
};
use crate::model3d::{self, Model3DFile, TextureData};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

const ENGINE_UNIT_SCALE: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SubmeshKey {
    SolidColor(u8),
    Textured(u16, u8),
}

#[derive(Debug, Default)]
struct SubmeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

#[must_use]
const fn sanitize_f32(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value }
}

#[must_use]
fn resolve_vertex_normal(
    model: &Model3DFile,
    vertex_index: usize,
    cumulative_fv_index: usize,
    face_normal: [f32; 3],
) -> [f32; 3] {
    let vn_index = if !model.normal_indices.is_empty() {
        model
            .normal_indices
            .get(cumulative_fv_index)
            .and_then(|&index| usize::try_from(index).ok())
    } else if !model.vertex_normals.is_empty() {
        Some(vertex_index)
    } else {
        None
    };

    if let Some(idx) = vn_index
        && let Some(vn) = model.vertex_normals.get(idx)
        && !vn.x.is_nan()
        && !vn.y.is_nan()
        && !vn.z.is_nan()
    {
        return [-sanitize_f32(vn.x), -sanitize_f32(vn.y), sanitize_f32(vn.z)];
    }

    face_normal
}

fn usize_to_i32(value: usize, name: &str) -> crate::Result<i32> {
    i32::try_from(value)
        .map_err(|_| crate::error::Error::Parse(format!("{name} exceeds i32::MAX: {value}")))
}

fn usize_to_u32(value: usize, name: &str) -> crate::Result<u32> {
    u32::try_from(value)
        .map_err(|_| crate::error::Error::Parse(format!("{name} exceeds u32::MAX: {value}")))
}

fn serialize_model_3d(model: &Model3DFile) -> crate::Result<Vec<u8>> {
    let mut submeshes: BTreeMap<SubmeshKey, SubmeshData> = BTreeMap::new();
    let mut cumulative_fv_base: usize = 0;

    for (face_index, face) in model.face_data.iter().enumerate() {
        if face.face_vertices.len() < 3 {
            cumulative_fv_base += face.face_vertices.len();
            continue;
        }

        let face_normal = if face_index < model.face_normals.len() {
            let fn_ = &model.face_normals[face_index];
            [
                -sanitize_f32(fn_.x),
                -sanitize_f32(fn_.y),
                sanitize_f32(fn_.z),
            ]
        } else {
            [0.0, 0.0, 1.0]
        };

        let submesh_key = match face.texture_data {
            TextureData::SolidColor(color_index) => SubmeshKey::SolidColor(color_index),
            TextureData::Texture {
                texture_id,
                image_id,
            } => SubmeshKey::Textured(texture_id, image_id),
        };
        let submesh = submeshes.entry(submesh_key).or_default();

        let v0 = &face.face_vertices[0];
        for i in 1..(face.face_vertices.len() - 1) {
            let v1 = &face.face_vertices[i];
            let v2 = &face.face_vertices[i + 1];
            let tri_fv = [v0, v1, v2];
            let tri_fv_indices = [0usize, i, i + 1];

            if tri_fv.iter().any(|fv| {
                usize::try_from(fv.vertex_index)
                    .ok()
                    .is_none_or(|index| index >= model.vertex_coords.len())
            }) {
                continue;
            }

            for (fv, &fv_idx) in tri_fv.iter().zip(&tri_fv_indices) {
                let Ok(idx) = usize::try_from(fv.vertex_index) else {
                    continue;
                };

                let pos = &model.vertex_coords[idx];
                let px = -sanitize_f32(pos.x) / ENGINE_UNIT_SCALE;
                let py = -sanitize_f32(pos.y) / ENGINE_UNIT_SCALE;
                let pz = sanitize_f32(pos.z) / ENGINE_UNIT_SCALE;
                submesh.positions.push([px, py, pz]);

                let normal =
                    resolve_vertex_normal(model, idx, cumulative_fv_base + fv_idx, face_normal);
                submesh.normals.push(normal);

                submesh.uvs.push([f32::from(fv.u), f32::from(fv.v)]);
                submesh.indices.push(usize_to_u32(
                    submesh.positions.len() - 1,
                    "submesh_vertex_index",
                )?);
            }
        }

        cumulative_fv_base += face.face_vertices.len();
    }

    let populated_submeshes: Vec<_> = submeshes
        .into_iter()
        .filter(|(_, submesh)| !submesh.indices.is_empty())
        .collect();

    let total_vertex_count = populated_submeshes
        .iter()
        .map(|(_, submesh)| submesh.positions.len())
        .sum::<usize>();
    let total_index_count = populated_submeshes
        .iter()
        .map(|(_, submesh)| submesh.indices.len())
        .sum::<usize>();

    let mut out = Vec::new();

    out.extend_from_slice(b"RGMD");
    out.extend_from_slice(&model.header.version);
    out.extend_from_slice(&usize_to_i32(populated_submeshes.len(), "submesh_count")?.to_le_bytes());
    out.extend_from_slice(
        &i32::try_from(model.header.num_frames)
            .map_err(|_| {
                crate::error::Error::Parse(format!(
                    "frame_count exceeds i32::MAX: {}",
                    model.header.num_frames
                ))
            })?
            .to_le_bytes(),
    );
    out.extend_from_slice(&usize_to_i32(total_vertex_count, "total_vertex_count")?.to_le_bytes());
    out.extend_from_slice(&usize_to_i32(total_index_count, "total_index_count")?.to_le_bytes());
    out.extend_from_slice(&model.header.radius.to_le_bytes());

    for (key, submesh) in populated_submeshes {
        match key {
            SubmeshKey::SolidColor(color_index) => {
                out.push(0);
                out.push(color_index);
                out.extend_from_slice(&0u16.to_le_bytes());
                out.push(0);
            }
            SubmeshKey::Textured(texture_id, image_id) => {
                out.push(1);
                out.extend_from_slice(&texture_id.to_le_bytes());
                out.push(image_id);
            }
        }
        out.extend_from_slice(&[0, 0]);
        out.push(1);
        out.extend_from_slice(
            &usize_to_i32(submesh.positions.len(), "submesh_vertex_count")?.to_le_bytes(),
        );
        out.extend_from_slice(
            &usize_to_i32(submesh.indices.len(), "submesh_index_count")?.to_le_bytes(),
        );

        for ((position, normal), uv) in submesh
            .positions
            .iter()
            .zip(submesh.normals.iter())
            .zip(submesh.uvs.iter())
        {
            out.extend_from_slice(&position[0].to_le_bytes());
            out.extend_from_slice(&position[1].to_le_bytes());
            out.extend_from_slice(&position[2].to_le_bytes());
            out.extend_from_slice(&normal[0].to_le_bytes());
            out.extend_from_slice(&normal[1].to_le_bytes());
            out.extend_from_slice(&normal[2].to_le_bytes());
            out.extend_from_slice(&uv[0].to_le_bytes());
            out.extend_from_slice(&uv[1].to_le_bytes());
        }

        for index in &submesh.indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
    }

    Ok(out)
}

fn pcm_to_wav_bytes(
    audio_type: sfx::AudioType,
    sample_rate: u32,
    pcm_data: &[u8],
) -> crate::Result<Vec<u8>> {
    let spec = WavSpec {
        channels: audio_type.channels(),
        sample_rate,
        bits_per_sample: audio_type.bits_per_sample(),
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| {
        crate::error::Error::Conversion(format!("failed to create WAV writer: {e}"))
    })?;

    if audio_type.bits_per_sample() == 8 {
        for &sample in pcm_data {
            writer.write_sample(sample.cast_signed()).map_err(|e| {
                crate::error::Error::Conversion(format!("failed to write WAV sample: {e}"))
            })?;
        }
    } else {
        for chunk in pcm_data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            writer.write_sample(sample).map_err(|e| {
                crate::error::Error::Conversion(format!("failed to write WAV sample: {e}"))
            })?;
        }
    }

    writer.finalize().map_err(|e| {
        crate::error::Error::Conversion(format!("failed to finalize WAV writer: {e}"))
    })?;
    Ok(cursor.into_inner())
}

fn rtx_index_json(rtx_file: &rtx::RtxFile) -> serde_json::Value {
    let metadata_entries: Vec<serde_json::Value> = rtx_file
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let tag_str = entry.tag_str();

            match entry {
                RtxEntry::Text { text, .. } => json!({
                    "index": i,
                    "tag": tag_str,
                    "type": "text",
                    "text": text,
                }),
                RtxEntry::Audio {
                    label,
                    header,
                    pcm_data,
                    ..
                } => json!({
                    "index": i,
                    "tag": tag_str,
                    "type": "audio",
                    "label": label,
                    "audio_type": format!("{:?}", header.audio_type),
                    "sample_rate": header.sample_rate,
                    "duration_secs": header.duration_secs(),
                    "pcm_bytes": pcm_data.len(),
                    "wav_file": format!("{tag_str}.wav"),
                }),
            }
        })
        .collect();

    json!({
        "entry_count": rtx_file.entries.len(),
        "audio_count": rtx_file.audio_count(),
        "text_count": rtx_file.text_count(),
        "entries": metadata_entries,
    })
}

/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_model_data(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let model = model3d::parse_3d_file(slice)?;
            serialize_model_3d(&model)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_rob_data(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let rob_file = rob::parse_rob_file(slice)?;

            let mut out = Vec::new();
            out.extend_from_slice(
                &usize_to_i32(rob_file.segments.len(), "segment_count")?.to_le_bytes(),
            );

            for segment in &rob_file.segments {
                out.extend_from_slice(&segment.segment_name);

                if segment.has_embedded_3d_data() {
                    out.push(1);
                    let model = segment.parse_embedded_3d_data()?;
                    let serialized = serialize_model_3d(&model)?;
                    out.extend_from_slice(
                        &usize_to_i32(serialized.len(), "model_data_size")?.to_le_bytes(),
                    );
                    out.extend_from_slice(&serialized);
                } else {
                    out.push(0);
                }
            }

            Ok(out)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texture_cache` must be a valid pointer from `rg_texture_cache_create`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture(
    texture_cache: *mut TextureCache,
    texture_id: u16,
    image_id: u8,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let cache = unsafe {
            texture_cache
                .as_mut()
                .ok_or_else(|| crate::error::Error::Parse("texture_cache is null".to_string()))?
        };
        run_on_large_stack(move || {
            let (rgba, width, height, frame_count) = cache
                .get_image_rgba_with_frame_count(texture_id, image_id)
                .ok_or_else(|| {
                    crate::error::Error::Parse(format!(
                        "texture not found: TEXBSI.{texture_id:03} image {image_id}"
                    ))
                })?;
            let mut out = Vec::new();
            out.extend_from_slice(&i32::from(width).to_le_bytes());
            out.extend_from_slice(&i32::from(height).to_le_bytes());
            out.extend_from_slice(&i32::from(frame_count).to_le_bytes());
            out.extend_from_slice(&usize_to_i32(rgba.len(), "rgba_size")?.to_le_bytes());
            out.extend_from_slice(&rgba);
            Ok(out)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texture_cache` must be a valid pointer from `rg_texture_cache_create`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture_all_frames(
    texture_cache: *mut TextureCache,
    texture_id: u16,
    image_id: u8,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let cache = unsafe {
            texture_cache
                .as_mut()
                .ok_or_else(|| crate::error::Error::Parse("texture_cache is null".to_string()))?
        };
        run_on_large_stack(move || {
            let info = cache
                .get_all_frames_by_image_id(texture_id, image_id)
                .ok_or_else(|| {
                    crate::error::Error::Parse(format!(
                        "texture not found: TEXBSI.{texture_id:03} image {image_id}"
                    ))
                })?;
            let mut out = Vec::new();
            out.extend_from_slice(&i32::from(info.width).to_le_bytes());
            out.extend_from_slice(&i32::from(info.height).to_le_bytes());
            out.extend_from_slice(&i32::from(info.frame_count).to_le_bytes());
            for frame in &info.frames {
                match frame {
                    Some(rgba) => {
                        out.extend_from_slice(
                            &usize_to_i32(rgba.len(), "rgba_size")?.to_le_bytes(),
                        );
                        out.extend_from_slice(rgba);
                    }
                    None => {
                        out.extend_from_slice(&0_i32.to_le_bytes());
                    }
                }
            }
            Ok(out)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texture_cache` must be a valid pointer from `rg_texture_cache_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_texbsi_image_count(
    texture_cache: *mut TextureCache,
    texture_id: u16,
) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let cache = unsafe {
            texture_cache
                .as_mut()
                .ok_or_else(|| crate::error::Error::Parse("texture_cache is null".to_string()))?
        };
        run_on_large_stack(move || {
            cache.ensure_bsi_available(texture_id);
            let count = cache.image_count(texture_id).unwrap_or(0);
            usize_to_i32(count, "image_count")
        })
    })();

    match result {
        Ok(count) => {
            clear_last_error();
            count
        }
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_sfx_effect_count(data: *const u8, len: i32) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = sfx::parse_sfx_file(slice)?;
            usize_to_i32(parsed.effects.len(), "effect_count")
        })
    })();

    match result {
        Ok(count) => {
            clear_last_error();
            count
        }
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rtx_entry_count(data: *const u8, len: i32) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(slice)?;
            usize_to_i32(parsed.entries.len(), "entry_count")
        })
    })();

    match result {
        Ok(count) => {
            clear_last_error();
            count
        }
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_sfx_to_wav(
    data: *const u8,
    len: i32,
    effect_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = sfx::parse_sfx_file(slice)?;
            let effect_idx = i32_to_usize(effect_index, "effect_index")?;
            let effect = parsed.effects.get(effect_idx).ok_or_else(|| {
                crate::error::Error::Parse(format!("effect_index out of range: {effect_index}"))
            })?;

            pcm_to_wav_bytes(effect.audio_type, effect.sample_rate, &effect.pcm_data)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_rtx_entry_to_wav(
    data: *const u8,
    len: i32,
    entry_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(slice)?;
            let entry_idx = i32_to_usize(entry_index, "entry_index")?;
            let entry = parsed.entries.get(entry_idx).ok_or_else(|| {
                crate::error::Error::Parse(format!("entry_index out of range: {entry_index}"))
            })?;

            match entry {
                RtxEntry::Audio {
                    header, pcm_data, ..
                } => pcm_to_wav_bytes(header.audio_type, header.sample_rate, pcm_data),
                RtxEntry::Text { .. } => Err(crate::error::Error::Parse(
                    "entry is text, not audio".to_string(),
                )),
            }
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rtx_metadata(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(slice)?;
            let metadata = rtx_index_json(&parsed);
            Ok(serde_json::to_vec(&metadata)?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_palette(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let palette = Palette::parse(slice)?;
            let colors = palette.colors.into_iter().collect::<Vec<[u8; 3]>>();
            Ok(serde_json::to_vec(&json!({ "colors": colors }))?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_pvo_to_json(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = pvo::parse_pvo_file(slice)?;

            let nodes = parsed
                .octr_nodes
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    json!({
                        "index": i,
                        "byte_offset": node.byte_offset,
                        "child_mask": format!("0x{:02X}", node.child_mask),
                        "child_count": node.child_count(),
                        "leaf_ref": if node.is_interior() {
                            "none".to_string()
                        } else {
                            format!("0x{:08X}", node.leaf_ref)
                        },
                        "child_refs": node.child_refs.iter()
                            .map(|(octant, offset)| json!({
                                "octant": octant,
                                "offset": format!("0x{offset:08X}"),
                            }))
                            .collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();

            let leaves = parsed
                .plst_leaves
                .iter()
                .map(|leaf| {
                    json!({
                        "byte_offset": leaf.byte_offset,
                        "entries": leaf.entries.iter().map(|e| json!({
                            "count": e.count,
                            "mlst_start": e.mlst_start,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();

            let output = json!({
                "header": {
                    "depth": parsed.header.depth,
                    "total_nodes": parsed.header.total_nodes,
                    "leaf_nodes": parsed.header.leaf_nodes,
                    "interior_nodes": parsed.header.interior_nodes(),
                    "mlst_polygon_count": parsed.header.mlst_polygon_count,
                    "cell_size": parsed.header.cell_size,
                    "center_x": parsed.header.center_x,
                    "center_y": parsed.header.center_y,
                    "center_z": parsed.header.center_z,
                },
                "octr_nodes": nodes,
                "plst_leaves": leaves,
                "mlst_indices": parsed.mlst_indices,
            });

            Ok(serde_json::to_vec(&output)?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_cht_to_json(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = cht::parse_cht_file(slice)?;

            let cheats: serde_json::Map<String, serde_json::Value> = parsed
                .named_cheats()
                .iter()
                .map(|e| {
                    let name = e.name.unwrap_or("unknown").to_string();
                    let value = if e.value > 1 {
                        json!(e.value)
                    } else {
                        json!(e.is_on())
                    };
                    (name, value)
                })
                .collect();

            Ok(serde_json::to_vec(&json!(cheats))?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_fnt_to_ttf(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let parsed = fnt::parse_fnt(slice)?;
            let ttf = fnt_ttf::build_ttf_from_fnt(&parsed, "RedguardFnt")?;
            Ok(ttf)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_ini(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let text = String::from_utf8_lossy(slice);
            let parsed = world_ini::WorldIni::parse(&text);
            let worlds = parsed
                .entries
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "index": entry.index,
                        "map": entry.map,
                        "world": entry.world,
                        "palette": entry.palette,
                    })
                })
                .collect::<Vec<_>>();
            Ok(serde_json::to_vec(
                &serde_json::json!({ "worlds": worlds }),
            )?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_gxa(data: *const u8, len: i32, frame: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let gxa_file = gxa::parse_gxa_file(slice)?;
            let frame_idx = i32_to_usize(frame, "frame")?;
            let selected = gxa_file.frames.get(frame_idx).ok_or_else(|| {
                crate::error::Error::Parse(format!(
                    "frame out of range: {frame} (frame_count={})",
                    gxa_file.frames.len()
                ))
            })?;

            let mut out = Vec::new();
            out.extend_from_slice(&i32::from(selected.width).to_le_bytes());
            out.extend_from_slice(&i32::from(selected.height).to_le_bytes());
            out.extend_from_slice(&1_i32.to_le_bytes());
            out.extend_from_slice(&usize_to_i32(selected.rgba.len(), "rgba_size")?.to_le_bytes());
            out.extend_from_slice(&selected.rgba);
            Ok(out)
        })
    })();

    into_ffi_result(result)
}

fn serialize_terrain_primitives(
    primitives: Vec<crate::gltf::UnrolledPrimitive>,
) -> crate::Result<Vec<u8>> {
    let total_vertex_count: usize = primitives.iter().map(|p| p.positions.len()).sum();
    let total_index_count: usize = primitives.iter().map(|p| p.indices.len()).sum();

    let mut out = Vec::new();
    out.extend_from_slice(b"RGMD");
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&usize_to_i32(primitives.len(), "submesh_count")?.to_le_bytes());
    out.extend_from_slice(&0_i32.to_le_bytes());
    out.extend_from_slice(&usize_to_i32(total_vertex_count, "total_vertex_count")?.to_le_bytes());
    out.extend_from_slice(&usize_to_i32(total_index_count, "total_index_count")?.to_le_bytes());
    out.extend_from_slice(&0.0_f32.to_le_bytes());

    for primitive in &primitives {
        match primitive.material_key {
            MaterialKey::Textured(texture_id, image_id) => {
                out.push(1);
                out.extend_from_slice(&texture_id.to_le_bytes());
                out.push(image_id);
            }
            MaterialKey::SolidColor(rgb) => {
                out.push(0);
                out.push(rgb[0]);
                out.extend_from_slice(&0u16.to_le_bytes());
                out.push(0);
            }
            _ => {
                out.push(0);
                out.push(0);
                out.extend_from_slice(&0u16.to_le_bytes());
                out.push(0);
            }
        }
        out.extend_from_slice(&[0, 0]);
        out.push(1);
        out.extend_from_slice(
            &usize_to_i32(primitive.positions.len(), "submesh_vertex_count")?.to_le_bytes(),
        );
        out.extend_from_slice(
            &usize_to_i32(primitive.indices.len(), "submesh_index_count")?.to_le_bytes(),
        );

        for ((position, normal), uv) in primitive
            .positions
            .iter()
            .zip(primitive.normals.iter())
            .zip(primitive.uvs.iter())
        {
            out.extend_from_slice(&position[0].to_le_bytes());
            out.extend_from_slice(&position[1].to_le_bytes());
            out.extend_from_slice(&position[2].to_le_bytes());
            out.extend_from_slice(&normal[0].to_le_bytes());
            out.extend_from_slice(&normal[1].to_le_bytes());
            out.extend_from_slice(&normal[2].to_le_bytes());
            out.extend_from_slice(&uv[0].to_le_bytes());
            out.extend_from_slice(&uv[1].to_le_bytes());
        }

        for index in &primitive.indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
    }

    Ok(out)
}

const RGPL_NAME_LEN: usize = 32;

fn write_fixed_string(out: &mut Vec<u8>, s: &str, max_len: usize) {
    let bytes = s.as_bytes();
    let write_len = bytes.len().min(max_len - 1);
    out.extend_from_slice(&bytes[..write_len]);
    out.extend(std::iter::repeat_n(0u8, max_len - write_len));
}

fn serialize_rgm_placements(
    placements: &[rgm::Placement],
    lights: &[rgm::PositionedLight],
) -> crate::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RGPL");
    out.extend_from_slice(&usize_to_i32(placements.len(), "placement_count")?.to_le_bytes());
    out.extend_from_slice(&usize_to_i32(lights.len(), "light_count")?.to_le_bytes());

    for p in placements {
        write_fixed_string(&mut out, &p.model_name, RGPL_NAME_LEN);
        write_fixed_string(&mut out, &p.source_id, RGPL_NAME_LEN);
        for val in &p.transform {
            out.extend_from_slice(&val.to_le_bytes());
        }
        out.push(p.object_type as u8);
        out.extend_from_slice(&p.texture_id.to_le_bytes());
        out.push(p.image_id);
    }

    for light in lights {
        write_fixed_string(&mut out, &light.name, RGPL_NAME_LEN);
        for val in &light.color {
            out.extend_from_slice(&val.to_le_bytes());
        }
        for val in &light.position {
            out.extend_from_slice(&val.to_le_bytes());
        }
        out.extend_from_slice(&light.range.to_le_bytes());
    }

    Ok(out)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_wld_terrain_data(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let wld_file = wld::parse_wld_file(slice)?;
            let texbsi_id = u16::from_le_bytes([
                wld_file.sections[0].header[6],
                wld_file.sections[0].header[7],
            ]);
            let primitives = build_wld_unrolled_primitives(&wld_file, texbsi_id)?;
            serialize_terrain_primitives(primitives)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `data` must point to readable bytes of length `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_rgm_placements(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let (placements, lights) = rgm::extract_rgm_placements(slice)?;
            serialize_rgm_placements(&placements, &lights)
        })
    })();

    into_ffi_result(result)
}

fn collect_model_texbsi_ids(model: &Model3DFile) -> BTreeSet<u16> {
    model
        .face_data
        .iter()
        .filter_map(|face| match face.texture_data {
            TextureData::Texture { texture_id, .. } => Some(texture_id),
            _ => None,
        })
        .collect()
}

/// Returns JSON: `{"model_names": ["TORCH", ...], "texbsi_ids": [302, ...]}`
///
/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rgm_dependencies(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let (placements, _) = rgm::extract_rgm_placements(slice)?;

            let mut model_names = BTreeSet::new();
            let mut texbsi_ids = BTreeSet::new();

            for p in &placements {
                if !p.model_name.is_empty() && p.object_type != rgm::PlacementType::FlatSprite {
                    model_names.insert(p.model_name.to_ascii_uppercase());
                }
                if p.texture_id >= 2 {
                    texbsi_ids.insert(p.texture_id);
                }
            }

            let json = json!({
                "model_names": model_names.into_iter().collect::<Vec<_>>(),
                "texbsi_ids": texbsi_ids.into_iter().collect::<Vec<_>>(),
            });
            Ok(serde_json::to_vec(&json)?)
        })
    })();

    into_ffi_result(result)
}

/// Returns JSON: `{"texbsi_ids": [302, ...]}`
///
/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_model_dependencies(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let model = model3d::parse_3d_file(slice)?;
            let texbsi_ids = collect_model_texbsi_ids(&model);
            let json = json!({
                "texbsi_ids": texbsi_ids.into_iter().collect::<Vec<_>>(),
            });
            Ok(serde_json::to_vec(&json)?)
        })
    })();

    into_ffi_result(result)
}

/// Returns JSON: `{"texbsi_ids": [302, ...]}`
///
/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rob_dependencies(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let (_, models) = rob::parse_rob_with_models(slice)?;
            let mut texbsi_ids = BTreeSet::new();
            for model in &models {
                texbsi_ids.extend(collect_model_texbsi_ids(model));
            }
            let json = json!({
                "texbsi_ids": texbsi_ids.into_iter().collect::<Vec<_>>(),
            });
            Ok(serde_json::to_vec(&json)?)
        })
    })();

    into_ffi_result(result)
}

/// Returns JSON: `{"texbsi_ids": [302]}`
///
/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_wld_dependencies(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        run_on_large_stack(move || {
            let wld_file = wld::parse_wld_file(slice)?;
            let texbsi_id = u16::from_le_bytes([
                wld_file.sections[0].header[6],
                wld_file.sections[0].header[7],
            ]);
            let json = json!({
                "texbsi_ids": [texbsi_id],
            });
            Ok(serde_json::to_vec(&json)?)
        })
    })();

    into_ffi_result(result)
}
