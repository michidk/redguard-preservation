use rgpre::import::savegame::SaveSessionGlobals;

#[test]
fn session_globals_preserve_every_payload_byte() {
    let bytes: Vec<u8> = (0..1666).map(|i| (i * 37 + 11) as u8).collect();
    let saved = SaveSessionGlobals::parse(&bytes).unwrap();
    assert_eq!(saved.encode(), bytes);
    assert_eq!(
        saved.journal_tags[0],
        u32::from_le_bytes(bytes[22..26].try_into().unwrap())
    );
    assert_eq!(
        saved.journal_tags[399],
        u32::from_le_bytes(bytes[1618..1622].try_into().unwrap())
    );
}

#[test]
fn session_globals_decode_health_selection_and_power_up_slots() {
    let mut bytes = vec![0; 1666];
    for (offset, value) in [
        (0, 74i32),
        (4, -2),
        (1622, 7),
        (1626, 31),
        (1630, 19),
        (1634, -1),
        (1638, 1),
        (1642, 0),
        (1646, 101),
        (1650, 102),
        (1654, 103),
        (1658, 104),
        (1662, 48),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let saved = SaveSessionGlobals::parse(&bytes).unwrap();
    assert_eq!(saved.player_health, 74);
    assert_eq!(saved.compass_display, -2);
    assert_eq!(saved.selected_player_switch, 7);
    assert_eq!(saved.power_ups.frames, [31, 19]);
    assert_eq!(saved.power_ups.frame_indices, [-1, 1]);
    assert_eq!(saved.world_state, [101, 102, 103, 104]);
    assert_eq!(saved.health_candle_display, 48);
    assert_eq!(saved.encode(), bytes);
}

#[test]
fn session_globals_reject_truncated_and_extended_payloads() {
    for size in [0, 1665, 1667] {
        let error = SaveSessionGlobals::parse(&vec![0; size])
            .unwrap_err()
            .to_string();
        assert!(error.contains("SVG2"));
        assert!(error.contains("1666"));
    }
}

#[test]
fn saved_controller_restores_fields_then_applies_world_entry_resets() {
    use rgpre::import::savegame::SaveGameplayGlobals;
    let mut bytes = vec![0; 1542];
    for (offset, value) in [
        (0x10, 3i32),
        (0x130, 12),
        (0x30, 1i32),
        (0x34, 74),
        (0x38, 29),
        (0x40, 2),
        (0x48, -512),
        (0x4c, 1024),
        (0x50, 2048),
        (0x5c, 145),
        (0x60, -1),
        (0x64, 9),
        (0x6c, 7),
        (0x90, 99),
        (0x10c, -1),
        (0x5ea, 6),
        (0x5ee, 4),
        (0x5f2, 10),
        (0x5f6, 11),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[0x224..0x226].copy_from_slice(&37u16.to_le_bytes());
    let mut saved = SaveGameplayGlobals::parse(&bytes).unwrap();
    assert_eq!(saved.activation_consumed_use, 3);
    assert_eq!(saved.body_response_contacts, 12);
    assert_eq!(saved.controller.hit_inhibited, 1);
    assert_eq!(saved.controller.death_animation_override, 74);
    assert_eq!(saved.controller.death_progress, 29);
    assert_eq!(saved.controller.death_position_q8, [-512, 1024, 2048]);
    assert_eq!(saved.controller.action_ticks, u32::MAX);
    assert_eq!(saved.controller.falling_elapsed, 145);
    assert_eq!(saved.lineup_result, 9);
    assert_eq!(saved.lockout, 37);
    assert_eq!(saved.health_candle_level, 6);
    assert_eq!(saved.health_candle_frame, 4);
    assert_eq!(saved.health_candle_position, [10, 11]);
    assert_eq!(saved.world_loop_frame, 99);
    saved.enter_world_loop();
    assert_eq!(saved.controller.hit_inhibited, 0);
    assert_eq!(saved.controller.death_animation_override, 0);
    assert_eq!(saved.controller.death_progress, 0);
    assert_eq!(saved.world_loop_frame, 0);
    assert_eq!(saved.controller.death_mode, 2);
    assert_eq!(saved.controller.action_ticks, u32::MAX);
    assert_eq!(saved.controller.falling_elapsed, 145);
    assert_eq!(saved.lineup_result, 9);
    assert_eq!(saved.controller.idle_counter, -1);
    assert_eq!(saved.rope_jump_latch, 7);
}

#[test]
fn gameplay_globals_reject_inexact_payloads() {
    use rgpre::import::savegame::SaveGameplayGlobals;
    for size in [0, 1541, 1543] {
        assert!(
            SaveGameplayGlobals::parse(&vec![0; size])
                .unwrap_err()
                .to_string()
                .contains("SVGV")
        );
    }
}
