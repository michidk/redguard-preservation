use super::buffer::*;
use super::{i32_to_usize, read_bytes};
use crate::import::{bsi, palette::Palette, rob};
use crate::model3d::{self, Model3DFile, TextureData};
use std::collections::BTreeMap;

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

/// # Safety
///
/// `data` must point to readable bytes of length `len`.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_parse_model_data(data: *const u8, len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let slice = unsafe { read_bytes(data, len, "data") }?;
        let model = model3d::parse_3d_file(slice)?;
        serialize_model_3d(&model)
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
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texbsi_data` and `palette_data` must point to readable bytes for their lengths.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture(
    texbsi_data: *const u8,
    texbsi_len: i32,
    palette_data: *const u8,
    palette_len: i32,
    image_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let texbsi_slice = unsafe { read_bytes(texbsi_data, texbsi_len, "texbsi_data") }?;
        let palette_slice = unsafe { read_bytes(palette_data, palette_len, "palette_data") }?;
        let bsi_file = bsi::parse_bsi_file(texbsi_slice)?;
        let palette = Palette::parse(palette_slice)?;
        let image_idx = i32_to_usize(image_index, "image_index")?;
        let image = bsi_file.images.get(image_idx).ok_or_else(|| {
            crate::error::Error::Parse(format!("image_index out of range: {image_index}"))
        })?;

        let rgba = image.decode_rgba(Some(&palette));
        let mut out = Vec::new();
        out.extend_from_slice(&i32::from(image.width).to_le_bytes());
        out.extend_from_slice(&i32::from(image.height).to_le_bytes());
        out.extend_from_slice(&i32::from(image.frame_count).to_le_bytes());
        out.extend_from_slice(&usize_to_i32(rgba.len(), "rgba_size")?.to_le_bytes());
        out.extend_from_slice(&rgba);
        Ok(out)
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texbsi_data` and `palette_data` must point to readable bytes for their lengths.
/// The returned pointer must be released with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture_all_frames(
    texbsi_data: *const u8,
    texbsi_len: i32,
    palette_data: *const u8,
    palette_len: i32,
    image_index: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let texbsi_slice = unsafe { read_bytes(texbsi_data, texbsi_len, "texbsi_data") }?;
        let palette_slice = unsafe { read_bytes(palette_data, palette_len, "palette_data") }?;
        let bsi_file = bsi::parse_bsi_file(texbsi_slice)?;
        let palette = Palette::parse(palette_slice)?;
        let image_idx = i32_to_usize(image_index, "image_index")?;
        let image = bsi_file.images.get(image_idx).ok_or_else(|| {
            crate::error::Error::Parse(format!("image_index out of range: {image_index}"))
        })?;

        let mut out = Vec::new();
        out.extend_from_slice(&i32::from(image.width).to_le_bytes());
        out.extend_from_slice(&i32::from(image.height).to_le_bytes());
        out.extend_from_slice(&i32::from(image.frame_count).to_le_bytes());

        for frame_idx in 0..usize::from(image.frame_count) {
            match image.decode_frame_rgba(frame_idx, Some(&palette)) {
                Some(rgba) => {
                    out.extend_from_slice(&usize_to_i32(rgba.len(), "rgba_size")?.to_le_bytes());
                    out.extend_from_slice(&rgba);
                }
                None => {
                    out.extend_from_slice(&0_i32.to_le_bytes());
                }
            }
        }

        Ok(out)
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `texbsi_data` must point to readable bytes of length `texbsi_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_texbsi_image_count(texbsi_data: *const u8, texbsi_len: i32) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let texbsi_slice = unsafe { read_bytes(texbsi_data, texbsi_len, "texbsi_data") }?;
        let bsi_file = bsi::parse_bsi_file(texbsi_slice)?;
        usize_to_i32(bsi_file.images.len(), "image_count")
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
