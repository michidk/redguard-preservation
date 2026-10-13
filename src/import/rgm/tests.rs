use super::*;

fn approx_eq(a: f32, b: f32) {
    let diff = (a - b).abs();
    assert!(diff < 1.0e-6, "expected {b}, got {a}, diff {diff}");
}

#[test]
fn decode_position_applies_fixed_point_scaling_on_all_axes() {
    let pos = positioning::decode_position(1, -2, 3);

    approx_eq(pos[0], -(256.0 / 5120.0));
    approx_eq(pos[1], 512.0 / 5120.0);
    approx_eq(pos[2], -((0x00FF_FFFF_i64 - (3_i64 * 256)) as f32) / 5120.0);
}

#[test]
fn decode_position_does_not_collapse_small_values_to_origin() {
    let pos = positioning::decode_position(1, 1, 1);

    assert!(pos[0].abs() > 0.01, "x too close to origin: {}", pos[0]);
    assert!(pos[1].abs() > 0.01, "y too close to origin: {}", pos[1]);
}

#[test]
fn marker_metadata_reads_positions_before_auxiliary_bytes() {
    let mut data = 2u32.to_le_bytes().to_vec();
    for coordinate in [20i32, -40, 65536, 100, 60, -3] {
        data.extend_from_slice(&coordinate.to_le_bytes());
    }
    data.extend_from_slice(&[7, 9]);
    let rgm = RgmFile {
        sections: vec![RgmSection::Mpm(
            RgmSectionHeader {
                name: *b"MPMK",
                data_length: 30,
                record_count: None,
            },
            data,
        )],
    };
    let metadata = export_rgm_metadata_json(&rgm, None);
    let markers = metadata["markers"].as_array().unwrap();
    assert_eq!(markers.len(), 2);
    assert_eq!(
        markers[0]["map_position"],
        serde_json::json!([20, -40, 65536])
    );
    assert_eq!(markers[1]["map_position"], serde_json::json!([100, 60, -3]));
    assert_eq!(markers[0]["position"][0], -1.0);
    assert_eq!(markers[0]["position"][1], 2.0);
    assert_eq!(markers[1]["position"][0], -5.0);
    assert_eq!(markers[1]["position"][1], -3.0);
    assert_eq!(markers[0]["reserved"], 7);
    assert_eq!(markers[1]["reserved"], 9);
}

#[test]
fn marker_metadata_rejects_truncated_tables_before_allocating() {
    let rgm = RgmFile {
        sections: vec![RgmSection::Mpm(
            RgmSectionHeader {
                name: *b"MPMK",
                data_length: 4,
                record_count: None,
            },
            u32::MAX.to_le_bytes().to_vec(),
        )],
    };
    let metadata = export_rgm_metadata_json(&rgm, None);
    assert_eq!(metadata["markers"], serde_json::json!([]));
}

#[test]
fn actor_metadata_preserves_model_loader_branch_conditions() {
    for (models, prefix, frames, expected) in [
        (0i32, -1i32, 0i32, "none"),
        (1, -1, 0, "static"),
        (1, -1, 1, "static"),
        (1, -1, -1, "static"),
        (1, -1, 2, "animation_table"),
        (1, 255, 0, "animation_table"),
        (1, 0, 0, "animation_table"),
        (2, -1, 0, "animation_table"),
    ] {
        let mut data = vec![0; 8 + 165];
        data[..4].copy_from_slice(&1u32.to_le_bytes());
        let record = &mut data[8..];
        record[4..9].copy_from_slice(b"ACTOR");
        record[0x21..0x25].copy_from_slice(&models.to_le_bytes());
        record[0x2D..0x31].copy_from_slice(&prefix.to_le_bytes());
        record[0x85..0x89].copy_from_slice(&frames.to_le_bytes());
        record[0x9D..0xA1].copy_from_slice(&18_i32.to_le_bytes());
        record[0xA1..0xA5].copy_from_slice(&3_i32.to_le_bytes());
        let rgm = RgmFile {
            sections: vec![RgmSection::Rahd(
                RgmSectionHeader {
                    name: *b"RAHD",
                    data_length: 173,
                    record_count: None,
                },
                data,
            )],
        };
        let metadata = export_rgm_metadata_json(&rgm, None);
        assert_eq!(metadata["actors"][0]["model_load_path"], expected);
        assert_eq!(metadata["actors"][0]["ravc_offset"], 18);
        assert_eq!(metadata["actors"][0]["ravc_count"], 3);
        assert_eq!(metadata["sections"][0]["tag"], "RAHD");
        assert_eq!(metadata["sections"][0]["data_length"], 173);
    }
}

#[test]
fn actor_collision_metadata_requires_a_valid_record_range() {
    for (offset, count, expected) in [
        (0_i32, 1_i32, true),
        (1, 1, false),
        (9, 1, false),
        (-1, 1, false),
        (0, -1, false),
    ] {
        let mut data = vec![0; 173];
        data[..4].copy_from_slice(&1_u32.to_le_bytes());
        data[12..17].copy_from_slice(b"ACTOR");
        data[8 + 0x9D..8 + 0xA1].copy_from_slice(&offset.to_le_bytes());
        data[8 + 0xA1..8 + 0xA5].copy_from_slice(&count.to_le_bytes());
        let rgm = RgmFile {
            sections: vec![
                RgmSection::Rahd(
                    RgmSectionHeader {
                        name: *b"RAHD",
                        data_length: 173,
                        record_count: None,
                    },
                    data,
                ),
                RgmSection::RavcParsed(
                    RgmSectionHeader {
                        name: *b"RAVC",
                        data_length: 9,
                        record_count: None,
                    },
                    vec![super::RavcRecord {
                        offset_x: -1,
                        offset_y: 2,
                        offset_z: 3,
                        vertex: 7,
                        radius: 42,
                    }],
                ),
            ],
        };
        let metadata = export_rgm_metadata_json(&rgm, None);
        assert_eq!(
            metadata["actors"][0].get("collision_volumes").is_some(),
            expected
        );
        if expected {
            assert_eq!(metadata["actors"][0]["collision_volumes"][0]["radius"], 42);
        }
    }
}

#[test]
fn pvo_world_centers_include_the_map_origin_translation() {
    let center = super::world_position([35584.0, -16384.0, 36608.0]);
    assert!((center[0] - -1779.2).abs() < 0.001);
    assert!((center[1] - 819.2).abs() < 0.001);
    assert!((center[2] - -1446.3998).abs() < 0.001);
    for (actual, expected) in center
        .into_iter()
        .zip(super::positioning::decode_position(35584, -16384, 36608))
    {
        assert!((actual - expected).abs() < 0.001);
    }
}
