use crate::{
    geometry::{
        SCENE_CONVENTION, resolve_vertex_normal, transform_normal, transform_position,
        triangle_vertex_offsets,
    },
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
    TerrainTextured(u16, u8),
    White,
}

#[derive(Clone)]
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

#[must_use]
pub(crate) fn material_for_face(
    texture_data: &TextureData,
    palette: Option<&Palette>,
    texture_cache_available: bool,
) -> (MaterialKey, [f32; 4]) {
    match texture_data {
        TextureData::SolidColor(index) => {
            let rgb_u8 = palette.map_or([128, 128, 128], |pal| pal.colors[usize::from(*index)]);
            let face_color = [
                f32::from(rgb_u8[0]) / 255.0,
                f32::from(rgb_u8[1]) / 255.0,
                f32::from(rgb_u8[2]) / 255.0,
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

#[allow(clippy::cast_possible_truncation)]
// GLTF indices are u32; generated vertex/index counts remain far below u32::MAX.
#[must_use]
pub(crate) fn build_unrolled_primitives(
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
            transform_normal(fn_.x, fn_.y, fn_.z, SCENE_CONVENTION)
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

        for i in 1..(face.face_vertices.len() - 1) {
            let tri_fv_indices = triangle_vertex_offsets(i);
            let tri_fv = [
                &face.face_vertices[tri_fv_indices[0]],
                &face.face_vertices[tri_fv_indices[1]],
                &face.face_vertices[tri_fv_indices[2]],
            ];
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
                let [x, y, z] =
                    transform_position(pos.x, pos.y, pos.z, ENGINE_UNIT_SCALE, SCENE_CONVENTION);
                group.positions.push([x, y, z]);

                group.min[0] = group.min[0].min(x);
                group.min[1] = group.min[1].min(y);
                group.min[2] = group.min[2].min(z);
                group.max[0] = group.max[0].max(x);
                group.max[1] = group.max[1].max(y);
                group.max[2] = group.max[2].max(z);

                let normal = resolve_vertex_normal(
                    model,
                    idx,
                    cumulative_fv_base + fv_idx,
                    face_normal,
                    SCENE_CONVENTION,
                );
                group.normals.push(normal);

                group.uvs.push([f32::from(fv.u), f32::from(fv.v)]);
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
