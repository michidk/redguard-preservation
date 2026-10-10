use rgpre::import::rgm::{RgmFile, RgmSection, RgmSectionHeader, export_rgm_metadata_json};

fn header(name: [u8; 4], length: u32) -> RgmSectionHeader {
    RgmSectionHeader {
        name,
        data_length: length,
        record_count: None,
    }
}

#[test]
fn bounds_metadata_keeps_leading_flag_and_asymmetric_extents() {
    let mut bounds = vec![2];
    for value in [30_i32, 90, 40, 1, 2, 3, 10, 60, 15, 20, 30, 25] {
        bounds.extend(value.to_le_bytes());
    }
    bounds.extend([0; 49]);
    let world = RgmFile {
        sections: vec![RgmSection::Mpsz(header(*b"MPSZ", 98), bounds)],
    };
    let metadata = export_rgm_metadata_json(&world, None);
    let entries = metadata["mpsz_entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["flags"], 2);
    assert_eq!(entries[0]["total_extent"], serde_json::json!([30, 90, 40]));
    assert_eq!(entries[0]["center_offset"], serde_json::json!([1, 2, 3]));
    assert_eq!(entries[0]["pos_extent"], serde_json::json!([10, 60, 15]));
    assert_eq!(entries[0]["neg_extent"], serde_json::json!([20, 30, 25]));
    assert_eq!(entries[1]["total_extent"], serde_json::json!([0, 0, 0]));
}

#[test]
fn actor_metadata_exposes_primary_bounds_and_omits_absent_alternates() {
    let mut actor = vec![0; 173];
    actor[..4].copy_from_slice(&1_u32.to_le_bytes());
    actor[12..17].copy_from_slice(b"ACTOR");
    actor[8 + 0x89..8 + 0x8d].copy_from_slice(&7_i32.to_le_bytes());
    actor[8 + 0x8d..8 + 0x91].copy_from_slice(&(-1_i32).to_le_bytes());
    actor[8 + 0x91..8 + 0x95].copy_from_slice(&(-1_i32).to_le_bytes());
    let world = RgmFile {
        sections: vec![RgmSection::Rahd(header(*b"RAHD", 173), actor)],
    };
    let metadata = export_rgm_metadata_json(&world, None);
    let actor = &metadata["actors"][0];
    assert_eq!(actor["mpsz_bounds_primary"], 7);
    assert!(actor.get("mpsz_bounds_0").is_none());
    assert!(actor.get("mpsz_bounds_1").is_none());
}
