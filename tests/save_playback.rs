use rgpre::import::savegame::{SaveAnimationPlayback, parse_save_clock};

#[test]
fn clock_keeps_exposed_steps_distinct_from_scaled_accumulators() {
    let mut bytes = [0u8; 1542];
    for (offset, value) in [
        (0x98, -3i32),
        (0xc4, 122),
        (0xc8, 1),
        (0xd0, 991),
        (0xd8, 4),
        (0xe0, 63),
        (0xf8, 3072),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let clock = parse_save_clock(&bytes).unwrap();
    assert_eq!(clock.measurement_remainder, -3);
    assert_eq!(clock.frame_scale, 122);
    assert_eq!(clock.nominal_scaled_total, 991);
    assert_eq!(clock.fine_scaled_total, 63);
    assert_eq!(clock.exposed_nominal_ticks, 1);
    assert_eq!(clock.exposed_fine_ticks, 4);
    assert_eq!(clock.normal_rate_scaled, 3072);
    assert!(
        parse_save_clock(&bytes[..1541])
            .unwrap_err()
            .to_string()
            .contains("SVGV")
    );
}

#[test]
fn animation_keeps_signed_words_countdowns_history_and_opaque_byte() {
    let mut bytes = std::array::from_fn::<_, 37, _>(|i| i as u8);
    bytes[..2].copy_from_slice(&152i16.to_le_bytes());
    bytes[2..4].copy_from_slice(&(-1i16).to_le_bytes());
    bytes[5..7].copy_from_slice(&(-2i16).to_le_bytes());
    let playback = SaveAnimationPlayback::from(&bytes);
    assert_eq!(playback.current, 152);
    assert_eq!(playback.next, -1);
    assert_eq!(playback.frame, -2);
    assert_eq!(
        (
            playback.speed_count,
            playback.opaque_9,
            playback.sound_handle
        ),
        (8, 9, 16)
    );
    assert_eq!(playback.position[0], i16::from_le_bytes([19, 20]));
    assert_eq!(playback.last_rotation[2], i16::from_le_bytes([35, 36]));
}
