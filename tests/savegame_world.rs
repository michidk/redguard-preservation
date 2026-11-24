use rgpre::import::savegame::{parse_save_families, parse_save_ropes, parse_save_spatial_grid};

#[test]
fn spatial_grid_retains_list_identity_and_cells() {
    let mut bytes = vec![0; 4 + 8 + 32768];
    bytes[..4].copy_from_slice(&1u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&42u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(-2i32).to_le_bytes());
    bytes[12..14].copy_from_slice(&42u16.to_le_bytes());
    let grid = parse_save_spatial_grid(&bytes).unwrap();
    assert_eq!(grid.lists[0].list_id, 42);
    assert_eq!(grid.lists[0].first_actor_id, -2);
    assert_eq!(grid.cells.len(), 16384);
    assert_eq!(grid.cells[0], 42);
    assert!(parse_save_spatial_grid(&bytes[..bytes.len() - 1]).is_err());
    bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_save_spatial_grid(&bytes).is_err());
}

#[test]
fn families_preserve_vectors_even_when_actor_is_absent() {
    let mut bytes = vec![0; 7168];
    bytes[..4].copy_from_slice(&(-1i32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(-123i32).to_le_bytes());
    bytes[24..28].copy_from_slice(&456i32.to_le_bytes());
    let families = parse_save_families(&bytes).unwrap();
    assert_eq!(families.len(), 256);
    assert_eq!(families[0].actor_id, -1);
    assert_eq!(families[0].position_q8, [-123, 0, 0]);
    assert_eq!(families[0].angles, [0, 0, 456]);
    assert!(parse_save_families(&bytes[..7167]).is_err());
}

#[test]
fn ropes_keep_owner_identity_separate_from_list_references() {
    let mut bytes = vec![0; 142];
    bytes[..4].copy_from_slice(&7u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&9u32.to_le_bytes());
    bytes[141] = 99;
    let ropes = parse_save_ropes(&bytes).unwrap();
    assert_eq!(ropes[0].rope_id, 7);
    assert_eq!(ropes[0].list_references(), [9, 0, 0]);
    assert_eq!(ropes[0].body[137], 99);
    assert!(parse_save_ropes(&bytes[..141]).is_err());
}

fn object() -> rgpre::import::savegame::SaveObject {
    rgpre::import::savegame::SaveObject {
        object_id: 1,
        rope_id: 0,
        record_kind: 0,
        body: [0; 783],
        named_resource: None,
        attributes: None,
        opaque_41: None,
        opaque_20: None,
        animation_header: None,
        script_state: None,
        attachment: None,
        vectors: [0; 48],
        opaque_60: None,
        private_variables: vec![],
        extension: vec![],
    }
}

#[test]
fn light_decodes_signed_strengths_and_retained_modulation_phase() {
    let mut object = object();
    let mut bytes = [0; 41];
    bytes[0..3].copy_from_slice(&[1, 0, 31]);
    for offset in (3..41).step_by(2) {
        bytes[offset..offset + 2].copy_from_slice(&(-(offset as i16)).to_le_bytes());
    }
    object.opaque_41 = Some(bytes);
    let light = object.light().unwrap();
    assert!(light.locked);
    assert!(!light.enabled);
    assert_eq!(light.inactive_ticks, 31);
    assert_eq!(light.offset, [-3, -5, -7]);
    assert_eq!(light.starting_intensity, -9);
    assert_eq!(light.intensity, -11);
    assert_eq!(light.starting_radius, -13);
    assert_eq!(light.radius, -15);
    assert_eq!(
        light.flicker,
        [(-17i16) as u16, (-19i16) as u16, (-21i16) as u16]
    );
    assert_eq!(light.size[1], (-25i16) as u16);
    assert_eq!(light.rgb, [-29, -31, -33]);
    assert_eq!(light.starting_rgb, [-35, -37, -39]);
}

#[test]
fn flat_excludes_process_cache_slot_and_preserves_playback() {
    let mut object = object();
    let mut bytes = [0; 20];
    bytes[..2].copy_from_slice(&[1, 31]);
    for offset in (2..18).step_by(2) {
        bytes[offset..offset + 2].copy_from_slice(&(offset as u16 * 13).to_le_bytes());
    }
    bytes[18..].copy_from_slice(&[3, 0x81]);
    object.opaque_20 = Some(bytes);
    let flat = object.flat().unwrap();
    assert!(flat.enabled);
    assert_eq!(flat.offset, [26, 52, 78]);
    assert_eq!(flat.texture, 104);
    assert_eq!(flat.last_frame, 156);
    assert_eq!(flat.counter, 182);
    assert_eq!(flat.frame, 208);
    assert_eq!(flat.animate, [3, 0x81]);
    object.opaque_20.as_mut().unwrap()[10..12].copy_from_slice(&0xffffu16.to_le_bytes());
    assert_eq!(object.flat().unwrap(), flat);
}

#[test]
fn rope_swing_state_is_independent_of_live_link_array_addresses() {
    let mut bytes = [0; 142];
    bytes[..4].copy_from_slice(&1u32.to_le_bytes());
    for (offset, value) in [
        (0x56, 21i32),
        (0x6a, 40000),
        (0x66, -22),
        (0x62, 3),
        (0x5e, -99),
    ] {
        bytes[4 + offset..8 + offset].copy_from_slice(&value.to_le_bytes());
    }
    bytes[4 + 0x38] = 1;
    let rope = parse_save_ropes(&bytes).unwrap().remove(0);
    let state = rope.swing();
    assert_eq!(
        (
            state.amplitude,
            state.phase,
            state.step,
            state.push,
            state.direction,
            state.previous_sum
        ),
        (21, 40000, -22, 3, 1, -99)
    );
}
