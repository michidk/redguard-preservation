use rgpre::import::palette::Palette;
use rgpre::model3d::{Model3DFile, parse_3d_file};
use std::process::Command;

fn word(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Three frames, two materials, and a quad. Compressed frame zero references frame one.
fn software_bytes(version: &[u8; 4], compressed: bool) -> Vec<u8> {
    let stride = if compressed { 12 } else { 16 };
    let faces = 88 + 3 * stride;
    let mut bytes = vec![0; faces];
    bytes[..4].copy_from_slice(version);
    for (offset, value) in [
        (4, 4),
        (8, 2),
        (16, 3),
        (20, 64),
        (44, 1),
        (60, faces as u32),
    ] {
        word(&mut bytes, offset, value);
    }
    word(&mut bytes, 64, 88);
    word(&mut bytes, 68, 72);
    bytes[76] = u8::from(compressed);
    bytes[78] = 1;
    bytes[87] = 2;
    for (color, indices) in [(42u16, vec![0u32, 1, 2, 3]), (7, vec![2, 1, 3])] {
        bytes.extend_from_slice(&[indices.len() as u8, 0]);
        bytes.extend_from_slice(&color.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        for index in indices {
            bytes.extend_from_slice(&(index * 12).to_le_bytes());
            bytes.extend_from_slice(&[0; 4]);
        }
    }
    for frame in 0..3 {
        let vertex_offset = bytes.len();
        for vertex in 0..4 {
            if compressed && frame != 1 {
                let delta = if frame == 0 {
                    [-64i16, 128, 256]
                } else {
                    [64, -128, 0]
                };
                for component in delta {
                    bytes.extend_from_slice(&component.to_le_bytes());
                }
            } else {
                let position = match frame {
                    0 => [vertex * 256i32, 1024, 1792],
                    1 => [(vertex + 1) * 256, 512, 768],
                    _ => [(vertex + 2) * 256, 0, 768],
                };
                for component in position {
                    bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
        }
        let normal_offset = bytes.len();
        for _ in 0..2 {
            if compressed {
                let normal = match frame {
                    0 => (256u32 << 20) | (896 << 10) | 64,
                    1 => 256 << 10,
                    _ => 256,
                };
                bytes.extend_from_slice(&normal.to_le_bytes());
            } else {
                let normal = match frame {
                    0 => [256i32, -128, 64],
                    1 => [0, 256, 0],
                    _ => [0, 0, 256],
                };
                for component in normal {
                    bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
        }
        word(&mut bytes, 88 + frame * stride, vertex_offset as u32);
        word(&mut bytes, 92 + frame * stride, normal_offset as u32);
    }
    bytes
}

fn palette() -> Palette {
    let mut palette = Palette {
        colors: [[0; 3]; 256],
    };
    palette.colors[7] = [1; 3];
    palette.colors[42] = [9; 3];
    palette
}

fn close(actual: [f32; 3], expected: [f32; 3]) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1e-6,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn software_morph_animation_preserves_decoded_poses_and_normals_across_material_splits() {
    for version in [b"v2.6", b"v2.7"] {
        for compressed in [false, true] {
            let model = parse_3d_file(&software_bytes(version, compressed)).unwrap();
            let (root, buffer) = rgpre::convert_animated_models_to_gltf(
                &[model],
                Some(&palette()),
                None,
                false,
                4.0,
            )
            .unwrap();
            let glb = rgpre::to_glb(&root, &buffer).unwrap();
            let decoded = gltf::Gltf::from_slice(&glb).unwrap();
            let mesh = decoded.meshes().next().unwrap();
            assert_eq!(mesh.primitives().count(), 2);
            assert_eq!(mesh.weights().unwrap(), [0.0, 0.0]);
            for primitive in mesh.primitives() {
                let extras: serde_json::Value =
                    serde_json::from_str(primitive.extras().as_ref().unwrap().get()).unwrap();
                let sources = extras["rgpre_geometry"]["source_vertices"]
                    .as_array()
                    .unwrap();
                let reader = primitive.reader(|_| decoded.blob.as_deref());
                let base: Vec<_> = reader.read_positions().unwrap().collect();
                let normals: Vec<_> = reader.read_normals().unwrap().collect();
                let targets: Vec<_> = reader.read_morph_targets().collect();
                assert_eq!(targets.len(), 2);
                for (frame, (positions, normal_deltas, _)) in targets.into_iter().enumerate() {
                    let positions: Vec<_> = positions.unwrap().collect();
                    let normal_deltas: Vec<_> = normal_deltas.unwrap().collect();
                    for vertex in 0..base.len() {
                        let source = sources[vertex][0].as_u64().unwrap() as f32;
                        close(base[vertex], [-source / 20.0, -0.2, 0.35]);
                        close(normals[vertex], [-1.0, 0.5, 0.25]);
                        let expected = if frame == 0 {
                            [-(source + 1.0) / 20.0, -0.1, 0.15]
                        } else {
                            [-(source + 2.0) / 20.0, 0.0, 0.15]
                        };
                        close(
                            std::array::from_fn(|axis| {
                                base[vertex][axis] + positions[vertex][axis]
                            }),
                            expected,
                        );
                        close(
                            std::array::from_fn(|axis| {
                                normals[vertex][axis] + normal_deltas[vertex][axis]
                            }),
                            if frame == 0 {
                                [0.0, -1.0, 0.0]
                            } else {
                                [0.0, 0.0, 1.0]
                            },
                        );
                    }
                }
            }
            let animation = decoded.animations().next().unwrap();
            assert_eq!(animation.channels().count(), 1);
            let channel = animation.channels().next().unwrap();
            assert_eq!(
                channel.target().node().mesh().unwrap().index(),
                mesh.index()
            );
            assert_eq!(
                channel.sampler().interpolation(),
                gltf::animation::Interpolation::Step
            );
            let reader = channel.reader(|_| decoded.blob.as_deref());
            assert_eq!(
                reader.read_inputs().unwrap().collect::<Vec<_>>(),
                [0.0, 0.25, 0.5, 0.75]
            );
            let gltf::animation::util::ReadOutputs::MorphTargetWeights(weights) =
                reader.read_outputs().unwrap()
            else {
                panic!()
            };
            assert_eq!(
                weights.into_f32().collect::<Vec<_>>(),
                [0., 0., 1., 0., 0., 1., 0., 1.]
            );
            for sampler in &root.animations[0].samplers {
                for accessor in [sampler.input, sampler.output] {
                    let view = root.accessors[accessor.value()].buffer_view.unwrap();
                    assert!(root.buffer_views[view.value()].target.is_none());
                }
            }
        }
    }
}

fn glide_model() -> Model3DFile {
    let mut bytes = vec![0; 64];
    bytes[..4].copy_from_slice(b"v4.0");
    for (color, indices) in [(42u32, vec![0u32, 1, 2, 3]), (7, vec![2, 1, 3])] {
        bytes.extend_from_slice(&[indices.len() as u8, 0xff]);
        bytes.extend_from_slice(&(0xfff00000 | (color << 8)).to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        for index in indices {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&[0; 4]);
        }
    }
    let base_offset = bytes.len();
    for vertex in 0..4 {
        for component in [vertex * 256i32, 1024, 1792] {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    let normals_offset = bytes.len();
    for _ in 0..2 {
        for component in [256i32, -128, 64] {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    let directory = bytes.len();
    bytes.resize(directory + 48, 0);
    for (offset, value) in [
        (4, 4),
        (8, 2),
        (16, 3),
        (20, directory as u32),
        (48, base_offset as u32),
        (52, normals_offset as u32),
        (60, 64),
    ] {
        word(&mut bytes, offset, value);
    }
    word(&mut bytes, directory, base_offset as u32);
    word(&mut bytes, directory + 4, normals_offset as u32);
    word(&mut bytes, directory + 12, 2);
    for frame in 1..3 {
        let positions = bytes.len();
        for _ in 0..4 {
            let deltas = if frame == 1 {
                [-256i16, 512, 1024]
            } else {
                [-512, 1024, 1024]
            };
            for delta in deltas {
                bytes.extend_from_slice(&delta.to_le_bytes());
            }
        }
        let normals = bytes.len();
        for _ in 0..2 {
            let packed = if frame == 1 { 256u32 << 10 } else { 256 << 20 };
            bytes.extend_from_slice(&packed.to_le_bytes());
        }
        word(&mut bytes, directory + frame * 16, positions as u32);
        word(&mut bytes, directory + frame * 16 + 4, normals as u32);
    }
    let vertex_normals = bytes.len();
    word(&mut bytes, 44, vertex_normals as u32);
    bytes.resize(vertex_normals + 48, 0);
    parse_3d_file(&bytes).unwrap()
}

#[test]
fn glide_animation_matches_frame_snapshots_and_default_export_stays_static() {
    let model = glide_model();
    let (root, bytes) = rgpre::convert_animated_models_to_gltf(
        std::slice::from_ref(&model),
        None,
        None,
        false,
        12.,
    )
    .unwrap();
    let glb = rgpre::to_glb(&root, &bytes).unwrap();
    let decoded = gltf::Gltf::from_slice(&glb).unwrap();
    for primitive in decoded.meshes().next().unwrap().primitives() {
        let reader = primitive.reader(|_| decoded.blob.as_deref());
        let positions: Vec<_> = reader.read_positions().unwrap().collect();
        let extras: serde_json::Value =
            serde_json::from_str(primitive.extras().as_ref().unwrap().get()).unwrap();
        for (frame, (deltas, _, _)) in reader.read_morph_targets().enumerate() {
            let pose = model.at_frame(frame + 1).unwrap();
            for (vertex, delta) in deltas.unwrap().enumerate() {
                let source = extras["rgpre_geometry"]["source_vertices"][vertex][0]
                    .as_u64()
                    .unwrap() as usize;
                let p = &pose.vertex_coords[source];
                close(
                    std::array::from_fn(|axis| positions[vertex][axis] + delta[axis]),
                    rgpre::gltf::model_position([p.x, p.y, p.z]),
                );
            }
        }
    }
    let (root, _) = rgpre::convert_models_to_gltf(&[model], None, None, false).unwrap();
    assert!(root.animations.is_empty());
    assert!(
        root.meshes
            .iter()
            .flat_map(|mesh| &mesh.primitives)
            .all(|p| p.targets.is_none())
    );
}

#[test]
fn animation_rejects_invalid_rates_incomplete_frames_and_static_only_inputs() {
    let model = glide_model();
    for fps in [
        0.,
        -1.,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE / 100.,
    ] {
        assert!(
            rgpre::convert_animated_models_to_gltf(
                std::slice::from_ref(&model),
                None,
                None,
                false,
                fps
            )
            .is_err()
        );
    }
    let mut oversized = model.clone();
    oversized.header.num_frames = u32::MAX;
    assert!(rgpre::convert_animated_models_to_gltf(&[oversized], None, None, false, 10.).is_err());
    let static_model = model.at_frame(0).unwrap();
    assert!(
        rgpre::convert_animated_models_to_gltf(
            std::slice::from_ref(&static_model),
            None,
            None,
            false,
            10.
        )
        .is_err()
    );
    let mut incomplete = model.clone();
    incomplete.frame_normal_data.clear();
    assert!(rgpre::convert_animated_models_to_gltf(&[incomplete], None, None, false, 10.).is_err());
    let (root, _) =
        rgpre::convert_animated_models_to_gltf(&[static_model, model], None, None, false, 10.)
            .unwrap();
    assert!(root.meshes[0].primitives[0].targets.is_none());
    assert_eq!(root.animations[0].channels[0].target.node.value(), 1);
}

#[test]
fn cli_exports_selected_pose_and_animation_and_rejects_invalid_options() {
    let dir = std::env::temp_dir().join(format!("rgpre-model-animation-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("model.3dc");
    let output = dir.join("model.glb");
    std::fs::write(&input, software_bytes(b"v2.7", true)).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rgpre"))
            .args(["convert", "model"])
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .args(args)
            .output()
            .unwrap()
    };
    let result = run(&["--frame", "1"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&output).unwrap();
    let glb = gltf::Gltf::from_slice(&bytes).unwrap();
    assert_eq!(glb.animations().count(), 0);
    let p = glb.meshes().next().unwrap().primitives().next().unwrap();
    let positions: Vec<_> = p
        .reader(|_| glb.blob.as_deref())
        .read_positions()
        .unwrap()
        .collect();
    close(positions[0], [-0.15, -0.1, 0.15]);
    let result = run(&["--animate", "--fps", "4"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        gltf::Gltf::from_slice(&std::fs::read(&output).unwrap())
            .unwrap()
            .animations()
            .count(),
        1
    );
    for args in [
        vec!["--frame", "3"],
        vec!["--animate"],
        vec!["--fps", "4"],
        vec!["--frame", "0", "--animate", "--fps", "4"],
        vec!["--animate", "--fps", "0"],
        vec!["--animate", "--fps", "NaN"],
        vec!["--animate", "--fps", "inf"],
        vec!["--animate", "--fps=-1"],
    ] {
        assert!(
            !run(&args).status.success(),
            "unexpected success for {args:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mixed_collection_animates_each_rendered_node_in_one_clip() {
    let model = glide_model();
    let static_model = model.at_frame(0).unwrap();
    let mut empty = static_model.clone();
    empty.vertex_coords.clear();
    let mut shorter = model.clone();
    shorter.header.num_frames = 2;
    shorter.frame_data.truncate(2);
    shorter.frame_vertex_data.truncate(2);
    shorter.frame_normal_data.truncate(2);
    let (root, buffer) = rgpre::convert_animated_models_to_gltf(
        &[empty, static_model, model, shorter],
        None,
        None,
        false,
        4.,
    )
    .unwrap();
    let glb = rgpre::to_glb(&root, &buffer).unwrap();
    let decoded = gltf::Gltf::from_slice(&glb).unwrap();
    assert_eq!(decoded.animations().count(), 1);
    let animation = decoded.animations().next().unwrap();
    let channels: Vec<_> = animation.channels().collect();
    assert_eq!(channels.len(), 2);
    for (sampler_index, (channel, (node, end))) in
        channels.iter().zip([(1, 0.75), (2, 0.5)]).enumerate()
    {
        assert_eq!(channel.sampler().index(), sampler_index);
        assert_eq!(channel.target().node().index(), node);
        let gltf::animation::util::ReadOutputs::MorphTargetWeights(weights) = channel
            .reader(|_| decoded.blob.as_deref())
            .read_outputs()
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(weights.into_f32().last(), Some(1.0));
        assert_eq!(
            channel
                .reader(|_| decoded.blob.as_deref())
                .read_inputs()
                .unwrap()
                .last(),
            Some(end)
        );
    }
}
