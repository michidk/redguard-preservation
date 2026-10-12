use rgpre::import::{
    bsi::{self, BsiIndex},
    rob::{self, RobIndex},
    sfx::{self, SfxIndex},
};
use std::io::{Cursor, Read, Seek, SeekFrom};

struct Observed {
    data: Cursor<Vec<u8>>,
    reads: Vec<(u64, usize)>,
}
impl Observed {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data: Cursor::new(data),
            reads: Vec::new(),
        }
    }
}
impl Read for Observed {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let position = self.data.position();
        let count = self.data.read(bytes)?;
        self.reads.push((position, count));
        Ok(count)
    }
}
impl Seek for Observed {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.data.seek(position)
    }
}

#[test]
fn rob_index_skips_models_and_reads_only_the_selected_segment() {
    let mut bytes = b"OARC".to_vec();
    bytes.extend(4_u32.to_be_bytes());
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(b"OARD");
    bytes.extend(166_u32.to_be_bytes());
    for (name, payload) in [(b"FIRST\0\0\0", b"abc"), (b"SECOND\0\0", b"xyz")] {
        let mut header = [0; 80];
        header[..4].copy_from_slice(&83_u32.to_le_bytes());
        header[4..12].copy_from_slice(name);
        header[76..80].copy_from_slice(&3_u32.to_le_bytes());
        bytes.extend(header);
        bytes.extend(payload);
    }
    bytes.extend(b"END ");
    let full = rob::parse_rob_file(&bytes).unwrap();
    let mut reader = Observed::new(bytes);
    let index = RobIndex::read(&mut reader).unwrap();
    assert_eq!(reader.reads, [(0, 20), (20, 80), (103, 80)]);
    assert_eq!(index.segments[1].name(), "SECOND");
    assert!(index.segments.iter().all(|entry| entry.data.is_empty()));
    reader.reads.clear();
    assert_eq!(
        index.read_segment(&mut reader, 1).unwrap().data,
        full.segments[1].data
    );
    assert_eq!(reader.reads, [(183, 3)]);
    assert!(index.read_segment(&mut reader, 2).is_err());
    reader.data.get_mut().truncate(182);
    assert!(RobIndex::read(&mut reader).is_err());
}

#[test]
fn bsi_index_skips_pixels_and_decodes_one_record() {
    let mut body = b"BHDR".to_vec();
    body.extend(26_u32.to_be_bytes());
    let mut header = [0; 26];
    header[4..6].copy_from_slice(&2_i16.to_le_bytes());
    header[6..8].copy_from_slice(&1_i16.to_le_bytes());
    header[14..16].copy_from_slice(&1_i16.to_le_bytes());
    body.extend(header);
    body.extend(b"DATA");
    body.extend(2_u32.to_be_bytes());
    body.extend([1, 2]);
    body.extend(b"END ");
    let mut bytes = Vec::new();
    for name in [b"D02000\0\0\0", b"D02001\0\0\0"] {
        bytes.extend(name);
        bytes.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
        bytes.extend(&body);
    }
    let full = bsi::parse_bsi_file(&bytes).unwrap();
    let mut reader = Observed::new(bytes);
    let index = BsiIndex::read(&mut reader).unwrap();
    assert_eq!(reader.reads, [(0, 4), (0, 13), (61, 13)]);
    assert_eq!(index.images[1].name, "D02001");
    assert_eq!(index.images[1].image_index, 1);
    reader.reads.clear();
    assert_eq!(
        index.read_image(&mut reader, 1).unwrap().pixel_data,
        full.images[1].pixel_data
    );
    assert_eq!(reader.reads, [(61, 61)]);
    reader.data.get_mut().truncate(100);
    assert!(BsiIndex::read(&mut reader).is_err());
}

#[test]
fn sfx_index_skips_pcm_and_preserves_the_selected_effect() {
    let mut bytes = vec![0; 52];
    bytes[..4].copy_from_slice(b"FXHD");
    bytes[40..44].copy_from_slice(&2_u32.to_le_bytes());
    bytes[44..48].copy_from_slice(b"FXDT");
    for pcm in [b"abc", b"xyz"] {
        let mut header = [0; 27];
        header[8..12].copy_from_slice(&11025_u32.to_le_bytes());
        header[22..26].copy_from_slice(&3_u32.to_le_bytes());
        bytes.extend(header);
        bytes.extend(pcm);
    }
    let full = sfx::parse_sfx_file(&bytes).unwrap();
    let mut reader = Observed::new(bytes);
    let index = SfxIndex::read(&mut reader).unwrap();
    assert_eq!(reader.reads, [(0, 52), (52, 27), (82, 27)]);
    assert!(index.effects.iter().all(|entry| entry.pcm_data.is_empty()));
    reader.reads.clear();
    assert_eq!(
        index.read_effect(&mut reader, 1).unwrap().pcm_data,
        full.effects[1].pcm_data
    );
    assert_eq!(reader.reads, [(109, 3)]);
    reader.data.get_mut().truncate(110);
    assert!(SfxIndex::read(&mut reader).is_err());
}
