use rgpre::import::gxa::parse_software_cursor_file;

fn archive(frame_count: i16, marker: [u8; 4]) -> Vec<u8> {
    let mut header = [0_u8; 34];
    header[32..34].copy_from_slice(&frame_count.to_le_bytes());
    let mut palette = [0_u8; 768];
    palette[48..51].copy_from_slice(&[1, 2, 3]);
    palette[51..54].copy_from_slice(&[1, 2, 3]);
    let mut frames = Vec::new();
    for index in 0..frame_count {
        let mut frame = [0_u8; 18];
        frame[2..4].copy_from_slice(&2_i16.to_le_bytes());
        frame[4..6].copy_from_slice(&2_i16.to_le_bytes());
        frames.extend_from_slice(&frame);
        frames.extend_from_slice(if index >= (frame_count - 1) / 2 * 2 {
            &[0, 0, 0, 0]
        } else if index % 2 == 0 {
            &[17, 17, 17, 17]
        } else {
            &marker
        });
    }
    let mut bytes = Vec::new();
    for (tag, data) in [
        (b"BMHD", header.as_slice()),
        (b"BPAL", palette.as_slice()),
        (b"BBMP", frames.as_slice()),
    ] {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        bytes.extend_from_slice(data);
    }
    bytes.extend_from_slice(b"END ");
    bytes
}

#[test]
fn frame_admission_handles_odd_even_and_capacity_boundaries() -> rgpre::Result<()> {
    for (frame_count, expected) in [(1, 0), (2, 0), (3, 1), (4, 1), (5, 2), (129, 64), (130, 64)] {
        let cursors = parse_software_cursor_file(&archive(frame_count, [0, 16, 0, 0]))?;
        assert_eq!(cursors.len(), expected, "frame count {frame_count}");
        for cursor in cursors {
            assert_eq!(cursor.hotspot, Some([1, 0]));
        }
    }
    Ok(())
}

#[test]
fn thirty_frames_admit_fourteen_images_with_last_marker_hotspots() -> rgpre::Result<()> {
    let cursors = parse_software_cursor_file(&archive(30, [17, 16, 16, 17]))?;
    assert_eq!(cursors.len(), 14);
    for cursor in cursors {
        assert_eq!(cursor.hotspot, Some([0, 1]));
        assert_eq!(cursor.image.indices, [17, 17, 17, 17]);
        assert_eq!(cursor.image.rgba, [4, 8, 12, 255].repeat(4));
    }
    Ok(())
}

#[test]
fn identical_palette_color_leaves_missing_marker_offset_unknown() -> rgpre::Result<()> {
    let cursors = parse_software_cursor_file(&archive(30, [17; 4]))?;
    assert_eq!(cursors.len(), 14);
    assert!(cursors.iter().all(|cursor| cursor.hotspot.is_none()));
    assert_eq!(cursors[0].image.indices, [17; 4]);
    Ok(())
}

#[test]
fn descriptor_capacity_overflow_is_explicit() {
    let error = parse_software_cursor_file(&archive(131, [16; 4])).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires 65 descriptors; capacity is 64")
    );
}
