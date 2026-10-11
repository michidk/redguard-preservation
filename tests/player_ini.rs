use rgpre::import::player_ini::PlayerConfig;

#[test]
fn installed_system_ini_retains_player_settings() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed player settings: REDGUARD_DIR is absent");
        return;
    };
    let path = std::path::PathBuf::from(root).join("SYSTEM.INI");
    let settings = PlayerConfig::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(settings.standing_height, -2);
    assert_eq!(settings.post_collide_height, 50);
    assert_eq!(settings.slide_range, -60000);
    assert_eq!(settings.turn_speed, 12);
    assert_eq!(settings.turn_max_speed, 48);
    assert_eq!(settings.poly_push_units, 16);
}
