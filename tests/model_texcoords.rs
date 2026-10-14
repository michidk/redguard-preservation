use rgpre::model3d::{FaceData, FaceVertex, TextureData, VertexCoord};

#[test]
fn source_pixels_survive_material_split_and_quad_fan() {
    let mut model =
        rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
    model.vertex_coords = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]]
        .map(|[x, y]| VertexCoord { x, y, z: 0.0 })
        .to_vec();
    let corners = [(0, -17, 32767), (1, 33, -32768), (2, 511, 1), (3, 4097, -1)];
    model.face_data = vec![
        FaceData {
            vertex_count: 4,
            tex_hi: 0,
            texture_data: TextureData::Texture {
                texture_id: 12,
                image_id: 0,
            },
            face_vertices: corners
                .into_iter()
                .map(|(vertex_index, u, v)| FaceVertex { vertex_index, u, v })
                .collect(),
        },
        FaceData {
            vertex_count: 3,
            tex_hi: 7,
            texture_data: TextureData::SolidColor(7),
            face_vertices: (0..3)
                .map(|vertex_index| FaceVertex {
                    vertex_index,
                    u: 0,
                    v: 0,
                })
                .collect(),
        },
    ];
    let palette = rgpre::import::palette::Palette {
        colors: [[0; 3]; 256],
    };
    let mut cache = rgpre::gltf::TextureCache::from_data(std::collections::HashMap::new(), None);
    let (document, bytes) =
        rgpre::convert_models_to_gltf(&[model], Some(&palette), Some(&mut cache), false).unwrap();
    let glb = rgpre::to_glb(&document, &bytes).unwrap();
    let decoded = gltf::Gltf::from_slice(&glb).unwrap();
    let mut textured = 0;
    let mut solid = 0;
    for primitive in decoded.meshes().next().unwrap().primitives() {
        let extras: serde_json::Value =
            serde_json::from_str(primitive.extras().as_ref().unwrap().get()).unwrap();
        if extras["rgpre_lighting"]["vertices"][0][1] == 1 {
            assert!(extras.get("rgpre_texcoords").is_none());
            solid += 1;
            continue;
        }
        assert_eq!(
            extras["rgpre_texcoords"],
            serde_json::json!({
                "version": 1,
                "pixels": [
                    [-1.0625, 2047.9375], [2.0625, -2048.0], [31.9375, 0.0625],
                    [-1.0625, 2047.9375], [31.9375, 0.0625], [256.0625, -0.0625]
                ]
            })
        );
        let reader = primitive.reader(|_| decoded.blob.as_deref());
        assert_eq!(reader.read_positions().unwrap().count(), 6);
        // Missing texture data retains the existing unit-dimension viewer fallback.
        let uv: Vec<_> = reader.read_tex_coords(0).unwrap().into_f32().collect();
        assert_eq!(uv[0], [-1.0625, -2046.9375]);
        textured += 1;
    }
    assert_eq!((textured, solid), (1, 1));
}

#[test]
fn model_uv_deltas_accumulate_beyond_the_signed_word_range() {
    use rgpre::model3d::{ModelVersion, parser::parse_face_data};
    for version in [
        ModelVersion::V26,
        ModelVersion::V27,
        ModelVersion::V40,
        ModelVersion::V50,
    ] {
        let mut bytes = vec![255, 0];
        bytes.extend(vec![
            0;
            if matches!(version, ModelVersion::V26 | ModelVersion::V27) {
                6
            } else {
                8
            }
        ]);
        for _ in 0..255 {
            bytes.extend(0_u32.to_le_bytes());
            bytes.extend(32767_i16.to_le_bytes());
            bytes.extend((-32768_i16).to_le_bytes());
        }
        let (remaining, face) = parse_face_data(&bytes, &version).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            (face.face_vertices[1].u, face.face_vertices[1].v),
            (65534, -65536)
        );
        assert_eq!(
            (face.face_vertices[254].u, face.face_vertices[254].v),
            (8355585, -8355840)
        );
    }
}
