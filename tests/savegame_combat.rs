use rgpre::import::savegame::{SaveObject, load_save_slot, parse_save_objects};
fn object() -> SaveObject {
    SaveObject {
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
fn combat_is_a_distinct_retained_owner_and_applies_loader_flag_reset() {
    let mut object = object();
    assert!(object.combat().is_none());
    let mut bytes = [0; 422];
    for (offset, value) in [
        (0, 9i32),
        (4, -2),
        (8, 43),
        (12, 7),
        (0x28, -8192),
        (0x38, -1),
        (0x48, 3),
        (0x4c, 2),
        (0x58, 81),
        (0x5c, 24),
        (0x62, -512),
        (0x139, 47),
        (0x141, 11),
        (0x145, 1),
        (0x14d, 2),
        (0x151, 3),
        (0x159, -1),
        (0x161, 51),
        (0x165, 52),
        (0x169, 53),
        (0x179, 1),
        (0x17d, -1),
        (0x185, 71),
        (0x189, 72),
        (0x18e, 8),
        (0x192, 10),
        (0x196, 12),
        (0x19a, 99),
        (0x19e, 1),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[0x60..0x62].copy_from_slice(&(-3i16).to_le_bytes());
    bytes[0x18d] = 0xff;
    object.script_state = Some(bytes);
    let state = object.combat().unwrap();
    assert_eq!(
        (
            state.next_combat_id,
            state.owner_id,
            state.target_id,
            state.allocation_index
        ),
        (9, -2, 43, 7)
    );
    assert_eq!(state.retained_target_position_q8, [-8192, 0, 0]);
    assert_eq!((state.action, state.retained_action_indices), (-1, [3, 2]));
    assert_eq!((state.contact_vertex, state.contact_radius_q8), (-3, -512));
    assert_eq!(state.cooldown, u32::MAX);
    assert_eq!(state.flags, 0xff);
    assert_eq!(state.restored_flags(), 0xbf);
    assert_eq!(
        (state.avoid_side, state.avoid_mask, state.avoid_hold),
        (8, 10, 12)
    );
    assert_eq!((state.lost_sight_ticks, state.lost_sight_routing), (99, 1));
    assert_eq!(
        (
            state.attack_pacing,
            state.defence_pacing,
            state.defence_counter
        ),
        (51, 52, 53)
    );
}

#[test]
fn retained_saves_bind_combat_owners_to_actual_saved_actors() {
    for variable in ["REDGUARD_ACTOR_KOTARO_SAVE", "REDGUARD_ACTOR_FAVIS_SAVE"] {
        let Some(path) = std::env::var_os(variable) else {
            eprintln!("SKIP combat retained save: {variable} absent");
            continue;
        };
        let slot = load_save_slot(path).unwrap();
        let bytes = &slot
            .game
            .sections
            .iter()
            .find(|s| s.name == *b"SMOB")
            .unwrap()
            .data;
        let objects = parse_save_objects(bytes).unwrap();
        let mut count = 0;
        for object in &objects {
            if let Some(combat) = object.combat() {
                count += 1;
                assert_eq!(combat.owner_id, object.object_id);
                if ![-1, 0].contains(&combat.target_id) {
                    assert!(
                        objects
                            .iter()
                            .any(|other| other.object_id == combat.target_id)
                    );
                }
                assert_eq!(combat.restored_flags() & 0x40, 0);
            }
        }
        assert!(count > 0);
    }
}
