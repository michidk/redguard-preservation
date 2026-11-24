use rgpre::import::savegame::{
    SaveActorBinding, SaveActorOwners, load_save_slot, parse_save_objects,
};

fn objects(variable: &str) -> Option<Vec<rgpre::import::savegame::SaveObject>> {
    let Some(path) = std::env::var_os(variable) else {
        eprintln!("skipping retained actor state: {variable} unset");
        return None;
    };
    let slot = load_save_slot(path).unwrap();
    let section = slot
        .game
        .sections
        .iter()
        .find(|s| s.name == *b"SMOB")
        .unwrap();
    Some(parse_save_objects(&section.data).unwrap())
}

#[test]
fn retained_kotaro_preserves_independent_histories_and_movement() {
    let Some(objects) = objects("REDGUARD_ACTOR_KOTARO_SAVE") else {
        return;
    };
    let initialized: Vec<_> = objects
        .iter()
        .map(|o| SaveActorBinding {
            object_id: o.object_id,
            runtime_identifier: o.runtime_identifier(),
        })
        .collect();
    let owners = SaveActorOwners::bind(&objects, &initialized).unwrap();
    let actor = objects.iter().find(|o| o.object_id == 243).unwrap();
    let state = actor.actor_state().unwrap();
    assert_eq!(state.spatial.angles, [0, 768, 103]);
    assert_eq!(state.spatial.previous_angles, [0, 768, 124]);
    assert_eq!(state.spatial.initial_angles, [0, 1792, 0]);
    assert_eq!(state.spatial.position_q8, [10127732, -193902, 9503496]);
    assert_eq!(
        state.spatial.previous_position_q8,
        [10131534, -195439, 9507297]
    );
    let player = objects
        .iter()
        .find(|o| o.object_id == -2)
        .unwrap()
        .actor_state()
        .unwrap();
    assert_eq!(player.controller.movement_counter, 48);
    assert_eq!(
        player.spatial.stable_position_q8,
        Some([9570252, -98304, 10322453])
    );
    assert_eq!(
        owners.receiver(243).unwrap(),
        initialized.iter().position(|o| o.object_id == 243).unwrap()
    );
    assert!(SaveActorOwners::bind(&objects, &initialized[1..]).is_err());
    let mut duplicate = initialized.clone();
    duplicate.push(initialized[0]);
    assert!(SaveActorOwners::bind(&objects, &duplicate).is_err());
    let mut wrong = initialized.clone();
    wrong[0].runtime_identifier ^= 1;
    assert!(SaveActorOwners::bind(&objects, &wrong).is_err());
}

#[test]
fn retained_favis_player_keeps_stable_recovery_and_creation_angles() {
    let Some(objects) = objects("REDGUARD_ACTOR_FAVIS_SAVE") else {
        return;
    };
    let player = objects
        .iter()
        .find(|o| o.object_id == -2)
        .unwrap()
        .actor_state()
        .unwrap();
    assert_eq!(player.spatial.position_q8, [562979, -16384, 590441]);
    assert_eq!(
        player.spatial.stable_position_q8,
        Some(player.spatial.position_q8)
    );
    assert_eq!(player.spatial.angles, [0, 1478, 0]);
    assert_eq!(player.spatial.previous_angles, [0, 1478, 0]);
    assert_eq!(player.spatial.initial_angles, [0, 0, 0]);
    assert_eq!(player.controller.movement_counter, 0);
}

fn synthetic() -> rgpre::import::savegame::SaveObject {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    bytes[..4].copy_from_slice(&840u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&7i32.to_le_bytes());
    let body = &mut bytes[13..796];
    for offset in [8, 12, 0x1d4, 0x23b] {
        body[offset..offset + 4].copy_from_slice(&(-1i32).to_le_bytes());
    }
    parse_save_objects(&bytes).unwrap().remove(0)
}

#[test]
fn raw_histories_and_controller_words_are_retained_without_normalization() {
    let mut object = synthetic();
    for (offset, value) in [
        (0x120, -4097i32),
        (0x144, 9999),
        (0x108, -713),
        (0x114, 31415),
        (0x1b6, -1),
        (0x1ba, i32::MIN),
    ] {
        object.body[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    object.body[0x107] = 255;
    object.body[0x1cc] = 255;
    object.body[0x1fe] = 123;
    object.body[0x1ff] = 87;
    object.body[0x51..0x55].copy_from_slice(&0.75f32.to_bits().to_le_bytes());
    object.body[0x39..0x3d].copy_from_slice(&(-0.5f32).to_bits().to_le_bytes());
    let state = object.actor_state().unwrap();
    assert_eq!(state.spatial.angles[0], -4097);
    assert_eq!(state.spatial.initial_angles[0], 9999);
    assert_eq!(state.spatial.water_rollback_position_q8, [31415, 0, 0]);
    assert_eq!(state.spatial.stable_validity, 255);
    assert_eq!(state.spatial.stable_position_q8, Some([-713, 0, 0]));
    assert_eq!(state.controller.pending_action, u32::MAX);
    assert_eq!(state.controller.accepted_action, 1 << 31);
    assert_eq!(state.controller.movement_counter, 255);
    assert_eq!(state.controller.torch_timer, 123);
    assert_eq!(state.controller.extinguished_torch_timer, 87);
    assert_eq!(state.model_transform.matrix()[0][0], 0.75);
    assert_eq!(state.model_transform.translation()[0], -0.5);
    object.body[0x107] = 0;
    let cleared = object.actor_state().unwrap();
    assert_eq!(cleared.spatial.stable_position_q8, None);
    assert_eq!(cleared.spatial.stored_stable_position_q8, [-713, 0, 0]);
}

#[test]
fn foreign_references_duplicate_saved_owners_and_missing_link_storage_reject() {
    let mut object = synthetic();
    let initialized = [SaveActorBinding {
        object_id: 7,
        runtime_identifier: 0,
    }];
    assert!(SaveActorOwners::bind(std::slice::from_ref(&object), &initialized).is_ok());
    for offset in [8, 12, 0x1d4] {
        object.body[offset..offset + 4].copy_from_slice(&99i32.to_le_bytes());
        let error = SaveActorOwners::bind(std::slice::from_ref(&object), &initialized)
            .unwrap_err()
            .to_string();
        assert!(error.contains("actor 7") && error.contains("99"), "{error}");
        object.body[offset..offset + 4].copy_from_slice(&(-1i32).to_le_bytes());
    }
    object.body[0x21..0x25].copy_from_slice(&1u32.to_le_bytes());
    assert!(object.actor_state().is_err());
    let mut link = [0; 60];
    link[..4].copy_from_slice(&99i32.to_le_bytes());
    object.opaque_60 = Some(link);
    assert!(SaveActorOwners::bind(std::slice::from_ref(&object), &initialized).is_err());
    object.body[0x21..0x25].copy_from_slice(&0u32.to_le_bytes());
    object.opaque_60 = None;
    assert!(
        SaveActorOwners::bind(
            &[object.clone(), object],
            &[
                initialized[0],
                SaveActorBinding {
                    object_id: 8,
                    runtime_identifier: 0
                }
            ]
        )
        .is_err()
    );
}

#[test]
fn position_link_preserves_detached_independent_caches() {
    let mut object = synthetic();
    object.body[0x21..0x25].copy_from_slice(&1u32.to_le_bytes());
    let mut record = [0u8; 60];
    for (index, value) in [-1i32, 11, 12, 13, 21, 22, 23, 31, 32, 33]
        .into_iter()
        .enumerate()
    {
        record[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    object.opaque_60 = Some(record);
    let link = object.position_link().unwrap().unwrap();
    assert_eq!(link.owner, None);
    assert_eq!(link.offset_q8, [11, 12, 13]);
    assert_eq!(link.position_q8, [21, 22, 23]);
    assert_eq!(link.previous_position_q8, [31, 32, 33]);
    object.opaque_60 = None;
    assert!(object.position_link().is_err());
}
