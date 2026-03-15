use crate::{
    import::palette::Palette,
    model3d::{Model3DFile, TextureData},
};
use std::collections::BTreeMap;

use super::ENGINE_UNIT_SCALE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum MaterialKey {
    SolidColor([u8; 3]),
    PaletteTexture([u8; 3]),
    Textured(u16, u8),
    White,
}

pub(crate) struct UnrolledPrimitive {
    pub(crate) material_key: MaterialKey,
    pub(crate) scale_uv_by_texture_dimensions: bool,
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) normals: Vec<[f32; 3]>,
    pub(crate) uvs: Vec<[f32; 2]>,
    pub(crate) indices: Vec<u32>,
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

pub(crate) fn sanitize_f32(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value }
}

pub(crate) fn material_for_face(
    texture_data: &TextureData,
    palette: Option<&Palette>,
    texture_cache_available: bool,
) -> (MaterialKey, [f32; 4]) {
    match texture_data {
        TextureData::SolidColor(index) => {
            let rgb_u8 = palette
                .map(|pal| pal.colors[*index as usize])
                .unwrap_or([128, 128, 128]);
            let face_color = [
                rgb_u8[0] as f32 / 255.0,
                rgb_u8[1] as f32 / 255.0,
                rgb_u8[2] as f32 / 255.0,
                1.0,
            ];
            let material = if texture_cache_available && palette.is_some() {
                MaterialKey::PaletteTexture(rgb_u8)
            } else {
                MaterialKey::SolidColor(rgb_u8)
            };
            (material, face_color)
        }
        TextureData::Texture {
            texture_id,
            image_id,
        } => {
            let material = if texture_cache_available {
                MaterialKey::Textured(*texture_id, *image_id)
            } else {
                MaterialKey::White
            };
            (material, [1.0, 1.0, 1.0, 1.0])
        }
    }
}

pub(crate) fn resolve_vertex_normal(
    model: &Model3DFile,
    vertex_index: usize,
    cumulative_fv_index: usize,
    face_normal: [f32; 3],
) -> [f32; 3] {
    let vn_index = if !model.normal_indices.is_empty() {
        model
            .normal_indices
            .get(cumulative_fv_index)
            .map(|&i| i as usize)
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

pub(super) fn build_unrolled_primitives(
    model: &Model3DFile,
    palette: Option<&Palette>,
    texture_cache_available: bool,
) -> Vec<UnrolledPrimitive> {
    if model.vertex_coords.is_empty() {
        return Vec::new();
    }

    let mut primitive_groups: BTreeMap<MaterialKey, UnrolledPrimitive> = BTreeMap::new();
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

        let (material_key, _) =
            material_for_face(&face.texture_data, palette, texture_cache_available);
        let group = primitive_groups
            .entry(material_key)
            .or_insert_with(|| UnrolledPrimitive {
                material_key,
                scale_uv_by_texture_dimensions: true,
                positions: Vec::new(),
                normals: Vec::new(),
                uvs: Vec::new(),
                indices: Vec::new(),
                min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
                max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
            });

        let v0 = &face.face_vertices[0];
        for i in 1..(face.face_vertices.len() - 1) {
            let v1 = &face.face_vertices[i];
            let v2 = &face.face_vertices[i + 1];

            let tri_fv = [v0, v1, v2];
            let tri_fv_indices = [0usize, i, i + 1];
            if tri_fv
                .iter()
                .any(|fv| fv.vertex_index as usize >= model.vertex_coords.len())
            {
                continue;
            }

            for (fv, &fv_idx) in tri_fv.iter().zip(&tri_fv_indices) {
                let idx = fv.vertex_index as usize;
                let pos = &model.vertex_coords[idx];
                let x = -sanitize_f32(pos.x) / ENGINE_UNIT_SCALE;
                let y = -sanitize_f32(pos.y) / ENGINE_UNIT_SCALE;
                let z = sanitize_f32(pos.z) / ENGINE_UNIT_SCALE;
                group.positions.push([x, y, z]);

                group.min[0] = group.min[0].min(x);
                group.min[1] = group.min[1].min(y);
                group.min[2] = group.min[2].min(z);
                group.max[0] = group.max[0].max(x);
                group.max[1] = group.max[1].max(y);
                group.max[2] = group.max[2].max(z);

                let normal =
                    resolve_vertex_normal(model, idx, cumulative_fv_base + fv_idx, face_normal);
                group.normals.push(normal);

                group.uvs.push([fv.u as f32, fv.v as f32]);
                group.indices.push((group.positions.len() - 1) as u32);
            }
        }

        cumulative_fv_base += face.face_vertices.len();
    }

    primitive_groups
        .into_values()
        .filter(|primitive| !primitive.indices.is_empty())
        .collect()
}
