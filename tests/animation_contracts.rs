use rgpre::import::rgm::RagrCommand;

#[test]
fn file_show_frame_uses_all_twenty_parameter_bits() {
    let command = RagrCommand {
        raw: 0x0bc030,
        opcode: 0,
    };
    assert_eq!(command.frame_reference(), Some(48131));
    assert_eq!(
        RagrCommand {
            raw: 0xfffff0,
            opcode: 0
        }
        .frame_reference(),
        Some(1048575)
    );
}

#[test]
fn sound_and_rotation_commands_are_not_frame_references() {
    assert_eq!(RagrCommand { raw: 4, opcode: 4 }.frame_reference(), None);
    assert_eq!(RagrCommand { raw: 6, opcode: 6 }.frame_reference(), None);
}

#[test]
fn animated_glb_preserves_vertex_identity_through_material_split_and_quad_fan() {
    use rgpre::import::palette::Palette;
    use rgpre::model3d::{FaceData, FaceVertex, TextureData, VertexCoord};
    let mut model =
        rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
    model.header.num_frames = 2;
    model.vertex_coords = vec![
        VertexCoord {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VertexCoord {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
        VertexCoord {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VertexCoord {
            x: 0.0,
            y: 20.0,
            z: 0.0,
        },
    ];
    model.face_data = [(42, vec![0, 1, 2, 3]), (7, vec![2, 1, 3])]
        .into_iter()
        .map(|(color, vertices)| FaceData {
            vertex_count: u8::try_from(vertices.len()).unwrap(),
            tex_hi: color,
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
    let mut palette = Palette {
        colors: [[0; 3]; 256],
    };
    palette.colors[7] = [1; 3];
    palette.colors[42] = [9; 3];
    let (document, bytes) =
        rgpre::convert_models_to_gltf(&[model], Some(&palette), None, false).unwrap();
    let glb = rgpre::to_glb(&document, &bytes).unwrap();
    let decoded = gltf::Gltf::from_slice(&glb).unwrap();
    let mapping: Vec<serde_json::Value> = document.meshes[0]
        .primitives
        .iter()
        .map(|primitive| serde_json::from_str(primitive.extras.as_ref().unwrap().get()).unwrap())
        .collect();
    assert_eq!(decoded.meshes().next().unwrap().primitives().count(), 2);
    assert_eq!(
        mapping[0]["rgpre_lighting"]["vertices"],
        serde_json::json!([[7, 1], [7, 1], [7, 1]])
    );
    assert_eq!(
        mapping[1]["rgpre_lighting"]["vertices"],
        serde_json::json!([[42, 1], [42, 1], [42, 1], [42, 1], [42, 1], [42, 1]])
    );
    assert_eq!(
        mapping[0]["rgpre_geometry"]["source_vertices"],
        serde_json::json!([[2, 1], [1, 1], [3, 1]])
    );
    assert_eq!(
        mapping[1]["rgpre_geometry"]["source_vertices"],
        serde_json::json!([[0, 0], [1, 0], [2, 0], [0, 0], [2, 0], [3, 0]])
    );
}

#[test]
fn static_glb_retains_face_brightness_without_animation_mapping() {
    let mut model =
        rgpre::model3d::parse_3d_file(include_bytes!("fixtures/model_v40.3dc")).unwrap();
    model.header.num_frames = 1;
    model.vertex_coords = vec![
        rgpre::model3d::VertexCoord {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        rgpre::model3d::VertexCoord {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
        rgpre::model3d::VertexCoord {
            x: 0.0,
            y: 20.0,
            z: 0.0,
        },
    ];
    model.face_data = vec![rgpre::model3d::FaceData {
        vertex_count: 3,
        tex_hi: 0,
        texture_data: rgpre::model3d::TextureData::SolidColor(0),
        face_vertices: (0..3)
            .map(|vertex_index| rgpre::model3d::FaceVertex {
                vertex_index,
                u: 0,
                v: 0,
            })
            .collect(),
    }];
    for face in &mut model.face_data {
        face.tex_hi = 73;
        face.texture_data = rgpre::model3d::TextureData::Texture {
            texture_id: 12,
            image_id: 0,
        };
    }
    model.header.radius = 12_345;
    let (document, _) = rgpre::convert_models_to_gltf(&[model], None, None, false).unwrap();
    let primitive = &document.meshes[0].primitives[0];
    let extras: serde_json::Value =
        serde_json::from_str(primitive.extras.as_ref().unwrap().get()).unwrap();
    assert!(extras.get("rgpre_geometry").is_none());
    assert_eq!(extras["rgpre_lighting"]["version"], 2);
    assert_eq!(extras["rgpre_lighting"]["radius_fixed"], 12_345);
    let vertices = extras["rgpre_lighting"]["vertices"].as_array().unwrap();
    assert!(!vertices.is_empty());
    assert!(
        vertices
            .iter()
            .all(|vertex| *vertex == serde_json::json!([73, 0]))
    );
}
