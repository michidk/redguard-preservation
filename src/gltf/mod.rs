mod builder;
mod primitives;
mod terrain;
mod texture_cache;

use builder::GltfBuilder;
use primitives::{MaterialKey, UnrolledPrimitive, build_unrolled_primitives};
use terrain::build_wld_unrolled_primitives;
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
use std::collections::HashMap;

/// Engine-to-export coordinate divisor. Engine coordinates are 20x export units.
const ENGINE_UNIT_SCALE: f32 = 20.0;

/// UV coordinates in the 3D format are stored as i16 values in 4-bit fixed-point
/// (1/16th pixel precision). The engine multiplies raw values by 1/16.0 to get
/// pixel-space texture coordinates, then scales by texture dimensions for rendering.
/// For GLB export, we normalize to 0..1 by dividing raw values by (texture_dim × 16).
pub(super) const UV_FIXED_POINT_SCALE: f32 = 16.0;

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

    for model in models {
        let unrolled = build_unrolled_primitives(model, palette, texture_cache_available);
        if unrolled.is_empty() {
            continue;
        }
        let mesh_index = builder.append_mesh(unrolled);
        builder.add_node(Node {
            mesh: Some(Index::new(mesh_index as u32)),
            ..Default::default()
        });
    }

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No valid models found to convert".to_string(),
        ));
    }

    builder.finish()
}

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
    let mut mesh_instance_cache: HashMap<String, u32> = HashMap::new();

    for pm in positioned_models {
        if let Some(source_id) = &pm.source_id
            && let Some(&cached_mesh) = mesh_instance_cache.get(source_id)
        {
            builder.add_node(Node {
                mesh: Some(Index::new(cached_mesh)),
                matrix: Some(pm.transform),
                name: Some(pm.model_name.clone()),
                ..Default::default()
            });
            continue;
        }

        let unrolled = build_unrolled_primitives(&pm.model, palette, texture_cache_available);
        if unrolled.is_empty() {
            builder.add_node(Node {
                matrix: Some(pm.transform),
                name: Some(pm.model_name.clone()),
                ..Default::default()
            });
            continue;
        }

        let mesh_index = builder.append_mesh(unrolled);
        if let Some(source_id) = &pm.source_id {
            mesh_instance_cache.insert(source_id.clone(), mesh_index as u32);
        }

        builder.add_node(Node {
            mesh: Some(Index::new(mesh_index as u32)),
            matrix: Some(pm.transform),
            name: Some(pm.model_name.clone()),
            ..Default::default()
        });
    }

    let mut light_definitions = Vec::new();
    for (light_index, light) in lights.iter().enumerate() {
        light_definitions.push(khr_lights_punctual::Light {
            color: light.color,
            intensity: 1.0,
            range: Some(light.range),
            type_: Checked::Valid(khr_lights_punctual::Type::Point),
            name: Some(light.name.clone()),
            spot: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        builder.add_node(Node {
            translation: Some(light.position),
            name: Some(light.name.clone()),
            extensions: Some(json::extensions::scene::Node {
                khr_lights_punctual: Some(khr_lights_punctual::KhrLightsPunctual {
                    light: Index::new(light_index as u32),
                }),
            }),
            ..Default::default()
        });
    }

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No valid positioned models found to convert".to_string(),
        ));
    }

    builder.finish_with_lights(light_definitions)
}

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
            mesh: Some(Index::new(mesh_index as u32)),
            matrix: Some([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            name: Some("Terrain".to_string()),
            ..Default::default()
        });
    }

    for pm in positioned_models {
        let unrolled = build_unrolled_primitives(&pm.model, palette, texture_cache_available);
        if unrolled.is_empty() {
            builder.add_node(Node {
                matrix: Some(pm.transform),
                name: Some(pm.model_name.clone()),
                ..Default::default()
            });
            continue;
        }

        let mesh_index = builder.append_mesh(unrolled);
        builder.add_node(Node {
            mesh: Some(Index::new(mesh_index as u32)),
            matrix: Some(pm.transform),
            name: Some(pm.model_name.clone()),
            ..Default::default()
        });
    }

    if builder.nodes.is_empty() {
        return Err(crate::error::Error::Conversion(
            "No terrain or positioned models found to convert".to_string(),
        ));
    }

    builder.finish()
}
