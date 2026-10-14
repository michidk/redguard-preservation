use rgpre::import::bsi::GlideTextureLayout;

#[test]
fn source_dimensions_survive_independent_axis_reduction() {
    let layout = GlideTextureLayout::new([65, 129], None).unwrap();
    assert_eq!(layout.source_dimensions(), [65, 129]);
    assert_eq!(layout.allocation_dimensions(), [64, 64]);
    assert_eq!(layout.sampling_stride(), [2, 4]);
    assert_eq!(
        layout.coordinate_scale().map(f32::to_bits),
        [0x3ffc_0fc1, 0x3f7e_03f8]
    );
    assert_eq!(layout.sampling_level(), 2);
    let source = (0..65 * 129).map(|i| (i % 251) as u8).collect::<Vec<_>>();
    let pixels = layout.sample_indexed(&source).unwrap();
    assert_eq!(pixels.len(), 4096);
    assert_eq!(&pixels[..4], &[0, 2, 4, 6]);
    assert_eq!(&pixels[64..68], &[9, 11, 13, 15]);
    assert_eq!(pixels[32 * 64 + 32], 101);
    assert!(pixels[33 * 64..].iter().all(|&p| p == 0));
    assert!(
        pixels
            .as_chunks::<64>()
            .0
            .iter()
            .all(|row| row[33..].iter().all(|&p| p == 0))
    );
}

#[test]
fn indexed_upload_pads_instead_of_stretching_or_repeating() {
    let layout = GlideTextureLayout::new([3, 5], None).unwrap();
    assert_eq!(layout.allocation_dimensions(), [4, 8]);
    assert_eq!(layout.coordinate_scale(), [32.0, 32.0]);
    assert_eq!(
        layout
            .sample_indexed(&(1..=15).collect::<Vec<_>>())
            .unwrap(),
        vec![
            1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 0, 10, 11, 12, 0, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn extreme_aspect_and_configured_full_resolution_keep_distinct_source_sizes() {
    let layout = GlideTextureLayout::new([1, 255], Some([1, 1])).unwrap();
    assert_eq!(layout.allocation_dimensions(), [32, 256]);
    assert_eq!(layout.sampling_stride(), [1, 1]);
    assert_eq!(layout.coordinate_scale(), [1.0, 1.0]);
    assert_eq!(layout.sampling_level(), 0);
    let minimum = GlideTextureLayout::new([1, 1], None).unwrap();
    assert_eq!(minimum.coordinate_scale(), [256.0, 256.0]);
    assert_eq!(minimum.sampling_level(), 8);
}

#[test]
fn malformed_dimensions_settings_and_frame_lengths_have_context() {
    for size in [[0, 8], [8, 257]] {
        assert!(
            GlideTextureLayout::new(size, None)
                .unwrap_err()
                .to_string()
                .contains("source dimensions")
        );
    }
    for pair in [[0, 2], [-1, 2], [3, 2], [512, 2], [4, 256]] {
        let error = GlideTextureLayout::new([64, 64], Some(pair))
            .unwrap_err()
            .to_string();
        assert!(error.contains("64x64") && error.contains("SYSTEM.INI [3dfx] text_scale"));
    }
    let error = GlideTextureLayout::new([3, 5], None)
        .unwrap()
        .sample_indexed(&[0; 14])
        .unwrap_err()
        .to_string();
    assert!(error.contains("3x5") && error.contains("expected 15") && error.contains("got 14"));
}
