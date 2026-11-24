use rgpre::import::savegame::{SaveActorOwners, encode_save_cameras, parse_save_cameras};

fn cameras() -> Vec<u8> {
    let mut data = vec![0; 1016];
    data[..4].copy_from_slice(&9u32.to_le_bytes());
    data
}

#[test]
fn retains_values_and_archival_pointer_bytes() {
    let mut data = cameras();
    for offset in (0..112).step_by(4) {
        data[4 + offset..8 + offset].copy_from_slice(&(offset as i32 - 80).to_le_bytes());
    }
    data[1012..].copy_from_slice(&2u32.to_le_bytes());
    let saved = parse_save_cameras(&data).unwrap();
    assert_eq!(saved.selected_record, 2);
    let values = saved.records[0].values();
    assert_eq!(values.position_offset, [-80, -76, -72]);
    assert_eq!(values.angle_offset, [-68, -64, -60]);
    assert_eq!(values.request, [-56, -52, -48]);
    assert_eq!(values.basis, [[-44, -40, -36], [-32, -28, -24]]);
    assert_eq!(values.bound_position_history, [-16, -12, -8]);
    assert_eq!(saved.records[0].owner(), None);
    assert_eq!(values.target_offset, [4, 8, 12]);
    assert_eq!(values.distance, 16);
    assert_eq!(values.minimum_distance, 20);
    assert_eq!(values.flags, 24);
    assert_eq!(values.maximum_movement, 28);
    assert_eq!(encode_save_cameras(&saved).unwrap(), data);
}

#[test]
fn rejects_incomplete_foreign_or_invalid_selection() {
    let data = cameras();
    assert!(parse_save_cameras(&data[..1015]).is_err());
    let mut invalid = data.clone();
    invalid[..4].copy_from_slice(&8u32.to_le_bytes());
    assert!(parse_save_cameras(&invalid).is_err());
    invalid = data;
    invalid[1012..].copy_from_slice(&9u32.to_le_bytes());
    assert!(parse_save_cameras(&invalid).is_err());
}

#[test]
fn owner_binding_does_not_interpret_saved_pointers() {
    let owners = SaveActorOwners::bind(&[], &[]).unwrap();
    let mut data = cameras();
    for null in [0i32, -1] {
        data[84..88].copy_from_slice(&null.to_le_bytes());
        let saved = parse_save_cameras(&data).unwrap();
        assert!(saved.records[0].bind(&owners, [1, 2, 3]).unwrap().is_none());
    }
    data[84..88].copy_from_slice(&7i32.to_le_bytes());
    let saved = parse_save_cameras(&data).unwrap();
    assert_eq!(saved.records[0].owner(), Some(7));
    assert!(saved.records[0].bind(&owners, [1, 2, 3]).is_err());
}

#[test]
fn retained_camera_binding_keeps_initialized_angles() {
    let Some(path) = std::env::var_os("REDGUARD_ACTOR_KOTARO_SAVE") else {
        eprintln!("skipping retained camera binding: REDGUARD_ACTOR_KOTARO_SAVE unset");
        return;
    };
    use rgpre::import::savegame::{SaveActorBinding, load_save_slot, parse_save_objects};
    let slot = load_save_slot(path).unwrap();
    let section = |name| {
        &slot
            .game
            .sections
            .iter()
            .find(|section| section.name == name)
            .unwrap()
            .data
    };
    let objects = parse_save_objects(section(*b"SMOB")).unwrap();
    let initialized: Vec<_> = objects
        .iter()
        .map(|object| SaveActorBinding {
            object_id: object.object_id,
            runtime_identifier: object.runtime_identifier(),
        })
        .collect();
    let owners = SaveActorOwners::bind(&objects, &initialized).unwrap();
    let saved = parse_save_cameras(section(*b"SVCM")).unwrap();
    let angles = std::array::from_fn(|index| [index as i32, 321, -987]);
    let bound = saved.bind(&owners, angles).unwrap();
    assert!(bound[saved.selected_record].is_some());
    for (index, record) in bound.iter().enumerate() {
        if let Some(record) = record {
            let mut expected = saved.records[index].values();
            expected.angle_offset = angles[index];
            assert_eq!(record.values, expected);
            assert_eq!(
                record.actor_receiver,
                owners
                    .receiver(saved.records[index].owner().unwrap())
                    .unwrap()
            );
        }
    }
}

#[test]
fn camera_globals_use_separate_payloads_and_low_index_byte() {
    use rgpre::import::savegame::parse_save_camera_globals;
    let mut svgv = vec![0; 1542];
    let mut svg2 = vec![0; 1666];
    svgv[0x24e..0x252].copy_from_slice(&0xaabbcc03u32.to_le_bytes());
    svgv[0x18c..0x190].copy_from_slice(&(-123i32).to_le_bytes());
    svgv[0x5fa..0x5fe].copy_from_slice(&456i32.to_le_bytes());
    svg2[8..10].copy_from_slice(&0x8042u16.to_le_bytes());
    let values = parse_save_camera_globals(&svgv, &svg2).unwrap();
    assert_eq!(values.index, 3);
    assert_eq!(values.current_position, [-123, 0, 0]);
    assert_eq!(values.render_position, [456, 0, 0]);
    assert_eq!(values.selected_flags, 0x8042);
    svgv[0x24e] = 9;
    assert!(parse_save_camera_globals(&svgv, &svg2).is_err());
    assert!(parse_save_camera_globals(&svgv[..1541], &svg2).is_err());
}
