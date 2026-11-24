use rgpre::import::savegame::{
    SaveModelAllocation, SaveModelFrame, load_save_slot, parse_save_objects,
};

#[test]
fn model_binding_requires_exact_unique_allocation_and_valid_frame() {
    let allocations = vec![SaveModelAllocation {
        handle: 107,
        source: "PLAYER.3DC".into(),
        frame_count: 100,
        texture: 0,
    }];
    let pose = SaveModelFrame {
        model_handle: 107,
        frame: 88,
    };
    let bound = pose.bind(&allocations).unwrap().unwrap();
    assert_eq!((bound.allocation, bound.frame), (0, 88));
    assert!(pose.bind(&[]).is_err());
    assert!(
        pose.bind(&[allocations[0].clone(), allocations[0].clone()])
            .is_err()
    );
    assert!(
        SaveModelFrame { frame: 100, ..pose }
            .bind(&allocations)
            .is_err()
    );
    assert_eq!(
        SaveModelFrame {
            model_handle: -1,
            ..pose
        }
        .bind(&allocations)
        .unwrap(),
        None
    );
    assert_eq!(
        SaveModelFrame {
            model_handle: -1,
            frame: -1
        }
        .bind(&[])
        .unwrap(),
        None
    );
}

#[test]
fn retained_poses_preserve_previous_selection_and_default_independently() {
    for (variable, expected) in [
        (
            "REDGUARD_POSE_KOTARO_SAVE",
            vec![
                (-2, 107, 88, 86, 0),
                (154, 1, 0, 1, 0),
                (243, 1, 4, 3, 0),
                (303, 50, 1, 1, 152),
            ],
        ),
        ("REDGUARD_POSE_FAVIS_SAVE", vec![(-2, 5, 12, 12, 0)]),
    ] {
        let Some(path) = std::env::var_os(variable) else {
            eprintln!("skipping retained pose test: {variable} unset");
            continue;
        };
        let slot = load_save_slot(path).unwrap();
        let section = slot
            .game
            .sections
            .iter()
            .find(|s| s.name == *b"SMOB")
            .unwrap();
        let actors = parse_save_objects(&section.data).unwrap();
        for (owner, handle, current, previous, default) in expected {
            let actor = actors.iter().find(|a| a.object_id == owner).unwrap();
            let pose = actor.pose();
            assert_eq!(
                pose.current,
                SaveModelFrame {
                    model_handle: handle,
                    frame: current
                }
            );
            assert_eq!(
                pose.previous,
                SaveModelFrame {
                    model_handle: handle,
                    frame: previous
                }
            );
            assert_eq!(pose.default_group, default);
        }
    }
}

#[test]
fn registration_preserves_base_and_texture_variant_ownership() {
    use rgpre::import::savegame::SaveModelAllocations;
    let mut table = SaveModelAllocations::default();
    assert_eq!(table.register("FXART/THUGA002.3DC", 80, 453).unwrap(), 2);
    assert_eq!(table.register("fxart\\thuga002.3dc", 80, 0).unwrap(), 1);
    assert_eq!(table.register("FXART/THUGA002.3DC", 80, 453).unwrap(), 2);
    assert_eq!(table.register("FXART/THUGA002.3DC", 80, 454).unwrap(), 3);
    let before = table.clone();
    assert!(table.register("FXART/THUGA002.3DC", 79, 455).is_err());
    assert_eq!(table, before);
    assert_eq!(table.entries()[1].texture, 453);
}

#[test]
fn retained_model_handles_bind_to_installed_world_registration() {
    use rgpre::import::{rgm::parse_rgm_file, savegame::SaveModelAllocations};
    let Some(install) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("skipping installed model binding: REDGUARD_DIR unset");
        return;
    };
    let install = std::path::Path::new(&install);
    for (map, variable, player_handle) in [
        ("ISLAND", "REDGUARD_POSE_KOTARO_SAVE", 107),
        ("BELLTOWR", "REDGUARD_POSE_FAVIS_SAVE", 5),
    ] {
        let Some(save) = std::env::var_os(variable) else {
            eprintln!("skipping installed model binding: {variable} unset");
            continue;
        };
        let world =
            parse_rgm_file(&std::fs::read(install.join(format!("maps/{map}.RGM"))).unwrap())
                .unwrap();
        let table = SaveModelAllocations::from_world(&world, &["CYRUS", "GREMLIN"], |source| {
            let stem = source
                .rsplit(['\\', '/'])
                .next()
                .unwrap()
                .split('.')
                .next()
                .unwrap()
                .to_ascii_uppercase();
            let path = ["3D", "3DC"]
                .into_iter()
                .map(|ext| install.join(format!("fxart/{stem}.{ext}")))
                .find(|p| p.is_file())
                .unwrap();
            let model = rgpre::model3d::parse_3d_file(&std::fs::read(&path).unwrap())?;
            Ok((
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                model.header.num_frames.max(1) as usize,
            ))
        })
        .unwrap();
        assert_eq!(
            table
                .entries()
                .iter()
                .find(|a| a.handle == player_handle)
                .unwrap()
                .source,
            if map == "ISLAND" {
                "CYRSA001.3DC"
            } else {
                "CYRSA004.3DC"
            }
        );
        let slot = load_save_slot(save).unwrap();
        let actors = parse_save_objects(
            &slot
                .game
                .sections
                .iter()
                .find(|s| s.name == *b"SMOB")
                .unwrap()
                .data,
        )
        .unwrap();
        for actor in actors.iter().filter(|a| a.animation_header.is_some()) {
            let pose = actor.pose();
            for frame in [pose.current, pose.previous] {
                frame
                    .bind(table.entries())
                    .unwrap_or_else(|e| panic!("{map} owner {}: {e}", actor.object_id));
            }
        }
        if map == "ISLAND" {
            let variant = &table.entries()[49];
            assert_eq!((&*variant.source, variant.texture), ("THUGA002.3DC", 453));
        }
    }
}
