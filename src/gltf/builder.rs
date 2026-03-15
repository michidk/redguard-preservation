use crate::Result;
use gltf_json as json;
use gltf_json::extensions::scene::khr_lights_punctual;
use json::validation::{Checked, USize64};
use json::{
    Asset, Index, Root, Scene,
    accessor::{Accessor, ComponentType, GenericComponentType, Type},
    buffer::{Buffer, Target, View},
    material::PbrMetallicRoughness,
    mesh::{Mesh, Primitive, Semantic},
    scene::Node,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

use super::texture_cache::create_palette_color_png;
use super::{MaterialKey, TextureCache, UV_FIXED_POINT_SCALE, UnrolledPrimitive};

pub(super) struct GltfBuilder<'a> {
    buffer_data: Vec<u8>,
    accessors: Vec<Accessor>,
    buffer_views: Vec<View>,
    meshes: Vec<Mesh>,
    pub(super) nodes: Vec<Node>,
    materials: Vec<json::Material>,
    textures: Vec<json::Texture>,
    images: Vec<json::Image>,
    samplers: Vec<json::texture::Sampler>,
    texture_cache: Option<&'a mut TextureCache>,
    solid_material_cache: HashMap<[u8; 3], usize>,
    palette_texture_material_cache: HashMap<[u8; 3], usize>,
    textured_material_cache: HashMap<(u16, u8), usize>,
    white_material_index: Option<usize>,
    palette_texture_index_cache: HashMap<[u8; 3], Option<usize>>,
    texture_index_cache: HashMap<(u16, u8), Option<(usize, bool)>>,
    nearest_sampler_index: Option<usize>,
    compress_textures: bool,
    texture_cache_available: bool,
}

impl<'a> GltfBuilder<'a> {
    pub(super) fn new(
        texture_cache: Option<&'a mut TextureCache>,
        compress_textures: bool,
    ) -> Self {
        let texture_cache_available = texture_cache.is_some();
        Self {
            buffer_data: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
            meshes: Vec::new(),
            nodes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            images: Vec::new(),
            samplers: Vec::new(),
            texture_cache,
            solid_material_cache: HashMap::new(),
            palette_texture_material_cache: HashMap::new(),
            textured_material_cache: HashMap::new(),
            white_material_index: None,
            palette_texture_index_cache: HashMap::new(),
            texture_index_cache: HashMap::new(),
            nearest_sampler_index: None,
            compress_textures,
            texture_cache_available,
        }
    }

    pub(super) fn has_texture_cache(&self) -> bool {
        self.texture_cache_available
    }

    pub(super) fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    fn align_buffer(&mut self, alignment: usize) {
        let padding = (alignment - (self.buffer_data.len() % alignment)) % alignment;
        self.buffer_data.extend(std::iter::repeat_n(0u8, padding));
    }

    fn push_vec3_accessor(
        &mut self,
        data: &[[f32; 3]],
        min: Option<[f32; 3]>,
        max: Option<[f32; 3]>,
    ) -> usize {
        self.align_buffer(4);
        let buffer_offset = self.buffer_data.len();
        let mut bytes = Vec::with_capacity(data.len() * 12);
        for item in data {
            bytes.extend_from_slice(&item[0].to_le_bytes());
            bytes.extend_from_slice(&item[1].to_le_bytes());
            bytes.extend_from_slice(&item[2].to_le_bytes());
        }
        self.buffer_data.extend_from_slice(&bytes);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(buffer_offset as u64)),
            byte_length: USize64(bytes.len() as u64),
            byte_stride: None,
            name: None,
            target: Some(Checked::Valid(Target::ArrayBuffer)),
            extensions: Default::default(),
            extras: Default::default(),
        });

        let accessor_index = self.accessors.len();
        self.accessors.push(Accessor {
            buffer_view: Some(Index::new(view_index as u32)),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
            count: USize64(data.len() as u64),
            type_: Checked::Valid(Type::Vec3),
            min: min.map(|v| {
                Value::Array(vec![
                    Value::from(v[0]),
                    Value::from(v[1]),
                    Value::from(v[2]),
                ])
            }),
            max: max.map(|v| {
                Value::Array(vec![
                    Value::from(v[0]),
                    Value::from(v[1]),
                    Value::from(v[2]),
                ])
            }),
            name: None,
            normalized: false,
            sparse: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        accessor_index
    }

    fn push_vec2_accessor(&mut self, data: &[[f32; 2]]) -> usize {
        self.align_buffer(4);
        let buffer_offset = self.buffer_data.len();
        let mut bytes = Vec::with_capacity(data.len() * 8);
        for item in data {
            bytes.extend_from_slice(&item[0].to_le_bytes());
            bytes.extend_from_slice(&item[1].to_le_bytes());
        }
        self.buffer_data.extend_from_slice(&bytes);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(buffer_offset as u64)),
            byte_length: USize64(bytes.len() as u64),
            byte_stride: None,
            name: None,
            target: Some(Checked::Valid(Target::ArrayBuffer)),
            extensions: Default::default(),
            extras: Default::default(),
        });

        let accessor_index = self.accessors.len();
        self.accessors.push(Accessor {
            buffer_view: Some(Index::new(view_index as u32)),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(ComponentType::F32)),
            count: USize64(data.len() as u64),
            type_: Checked::Valid(Type::Vec2),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        accessor_index
    }

    fn push_index_accessor(&mut self, indices: &[u32]) -> usize {
        self.align_buffer(4);
        let buffer_offset = self.buffer_data.len();
        let mut bytes = Vec::with_capacity(indices.len() * 4);
        for index in indices {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
        self.buffer_data.extend_from_slice(&bytes);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(buffer_offset as u64)),
            byte_length: USize64(bytes.len() as u64),
            target: Some(Checked::Valid(Target::ElementArrayBuffer)),
            byte_stride: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        let accessor_index = self.accessors.len();
        self.accessors.push(Accessor {
            buffer_view: Some(Index::new(view_index as u32)),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(ComponentType::U32)),
            count: USize64(indices.len() as u64),
            type_: Checked::Valid(Type::Scalar),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        accessor_index
    }

    fn push_blob_buffer_view(&mut self, data: &[u8]) -> usize {
        let buffer_offset = self.buffer_data.len();
        self.buffer_data.extend_from_slice(data);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(buffer_offset as u64)),
            byte_length: USize64(data.len() as u64),
            byte_stride: None,
            name: None,
            target: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        view_index
    }

    fn nearest_sampler(&mut self) -> usize {
        if let Some(index) = self.nearest_sampler_index {
            return index;
        }

        let index = self.samplers.len();
        self.samplers.push(json::texture::Sampler {
            mag_filter: Some(Checked::Valid(json::texture::MagFilter::Nearest)),
            min_filter: Some(Checked::Valid(json::texture::MinFilter::Nearest)),
            wrap_s: Checked::Valid(json::texture::WrappingMode::Repeat),
            wrap_t: Checked::Valid(json::texture::WrappingMode::Repeat),
            ..Default::default()
        });
        self.nearest_sampler_index = Some(index);
        index
    }

    fn create_solid_material(&self, rgb: [u8; 3]) -> json::Material {
        json::Material {
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_factor: json::material::PbrBaseColorFactor([
                    rgb[0] as f32 / 255.0,
                    rgb[1] as f32 / 255.0,
                    rgb[2] as f32 / 255.0,
                    1.0,
                ]),
                metallic_factor: json::material::StrengthFactor(0.0),
                roughness_factor: json::material::StrengthFactor(1.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_textured_material(&self, texture_index: usize, has_alpha: bool) -> json::Material {
        json::Material {
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_texture: Some(json::texture::Info {
                    index: Index::new(texture_index as u32),
                    tex_coord: 0,
                    extensions: Default::default(),
                    extras: Default::default(),
                }),
                metallic_factor: json::material::StrengthFactor(0.0),
                roughness_factor: json::material::StrengthFactor(1.0),
                ..Default::default()
            },
            alpha_mode: if has_alpha {
                Checked::Valid(json::material::AlphaMode::Mask)
            } else {
                Checked::Valid(json::material::AlphaMode::Opaque)
            },
            alpha_cutoff: if has_alpha {
                Some(json::material::AlphaCutoff(0.5))
            } else {
                None
            },
            double_sided: has_alpha,
            ..Default::default()
        }
    }

    fn create_white_material(&self) -> json::Material {
        json::Material {
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_factor: json::material::PbrBaseColorFactor([1.0, 1.0, 1.0, 1.0]),
                metallic_factor: json::material::StrengthFactor(0.0),
                roughness_factor: json::material::StrengthFactor(1.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn resolve_white_material(&mut self) -> usize {
        if let Some(index) = self.white_material_index {
            index
        } else {
            let index = self.materials.len();
            self.materials.push(self.create_white_material());
            self.white_material_index = Some(index);
            index
        }
    }

    fn resolve_material(&mut self, material_key: MaterialKey) -> usize {
        match material_key {
            MaterialKey::SolidColor(rgb) => {
                if let Some(index) = self.solid_material_cache.get(&rgb) {
                    *index
                } else {
                    let index = self.materials.len();
                    self.materials.push(self.create_solid_material(rgb));
                    self.solid_material_cache.insert(rgb, index);
                    index
                }
            }
            MaterialKey::PaletteTexture(rgb) => {
                if let Some(index) = self.palette_texture_material_cache.get(&rgb) {
                    *index
                } else {
                    let texture_index =
                        if let Some(cached) = self.palette_texture_index_cache.get(&rgb) {
                            *cached
                        } else {
                            let sampler_idx = self.nearest_sampler();
                            let resolved = if let Some(png_bytes) =
                                create_palette_color_png(rgb, self.compress_textures)
                            {
                                let image_view_index = self.push_blob_buffer_view(&png_bytes);
                                let image_index = self.images.len();
                                self.images.push(json::Image {
                                    buffer_view: Some(Index::new(image_view_index as u32)),
                                    mime_type: Some(json::image::MimeType("image/png".to_string())),
                                    uri: None,
                                    name: None,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                });
                                let tex_idx = self.textures.len();
                                self.textures.push(json::Texture {
                                    sampler: Some(Index::new(sampler_idx as u32)),
                                    source: Index::new(image_index as u32),
                                    name: None,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                });
                                Some(tex_idx)
                            } else {
                                None
                            };
                            self.palette_texture_index_cache.insert(rgb, resolved);
                            resolved
                        };

                    let index = if let Some(tex_idx) = texture_index {
                        let new_index = self.materials.len();
                        self.materials
                            .push(self.create_textured_material(tex_idx, false));
                        new_index
                    } else {
                        self.resolve_white_material()
                    };

                    self.palette_texture_material_cache.insert(rgb, index);
                    index
                }
            }
            MaterialKey::Textured(texture_id, image_id) => {
                if let Some(index) = self.textured_material_cache.get(&(texture_id, image_id)) {
                    *index
                } else {
                    let texture_info = if let Some(cached) =
                        self.texture_index_cache.get(&(texture_id, image_id))
                    {
                        *cached
                    } else {
                        let sampler_idx = self.nearest_sampler();
                        let resolved = if let Some(cache) = self.texture_cache.as_deref_mut() {
                            if let Some((png_bytes, _, _, has_alpha)) =
                                cache.get_image_png(texture_id, image_id, self.compress_textures)
                            {
                                let image_view_index = self.push_blob_buffer_view(&png_bytes);
                                let image_index = self.images.len();
                                self.images.push(json::Image {
                                    buffer_view: Some(Index::new(image_view_index as u32)),
                                    mime_type: Some(json::image::MimeType("image/png".to_string())),
                                    uri: None,
                                    name: None,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                });
                                let tex_idx = self.textures.len();
                                self.textures.push(json::Texture {
                                    sampler: Some(Index::new(sampler_idx as u32)),
                                    source: Index::new(image_index as u32),
                                    name: None,
                                    extensions: Default::default(),
                                    extras: Default::default(),
                                });
                                Some((tex_idx, has_alpha))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        self.texture_index_cache
                            .insert((texture_id, image_id), resolved);
                        resolved
                    };

                    let index = if let Some((tex_idx, has_alpha)) = texture_info {
                        let new_index = self.materials.len();
                        self.materials
                            .push(self.create_textured_material(tex_idx, has_alpha));
                        new_index
                    } else {
                        self.resolve_white_material()
                    };

                    self.textured_material_cache
                        .insert((texture_id, image_id), index);
                    index
                }
            }
            MaterialKey::White => self.resolve_white_material(),
        }
    }

    pub(super) fn append_mesh(&mut self, unrolled_primitives: Vec<UnrolledPrimitive>) -> usize {
        let mesh_index = self.meshes.len();
        let mut primitives = Vec::new();

        for primitive in unrolled_primitives {
            let scaled_uvs: Vec<[f32; 2]> = match primitive.material_key {
                MaterialKey::Textured(texture_id, image_id)
                    if primitive.scale_uv_by_texture_dimensions =>
                {
                    let texture_dims = if let Some(cache) = self.texture_cache.as_deref_mut() {
                        cache.get_image_dimensions(texture_id, image_id)
                    } else {
                        None
                    };

                    if let Some((width, height)) = texture_dims {
                        let u_scale = (width.max(1) as f32) * UV_FIXED_POINT_SCALE;
                        let v_scale = (height.max(1) as f32) * UV_FIXED_POINT_SCALE;
                        primitive
                            .uvs
                            .iter()
                            .map(|[u, v]| [u / u_scale, v / v_scale])
                            .collect()
                    } else {
                        primitive
                            .uvs
                            .iter()
                            .map(|[u, v]| [u / UV_FIXED_POINT_SCALE, v / UV_FIXED_POINT_SCALE])
                            .collect()
                    }
                }
                MaterialKey::Textured(_, _) => primitive.uvs.clone(),
                _ => primitive
                    .uvs
                    .iter()
                    .map(|[u, v]| [u / UV_FIXED_POINT_SCALE, v / UV_FIXED_POINT_SCALE])
                    .collect(),
            };

            let position_accessor_index = self.push_vec3_accessor(
                &primitive.positions,
                Some(primitive.min),
                Some(primitive.max),
            );
            let normal_accessor_index = self.push_vec3_accessor(&primitive.normals, None, None);
            let texcoord_accessor_index = self.push_vec2_accessor(&scaled_uvs);
            let index_accessor_index = self.push_index_accessor(&primitive.indices);

            let mut attributes = BTreeMap::new();
            attributes.insert(
                Checked::Valid(Semantic::Positions),
                Index::new(position_accessor_index as u32),
            );
            attributes.insert(
                Checked::Valid(Semantic::Normals),
                Index::new(normal_accessor_index as u32),
            );
            attributes.insert(
                Checked::Valid(Semantic::TexCoords(0)),
                Index::new(texcoord_accessor_index as u32),
            );

            let material_index = self.resolve_material(primitive.material_key);

            primitives.push(Primitive {
                attributes,
                indices: Some(Index::new(index_accessor_index as u32)),
                material: Some(Index::new(material_index as u32)),
                mode: Checked::Valid(json::mesh::Mode::Triangles),
                targets: None,
                extensions: Default::default(),
                extras: Default::default(),
            });
        }

        self.meshes.push(Mesh {
            primitives,
            weights: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        });

        mesh_index
    }

    pub(super) fn finish(self) -> Result<(Root, Vec<u8>)> {
        self.finish_internal(None)
    }

    pub(super) fn finish_with_lights(
        self,
        lights: Vec<khr_lights_punctual::Light>,
    ) -> Result<(Root, Vec<u8>)> {
        self.finish_internal(Some(lights))
    }

    fn finish_internal(
        mut self,
        lights: Option<Vec<khr_lights_punctual::Light>>,
    ) -> Result<(Root, Vec<u8>)> {
        let child_indices: Vec<Index<Node>> =
            (0..self.nodes.len() as u32).map(Index::new).collect();
        self.nodes.push(Node {
            children: Some(child_indices),
            name: Some("Root".to_string()),
            ..Default::default()
        });
        let root_node_index = (self.nodes.len() - 1) as u32;

        let scene = Scene {
            nodes: vec![Index::new(root_node_index)],
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        };

        let buffer = Buffer {
            byte_length: USize64(self.buffer_data.len() as u64),
            uri: None,
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        };

        let mut root = Root {
            asset: Asset {
                version: "2.0".to_string(),
                generator: Some(format!(
                    "redguard-preservation {}",
                    env!("CARGO_PKG_VERSION")
                )),
                ..Default::default()
            },
            accessors: self.accessors,
            buffers: vec![buffer],
            buffer_views: self.buffer_views,
            images: self.images,
            textures: self.textures,
            materials: self.materials,
            samplers: self.samplers,
            meshes: self.meshes,
            nodes: self.nodes,
            scenes: vec![scene],
            scene: Some(Index::new(0)),
            ..Default::default()
        };

        if let Some(lights) = lights
            && !lights.is_empty()
        {
            root.extensions = Some(gltf_json::extensions::root::Root {
                khr_lights_punctual: Some(gltf_json::extensions::root::KhrLightsPunctual {
                    lights,
                }),
            });
            root.extensions_used = vec!["KHR_lights_punctual".to_string()];
        }

        Ok((root, self.buffer_data))
    }
}
