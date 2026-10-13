use rgpre::import::rgm::{RgmFile, RgmSection, RgmSectionHeader, actor_location_offsets};

fn map(count: i32, offset: i32, locations: Option<Vec<u8>>) -> RgmFile {
    let header = |name| RgmSectionHeader {
        name,
        data_length: 0,
        record_count: None,
    };
    let mut actor = vec![0; 173];
    actor[..4].copy_from_slice(&1_i32.to_le_bytes());
    actor[8 + 0x61..8 + 0x65].copy_from_slice(&count.to_le_bytes());
    actor[8 + 0x69..8 + 0x6d].copy_from_slice(&offset.to_le_bytes());
    let mut sections = vec![RgmSection::Rahd(header(*b"RAHD"), actor)];
    if let Some(data) = locations {
        sections.push(RgmSection::Ralc(header(*b"RALC"), data));
    }
    RgmFile { sections }
}

#[test]
fn offsets_use_authored_count_and_rebased_twelve_byte_boundary() {
    let data = [99_i32, 99, 99, -256, 512, 0, 3, -4, 5]
        .into_iter()
        .flat_map(i32::to_le_bytes)
        .collect();
    assert_eq!(
        actor_location_offsets(&map(2, 13, Some(data)), 0).unwrap(),
        vec![[-256, 512, 0], [3, -4, 5]]
    );
}

#[test]
fn empty_locations_do_not_require_a_section() {
    assert!(
        actor_location_offsets(&map(0, 0, None), 0)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_ranges_and_counts_report_actor_context() {
    for world in [
        map(-1, 12, None),
        map(1, 0, None),
        map(1, -12, None),
        map(1, 12, None),
        map(2, 12, Some(vec![0; 24])),
    ] {
        assert!(
            actor_location_offsets(&world, 0)
                .unwrap_err()
                .contains("RAHD actor 0 RALC locations")
        );
    }
    assert!(actor_location_offsets(&map(0, 0, None), 1).is_err());
}

#[test]
fn installed_maps_validate_each_actor_location_range() {
    let Some(root) = std::env::var_os("REDGUARD_DIR") else {
        eprintln!("SKIP installed actor locations: REDGUARD_DIR absent");
        return;
    };
    let mut maps = 0;
    let mut nonempty = 0;
    for entry in std::fs::read_dir(std::path::Path::new(&root).join("maps")).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("rgm"))
        {
            continue;
        }
        let world = rgpre::import::rgm::parse_rgm_file(&std::fs::read(&path).unwrap()).unwrap();
        maps += 1;
        let metadata = rgpre::import::rgm::export_rgm_metadata_json(&world, None);
        for actor in metadata["actors"].as_array().unwrap() {
            let index = actor["index"].as_u64().unwrap() as usize;
            let offsets = actor_location_offsets(&world, index)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            nonempty += usize::from(!offsets.is_empty());
        }
    }
    assert!(maps > 0 && nonempty > 0);
}
