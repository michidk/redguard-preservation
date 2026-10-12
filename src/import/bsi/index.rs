use super::BsiImage;
use crate::{Result, error::Error};
use std::io::{Read, Seek, SeekFrom};

/// A named image record without decoded pixels.
pub struct BsiIndexEntry {
    pub name: String,
    pub image_index: u16,
    offset: u64,
    length: usize,
}
/// Image record boundaries, read without consuming pixel or animation payloads.
pub struct BsiIndex {
    pub images: Vec<BsiIndexEntry>,
}
impl BsiIndex {
    /// Reads standalone identity or named TEXBSI record envelopes only.
    pub fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut tag = [0; 4];
        reader.read_exact(&mut tag)?;
        if &tag == b"IFHD" || &tag == b"BSIF" {
            return Ok(Self {
                images: vec![BsiIndexEntry {
                    name: String::new(),
                    image_index: 0,
                    offset: 0,
                    length: usize::try_from(length).map_err(|e| Error::Parse(e.to_string()))?,
                }],
            });
        }
        let mut position = 0;
        let mut images = Vec::new();
        while length.saturating_sub(position) >= 13 {
            reader.seek(SeekFrom::Start(position))?;
            let mut header = [0; 13];
            reader.read_exact(&mut header)?;
            if header.starts_with(b"END ") {
                position += 8;
                continue;
            }
            if header[..9].iter().all(|byte| *byte == 0) {
                break;
            }
            let name = String::from_utf8_lossy(&header[..9])
                .trim_matches('\0')
                .to_string();
            let size = u64::from(super::read_le_u32(&header, 9)?) + 13;
            if size > length - position {
                return Err(Error::Parse(format!(
                    "BSI record {name}: payload exceeds file length"
                )));
            }
            let image_index = name
                .get(3..)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            images.push(BsiIndexEntry {
                name,
                image_index,
                offset: position,
                length: usize::try_from(size).map_err(|e| Error::Parse(e.to_string()))?,
            });
            position += size;
        }
        Ok(Self { images })
    }
    /// Reads and decodes the selected image record and its animation frames.
    pub fn read_image(&self, reader: &mut (impl Read + Seek), index: usize) -> Result<BsiImage> {
        let entry = self
            .images
            .get(index)
            .ok_or_else(|| Error::Parse(format!("BSI image {index} missing")))?;
        reader.seek(SeekFrom::Start(entry.offset))?;
        let mut bytes = vec![0; entry.length];
        reader.read_exact(&mut bytes)?;
        super::parse_bsi_file(&bytes)?
            .images
            .into_iter()
            .next()
            .ok_or_else(|| Error::Parse(format!("BSI image {index}: empty record")))
    }
}
