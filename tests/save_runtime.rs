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
            for actor in &records {
                let slots = actor.task_slots();
                assert_eq!(
                    slots.map(|slot| slot.callback),
                    actor.script_references().callbacks
                );
                for (index, slot) in slots.iter().enumerate() {
                    assert_eq!(
                        slot.timer,
                        u16::from_le_bytes(
                            actor.body[699 + index * 2..701 + index * 2]
                                .try_into()
                                .unwrap()
                        )
                    );
                    assert_eq!(slot.status, actor.body[687 + index]);
                    assert_eq!(slot.pause, actor.body[693 + index]);
                }
                assert_eq!(
                    actor.private_variable_words().len() * 4,
                    actor.private_variables.len()
                );
            }
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

#[test]
fn task_continuation_survives_round_trip_and_first_explicit_frame() {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let size = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&size.to_le_bytes());
    bytes[13 + 0x23b..17 + 0x23b].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut objects = parse_save_objects(&bytes).unwrap();
    let actor = &mut objects[0];
    actor.body[0x27f..0x283].copy_from_slice(&203u32.to_le_bytes());
    actor.body[0x2af] = 1;
    actor.body[0x2bb..0x2bd].copy_from_slice(&12u16.to_le_bytes());
    actor.body[0x2b0] = 7;
    actor.body[0x2b6] = 2;
    actor.body[0x2bd..0x2bf].copy_from_slice(&u16::MAX.to_le_bytes());
    actor.body[0x1c..0x20].copy_from_slice(&1u32.to_le_bytes());
    actor.body[0x20] = 2;
    actor.private_variables = [(-1i32).to_le_bytes(), 37i32.to_le_bytes()].concat();
    let encoded = encode_save_objects(&objects).unwrap();
    let decoded = parse_save_objects(&encoded).unwrap();
    assert_eq!(encode_save_objects(&decoded).unwrap(), encoded);
    let tasks = decoded[0].task_slots();
    assert_eq!(tasks[0].callback, 203);
    assert_eq!(
        (tasks[0].status, tasks[0].pause, tasks[0].timer),
        (1, 0, 12)
    );
    assert_eq!(tasks[0].elapsed(118), 5);
    assert_eq!(
        tasks[0].timer, 12,
        "observing the next frame must not consume it"
    );
    assert_eq!(
        (
            tasks[1].callback,
            tasks[1].status,
            tasks[1].pause,
            tasks[1].timer
        ),
        (0, 7, 2, u16::MAX)
    );
    assert_eq!(
        tasks[1].elapsed(u32::MAX),
        (u32::from(u16::MAX).wrapping_mul(u32::MAX) >> 8) as i32
    );
    assert_eq!(decoded[0].private_variable_words(), [-1, 37]);
}

#[test]
fn actor_invocation_words_and_script_results_survive_lossless_save_round_trip() {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let size = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&size.to_le_bytes());
    bytes[13 + 0x23b..17 + 0x23b].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut objects = parse_save_objects(&bytes).unwrap();
    let actor = &mut objects[0];
    let references = [101u32, 203, 307, 409];
    for (index, reference) in references.into_iter().enumerate() {
        actor.body[0x260 + index * 4..0x264 + index * 4].copy_from_slice(&reference.to_le_bytes());
    }
    actor.body[0x270] = 0x81;
    actor.body[0x271] = 0xfe;
    // Every slot and every parallel word array has a distinct bit pattern,
    // including empty/paused slots. Do not normalize pointers or signed words.
    for slot in 0..6 {
        for (array, base) in [0x297, 0x2c7, 0x2df, 0x2f7].into_iter().enumerate() {
            let value = 0x8000_0000u32 | (array as u32) << 16 | slot as u32;
            actor.body[base + slot * 4..base + slot * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        actor.body[0x2af + slot] = slot as u8;
        actor.body[0x2b5 + slot] = (slot * 2) as u8;
    }
    let encoded = encode_save_objects(&objects).unwrap();
    let decoded = parse_save_objects(&encoded).unwrap();
    let continuation = decoded[0].script_continuation();
    assert_eq!(
        [
            continuation.data,
            continuation.entry,
            continuation.cursor,
            continuation.return_to
        ],
        references
    );
    assert_eq!(
        (continuation.result, continuation.saved_result),
        (0x81, 0xfe)
    );
    for (slot, task) in decoded[0].task_slots().iter().enumerate() {
        assert_eq!(task.callback, 0);
        assert_eq!((task.status, task.pause), (slot as u8, (slot * 2) as u8));
        for (array, value) in task.variables.iter().enumerate() {
            assert_eq!(*value, 0x8000_0000u32 | (array as u32) << 16 | slot as u32);
        }
    }
    assert_eq!(encode_save_objects(&decoded).unwrap(), encoded);
}

#[test]
fn authored_locals_bind_to_saved_actor_ranges_not_private_tails() {
    use rgpre::import::savegame::parse_save_script_locals;
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    let size = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&size.to_le_bytes());
    bytes[13 + 0x23b..17 + 0x23b].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut actors = parse_save_objects(&bytes).unwrap();
    let actor = &mut actors[0];
    actor.object_id = -2;
    actor.body[20..22].copy_from_slice(&1u16.to_le_bytes());
    actor.body[0x276..0x27a].copy_from_slice(&4u32.to_le_bytes());
    actor.body[0x1c..0x20].copy_from_slice(&1u32.to_le_bytes());
    actor.body[0x20] = 1;
    actor.private_variables = 77i32.to_le_bytes().to_vec();
    let payload = [0i32, i32::MIN, -1, i32::MAX]
        .map(i32::to_le_bytes)
        .concat();
    let region = parse_save_script_locals(&payload).unwrap();
    let bound = region.bind_actor(actor, 3).unwrap().unwrap();
    assert_eq!(
        (bound.object_id, bound.runtime_identifier, bound.byte_offset),
        (-2, 1, 4)
    );
    assert_eq!(bound.words, [i32::MIN, -1, i32::MAX]);
    assert_eq!(actor.private_variable_words(), [77]);
    assert!(region.bind_actor(actor, 4).is_err());
    actor.body[0x276..0x27a].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(region.bind_actor(actor, 1).unwrap().unwrap().words, [0]);
    actor.body[0x276..0x27a].copy_from_slice(&3u32.to_le_bytes());
    assert!(region.bind_actor(actor, 1).is_err());
    actor.body[0x276..0x27a].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(region.bind_actor(actor, 0).unwrap().is_none());
    assert!(region.bind_actor(actor, 1).is_err());
    assert!(parse_save_script_locals(&payload[..15]).is_err());
}

#[test]
fn installed_authored_locals_use_initialized_script_counts() {
    use rgpre::import::{rgm, savegame::parse_save_script_locals};
    let (Some(root), Some(path)) = (
        std::env::var_os("REDGUARD_DIR"),
        std::env::var_os("REDGUARD_SAVE_SLOT"),
    ) else {
        eprintln!("SKIP installed authored locals: REDGUARD_DIR/REDGUARD_SAVE_SLOT absent");
        return;
    };
    let slot = load_save_slot(path).unwrap();
    let (map_name, actor_id, actor_name) = match slot.game.metadata.world_id {
        1 => ("ISLAND.RGM", 303, "KOTARO"),
        20 => ("BELLTOWR.RGM", 37, "FAVIS"),
        world => panic!("authored-local fixture world {world} is unsupported"),
    };
    let map = rgm::parse_rgm_file(
        &std::fs::read(std::path::Path::new(&root).join("maps").join(map_name)).unwrap(),
    )
    .unwrap();
    let scripts = rgm::disassemble_rgm_scripts(&map, None);
    let objects = parse_save_objects(
        &slot
            .game
            .sections
            .iter()
            .find(|s| s.name == *b"SMOB")
            .unwrap()
            .data,
    )
    .unwrap();
    let region = parse_save_script_locals(
        &slot
            .game
            .sections
            .iter()
            .find(|s| s.name == *b"SVRG")
            .unwrap()
            .data,
    )
    .unwrap();
    let mut bound_count = 0;
    for actor in objects
        .iter()
        .filter(|a| a.runtime_identifier() == 1 || a.runtime_identifier() == actor_id)
    {
        let name = if actor.runtime_identifier() == 1 {
            "CYRUS"
        } else {
            actor_name
        };
        let (_, script) = scripts.iter().find(|(n, _)| n == name).unwrap();
        let count = usize::try_from(script.num_variables).unwrap();
        let bound = region.bind_actor(actor, count).unwrap().unwrap();
        assert_eq!(bound.words.len(), script.variables.len());
        assert_eq!(bound.object_id, actor.object_id);
        assert_eq!(bound.runtime_identifier, actor.runtime_identifier());
        assert!(actor.private_variable_words().is_empty());
        eprintln!(
            "bound {name} saved owner {} runtime {}: {} authored words",
            bound.object_id,
            bound.runtime_identifier,
            bound.words.len()
        );
        bound_count += 1;
    }
    assert!(
        bound_count >= 3,
        "retained world must contain both player records and dialogue actor"
    );
}
