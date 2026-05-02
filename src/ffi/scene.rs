use super::buffer::*;
use super::types::*;
use super::world::WorldHandle;
use super::{i32_to_usize, read_c_str};
use crate::gltf::{
    ENGINE_UNIT_SCALE, MaterialKey, UV_FIXED_POINT_SCALE, build_wld_unrolled_primitives,
};
use crate::import::palette::Palette;
use crate::import::rtx::RtxEntry;
use crate::import::{fnt, fnt_ttf, gxa, rgm, rob, rtx, sfx, wld};
use crate::model3d::{self, Model3DFile, TextureData};
use bytemuck;
use hound::{SampleFormat, WavSpec, WavWriter};

use std::collections::BTreeMap;
use std::io::Cursor;
use std::mem::size_of;
use std::os::raw::c_char;
use std::path::PathBuf;

fn fixed_string<const N: usize>(s: &str) -> [u8; N] {
    let mut buf = [0u8; N];
    let bytes = s.as_bytes();
    let len = bytes.len().min(N - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

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
        return [sanitize_f32(vn.x), -sanitize_f32(vn.y), sanitize_f32(vn.z)];
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

pub(crate) fn serialize_model_3d(
    model: &Model3DFile,
    palette: Option<&Palette>,
    mut texture_cache: Option<&mut crate::gltf::TextureCache>,
) -> crate::Result<Vec<u8>> {
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
                sanitize_f32(fn_.x),
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
            let tri_fv = [v0, v2, v1];
            let tri_fv_indices = [0usize, i + 1, i];

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
                let px = sanitize_f32(pos.x) / ENGINE_UNIT_SCALE;
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

    let mut populated_submeshes: Vec<_> = submeshes
        .into_iter()
        .filter(|(_, submesh)| !submesh.indices.is_empty())
        .collect();

    for (key, submesh) in &mut populated_submeshes {
        let (tex_w, tex_h) = match key {
            SubmeshKey::Textured(texture_id, image_id) => {
                let cache = texture_cache.as_mut().expect("texture_cache required");
                let (w, h) = cache
                    .get_image_dimensions(*texture_id, *image_id)
                    .unwrap_or((1, 1));
                (f32::from(w.max(1)), f32::from(h.max(1)))
            }
            SubmeshKey::SolidColor(_) => (8.0, 8.0),
        };
        for uv in &mut submesh.uvs {
            uv[0] /= UV_FIXED_POINT_SCALE * tex_w;
            uv[1] = 1.0 - uv[1] / (UV_FIXED_POINT_SCALE * tex_h);
        }
    }

    let total_vertex_count = populated_submeshes
        .iter()
        .map(|(_, submesh)| submesh.positions.len())
        .sum::<usize>();
    let total_index_count = populated_submeshes
        .iter()
        .map(|(_, submesh)| submesh.indices.len())
        .sum::<usize>();

    let header = RgmdHeader {
        magic: *b"RGMD",
        version: [1, 0, 0, 0],
        submesh_count: usize_to_i32(populated_submeshes.len(), "submesh_count")?,
        frame_count: 1,
        total_vertex_count: usize_to_i32(total_vertex_count, "total_vertex_count")?,
        total_index_count: usize_to_i32(total_index_count, "total_index_count")?,
        radius: model.header.radius as f32 / ENGINE_UNIT_SCALE,
    };

    let estimated_size = size_of::<RgmdHeader>()
        + populated_submeshes.len() * size_of::<RgmdSubmeshHeader>()
        + total_vertex_count * size_of::<RgmdVertex>()
        + total_index_count * size_of::<u32>();
    let mut out = Vec::with_capacity(estimated_size);
    out.extend_from_slice(bytemuck::bytes_of(&header));

    for (key, submesh) in populated_submeshes {
        let (textured, color_rgb, texture_id, image_id) = match key {
            SubmeshKey::SolidColor(ci) => {
                let rgb = palette.map_or([128, 128, 128], |pal| pal.colors[usize::from(ci)]);
                (0u8, rgb, 0u16, 0u8)
            }
            SubmeshKey::Textured(tid, iid) => (1u8, [0u8; 3], tid, iid),
        };

        let sub_header = RgmdSubmeshHeader {
            textured,
            color_r: color_rgb[0],
            color_g: color_rgb[1],
            color_b: color_rgb[2],
            texture_id,
            image_id,
            _pad: 0,
            vertex_count: usize_to_i32(submesh.positions.len(), "submesh_vertex_count")?,
            index_count: usize_to_i32(submesh.indices.len(), "submesh_index_count")?,
        };
        out.extend_from_slice(bytemuck::bytes_of(&sub_header));

        let vertices: Vec<RgmdVertex> = submesh
            .positions
            .iter()
            .zip(&submesh.normals)
            .zip(&submesh.uvs)
            .map(|((pos, norm), uv)| RgmdVertex {
                position: *pos,
                normal: *norm,
                uv: *uv,
            })
            .collect();
        out.extend_from_slice(bytemuck::cast_slice::<RgmdVertex, u8>(&vertices));
        out.extend_from_slice(bytemuck::cast_slice::<u32, u8>(&submesh.indices));
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
        debug_assert!(
            pcm_data.len().is_multiple_of(2),
            "16-bit PCM data has odd byte count: {}",
            pcm_data.len()
        );
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

/// # Safety
///
/// `file_path` and `assets_dir` must be valid null-terminated UTF-8 strings.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_model_data(
    file_path: *const c_char,
    assets_dir: *const c_char,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let file_path = PathBuf::from(file_path);
        let assets_dir = PathBuf::from(assets_dir);
        let model_bytes = std::fs::read(&file_path)?;
        let palette =
            super::auto_resolve_palette(&assets_dir, &file_path, crate::import::FileType::Model3d);
        let mut texture_cache = crate::gltf::TextureCache::new(
            assets_dir.clone(),
            palette.as_ref().map(|pal| Palette { colors: pal.colors }),
        );
        run_on_large_stack(move || {
            let model = model3d::parse_3d_file(&model_bytes)?;
            serialize_model_3d(&model, palette.as_ref(), Some(&mut texture_cache))
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `file_path` and `assets_dir` must be valid null-terminated UTF-8 strings.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_rob_data(
    file_path: *const c_char,
    assets_dir: *const c_char,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let file_path = PathBuf::from(file_path);
        let assets_dir = PathBuf::from(assets_dir);
        let rob_bytes = std::fs::read(&file_path)?;
        let palette =
            super::auto_resolve_palette(&assets_dir, &file_path, crate::import::FileType::Rob);
        let mut texture_cache = crate::gltf::TextureCache::new(
            assets_dir.clone(),
            palette.as_ref().map(|pal| Palette { colors: pal.colors }),
        );
        run_on_large_stack(move || {
            let rob_file = rob::parse_rob_file(&rob_bytes)?;

            let rob_header = RobHeader {
                segment_count: usize_to_i32(rob_file.segments.len(), "segment_count")?,
            };
            let mut out = Vec::new();
            out.extend_from_slice(bytemuck::bytes_of(&rob_header));

            for segment in &rob_file.segments {
                let (has_model, model_data) = if segment.has_embedded_3d_data() {
                    let model = segment.parse_embedded_3d_data()?;
                    (
                        1u8,
                        Some(serialize_model_3d(
                            &model,
                            palette.as_ref(),
                            Some(&mut texture_cache),
                        )?),
                    )
                } else {
                    (0u8, None)
                };

                let seg_header = RobSegmentHeader {
                    segment_name: segment.segment_name,
                    has_model,
                    _pad: [0; 3],
                    model_data_size: model_data
                        .as_ref()
                        .map(|d| usize_to_i32(d.len(), "model_data_size"))
                        .transpose()?
                        .unwrap_or(0),
                };
                out.extend_from_slice(bytemuck::bytes_of(&seg_header));

                if let Some(data) = &model_data {
                    out.extend_from_slice(data);
                }
            }

            Ok(out)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// `file_path` must be a valid null-terminated UTF-8 string.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_model_data_world(
    world: *mut WorldHandle,
    file_path: *const c_char,
) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return std::ptr::null_mut();
    }

    let result = (|| -> crate::Result<Vec<u8>> {
        let handle = unsafe { &mut *world };
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let file_path = PathBuf::from(file_path);
        let model_bytes = std::fs::read(&file_path)?;
        let palette = Palette {
            colors: handle.palette().colors,
        };

        run_on_large_stack(move || {
            let model = model3d::parse_3d_file(&model_bytes)?;
            serialize_model_3d(&model, Some(&palette), Some(handle.texture_cache_mut()))
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// `file_path` must be a valid null-terminated UTF-8 string.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_rob_data_world(
    world: *mut WorldHandle,
    file_path: *const c_char,
) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return std::ptr::null_mut();
    }

    let result = (|| -> crate::Result<Vec<u8>> {
        let handle = unsafe { &mut *world };
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let file_path = PathBuf::from(file_path);
        let rob_bytes = std::fs::read(&file_path)?;
        let palette = Palette {
            colors: handle.palette().colors,
        };

        run_on_large_stack(move || {
            let rob_file = rob::parse_rob_file(&rob_bytes)?;

            let rob_header = RobHeader {
                segment_count: usize_to_i32(rob_file.segments.len(), "segment_count")?,
            };
            let mut out = Vec::new();
            out.extend_from_slice(bytemuck::bytes_of(&rob_header));

            for segment in &rob_file.segments {
                let (has_model, model_data) = if segment.has_embedded_3d_data() {
                    let model = segment.parse_embedded_3d_data()?;
                    (
                        1u8,
                        Some(serialize_model_3d(
                            &model,
                            Some(&palette),
                            Some(handle.texture_cache_mut()),
                        )?),
                    )
                } else {
                    (0u8, None)
                };

                let seg_header = RobSegmentHeader {
                    segment_name: segment.segment_name,
                    has_model,
                    _pad: [0; 3],
                    model_data_size: model_data
                        .as_ref()
                        .map(|d| usize_to_i32(d.len(), "model_data_size"))
                        .transpose()?
                        .unwrap_or(0),
                };
                out.extend_from_slice(bytemuck::bytes_of(&seg_header));

                if let Some(data) = &model_data {
                    out.extend_from_slice(data);
                }
            }

            Ok(out)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_sfx_effect_count(file_path: *const c_char) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let sfx_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = sfx::parse_sfx_file(&sfx_bytes)?;
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
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rtx_entry_count(file_path: *const c_char) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let rtx_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(&rtx_bytes)?;
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
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_sfx_to_wav(
    file_path: *const c_char,
    effect_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let sfx_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = sfx::parse_sfx_file(&sfx_bytes)?;
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
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_rtx_entry_to_wav(
    file_path: *const c_char,
    entry_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let rtx_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(&rtx_bytes)?;
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
/// `file_path` must be a valid null-terminated UTF-8 string.
/// The returned buffer contains UTF-8 subtitle text (no null terminator).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_rtx_subtitle(
    file_path: *const c_char,
    entry_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let rtx_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = rtx::parse_rtx_file(&rtx_bytes)?;
            let entry_idx = i32_to_usize(entry_index, "entry_index")?;
            let entry = parsed.entries.get(entry_idx).ok_or_else(|| {
                crate::error::Error::Parse(format!("entry_index out of range: {entry_index}"))
            })?;

            match entry {
                RtxEntry::Text { text, .. } => Ok(text.as_bytes().to_vec()),
                RtxEntry::Audio { .. } => Err(crate::error::Error::Parse(
                    "entry is audio, not text".to_string(),
                )),
            }
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_fnt_to_ttf(file_path: *const c_char) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let fnt_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let parsed = fnt::parse_fnt(&fnt_bytes)?;
            let ttf = fnt_ttf::build_ttf_from_fnt(&parsed, "RedguardFnt")?;
            Ok(ttf)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_gxa_frame_count(file_path: *const c_char) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let gxa_bytes = std::fs::read(file_path)?;
        let gxa_file = gxa::parse_gxa_file(&gxa_bytes)?;
        i32::try_from(gxa_file.frames.len()).map_err(|_| {
            crate::error::Error::Parse(format!(
                "frame count exceeds i32::MAX: {}",
                gxa_file.frames.len()
            ))
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
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_gxa(file_path: *const c_char, frame: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let gxa_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let gxa_file = gxa::parse_gxa_file(&gxa_bytes)?;
            let frame_idx = i32_to_usize(frame, "frame")?;
            let selected = gxa_file.frames.get(frame_idx).ok_or_else(|| {
                crate::error::Error::Parse(format!(
                    "frame out of range: {frame} (frame_count={})",
                    gxa_file.frames.len()
                ))
            })?;

            let header = TextureHeader {
                width: i32::from(selected.width),
                height: i32::from(selected.height),
                frame_count: 1,
                rgba_size: usize_to_i32(selected.rgba.len(), "rgba_size")?,
            };
            let mut out = Vec::with_capacity(size_of::<TextureHeader>() + selected.rgba.len());
            out.extend_from_slice(bytemuck::bytes_of(&header));
            out.extend_from_slice(&selected.rgba);
            Ok(out)
        })
    })();

    into_ffi_result(result)
}

pub(crate) fn serialize_terrain_primitives(
    primitives: Vec<crate::gltf::UnrolledPrimitive>,
) -> crate::Result<Vec<u8>> {
    let total_vertex_count: usize = primitives.iter().map(|p| p.positions.len()).sum();
    let total_index_count: usize = primitives.iter().map(|p| p.indices.len()).sum();

    let header = RgmdHeader {
        magic: *b"RGMD",
        version: [1, 0, 0, 0],
        submesh_count: usize_to_i32(primitives.len(), "submesh_count")?,
        frame_count: 0,
        total_vertex_count: usize_to_i32(total_vertex_count, "total_vertex_count")?,
        total_index_count: usize_to_i32(total_index_count, "total_index_count")?,
        radius: 0.0,
    };

    let estimated_size = size_of::<RgmdHeader>()
        + primitives.len() * size_of::<RgmdSubmeshHeader>()
        + total_vertex_count * size_of::<RgmdVertex>()
        + total_index_count * size_of::<u32>();
    let mut out = Vec::with_capacity(estimated_size);
    out.extend_from_slice(bytemuck::bytes_of(&header));

    for primitive in &primitives {
        let (textured, color_rgb, texture_id, image_id) = match primitive.material_key {
            MaterialKey::Textured(tid, iid) => (1u8, [0u8; 3], tid, iid),
            MaterialKey::SolidColor(rgb) | MaterialKey::PaletteTexture(rgb) => {
                (0u8, rgb, 0u16, 0u8)
            }
            MaterialKey::White => (0u8, [255u8; 3], 0u16, 0u8),
        };

        let sub_header = RgmdSubmeshHeader {
            textured,
            color_r: color_rgb[0],
            color_g: color_rgb[1],
            color_b: color_rgb[2],
            texture_id,
            image_id,
            _pad: 0,
            vertex_count: usize_to_i32(primitive.positions.len(), "submesh_vertex_count")?,
            index_count: usize_to_i32(primitive.indices.len(), "submesh_index_count")?,
        };
        out.extend_from_slice(bytemuck::bytes_of(&sub_header));

        let vertices: Vec<RgmdVertex> = primitive
            .positions
            .iter()
            .zip(&primitive.normals)
            .zip(&primitive.uvs)
            .map(|((pos, norm), uv)| RgmdVertex {
                position: *pos,
                normal: *norm,
                uv: *uv,
            })
            .collect();
        out.extend_from_slice(bytemuck::cast_slice::<RgmdVertex, u8>(&vertices));
        out.extend_from_slice(bytemuck::cast_slice::<u32, u8>(&primitive.indices));
    }

    Ok(out)
}

pub(crate) fn serialize_rgm_placements(
    placements: &[rgm::Placement],
    lights: &[rgm::PositionedLight],
) -> crate::Result<Vec<u8>> {
    let header = RgplHeader {
        magic: *b"RGPL",
        placement_count: usize_to_i32(placements.len(), "placement_count")?,
        light_count: usize_to_i32(lights.len(), "light_count")?,
    };

    let estimated_size = size_of::<RgplHeader>()
        + placements.len() * size_of::<RgplPlacement>()
        + lights.len() * size_of::<RgplLight>();
    let mut out = Vec::with_capacity(estimated_size);
    out.extend_from_slice(bytemuck::bytes_of(&header));

    for p in placements {
        let placement = RgplPlacement {
            model_name: fixed_string::<32>(&p.model_name),
            source_id: fixed_string::<32>(&p.source_id),
            transform: p.transform,
            texture_id: p.texture_id,
            image_id: p.image_id,
            object_type: p.object_type as u8,
        };
        out.extend_from_slice(bytemuck::bytes_of(&placement));
    }

    for light in lights {
        let l = RgplLight {
            name: fixed_string::<32>(&light.name),
            color: light.color,
            position: light.position,
            range: light.range,
            light_type: 0,
            _pad: [0; 3],
        };
        out.extend_from_slice(bytemuck::bytes_of(&l));
    }

    Ok(out)
}

/// # Safety
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_wld_terrain_data(file_path: *const c_char) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let wld_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let wld_file = wld::parse_wld_file(&wld_bytes)?;
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
/// `file_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_rgm_placements(file_path: *const c_char) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let rgm_bytes = std::fs::read(file_path)?;
        run_on_large_stack(move || {
            let (placements, lights) = rgm::extract_rgm_placements(&rgm_bytes)?;
            serialize_rgm_placements(&placements, &lights)
        })
    })();

    into_ffi_result(result)
}

pub(crate) fn scan_rgm_sections<'a>(data: &'a [u8], target_tag: &[u8; 4]) -> Vec<&'a [u8]> {
    let mut results = Vec::new();
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let tag = &data[offset..offset + 4];
        let length = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap_or_default())
            as usize;
        let payload_start = offset + 8;
        let payload_end = (payload_start + length).min(data.len());
        if tag == target_tag {
            results.push(&data[payload_start..payload_end]);
        }
        offset = payload_end;
        if tag == b"END " {
            break;
        }
    }
    results
}
