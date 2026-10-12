use rgpre::import::{
    combat_ini::{AttackType, CombatIni, Elevation, TauntTrigger},
    item_ini::{ItemIni, ItemType},
    player_ini::PlayerConfig,
    system_ini::SystemIni,
};

#[test]
fn system_settings_have_section_scoped_types_and_keep_original_values() {
    let ini = SystemIni::parse(
        "\u{feff}[SCREEN]\r\nPalette_red=3\r\nresolution=1\r\n[system]\r\n\
         pointers=system\\pointers.bmp\r\nstanding_height=-2\r\nslide_range=-60000\r\n\
         sound=1\r\nredbook=ON ; CD music\r\nredbook_volume=200\r\n\
         [cyrus]\r\nturn_speed=12\r\nwalk_mode=0\r\n\
         [camera]\r\ntarget_offset_y=-14000\r\nplayer_control_y_inc=-70\r\n\
         glide_x=0.9\r\nglide_y=0.2\r\ncam_prox_up=70.0\r\n\
         [3dfx]\r\nresolution=12\r\nfont_used=125, 128\r\ntext_scale=1,1\r\n\
         [debug]\r\nmonitor_object=\r\nignore_errors=Yes\r\n\
         [other]\r\nstanding_height=999\r\n",
    )
    .unwrap();
    assert_eq!(ini.screen.palette_red, Some(3));
    assert_eq!(ini.screen.resolution, Some(1));
    assert_eq!(ini.system.pointers.as_deref(), Some("system\\pointers.bmp"));
    assert_eq!(ini.system.standing_height, Some(-2));
    assert_eq!(ini.system.slide_range, Some(-60000));
    assert_eq!(ini.system.sound, Some(true));
    assert_eq!(ini.system.redbook, Some(true));
    assert_eq!(ini.system.redbook_volume, Some(200));
    assert_eq!(ini.cyrus.turn_speed, Some(12));
    assert_eq!(ini.cyrus.walk_mode, Some(0));
    assert_eq!(ini.camera.target_offset_y, Some(-14000));
    assert_eq!(ini.camera.player_control_y_inc, Some(-70));
    assert_eq!(ini.camera.glide_x, Some(0.9));
    assert_eq!(ini.camera.glide_y, Some(0.2));
    assert_eq!(ini.camera.cam_prox_up, Some(70.0));
    assert_eq!(ini.glide.resolution, Some(12));
    assert_eq!(ini.glide.font_used, Some([125, 128]));
    assert_eq!(ini.glide.text_scale, Some([1, 1]));
    assert_eq!(ini.debug.monitor_object, None);
    assert_eq!(ini.debug.ignore_errors, Some(true));
    assert_eq!(ini.game.old_combat, None);
}

#[test]
fn combat_preserves_sparse_ids_optional_fields_and_combo_references() {
    let ini = CombatIni::parse(
        "[misc]\nplayer_can_die=1\nmax_taunts=600\n\
         [ATTACK10]\ntype=2\nnum_attacks=3\nattack00=4\nattack01=5\nattack02=7\n\
         [attack01]\ntype=3\narc_center=-512\nelevation=2\n\
         first_collide_frame=7\ncollide_duration=3\nmin_range=0\nmax_range=35000\n\
         [defend00]\nmin_range=00000\ncollide_duration=5\nanimation=1\n\
         [taunt599]\ntype=8\nrtx_label=&607\nanimation=0\n\
         [taunt00]\ntype=10\nrtx_label=CGra\n\
         [unrelated]\nmax_taunts=0\n",
    )
    .unwrap();
    assert_eq!(ini.misc.player_can_die, Some(true));
    assert_eq!(ini.misc.max_taunts, Some(600));
    assert_eq!(ini.attacks.keys().copied().collect::<Vec<_>>(), [1, 10]);
    let stab = &ini.attacks[&1];
    assert_eq!(stab.attack_type, Some(AttackType::Stab));
    assert_eq!(stab.arc_center, Some(-512));
    assert_eq!(stab.elevation, Some(Elevation::Below));
    assert_eq!(stab.first_collide_frame, Some(7));
    assert_eq!(stab.collide_duration, Some(3));
    assert_eq!(stab.min_range, Some(0));
    assert_eq!(stab.max_range, Some(35000));
    let combo = &ini.attacks[&10];
    assert_eq!(combo.attack_type, Some(AttackType::Combo));
    assert_eq!(combo.num_attacks, Some(3));
    assert_eq!(
        [combo.attack00, combo.attack01, combo.attack02],
        [Some(4), Some(5), Some(7)]
    );
    assert_eq!(combo.damage, None);
    assert_eq!(ini.defenses[&0].arc, None);
    assert_eq!(ini.defenses[&0].min_range, Some(0));
    assert_eq!(ini.taunts.keys().copied().collect::<Vec<_>>(), [0, 599]);
    assert_eq!(ini.taunts[&599].trigger, Some(TauntTrigger::WantSwitchOut));
    assert_eq!(ini.taunts[&599].rtx_label.as_deref(), Some("&607"));
    assert_eq!(ini.taunts[&0].rtx_label.as_deref(), Some("CGra"));
}

#[test]
fn item_settings_preserve_ids_lists_paths_and_empty_fields() {
    let ini = ItemIni::parse(
        "[ITEMS]\nbitmap_file=SYSTEM\\PICKUPS.GXA\nstart_item_list=1, 2,4,18\n\
         start_item_select=1\nweapon_sphere_size=10\n\
         name[45]=?xTo\ntype[45]=3\nuse_script[45]=itorch\nscript_instances[45]=-1\n\
         drop_use_item_list[45]=2, 4\nrequired_item_list[45]=1,18\n\
         inventory_object_file[45]=3DART\\Itorch.3D\n\
         name[0]=?xcm\ntype[0]=0\nplayer_total[0]=0\nhide[0]=0\n\
         use_script[0]=\nuse_script[0]=ignored\n\
         name[7]=?xsw\ntype[7]=1\nflags[7]=7\nhide[7]=1\n\
         add_item_list[7]=0, 1\ngame_object_file[7]=\nunknown[bad]=ignored\nname[99]=\n\
         [other]\nname[0]=wrong section\n",
    )
    .unwrap();
    assert_eq!(
        ini.settings.bitmap_file.as_deref(),
        Some("SYSTEM\\PICKUPS.GXA")
    );
    assert_eq!(ini.settings.start_item_list, Some(vec![1, 2, 4, 18]));
    assert_eq!(ini.settings.start_item_select, Some(1));
    assert_eq!(ini.settings.weapon_sphere_size, Some(10));
    assert_eq!(ini.items.keys().copied().collect::<Vec<_>>(), [0, 7, 45]);
    let compass = &ini.items[&0];
    assert_eq!(compass.item_type, Some(ItemType::General));
    assert_eq!(compass.name.as_deref(), Some("?xcm"));
    assert_eq!(compass.player_total, Some(0));
    assert_eq!(compass.use_script, None);
    assert_eq!(compass.bitmap, None);
    assert_eq!(ini.items[&7].item_type, Some(ItemType::Weapon));
    assert_eq!(ini.items[&7].flags, Some(7));
    assert_eq!(ini.items[&7].hide, Some(true));
    assert_eq!(ini.items[&7].add_item_list, Some(vec![0, 1]));
    assert_eq!(ini.items[&7].game_object_file, None);
    let torch = &ini.items[&45];
    assert_eq!(torch.item_type, Some(ItemType::Unknown(3)));
    assert_eq!(torch.name.as_deref(), Some("?xTo"));
    assert_eq!(torch.script_instances, Some(-1));
    assert_eq!(torch.drop_use_item_list, Some(vec![2, 4]));
    assert_eq!(torch.required_item_list, Some(vec![1, 18]));
    assert_eq!(
        torch.inventory_object_file.as_deref(),
        Some("3DART\\Itorch.3D")
    );
}

#[test]
fn duplicates_use_first_values_even_across_repeated_sections() {
    let system =
        SystemIni::parse("[system]\nvolume=22\nVOLUME=invalid\n[System]\nvolume=33\n").unwrap();
    assert_eq!(system.system.volume, Some(22));
    let combat =
        CombatIni::parse("[misc]\n[attack01]\narc=12\n[ATTACK01]\narc=invalid\ntype=99\n").unwrap();
    assert_eq!(combat.attacks[&1].arc, Some(12));
    assert_eq!(
        combat.attacks[&1].attack_type,
        Some(AttackType::Unknown(99))
    );
    let items =
        ItemIni::parse("[items]\nuse_script[69]=ibandage\nUSE_SCRIPT[69]=inouse\n").unwrap();
    assert_eq!(items.items[&69].use_script.as_deref(), Some("ibandage"));
    assert!(SystemIni::parse("[system]\nvolume=invalid\nvolume=22\n").is_err());
}

#[test]
fn switch_spellings_are_typed_and_missing_sections_stay_empty() {
    for (token, expected) in [
        ("0", false),
        ("1", true),
        ("NO", false),
        ("yes", true),
        ("Off", false),
        ("ON", true),
        ("false", false),
        ("TRUE", true),
    ] {
        let system = SystemIni::parse(&format!("[system]\nsound={token}")).unwrap();
        assert_eq!(system.system.sound, Some(expected));
        assert_eq!(system.screen.resolution, None);
        assert_eq!(system.glide.font_norm, None);
    }
}

#[test]
fn malformed_known_values_report_file_field_and_line() {
    for (text, field) in [
        ("[system]\nsound=2", "sound"),
        ("[system]\nvolume=2147483648", "volume"),
        ("[system]\nstanding_height=1.2", "standing_height"),
        ("[system]\n[cyrus]\nturn_speed=-1", "turn_speed"),
        ("[system]\n[camera]\nglide_x=NaN", "glide_x"),
        ("[system]\n[camera]\nglide_y=inf", "glide_y"),
        ("[system]\n[3dfx]\nfont_sel=1,2,3", "font_sel"),
        ("[system]\n[3dfx]\ntext_scale=1", "text_scale"),
    ] {
        let error = SystemIni::parse(text).unwrap_err().to_string();
        assert!(
            error.contains("SYSTEM.INI") && error.contains(field) && error.contains("line"),
            "{error}"
        );
    }
    for (text, field) in [
        ("[misc]\nmax_taunts=-1", "max_taunts"),
        ("[misc]\n[attack01]\ntype=melee", "type"),
        ("[misc]\n[attack01]\narc=bad", "arc"),
        ("[misc]\n[attack01]\nattack00=-1", "attack00"),
        ("[misc]\n[taunt599]\nanimation=2147483648", "animation"),
    ] {
        let error = CombatIni::parse(text).unwrap_err().to_string();
        assert!(
            error.contains("COMBAT.INI") && error.contains(field) && error.contains("line"),
            "{error}"
        );
    }
    for (text, field) in [
        ("[items]\nstart_item_list=1,,2", "start_item_list"),
        ("[items]\nstart_item_list=1,", "start_item_list"),
        ("[items]\nstart_item_select=-1", "start_item_select"),
        ("[items]\nhide[0]=perhaps", "hide"),
        ("[items]\nbitmap[4]=4294967296", "bitmap"),
        ("[items]\nrequired_item_list[4]=1,-2", "required_item_list"),
        ("[items]\nscript_instances[4]=bad", "script_instances"),
    ] {
        let error = ItemIni::parse(text).unwrap_err().to_string();
        assert!(
            error.contains("ITEM.INI") && error.contains(field) && error.contains("line"),
            "{error}"
        );
    }
}

#[test]
fn missing_sections_malformed_records_and_invalid_indices_fail() {
    assert!(
        SystemIni::parse("")
            .unwrap_err()
            .to_string()
            .contains("missing [system]")
    );
    assert!(
        CombatIni::parse("")
            .unwrap_err()
            .to_string()
            .contains("missing [misc]")
    );
    assert!(
        ItemIni::parse("")
            .unwrap_err()
            .to_string()
            .contains("missing [items]")
    );
    for text in [
        "[system",
        "[]=1",
        "volume=1",
        "[system]\n=1",
        "[system]\nvolume 1",
    ] {
        assert!(SystemIni::parse(text).is_err(), "{text}");
    }
    for section in ["attackbad", "attack-1", "taunt4294967296", "defend"] {
        assert!(
            CombatIni::parse(&format!("[misc]\n[{section}]\n")).is_err(),
            "{section}"
        );
    }
    assert!(CombatIni::parse("[misc]\n[attack1]\n[attack01]\n").is_err());
    for key in [
        "name[-1]",
        "name[bad]",
        "name[4294967296]",
        "name[1",
        "name[1][2]",
        "name",
    ] {
        assert!(
            ItemIni::parse(&format!("[items]\n{key}=value")).is_err(),
            "{key}"
        );
    }
    assert!(ItemIni::parse("[items]\nname[1]=one\nname[01]=other").is_err());
}

#[test]
fn parses_installed_configs_and_matches_existing_player_settings() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed config parsers: REDGUARD_DIR is absent");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let text = std::fs::read_to_string(root.join("SYSTEM.INI")).unwrap();
    let system = SystemIni::parse(&text).unwrap();
    let player = PlayerConfig::parse(&text).unwrap();
    assert_eq!(system.system.standing_height, Some(player.standing_height));
    assert_eq!(
        system.system.post_collide_height,
        Some(player.post_collide_height)
    );
    assert_eq!(system.system.slide_range, Some(player.slide_range));
    assert_eq!(system.cyrus.turn_speed, Some(player.turn_speed));
    assert_eq!(system.cyrus.turn_max_speed, Some(player.turn_max_speed));
    assert_eq!(system.cyrus.poly_push_units, Some(player.poly_push_units));
    assert_eq!(system.system.volume, Some(255));
    assert_eq!(system.camera.target_offset_y, Some(-14000));
    assert_eq!(system.camera.glide_x, Some(0.9));
    assert_eq!(system.glide.font_norm, Some([125, 255]));
    let combat =
        CombatIni::parse(&std::fs::read_to_string(root.join("COMBAT.INI")).unwrap()).unwrap();
    assert_eq!(combat.attacks.len(), 89);
    assert_eq!(combat.defenses.len(), 5);
    assert_eq!(combat.taunts.len(), 379);
    assert_eq!(combat.attacks[&0].min_range, Some(12001));
    assert_eq!(combat.attacks[&9].attack_type, Some(AttackType::Combo));
    assert_eq!(combat.attacks[&9].attack00, Some(4));
    assert_eq!(combat.attacks[&9].attack01, Some(8));
    assert_eq!(combat.taunts[&599].rtx_label.as_deref(), Some("za54"));
    assert_eq!(combat.taunts[&599].trigger, Some(TauntTrigger::Death));
    assert_eq!(combat.taunts[&0].rtx_label.as_deref(), Some("&004"));
    let items = ItemIni::parse(&std::fs::read_to_string(root.join("ITEM.INI")).unwrap()).unwrap();
    assert_eq!(items.items.len(), 87);
    assert_eq!(items.settings.start_item_list, Some(vec![1, 2, 4, 18]));
    assert_eq!(items.items[&2].player_total, Some(52));
    assert_eq!(items.items[&45].item_type, Some(ItemType::Unknown(3)));
    assert_eq!(items.items[&69].use_script.as_deref(), Some("inouse"));
    assert_eq!(items.items[&74].use_script, None);
    assert_eq!(items.items[&86].name.as_deref(), Some("?xsw"));
}
