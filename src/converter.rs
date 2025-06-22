//! GLTF conversion functionality
//!
//! This module provides functions to convert 3D model files to GLTF format.

use crate::{ParseResult, model3d::Model3DFile};
use gltf::binary::{Glb, Header};
use gltf_json as json;
use json::validation::{Checked, USize64};
use json::{
    Asset, Index, Root, Scene,
    accessor::{Accessor, ComponentType, GenericComponentType, Type},
    buffer::{Buffer, Target, View},
    mesh::{Mesh, Primitive, Semantic},
    scene::Node,
};
use log::debug;
use serde_json::Value;
use std::collections::BTreeMap;

/// Convert one or more 3D models to a GLTF `Root` and a binary buffer.
pub fn convert_models_to_gltf(models: &[Model3DFile]) -> ParseResult<(Root, Vec<u8>)> {
    if models.is_empty() {
        return Err("No models to convert".to_string());
    }

    debug!("Converting {} models to GLTF", models.len());

    let mut buffer_data = Vec::new();
    let mut accessors = Vec::new();
    let mut buffer_views = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();

    for (model_index, model) in models.iter().enumerate() {
        debug!(
            "Processing model {}: {} vertices, {} faces",
            model_index,
            model.vertex_coords.len(),
            model.face_data.len()
        );

        // A model must have vertices and faces to be valid.
        if model.vertex_coords.is_empty() {
            debug!("Model {}: Skipping - no vertices", model_index);
            continue;
        }

        let mut indices = Vec::new();
        let vertex_limit = model.vertex_coords.len() as u32;
        let mut valid_faces = 0;

        for (face_index, face) in model.face_data.iter().enumerate() {
            if face.face_vertices.len() >= 3 {
                valid_faces += 1;
                let first_vertex = face.face_vertices[0].vertex_index;
                for i in 1..(face.face_vertices.len() - 1) {
                    let v1 = first_vertex;
                    let v2 = face.face_vertices[i].vertex_index;
                    let v3 = face.face_vertices[i + 1].vertex_index;

                    // Skip this triangle if any referenced vertex index is out of bounds.
                    if v1 < vertex_limit && v2 < vertex_limit && v3 < vertex_limit {
                        indices.push(v1);
                        indices.push(v2);
                        indices.push(v3);
                    } else {
                        debug!(
                            "Model {} face {}: Skipping triangle with out-of-bounds indices: {}, {}, {}",
                            model_index, face_index, v1, v2, v3
                        );
                    }
                }
            } else {
                debug!(
                    "Model {} face {}: Skipping face with {} vertices (need >= 3)",
                    model_index,
                    face_index,
                    face.face_vertices.len()
                );
            }
        }

        debug!(
            "Model {}: {} valid faces, {} triangles generated",
            model_index,
            valid_faces,
            indices.len() / 3
        );

        if indices.is_empty() {
            debug!("Model {}: Skipping - no valid triangles", model_index);
            continue;
        }

        // Calculate min/max for vertex positions, replacing NaN with 0.0 during calculation.
        let (min_x, max_x, min_y, max_y, min_z, max_z) = model.vertex_coords.iter().fold(
            (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ),
            |(min_x, max_x, min_y, max_y, min_z, max_z), v| {
                let x = if v.x.is_nan() { 0.0 } else { v.x };
                let y = if v.y.is_nan() { 0.0 } else { v.y };
                let z = if v.z.is_nan() { 0.0 } else { v.z };
                (
                    min_x.min(x),
                    max_x.max(x),
                    min_y.min(y),
                    max_y.max(y),
                    min_z.min(z),
                    max_z.max(z),
                )
            },
        );

        debug!(
            "Model {}: Bounding box: ({:.2}, {:.2}, {:.2}) to ({:.2}, {:.2}, {:.2})",
            model_index, min_x, min_y, min_z, max_x, max_y, max_z
        );

        // Vertices
        let vertex_buffer_offset = buffer_data.len();
        let mut vertex_buffer = Vec::new();
        for vertex in &model.vertex_coords {
            // Replace NaN with 0.0 to ensure valid float values.
            let x = if vertex.x.is_nan() { 0.0 } else { vertex.x };
            let y = if vertex.y.is_nan() { 0.0 } else { vertex.y };
            let z = if vertex.z.is_nan() { 0.0 } else { vertex.z };
            vertex_buffer.extend_from_slice(&x.to_le_bytes());
            vertex_buffer.extend_from_slice(&y.to_le_bytes());
            vertex_buffer.extend_from_slice(&z.to_le_bytes());
        }
        buffer_data.extend_from_slice(&vertex_buffer);

        let vertex_buffer_view_index = buffer_views.len();
        buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(vertex_buffer_offset as u64)),
            byte_length: USize64(vertex_buffer.len() as u64),
            byte_stride: None,
            name: None,
            target: Some(Checked::Valid(Target::ArrayBuffer)),
            extensions: Default::default(),
            extras: Default::default(),
        });

        let vertex_accessor_index = accessors.len();
        accessors.push(Accessor {
            buffer_view: Some(Index::new(vertex_buffer_view_index as u32)),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
            count: USize64(model.vertex_coords.len() as u64),
            type_: Checked::Valid(Type::Vec3),
            min: Some(Value::Array(vec![
                Value::from(min_x),
                Value::from(min_y),
                Value::from(min_z),
            ])),
            max: Some(Value::Array(vec![
                Value::from(max_x),
                Value::from(max_y),
                Value::from(max_z),
            ])),
            name: None,
            normalized: false,
            sparse: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        // Indices
        let index_buffer_offset = buffer_data.len();
        let mut index_buffer = Vec::new();
        for &index in &indices {
            index_buffer.extend_from_slice(&(index as u32).to_le_bytes());
        }
        buffer_data.extend_from_slice(&index_buffer);

        let index_buffer_view_index = buffer_views.len();
        buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(index_buffer_offset as u64)),
            byte_length: USize64(index_buffer.len() as u64),
            target: Some(Checked::Valid(Target::ElementArrayBuffer)),
            byte_stride: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        let index_accessor_index = accessors.len();
        accessors.push(Accessor {
            buffer_view: Some(Index::new(index_buffer_view_index as u32)),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(ComponentType::U32)),
            count: USize64(indices.len() as u64),
            type_: Checked::Valid(Type::Scalar),
            max: None,
            min: None,
            name: None,
            normalized: false,
            sparse: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        // Mesh
        let mesh_index = meshes.len();
        let mut attributes = BTreeMap::new();
        attributes.insert(
            Checked::Valid(Semantic::Positions),
            Index::new(vertex_accessor_index as u32),
        );

        meshes.push(Mesh {
            primitives: vec![Primitive {
                attributes,
                indices: Some(Index::new(index_accessor_index as u32)),
                material: None,
                mode: Checked::Valid(json::mesh::Mode::Triangles),
                targets: None,
                extensions: Default::default(),
                extras: Default::default(),
            }],
            weights: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        // Node
        nodes.push(Node {
            mesh: Some(Index::new(mesh_index as u32)),
            camera: None,
            children: None,
            skin: None,
            matrix: None,
            rotation: None,
            scale: None,
            translation: None,
            weights: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        debug!("Model {}: Successfully converted to mesh", model_index);
    }

    debug!(
        "Conversion complete: {} meshes, {} nodes, buffer size: {} bytes",
        meshes.len(),
        nodes.len(),
        buffer_data.len()
    );

    let root = Root {
        asset: Asset {
            version: "2.0".to_string(),
            generator: Some("redguard-preservation".to_string()),
            copyright: None,
            min_version: None,
            extensions: Default::default(),
            extras: Default::default(),
        },
        scene: Some(Index::new(0)),
        scenes: vec![Scene {
            nodes: (0..nodes.len()).map(|i| Index::new(i as u32)).collect(),
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        }],
        nodes,
        meshes,
        accessors,
        buffer_views,
        buffers: vec![Buffer {
            uri: None,
            byte_length: USize64(buffer_data.len() as u64),
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        }],
        animations: Default::default(),
        cameras: Default::default(),
        images: Default::default(),
        materials: Default::default(),
        samplers: Default::default(),
        skins: Default::default(),
        textures: Default::default(),
        extensions_used: Default::default(),
        extensions_required: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
    };

    Ok((root, buffer_data))
}

/// Serializes a `Root` struct and its binary buffer into a GLB byte vector.
pub fn to_glb(root: &Root, buffer: &[u8]) -> Result<Vec<u8>, serde_json::Error> {
    let json_string = serde_json::to_string(root)?;
    let glb = Glb {
        header: Header {
            magic: *b"glTF",
            version: 2,
            length: 0, // This will be calculated by to_vec
        },
        json: json_string.as_bytes().into(),
        bin: Some(buffer.into()),
    };
    Ok(glb.to_vec().unwrap())
}
