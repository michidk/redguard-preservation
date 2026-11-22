use rgpre::import::savegame::{encode_save_objects, load_save_slot, parse_save_objects};

#[test]
fn installed_object_fields_and_round_trip() {
    let Some(path) = std::env::var_os("REDGUARD_SAVE_SLOT") else {
        eprintln!("SKIP installed object fields: REDGUARD_SAVE_SLOT is absent");
        return;
    };
    let slot = load_save_slot(path).unwrap();
    for game in std::iter::once(&slot.game).chain(slot.maps.values()) {
        for section in &game.sections {
            if section.name != *b"SMOB" {
                continue;
            }
            let records = parse_save_objects(&section.data).unwrap();
            assert_eq!(encode_save_objects(&records).unwrap(), section.data);
            for player in records.iter().filter(|r| matches!(r.object_id, -2 | -3)) {
                assert!(player.script_state.is_some());
                if let Some(identity) = player.support_object_id() {
                    let owner = records
                        .iter()
                        .find(|record| record.object_id == identity)
                        .expect("saved player support record must be present");
                    assert_eq!(i32::from(owner.runtime_identifier()), identity);
                }
                assert_eq!(
                    player.position_q8(),
                    std::array::from_fn(|i| {
                        i32::from_le_bytes(
                            player.body[203 + i * 4..207 + i * 4].try_into().unwrap(),
                        )
                    })
                );
            }
        }
    }
}

#[test]
fn malformed_object_records_are_contextual_errors() {
    for bytes in [&[0u8][..], &[8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]] {
        assert!(
            parse_save_objects(bytes)
                .unwrap_err()
                .to_string()
                .contains("SMOB")
        );
    }
}

#[test]
fn installed_code_and_shell_round_trips() {
    use rgpre::import::savegame::*;
    let Some(path) = std::env::var_os("REDGUARD_SAVE_SLOT") else {
        eprintln!("SKIP installed code/shell state: REDGUARD_SAVE_SLOT is absent");
        return;
    };
    let slot = load_save_slot(path).unwrap();
    for game in std::iter::once(&slot.game).chain(slot.maps.values()) {
        for section in &game.sections {
            if section.name == *b"SMCO" {
                let owners = parse_save_code_owners(&section.data).unwrap();
                assert_eq!(encode_save_code_owners(&owners), section.data);
            }
            if section.name == *b"SMSH" {
                let shells = parse_save_shells(&section.data).unwrap();
                assert_eq!(encode_save_shells(&shells).unwrap(), section.data);
                assert!(shells.iter().all(|s| s.name().len() == 8));
            }
        }
    }
}

#[test]
fn edited_position_and_script_identity_survive_encoding() {
    use rgpre::import::savegame::*;
    // A synthetic full record: no optional tails, absent script owner, vectors.
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let size = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&size.to_le_bytes());
    bytes[4..8].copy_from_slice(&(-2i32).to_le_bytes());
    bytes[13 + 0x23b..17 + 0x23b].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut objects = parse_save_objects(&bytes).unwrap();
    let position = [-123456i32, 789, 654321];
    for (i, value) in position.iter().enumerate() {
        objects[0].body[203 + i * 4..207 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    assert_eq!(
        parse_save_objects(&encode_save_objects(&objects).unwrap()).unwrap()[0].position_q8(),
        position
    );
    objects[0].body[0x23b..0x23f].copy_from_slice(&0u32.to_le_bytes());
    // Zero is a present script identity, so omitting its tail must be rejected.
    assert!(encode_save_objects(&objects).is_err());
    objects[0].script_state = Some([0; 422]);
    let encoded = encode_save_objects(&objects).unwrap();
    assert_eq!(parse_save_objects(&encoded).unwrap(), objects);
    for end in 1..encoded.len() {
        assert!(parse_save_objects(&encoded[..end]).is_err(), "prefix {end}");
    }
}

#[test]
fn code_and_shell_fields_and_invalid_lengths() {
    use rgpre::import::savegame::*;
    let mut bytes = vec![0; 208];
    bytes[..4].copy_from_slice(&123u32.to_le_bytes());
    let owners = parse_save_code_owners(&bytes).unwrap();
    assert_eq!(owners[0].owner_id, 123);
    assert_eq!(encode_save_code_owners(&owners), bytes);
    assert!(parse_save_code_owners(&bytes[..207]).is_err());
    let mut shell = SaveShell {
        body: [0; 186],
        variables: vec![42, 0, 0, 0],
    };
    shell.body[..8].copy_from_slice(b"SHELL001");
    shell.body[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
    let encoded = encode_save_shells(std::slice::from_ref(&shell)).unwrap();
    assert_eq!(parse_save_shells(&encoded).unwrap(), vec![shell.clone()]);
    assert_eq!(shell.name(), b"SHELL001");
    for end in 0..encoded.len() {
        assert!(parse_save_shells(&encoded[..end]).is_err());
    }
    shell.variables.clear();
    assert!(encode_save_shells(&[shell]).is_err());
}

#[test]
fn relative_references_keep_their_saved_values() {
    use rgpre::import::savegame::*;
    let mut owner = SaveCodeOwner {
        owner_id: 7,
        body: [0; 204],
    };
    owner.body[0x1f..0x23].copy_from_slice(&11u32.to_le_bytes());
    owner.body[0x23..0x27].copy_from_slice(&22u32.to_le_bytes());
    owner.body[0x42..0x46].copy_from_slice(&33u32.to_le_bytes());
    let refs = owner.script_references();
    assert_eq!(refs.native_hook, 11);
    assert_eq!(refs.scripts, [22, 0, 0, 0]);
    assert_eq!(refs.callbacks, [33, 0, 0, 0, 0, 0]);
    let encoded = encode_save_code_owners(std::slice::from_ref(&owner));
    assert_eq!(
        parse_save_code_owners(&encoded).unwrap()[0].script_references(),
        refs
    );
}

#[test]
fn saved_support_identity_and_pose_round_trip() {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let size = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&size.to_le_bytes());
    bytes[13 + 0x23b..17 + 0x23b].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut objects = parse_save_objects(&bytes).unwrap();
    let actor = &mut objects[0];
    actor.body[20..22].copy_from_slice(&17u16.to_le_bytes());
    actor.body[468..472].copy_from_slice(&23i32.to_le_bytes());
    for (axis, angle) in [7i32, 1024, -2].into_iter().enumerate() {
        actor.body[288 + axis * 4..292 + axis * 4].copy_from_slice(&angle.to_le_bytes());
    }
    let decoded = parse_save_objects(&encode_save_objects(&objects).unwrap()).unwrap();
    assert_eq!(decoded[0].runtime_identifier(), 17);
    assert_eq!(decoded[0].orientation(), [7, 1024, -2]);
    assert_eq!(decoded[0].support_object_id(), Some(23));
    objects[0].body[468..472].copy_from_slice(&(-1i32).to_le_bytes());
    assert_eq!(objects[0].support_object_id(), None);
    objects[0].body[468..472].copy_from_slice(&0i32.to_le_bytes());
    assert_eq!(objects[0].support_object_id(), Some(0));
}
