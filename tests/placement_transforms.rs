use rgpre::import::{
    registry::Registry,
    rgm::{extract_rgm_placements, parse_rgm_with_models},
};

fn scene(tag: &[u8; 4], record: &[u8; 66]) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend(70_u32.to_be_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(record);
    bytes.extend(b"END \0\0\0\0");
    bytes
}

#[test]
fn fixed_matrix_placement_has_portable_binary32_rotation() {
    let mut record = [0_u8; 66];
    record[4..9].copy_from_slice(b"MODEL");
    let rotation = [
        268_435_456_i32,
        0,
        0,
        0,
        249_243_953,
        -99_674_700,
        0,
        99_674_700,
        249_243_953,
    ];
    for (slot, value) in record[28..64]
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(rotation)
    {
        slot.copy_from_slice(&value.to_le_bytes());
    }
    let (placements, _) = extract_rgm_placements(&scene(b"MPSO", &record)).unwrap();
    assert_eq!(placements.len(), 1);
    assert_eq!(
        placements[0].transform[..12]
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [
            1_065_353_216,
            0,
            2_147_483_648,
            0,
            2_147_483_648,
            1_064_153_747,
            3_200_130_377,
            0,
            0,
            1_052_646_729,
            1_064_153_747,
            0,
        ]
    );
}

#[test]
fn object_angle_placement_has_portable_binary32_rotation() {
    let mut record = [0_u8; 66];
    record[4] = 1;
    record[5] = 1;
    record[6..10].copy_from_slice(b"TEST");
    record[15..20].copy_from_slice(b"MODEL");
    for slot in record[38..50].as_chunks_mut::<4>().0.iter_mut() {
        slot.copy_from_slice(&39_u32.to_le_bytes());
    }
    let (_, models, _) =
        parse_rgm_with_models(&scene(b"MPOB", &record), &Registry::new(Default::default()))
            .unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(
        models[0].transform[..12]
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [
            1_065_085_640,
            3_186_800_208,
            3_188_184_959,
            0,
            1_040_701_311,
            1_065_114_174,
            1_037_290_495,
            0,
            1_039_316_560,
            3_186_914_751,
            1_065_114_174,
            0,
        ]
    );
}
