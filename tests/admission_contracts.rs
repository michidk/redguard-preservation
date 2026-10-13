use rgpre::import::palette::Palette;
use rgpre::model3d::{
    FaceData, FaceNormal, FaceVertex, Model3DFile, TextureData, VertexCoord, VertexNormal,
};

fn model() -> Model3DFile {
    let mut model =
        rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
    model.header.num_frames = 1;
    model.header.radius = 6400;
    model.vertex_coords = [
        [0.0, 0.0, 0.0],
        [20.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 20.0, 0.0],
    ]
    .into_iter()
    .map(|[x, y, z]| VertexCoord { x, y, z })
    .collect();
    model.face_data = [(42, vec![0, 1, 2, 3]), (7, vec![2, 1, 3])]
        .into_iter()
        .map(|(color, vertices)| FaceData {
            vertex_count: u8::try_from(vertices.len()).unwrap(),
            tex_hi: 0,
            texture_data: TextureData::SolidColor(color),
            face_vertices: vertices
                .into_iter()
                .map(|vertex_index| FaceVertex {
                    vertex_index,
                    u: 0,
                    v: 0,
                })
                .collect(),
        })
        .collect();
    model.face_normals = vec![
        FaceNormal {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        FaceNormal {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
    ];
    model.vertex_normals = vec![VertexNormal {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    }];
    model.normal_indices = vec![0; 7];
    model
}

fn export(model: Model3DFile) -> (gltf_json::Root, Vec<u8>) {
    let mut palette = Palette {
        colors: [[0; 3]; 256],
    };
    palette.colors[7] = [1; 3];
    palette.colors[42] = [9; 3];
    rgpre::convert_models_to_gltf(&[model], Some(&palette), None, false).unwrap()
}

#[test]
fn admission_retains_authored_polygons_across_material_splits_and_triangulation() {
    for frames in [1, 2] {
        let mut model = model();
        model.header.num_frames = frames;
        let (document, bytes) = export(model);
        let glb = rgpre::to_glb(&document, &bytes).unwrap();
        let decoded = gltf::Gltf::from_slice(&glb).unwrap();
        let primitives: Vec<_> = decoded.meshes().next().unwrap().primitives().collect();
        let extras: Vec<serde_json::Value> = primitives
            .iter()
            .map(|primitive| {
                serde_json::from_str(primitive.extras().as_ref().unwrap().get()).unwrap()
            })
            .collect();
        assert_eq!(
            extras[0]["rgpre_admission"],
            serde_json::json!({
                "version": 1, "radius_fixed": 6400,
                "polygons": [{"source_face": 1, "vertices": [0, 1, 2], "normal": [-4.0, -5.0, 6.0]}]
            })
        );
        assert_eq!(
            extras[1]["rgpre_admission"],
            serde_json::json!({
                "version": 1, "radius_fixed": 6400,
                "polygons": [{"source_face": 0, "vertices": [0, 1, 2, 5], "normal": [-1.0, -2.0, 3.0]}]
            })
        );
        assert_eq!(extras[0].get("rgpre_geometry").is_some(), frames > 1);
        let reader = primitives[1].reader(|_| decoded.blob.as_deref());
        let positions: Vec<_> = reader.read_positions().unwrap().collect();
        assert_eq!(positions[0], positions[2]);
        assert_eq!(positions[5], [0.0, -1.0, 0.0]);
        assert!(
            reader
                .read_normals()
                .unwrap()
                .all(|normal| normal == [0.0, 0.0, 1.0])
        );
    }
}

#[test]
fn missing_authored_data_is_explicit_instead_of_using_lighting_fallbacks() {
    let mut model = model();
    model.face_normals.clear();
    model.face_data[0].face_vertices[3].vertex_index = 99;
    let (document, _) = export(model);
    let extras: Vec<serde_json::Value> = document.meshes[0]
        .primitives
        .iter()
        .map(|primitive| serde_json::from_str(primitive.extras.as_ref().unwrap().get()).unwrap())
        .collect();
    assert!(extras[0]["rgpre_admission"]["polygons"][0]["normal"].is_null());
    let polygon = &extras[1]["rgpre_admission"]["polygons"][0];
    assert!(polygon["normal"].is_null());
    assert!(polygon["vertices"].is_null());
}
