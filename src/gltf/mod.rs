// Struct updates on gltf-json extension types are needed when a downstream crate
// enables gltf-json's `extensions` feature, which adds fields.
#![allow(clippy::needless_update)]
mod builder;
mod primitives;
mod terrain;
mod texture_cache;

use builder::GltfBuilder;
use primitives::build_unrolled_primitives;
pub(crate) use primitives::{MaterialKey, UnrolledPrimitive};
pub(crate) use terrain::build_wld_unrolled_primitives;
pub use texture_cache::TextureCache;

use crate::{
    Result,
    import::{palette::Palette, rgm::PositionedLight},
    model3d::Model3DFile,
};
use gltf::binary::{Glb, Header};
use gltf_json as json;
use gltf_json::extensions::scene::khr_lights_punctual;
use json::scene::Node;
use json::validation::Checked;
use json::{Index, Root};
use log::trace;
use rayon::prelude::*;
use std::collections::HashMap;

/// Engine-to-export coordinate divisor. Engine coordinates are 20x export units.
pub(crate) const ENGINE_UNIT_SCALE: f32 = 20.0;

const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn non_identity_matrix(matrix: [f32; 16]) -> Option<[f32; 16]> {
    if matrix == IDENTITY_MATRIX {
        None
    } else {
        Some(matrix)
    }
}

/// UV coordinates in the 3D format are stored as i16 values in 4-bit fixed-point
/// (1/16th pixel precision). The engine multiplies raw values by 1/16.0 to get
/// pixel-space texture coordinates, then scales by texture dimensions for rendering.
/// For GLB export, we normalize to 0..1 by dividing raw values by (`texture_dim` × 16).
pub(crate) const UV_FIXED_POINT_SCALE: f32 = 16.0;

#[allow(clippy::cast_possible_truncation)]
// GLTF indices are u32; generated mesh/light indices are far below u32::MAX.
const fn index_u32(value: usize) -> u32 {
    value as u32
}

#[allow(clippy::missing_errors_doc)]
pub fn convert_models_to_gltf(
    models: &[Model3DFile],
    palette: Option<&Palette>,
    texture_cache: Option<&mut TextureCache>,
    compress_textures: bool,
) -> Result<(Root, Vec<u8>)> {
    if models.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No models to convert".to_string(),
        ));
    }

    trace!("Converting {} models to GLTF", models.len());

    let mut builder = GltfBuilder::new(texture_cache, compress_textures);
    let texture_cache_available = builder.has_texture_cache();

    let all_unrolled: Vec<_> = models
        .par_iter()
        .map(|model| build_unrolled_primitives(model, palette, texture_cache_available))
        .collect();

    for unrolled in all_unrolled {
        if unrolled.is_empty() {
            continue;
        }
        let mesh_index = builder.append_mesh(unrolled);
        builder.add_node(Node {
            mesh: Some(Index::new(index_u32(mesh_index))),
            ..Default::default()
        });
    }

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No valid models found to convert".to_string(),
        ));
    }

    Ok(builder.finish())
}

#[allow(clippy::missing_errors_doc)]
pub fn to_glb(root: &Root, buffer: &[u8]) -> Result<Vec<u8>> {
    let json_string = serde_json::to_string(root)?;
    let glb = Glb {
        header: Header {
            magic: *b"glTF",
            version: 2,
            length: 0,
        },
        json: json_string.as_bytes().into(),
        bin: Some(buffer.into()),
    };
    Ok(glb.to_vec()?)
}

/// Adds positioned models to the builder with mesh deduplication and instance caching.
fn add_positioned_models(
    builder: &mut GltfBuilder,
    positioned_models: &[crate::import::rgm::PositionedModel],
    palette: Option<&Palette>,
    texture_cache_available: bool,
) {
    let mut unique_source_models: HashMap<&str, &crate::model3d::Model3DFile> = HashMap::new();
    for pm in positioned_models {
        if let Some(source_id) = &pm.source_id {
            unique_source_models.entry(source_id).or_insert(&pm.model);
        }
    }

    let unrolled_cache: HashMap<&str, Vec<UnrolledPrimitive>> = unique_source_models
        .par_iter()
        .map(|(&id, model)| {
            (
                id,
                build_unrolled_primitives(model, palette, texture_cache_available),
            )
        })
        .collect();

    let mut mesh_instance_cache: HashMap<&str, u32> = HashMap::new();
    for pm in positioned_models {
        let mut metadata = serde_json::json!({
            "rgpre_render": { "version": 1, "rob_attributes": pm.render_attributes }
        });
        if let Some(flat) = pm.flat {
            metadata["rgpre_flat"] = builder.flat_metadata(flat);
        }
        let extras = Some(
            serde_json::value::to_raw_value(&metadata).expect("render metadata is serializable"),
        );
        if let Some(source_id) = &pm.source_id
            && let Some(&cached_mesh) = mesh_instance_cache.get(source_id.as_str())
        {
            builder.add_node(Node {
                mesh: Some(Index::new(cached_mesh)),
                matrix: non_identity_matrix(pm.transform),
                name: Some(pm.model_name.clone()),
                extras,
                ..Default::default()
            });
            continue;
        }

        let unrolled = pm.source_id.as_ref().map_or_else(
            || build_unrolled_primitives(&pm.model, palette, texture_cache_available),
            |source_id| {
                unrolled_cache
                    .get(source_id.as_str())
                    .cloned()
                    .unwrap_or_default()
            },
        );

        if unrolled.is_empty() {
            builder.add_node(Node {
                matrix: non_identity_matrix(pm.transform),
                name: Some(pm.model_name.clone()),
                extras,
                ..Default::default()
            });
            continue;
        }

        let mesh_index = builder.append_mesh(unrolled);
        if let Some(source_id) = &pm.source_id {
            mesh_instance_cache.insert(source_id, index_u32(mesh_index));
        }
        builder.add_node(Node {
            mesh: Some(Index::new(index_u32(mesh_index))),
            matrix: non_identity_matrix(pm.transform),
            name: Some(pm.model_name.clone()),
            extras,
            ..Default::default()
        });
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn convert_positioned_models_to_gltf(
    positioned_models: &[crate::import::rgm::PositionedModel],
    lights: &[PositionedLight],
    palette: Option<&Palette>,
    texture_cache: Option<&mut TextureCache>,
    compress_textures: bool,
) -> Result<(Root, Vec<u8>)> {
    if positioned_models.is_empty() && lights.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No positioned models or lights to convert".to_string(),
        ));
    }

    trace!(
        "Converting {} positioned models to GLTF",
        positioned_models.len()
    );

    let mut builder = GltfBuilder::new(texture_cache, compress_textures);
    let texture_cache_available = builder.has_texture_cache();

    add_positioned_models(
        &mut builder,
        positioned_models,
        palette,
        texture_cache_available,
    );

    let mut light_definitions = Vec::new();
    for (light_index, light) in lights.iter().enumerate() {
        light_definitions.push(khr_lights_punctual::Light {
            color: light.color,
            intensity: 1.0,
            range: Some(light.range),
            type_: Checked::Valid(khr_lights_punctual::Type::Point),
            name: Some(light.name.clone()),
            spot: None,
            extensions: None,
            extras: Default::default(),
        });

        builder.add_node(Node {
            translation: Some(light.position),
            name: Some(light.name.clone()),
            extensions: Some(json::extensions::scene::Node {
                khr_lights_punctual: Some(khr_lights_punctual::KhrLightsPunctual {
                    light: Index::new(index_u32(light_index)),
                }),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No valid positioned models found to convert".to_string(),
        ));
    }

    Ok(builder.finish_with_lights(light_definitions))
}

#[allow(clippy::missing_errors_doc)]
pub fn convert_wld_scene_to_gltf(
    wld_file: &crate::import::wld::WldFile,
    texbsi_id: u16,
    positioned_models: &[crate::import::rgm::PositionedModel],
    palette: Option<&Palette>,
    texture_cache: Option<&mut TextureCache>,
    compress_textures: bool,
) -> Result<(Root, Vec<u8>)> {
    let mut builder = GltfBuilder::new(texture_cache, compress_textures);
    let texture_cache_available = builder.has_texture_cache();

    let terrain_primitives = build_wld_unrolled_primitives(wld_file, texbsi_id)?;
    if !terrain_primitives.is_empty() {
        let mesh_index = builder.append_mesh(terrain_primitives);
        builder.add_node(Node {
            mesh: Some(Index::new(index_u32(mesh_index))),
            name: Some("Terrain".to_string()),
            ..Default::default()
        });
    }

    add_positioned_models(
        &mut builder,
        positioned_models,
        palette,
        texture_cache_available,
    );

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No terrain or positioned models found to convert".to_string(),
        ));
    }

    Ok(builder.finish())
}

/// Converts decoded model positions (model units) to the scene coordinates used by GLB exports.
#[must_use]
pub fn model_position(position: [f32; 3]) -> [f32; 3] {
    crate::geometry::transform_position(
        position[0],
        position[1],
        position[2],
        ENGINE_UNIT_SCALE,
        crate::geometry::SCENE_CONVENTION,
    )
}

/// Converts decoded model normals to the axis convention used by GLB exports, without normalization.
#[must_use]
pub fn model_normal(normal: [f32; 3]) -> [f32; 3] {
    crate::geometry::transform_normal(
        normal[0],
        normal[1],
        normal[2],
        crate::geometry::SCENE_CONVENTION,
    )
}

/// Returns a WLD grid vertex in GLB scene coordinates, with the stored height decoded.
/// Coordinates use the combined 256-by-256 map's row-major column and row indices.
pub fn wld_grid_position(
    wld: &crate::import::wld::WldFile,
    column: usize,
    row: usize,
) -> crate::Result<[f32; 3]> {
    let side = crate::import::wld::WLD_MAP_SIDE;
    if column >= side * 2 || row >= side * 2 {
        return Err(crate::error::Error::Conversion(format!(
            "WLD grid vertex ({column}, {row}) outside 256-by-256 grid"
        )));
    }
    let section = column / side + (row / side) * 2;
    let height = wld.sections[section].maps[0][column % side + (row % side) * side];
    Ok([
        -(column as f32) * terrain::WLD_SIZE_SCALE,
        terrain::wld_height(height),
        -(row as f32) * terrain::WLD_SIZE_SCALE,
    ])
}

#[cfg(test)]
mod inspection_tests {
    #[test]
    fn model_inspection_uses_export_axes_and_units() {
        assert_eq!(super::model_position([20.0, -40.0, 60.0]), [-1.0, 2.0, 3.0]);
        assert_eq!(super::model_normal([1.0, -2.0, 3.0]), [-1.0, 2.0, 3.0]);
    }
}
