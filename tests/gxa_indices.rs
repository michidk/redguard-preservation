use rgpre::import::gxa::parse_gxa_file;

#[test]
fn palette_aliases_preserve_cursor_marker_indices() -> rgpre::Result<()> {
    for compression in [0_i16, 1] {
        let mut header = [0_u8; 34];
        header[32..34].copy_from_slice(&1_i16.to_le_bytes());
        let mut palette = [0_u8; 768];
        palette[48..51].copy_from_slice(&[1, 2, 3]);
        palette[51..54].copy_from_slice(&[1, 2, 3]);
        let mut frame = vec![0_u8; 18];
        frame[2..4].copy_from_slice(&2_i16.to_le_bytes());
        frame[4..6].copy_from_slice(&2_i16.to_le_bytes());
        frame[10..12].copy_from_slice(&compression.to_le_bytes());
        if compression == 1 {
            frame.extend_from_slice(&5_u32.to_le_bytes());
            frame.push(3);
        }
        frame.extend_from_slice(&[16, 17, 0, 16]);

        let mut bytes = Vec::new();
        for (tag, data) in [
            (b"BMHD", header.as_slice()),
            (b"BPAL", palette.as_slice()),
            (b"BBMP", frame.as_slice()),
        ] {
            bytes.extend_from_slice(tag);
            bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
            bytes.extend_from_slice(data);
        }
        bytes.extend_from_slice(b"END ");

        let decoded = parse_gxa_file(&bytes)?;
        assert_eq!(decoded.frames.len(), 1);
        let frame = &decoded.frames[0];
        assert_eq!([frame.width, frame.height], [2, 2]);
        assert_eq!(frame.indices, [16, 17, 0, 16]);
        assert_eq!(
            frame.rgba,
            [4, 8, 12, 255, 4, 8, 12, 255, 0, 0, 0, 0, 4, 8, 12, 255]
        );
    }
    Ok(())
}
