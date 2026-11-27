use rgpre::import::savegame::{SaveGame, SaveImageKind, SaveMetadata, SaveSection};

fn save() -> SaveGame {
    SaveGame {
        metadata: SaveMetadata {
            version: *b"1.31",
            unknown_04: 0,
            unknown_08: 0,
            description: String::new(),
            world_id: 30,
            bitmap_filename: "must-not-be-opened.bmp".into(),
        },
        sections: Vec::new(),
    }
}

#[test]
fn screen_and_preview_decode_independently_without_changing_the_container() {
    let mut game = save();
    for (kind, name) in [
        (SaveImageKind::Screen, *b"SCR3"),
        (SaveImageKind::Thumbnail, *b"THMB"),
    ] {
        let mut data = vec![0; kind.width() * kind.height() * 2];
        for (index, word) in [
            (0, 0x1234u16),
            (kind.width(), 0xabcd),
            (kind.width() * kind.height() - 1, 0xffff),
        ] {
            data[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
        }
        game.sections.push(SaveSection { name, data });
    }
    let before = game.clone();
    for kind in [SaveImageKind::Screen, SaveImageKind::Thumbnail] {
        let image = game.image(kind).unwrap().unwrap();
        assert_eq!(image.kind(), kind);
        assert_eq!(image.pixels().len(), kind.width() * kind.height());
        assert_eq!(image.pixels()[0], 0x1234);
        assert_eq!(image.pixels()[kind.width()], 0xabcd);
        assert_eq!(image.pixels().last(), Some(&0xffff));
    }
    assert_eq!(game, before);
}

#[test]
fn missing_and_empty_images_are_optional_but_duplicates_and_wrong_sizes_reject() {
    for (kind, name) in [
        (SaveImageKind::Screen, *b"SCR3"),
        (SaveImageKind::Thumbnail, *b"THMB"),
    ] {
        let mut game = save();
        assert!(game.image(kind).unwrap().is_none());
        game.sections.push(SaveSection {
            name,
            data: Vec::new(),
        });
        assert!(game.image(kind).unwrap().is_none());
        game.sections.push(SaveSection {
            name,
            data: Vec::new(),
        });
        let error = game.image(kind).unwrap_err().to_string();
        assert!(error.contains("world 30") && error.contains("duplicate"));
        game.sections.pop();
        for length in [
            1,
            kind.width() * kind.height() * 2 - 1,
            kind.width() * kind.height() * 2 + 2,
        ] {
            game.sections[0].data = vec![0; length];
            let error = game.image(kind).unwrap_err().to_string();
            assert!(error.contains("world 30") && error.contains("expected"));
        }
    }
}

#[test]
fn installed_images_retain_every_stored_framebuffer_word() {
    let Some(path) = std::env::var_os("REDGUARD_SAVE_SLOT") else {
        eprintln!("SKIP installed save images: REDGUARD_SAVE_SLOT is absent");
        return;
    };
    let slot = rgpre::import::savegame::load_save_slot(path).unwrap();
    for (kind, name) in [
        (SaveImageKind::Screen, *b"SCR3"),
        (SaveImageKind::Thumbnail, *b"THMB"),
    ] {
        let image = slot
            .game
            .image(kind)
            .unwrap()
            .expect("installed saved image");
        let section = slot
            .game
            .sections
            .iter()
            .find(|section| section.name == name)
            .unwrap();
        let encoded: Vec<_> = image
            .pixels()
            .iter()
            .flat_map(|pixel| pixel.to_le_bytes())
            .collect();
        assert_eq!(encoded, section.data);
    }
}
