use rgpre::import::savegame::{SaveActorNavigation, encode_save_objects, parse_save_objects};

fn object(map: u32, nodes: [i32; 4]) -> rgpre::import::savegame::SaveObject {
    let mut bytes = vec![0; 4 + 9 + 783 + 48];
    bytes[..4].copy_from_slice(&((9 + 783 + 48) as u32).to_le_bytes());
    bytes[4..8].copy_from_slice(&7i32.to_le_bytes());
    let body = &mut bytes[13..13 + 783];
    body[0x23b..0x23f].copy_from_slice(&u32::MAX.to_le_bytes());
    body[0x1e9..0x1ed].copy_from_slice(&map.to_le_bytes());
    for (i, value) in nodes.into_iter().enumerate() {
        body[0x1ed + i * 4..0x1f1 + i * 4].copy_from_slice(&value.to_le_bytes());
    }
    parse_save_objects(&bytes).unwrap().remove(0)
}

#[test]
fn navigation_uses_saved_identity_and_preserves_node_history() {
    let actor = object(0, [-1, 39, 169, -1]);
    let navigation = actor.navigation();
    assert_eq!(
        navigation,
        SaveActorNavigation {
            map_id: Some(0),
            current_node: -1,
            next_node: 39,
            previous_next_node: 169,
            destination_node: -1,
        }
    );
    assert_eq!(navigation.bind_map(&[8, 0, 5]).unwrap(), Some(1));
    assert!(navigation.bind_map(&[8, 5]).is_err());
    assert!(navigation.bind_map(&[0, 0]).is_err());
    assert_eq!(
        parse_save_objects(&encode_save_objects(&[actor]).unwrap()).unwrap()[0].navigation(),
        navigation
    );
}

#[test]
fn absent_map_and_stale_nodes_are_not_reconstructed_at_decode() {
    let navigation = object(u32::MAX, [-1, 900, 123, -1]).navigation();
    assert_eq!(navigation.map_id, None);
    assert_eq!(navigation.bind_map(&[]).unwrap(), None);
    assert_eq!(navigation.next_node, 900);
    assert_eq!(navigation.previous_next_node, 123);
}

#[test]
fn retained_kotaro_navigation_matches_original_load_boundary() {
    let Some(path) = std::env::var_os("REDGUARD_NAVIGATION_SAVE") else {
        eprintln!("skipping retained navigation test: REDGUARD_NAVIGATION_SAVE unset");
        return;
    };
    let slot = rgpre::import::savegame::load_save_slot(path).unwrap();
    let section = slot
        .game
        .sections
        .iter()
        .find(|s| s.name == *b"SMOB")
        .unwrap();
    let actors = parse_save_objects(&section.data).unwrap();
    for (id, next, previous, timer) in [(298, 39, 169, 225), (278, 4, 39, 757)] {
        let actor = actors.iter().find(|a| a.object_id == id).unwrap();
        assert_eq!(
            actor.navigation(),
            SaveActorNavigation {
                map_id: Some(0),
                current_node: -1,
                next_node: next,
                previous_next_node: previous,
                destination_node: -1,
            }
        );
        assert_eq!(actor.navigation().bind_map(&[0]).unwrap(), Some(0));
        let task = actor.task_slots()[0];
        assert_ne!(task.callback, 0);
        assert_eq!((task.status, task.pause, task.timer), (1, 0, timer));
        assert_eq!(task.variables, [0, 0, 0, 131072]);
    }
}

#[test]
fn retained_favis_has_no_map_and_keeps_allocation_history() {
    let Some(path) = std::env::var_os("REDGUARD_NAVIGATION_FAVIS_SAVE") else {
        eprintln!("skipping retained Favis navigation test: REDGUARD_NAVIGATION_FAVIS_SAVE unset");
        return;
    };
    let slot = rgpre::import::savegame::load_save_slot(path).unwrap();
    let section = slot
        .game
        .sections
        .iter()
        .find(|s| s.name == *b"SMOB")
        .unwrap();
    let actors = parse_save_objects(&section.data).unwrap();
    let favis = actors.iter().find(|a| a.object_id == 37).unwrap();
    assert_eq!(
        favis.navigation(),
        SaveActorNavigation {
            map_id: None,
            current_node: -1,
            next_node: -1,
            previous_next_node: 0,
            destination_node: -1,
        }
    );
    assert_eq!(favis.navigation().bind_map(&[]).unwrap(), None);
    assert_eq!(
        (favis.task_slots()[0].status, favis.task_slots()[0].timer),
        (1, 12)
    );
}
