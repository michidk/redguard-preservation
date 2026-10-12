use rgpre::import::rgm::{export_rgm_metadata_json, extract_rgm_placements, parse_rgm_file};

fn scene() -> Vec<u8> {
    let mut payload = 4_u32.to_le_bytes().to_vec();
    for (world, active) in [(20_u32, 1_u8), (1, 1), (20, 0), (20, 2)] {
        payload.extend(0x8321_4567_u32.to_le_bytes());
        payload.extend(world.to_le_bytes());
        for coordinate in [1024_i32, -144, -16_777_217] {
            payload.extend(coordinate.to_le_bytes());
        }
        for value in [0_i16, 800, -255, 128, 255] {
            payload.extend(value.to_le_bytes());
        }
        payload.extend([9, active, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }
    let mut data = b"MPSL".to_vec();
    data.extend(u32::try_from(payload.len()).unwrap().to_be_bytes());
    data.extend(payload);
    data.extend(b"END \0\0\0\0");
    data
}

#[test]
fn installed_light_contract_preserves_signed_fields_and_selects_exact_world() {
    let world = parse_rgm_file(&scene()).unwrap();
    let lights = world.local_lights(20).collect::<Vec<_>>();
    assert_eq!(lights.len(), 1);
    let light = lights[0];
    assert_eq!(light.link, 0x8321_4567);
    assert_eq!(light.position, [1024, -144, -16_777_217]);
    assert_eq!(light.radius, 0);
    assert_eq!(light.field_16, 800);
    assert_eq!(light.rgb, [-255, 128, 255]);
    assert_eq!(light.field_1e, 9);
    assert_eq!(light.field_20, [2, 3, 4, 5, 6, 7]);
    assert_eq!(light.field_26, [8, 9, 10, 11]);
    assert_eq!(world.local_lights(1).count(), 1);
    assert_eq!(world.local_lights(99).count(), 0);

    let metadata = export_rgm_metadata_json(&world, None);
    let light = &metadata["lights"][0];
    assert_eq!(
        light["engine_position"],
        serde_json::json!([1024, -144, -16_777_217])
    );
    assert_eq!(light["rgb"], serde_json::json!([-255, 128, 255]));
    assert_eq!(light["world_id"], 20);
    assert_eq!(light["active"], 1);
    assert!(light.get("direction_attenuation").is_none());
    assert!(light.get("color_r").is_none());
}

#[test]
fn map_lights_are_not_misrepresented_as_standard_point_lights() {
    let (_, lights) = extract_rgm_placements(&scene()).unwrap();
    assert!(lights.is_empty());
}
