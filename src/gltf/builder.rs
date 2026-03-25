use gltf_json as json;
use gltf_json::extensions::scene::khr_lights_punctual;
use json::extras::Void;
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
    #[allow(clippy::cast_possible_truncation)]
    // GLTF indices are u32; input data is far below u32::MAX.
    const fn index_u32(value: usize) -> u32 {
        value as u32
    }

    #[allow(clippy::cast_possible_truncation)]
    // glTF JSON uses u64 byte offsets/lengths; buffers here are far below u64::MAX.
    const fn usize_u64(value: usize) -> u64 {
        value as u64
    }

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

    #[must_use]
    pub(super) const fn has_texture_cache(&self) -> bool {
        self.texture_cache_available
    }

    pub(super) fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    fn align_buffer(&mut self, alignment: usize) {
        let padding = (alignment - (self.buffer_data.len() % alignment)) % alignment;
        self.buffer_data.extend(std::iter::repeat_n(0u8, padding));
    }

    #[allow(clippy::too_many_arguments)]
    fn push_accessor_raw(
        &mut self,
        bytes: Vec<u8>,
        count: usize,
        component_type: ComponentType,
        type_: Type,
        target: Target,
        min: Option<Value>,
        max: Option<Value>,
    ) -> usize {
        self.align_buffer(4);
        let buffer_offset = self.buffer_data.len();
        self.buffer_data.extend_from_slice(&bytes);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(Self::usize_u64(buffer_offset))),
            byte_length: USize64(Self::usize_u64(bytes.len())),
            byte_stride: None,
            name: None,
            target: Some(Checked::Valid(target)),
            extensions: None,
            extras: Void::default(),
        });

        let accessor_index = self.accessors.len();
        self.accessors.push(Accessor {
            buffer_view: Some(Index::new(Self::index_u32(view_index))),
            byte_offset: Some(USize64(0)),
            component_type: Checked::Valid(GenericComponentType(component_type)),
            count: USize64(Self::usize_u64(count)),
            type_: Checked::Valid(type_),
            min,
            max,
            name: None,
            normalized: false,
            sparse: None,
            extensions: None,
            extras: Void::default(),
        });

        accessor_index
    }

    fn push_vec3_accessor(
        &mut self,
        data: &[[f32; 3]],
        min: Option<[f32; 3]>,
        max: Option<[f32; 3]>,
    ) -> usize {
        let mut bytes = Vec::with_capacity(data.len() * 12);
        for item in data {
            bytes.extend_from_slice(&item[0].to_le_bytes());
            bytes.extend_from_slice(&item[1].to_le_bytes());
            bytes.extend_from_slice(&item[2].to_le_bytes());
        }
        self.push_accessor_raw(
            bytes,
            data.len(),
            ComponentType::F32,
            Type::Vec3,
            Target::ArrayBuffer,
            min.map(|v| {
                Value::Array(vec![
                    Value::from(v[0]),
                    Value::from(v[1]),
                    Value::from(v[2]),
                ])
            }),
            max.map(|v| {
                Value::Array(vec![
                    Value::from(v[0]),
                    Value::from(v[1]),
                    Value::from(v[2]),
                ])
            }),
        )
    }

    fn push_vec2_accessor(&mut self, data: &[[f32; 2]]) -> usize {
        let mut bytes = Vec::with_capacity(data.len() * 8);
        for item in data {
            bytes.extend_from_slice(&item[0].to_le_bytes());
            bytes.extend_from_slice(&item[1].to_le_bytes());
        }
        self.push_accessor_raw(
            bytes,
            data.len(),
            ComponentType::F32,
            Type::Vec2,
            Target::ArrayBuffer,
            None,
            None,
        )
    }

    fn push_index_accessor(&mut self, indices: &[u32]) -> usize {
        let mut bytes = Vec::with_capacity(indices.len() * 4);
        for index in indices {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
        self.push_accessor_raw(
            bytes,
            indices.len(),
            ComponentType::U32,
            Type::Scalar,
            Target::ElementArrayBuffer,
            None,
            None,
        )
    }

    fn push_blob_buffer_view(&mut self, data: &[u8]) -> usize {
        let buffer_offset = self.buffer_data.len();
        self.buffer_data.extend_from_slice(data);

        let view_index = self.buffer_views.len();
        self.buffer_views.push(View {
            buffer: Index::new(0),
            byte_offset: Some(USize64(Self::usize_u64(buffer_offset))),
            byte_length: USize64(Self::usize_u64(data.len())),
            byte_stride: None,
            name: None,
            target: None,
            extensions: None,
            extras: Void::default(),
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

    fn create_solid_material(rgb: [u8; 3]) -> json::Material {
        json::Material {
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_factor: json::material::PbrBaseColorFactor([
                    f32::from(rgb[0]) / 255.0,
                    f32::from(rgb[1]) / 255.0,
                    f32::from(rgb[2]) / 255.0,
                    1.0,
                ]),
                metallic_factor: json::material::StrengthFactor(0.0),
                roughness_factor: json::material::StrengthFactor(1.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_textured_material(texture_index: usize, has_alpha: bool) -> json::Material {
        json::Material {
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_texture: Some(json::texture::Info {
                    index: Index::new(Self::index_u32(texture_index)),
                    tex_coord: 0,
                    extensions: None,
                    extras: Void::default(),
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

    fn create_white_material() -> json::Material {
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
            self.materials.push(Self::create_white_material());
            self.white_material_index = Some(index);
            index
        }
    }

    fn resolve_solid_color_material(&mut self, rgb: [u8; 3]) -> usize {
        if let Some(index) = self.solid_material_cache.get(&rgb) {
            *index
        } else {
            let index = self.materials.len();
            self.materials.push(Self::create_solid_material(rgb));
            self.solid_material_cache.insert(rgb, index);
            index
        }
    }

    fn push_png_texture(&mut self, png_bytes: &[u8], sampler_index: usize) -> usize {
        let image_view_index = self.push_blob_buffer_view(png_bytes);
        let image_index = self.images.len();
        self.images.push(json::Image {
            buffer_view: Some(Index::new(Self::index_u32(image_view_index))),
            mime_type: Some(json::image::MimeType("image/png".to_string())),
            uri: None,
            name: None,
            extensions: None,
            extras: Void::default(),
        });
        let texture_index = self.textures.len();
        self.textures.push(json::Texture {
            sampler: Some(Index::new(Self::index_u32(sampler_index))),
            source: Index::new(Self::index_u32(image_index)),
            name: None,
            extensions: None,
            extras: Void::default(),
        });

        texture_index
    }

    fn resolve_palette_texture_index(&mut self, rgb: [u8; 3]) -> Option<usize> {
        if let Some(cached) = self.palette_texture_index_cache.get(&rgb) {
            return *cached;
        }

        let sampler_index = self.nearest_sampler();
        let resolved = create_palette_color_png(rgb, self.compress_textures)
            .map(|png_bytes| self.push_png_texture(&png_bytes, sampler_index));
        self.palette_texture_index_cache.insert(rgb, resolved);
        resolved
    }

    fn resolve_palette_texture_material(&mut self, rgb: [u8; 3]) -> usize {
        if let Some(index) = self.palette_texture_material_cache.get(&rgb) {
            return *index;
        }

        let index = if let Some(texture_index) = self.resolve_palette_texture_index(rgb) {
            let new_index = self.materials.len();
            self.materials
                .push(Self::create_textured_material(texture_index, false));
            new_index
        } else {
            self.resolve_white_material()
        };

        self.palette_texture_material_cache.insert(rgb, index);
        index
    }

    fn resolve_textured_texture_index(
        &mut self,
        texture_id: u16,
        image_id: u8,
    ) -> Option<(usize, bool)> {
        if let Some(cached) = self.texture_index_cache.get(&(texture_id, image_id)) {
            return *cached;
        }

        let sampler_index = self.nearest_sampler();
        let texture_png = if let Some(cache) = self.texture_cache.as_deref_mut() {
            cache.get_image_png(texture_id, image_id, self.compress_textures)
        } else {
            None
        };
        let resolved = texture_png.map(|(png_bytes, _, _, has_alpha)| {
            (self.push_png_texture(&png_bytes, sampler_index), has_alpha)
        });

        self.texture_index_cache
            .insert((texture_id, image_id), resolved);
        resolved
    }

    fn resolve_textured_material(&mut self, texture_id: u16, image_id: u8) -> usize {
        if let Some(index) = self.textured_material_cache.get(&(texture_id, image_id)) {
            return *index;
        }

        let index = if let Some((texture_index, has_alpha)) =
            self.resolve_textured_texture_index(texture_id, image_id)
        {
            let new_index = self.materials.len();
            self.materials
                .push(Self::create_textured_material(texture_index, has_alpha));
            new_index
        } else {
            self.resolve_white_material()
        };

        self.textured_material_cache
            .insert((texture_id, image_id), index);
        index
    }

    fn resolve_material(&mut self, material_key: MaterialKey) -> usize {
        match material_key {
            MaterialKey::SolidColor(rgb) => self.resolve_solid_color_material(rgb),
            MaterialKey::PaletteTexture(rgb) => self.resolve_palette_texture_material(rgb),
            MaterialKey::Textured(texture_id, image_id) => {
                self.resolve_textured_material(texture_id, image_id)
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
                    let texture_dims = self
                        .texture_cache
                        .as_deref_mut()
                        .and_then(|cache| cache.get_image_dimensions(texture_id, image_id));

                    if let Some((width, height)) = texture_dims {
                        let u_scale = f32::from(width.max(1)) * UV_FIXED_POINT_SCALE;
                        let v_scale = f32::from(height.max(1)) * UV_FIXED_POINT_SCALE;
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
                Index::new(Self::index_u32(position_accessor_index)),
            );
            attributes.insert(
                Checked::Valid(Semantic::Normals),
                Index::new(Self::index_u32(normal_accessor_index)),
            );
            attributes.insert(
                Checked::Valid(Semantic::TexCoords(0)),
                Index::new(Self::index_u32(texcoord_accessor_index)),
            );

            let material_index = self.resolve_material(primitive.material_key);

            primitives.push(Primitive {
                attributes,
                indices: Some(Index::new(Self::index_u32(index_accessor_index))),
                material: Some(Index::new(Self::index_u32(material_index))),
                mode: Checked::Valid(json::mesh::Mode::Triangles),
                targets: None,
                extensions: None,
                extras: Void::default(),
            });
        }

        self.meshes.push(Mesh {
            primitives,
            weights: None,
            name: None,
            extensions: None,
            extras: Void::default(),
        });

        mesh_index
    }

    pub(super) fn finish(self) -> (Root, Vec<u8>) {
        self.finish_internal(None)
    }

    pub(super) fn finish_with_lights(
        self,
        lights: Vec<khr_lights_punctual::Light>,
    ) -> (Root, Vec<u8>) {
        self.finish_internal(Some(lights))
    }

    fn finish_internal(
        mut self,
        lights: Option<Vec<khr_lights_punctual::Light>>,
    ) -> (Root, Vec<u8>) {
        let child_indices: Vec<Index<Node>> = (0..Self::index_u32(self.nodes.len()))
            .map(Index::new)
            .collect();
        self.nodes.push(Node {
            children: Some(child_indices),
            name: Some("Root".to_string()),
            ..Default::default()
        });
        let root_node_index = Self::index_u32(self.nodes.len() - 1);

        let scene = Scene {
            nodes: vec![Index::new(root_node_index)],
            name: None,
            extensions: None,
            extras: Void::default(),
        };

        let buffer = Buffer {
            byte_length: USize64(Self::usize_u64(self.buffer_data.len())),
            uri: None,
            name: None,
            extensions: None,
            extras: Void::default(),
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

        (root, self.buffer_data)
    }
}
