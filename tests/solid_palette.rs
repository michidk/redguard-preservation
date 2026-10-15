use rgpre::{
    import::palette::Palette,
    model3d::{FaceData, FaceVertex, TextureData, VertexCoord},
};

#[test]
fn equal_colors_preserve_distinct_palette_indices_through_glb() {
    for textured_material in [false, true] {
        let mut model =
            rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
        model.vertex_coords = [[0.0, 0.0], [20.0, 0.0], [0.0, 20.0]]
            .map(|[x, y]| VertexCoord { x, y, z: 0.0 })
            .to_vec();
        model.face_data = [0, 7, 0]
            .map(|index| FaceData {
                vertex_count: 3,
                tex_hi: 7,
                texture_data: TextureData::SolidColor(index),
                face_vertices: (0..3)
                    .map(|vertex_index| FaceVertex {
                        vertex_index,
                        u: 0,
                        v: 0,
                    })
                    .collect(),
            })
            .to_vec();
        let palette = Palette {
            colors: [[19, 31, 47]; 256],
        };
        let mut cache = rgpre::gltf::TextureCache::from_data(Default::default(), None);
        let (document, bytes) = rgpre::convert_models_to_gltf(
            &[model],
            Some(&palette),
            textured_material.then_some(&mut cache),
            false,
        )
        .unwrap();
        let glb = rgpre::to_glb(&document, &bytes).unwrap();
        let decoded = gltf::Gltf::from_slice(&glb).unwrap();
        let mut counts = std::collections::BTreeMap::new();
        for primitive in decoded.meshes().next().unwrap().primitives() {
            let extras: serde_json::Value =
                serde_json::from_str(primitive.extras().as_ref().unwrap().get()).unwrap();
            assert_eq!(extras["rgpre_solid_palette"]["version"], 1);
            assert_eq!(
                extras["rgpre_lighting"]["solid_rgb"],
                serde_json::json!([19, 31, 47])
            );
            let index = extras["rgpre_solid_palette"]["index"].as_u64().unwrap();
            let reader = primitive.reader(|_| decoded.blob.as_deref());
            assert!(
                counts
                    .insert(index, reader.read_positions().unwrap().count())
                    .is_none()
            );
        }
        assert_eq!(counts, std::collections::BTreeMap::from([(0, 6), (7, 3)]));
    }
}
