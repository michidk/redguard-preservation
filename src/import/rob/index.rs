use super::{RobHeader, RobSegment, parser};
use crate::{Result, error::Error};
use std::io::{Read, Seek, SeekFrom};

/// ROB headers indexed without reading embedded model payloads.
pub struct RobIndex {
    pub header: RobHeader,
    /// Segment metadata; payloads are loaded by `read_segment`.
    pub segments: Vec<RobSegment>,
    offsets: Vec<u64>,
}
impl RobIndex {
    /// Reads headers and seeks over payloads, rejecting ranges beyond the file.
    pub fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut bytes = [0; 20];
        reader.read_exact(&mut bytes)?;
        let (remaining, header) = parser::parse_rob_header(&bytes)?;
        let mut position = 20 - remaining.len() as u64;
        if u64::from(header.num_segments) > length.saturating_sub(position) / 80 {
            return Err(Error::Parse("ROB segment count exceeds file length".into()));
        }
        let mut segments = Vec::new();
        let mut offsets = Vec::new();
        for index in 0..header.num_segments {
            reader.seek(SeekFrom::Start(position))?;
            let mut bytes = [0; 80];
            reader.read_exact(&mut bytes)?;
            let (_, segment) = parser::parse_rob_segment_header(&bytes)?;
            position += 80;
            offsets.push(position);
            if !segment.points_to_external_file() {
                position += u64::from(segment.data_size);
            }
            if position > length {
                return Err(Error::Parse(format!(
                    "ROB segment {index}: payload exceeds file length"
                )));
            }
            segments.push(segment);
        }
        Ok(Self {
            header,
            segments,
            offsets,
        })
    }

    /// Reads just one indexed model payload, preserving its segment metadata.
    pub fn read_segment(
        &self,
        reader: &mut (impl Read + Seek),
        index: usize,
    ) -> Result<RobSegment> {
        let mut segment = self
            .segments
            .get(index)
            .ok_or_else(|| Error::Parse(format!("ROB segment {index} missing")))?
            .clone();
        if !segment.points_to_external_file() {
            reader.seek(SeekFrom::Start(self.offsets[index]))?;
            segment.data.resize(
                usize::try_from(segment.data_size).map_err(|e| Error::Parse(e.to_string()))?,
                0,
            );
            reader.read_exact(&mut segment.data)?;
        }
        Ok(segment)
    }
}
