use super::sfx::AudioType;
use crate::{Result, error::Error};

const FOOTER_TAG: &[u8; 4] = b"RNAV";
const FOOTER_SIZE: usize = 12;
const INDEX_ENTRY_SIZE: usize = 12;
const AUDIO_HEADER_SIZE: usize = 27;
const PAYLOAD_PREFIX_SIZE: usize = 6;

#[derive(Debug, Clone)]
pub struct RtxIndexEntry {
    pub tag: [u8; 4],
    pub payload_offset: u32,
    pub payload_size: u32,
}

#[derive(Debug, Clone)]
/// Audio metadata embedded in voice entries (same 27-byte layout as SFX).
pub struct RtxAudioHeader {
    pub audio_type: AudioType,
    pub sample_rate: u32,
    pub loop_flag: i8,
    pub loop_offset: u32,
    pub loop_end: u32,
    pub audio_length: u32,
}

#[derive(Debug, Clone)]
pub enum RtxEntry {
    Text {
        tag: [u8; 4],
        text: String,
    },
    Audio {
        tag: [u8; 4],
        label: String,
        header: RtxAudioHeader,
        pcm_data: Vec<u8>,
    },
}

impl RtxEntry {
    #[must_use]
    pub fn tag_str(&self) -> String {
        let tag = match self {
            Self::Text { tag, .. } | Self::Audio { tag, .. } => tag,
        };
        String::from_utf8_lossy(tag).to_string()
    }

    #[must_use]
    pub const fn is_audio(&self) -> bool {
        matches!(self, Self::Audio { .. })
    }
}

#[derive(Debug, Clone)]
pub struct RtxFile {
    pub index_count: u32,
    pub entries: Vec<RtxEntry>,
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn parse_footer(input: &[u8]) -> Result<(u32, u32)> {
    if input.len() < FOOTER_SIZE {
        return Err(Error::Parse("RTX file too small for footer".into()));
    }

    let footer_start = input.len() - FOOTER_SIZE;

    let tag = &input[footer_start..footer_start + 4];
    if tag != FOOTER_TAG {
        return Err(Error::Parse(format!(
            "RTX footer tag mismatch: expected RNAV, got {:?}",
            String::from_utf8_lossy(tag)
        )));
    }

    let index_offset = read_u32_le(input, footer_start + 4)
        .ok_or_else(|| Error::Parse("failed to read footer index_offset".into()))?;
    let index_count = read_u32_le(input, footer_start + 8)
        .ok_or_else(|| Error::Parse("failed to read footer index_count".into()))?;

    Ok((index_offset, index_count))
}

fn parse_index_table(
    input: &[u8],
    index_offset: u32,
    index_count: u32,
) -> Result<Vec<RtxIndexEntry>> {
    let offset = usize::try_from(index_offset)
        .map_err(|_| Error::Parse("RTX index_offset does not fit usize".to_string()))?;
    let count = usize::try_from(index_count)
        .map_err(|_| Error::Parse("RTX index_count does not fit usize".to_string()))?;
    let required = offset + count * INDEX_ENTRY_SIZE;

    if required > input.len() {
        return Err(Error::Parse(format!(
            "RTX index table extends beyond file (need {required} bytes, have {})",
            input.len()
        )));
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * INDEX_ENTRY_SIZE;
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&input[base..base + 4]);

        let payload_offset = read_u32_le(input, base + 4)
            .ok_or_else(|| Error::Parse(format!("index {i}: failed to read payload_offset")))?;
        let payload_size = read_u32_le(input, base + 8)
            .ok_or_else(|| Error::Parse(format!("index {i}: failed to read payload_size")))?;

        let payload_offset_usize = usize::try_from(payload_offset)
            .map_err(|_| Error::Parse(format!("index {i}: payload_offset does not fit usize")))?;
        let payload_size_usize = usize::try_from(payload_size)
            .map_err(|_| Error::Parse(format!("index {i}: payload_size does not fit usize")))?;
        let end = payload_offset_usize + payload_size_usize;
        if end > input.len() {
            return Err(Error::Parse(format!(
                "index {i} (tag '{}'): payload extends beyond file ({end} > {})",
                String::from_utf8_lossy(&tag),
                input.len()
            )));
        }

        entries.push(RtxIndexEntry {
            tag,
            payload_offset,
            payload_size,
        });
    }

    Ok(entries)
}

fn parse_audio_header(data: &[u8]) -> Result<RtxAudioHeader> {
    if data.len() < AUDIO_HEADER_SIZE {
        return Err(Error::Parse(format!(
            "audio header too small: need {AUDIO_HEADER_SIZE} bytes, have {}",
            data.len()
        )));
    }

    let type_id = read_u32_le(data, 0)
        .ok_or_else(|| Error::Parse("audio header: failed to read type_id".into()))?;
    let _bit_depth = read_u32_le(data, 4);
    let sample_rate = read_u32_le(data, 8)
        .ok_or_else(|| Error::Parse("audio header: failed to read sample_rate".into()))?;
    #[allow(clippy::cast_possible_wrap)] // Engine stores this as a signed flag byte in binary data.
    let loop_flag = data[13] as i8;
    let loop_offset = read_u32_le(data, 14)
        .ok_or_else(|| Error::Parse("audio header: failed to read loop_offset".into()))?;
    let loop_end = read_u32_le(data, 18)
        .ok_or_else(|| Error::Parse("audio header: failed to read loop_end".into()))?;
    let audio_length = read_u32_le(data, 22)
        .ok_or_else(|| Error::Parse("audio header: failed to read audio_length".into()))?;

    let audio_type = AudioType::from_type_id(type_id)
        .ok_or_else(|| Error::Parse(format!("audio header: unknown type_id {type_id}")))?;

    Ok(RtxAudioHeader {
        audio_type,
        sample_rate,
        loop_flag,
        loop_offset,
        loop_end,
        audio_length,
    })
}

fn parse_payload(tag: [u8; 4], payload: &[u8]) -> Result<RtxEntry> {
    let tag_str = String::from_utf8_lossy(&tag);

    if payload.len() < PAYLOAD_PREFIX_SIZE {
        return Err(Error::Parse(format!(
            "entry '{}': payload too small ({} < {PAYLOAD_PREFIX_SIZE})",
            tag_str,
            payload.len()
        )));
    }

    let subtype = payload[1];
    let string_len: usize = read_u16_le(payload, 2)
        .ok_or_else(|| Error::Parse(format!("entry '{tag_str}': failed to read string_len")))?
        .into();

    let string_start = PAYLOAD_PREFIX_SIZE;
    let string_end = string_start + string_len;

    if string_end > payload.len() {
        return Err(Error::Parse(format!(
            "entry '{tag_str}': string extends beyond payload ({string_end} > {})",
            payload.len()
        )));
    }

    let text = String::from_utf8_lossy(&payload[string_start..string_end]).to_string();

    match subtype {
        0 => Ok(RtxEntry::Text { tag, text }),
        1 => {
            let audio_start = string_end;
            if audio_start + AUDIO_HEADER_SIZE > payload.len() {
                return Err(Error::Parse(format!(
                    "entry '{tag_str}': not enough bytes for audio header at offset {audio_start}"
                )));
            }

            let header = parse_audio_header(&payload[audio_start..])?;
            let pcm_start = audio_start + AUDIO_HEADER_SIZE;
            let pcm_len = usize::try_from(header.audio_length).map_err(|_| {
                Error::Parse(format!(
                    "entry '{tag_str}': audio_length does not fit usize"
                ))
            })?;
            let pcm_end = pcm_start + pcm_len;

            if pcm_end > payload.len() {
                return Err(Error::Parse(format!(
                    "entry '{tag_str}': PCM data extends beyond payload ({pcm_end} > {})",
                    payload.len()
                )));
            }

            let pcm_data = payload[pcm_start..pcm_end].to_vec();

            Ok(RtxEntry::Audio {
                tag,
                label: text,
                header,
                pcm_data,
            })
        }
        other => Err(Error::Parse(format!(
            "entry '{tag_str}': unknown subtype {other}"
        ))),
    }
}

/// Parses a Redguard RTX dialogue/audio container from raw bytes.
pub fn parse_rtx_file(input: &[u8]) -> Result<RtxFile> {
    let (index_offset, index_count) = parse_footer(input)?;
    let index = parse_index_table(input, index_offset, index_count)?;

    let mut entries = Vec::with_capacity(index.len());
    for (i, idx) in index.iter().enumerate() {
        let start = usize::try_from(idx.payload_offset)
            .map_err(|_| Error::Parse(format!("index {i}: payload_offset does not fit usize")))?;
        let size = usize::try_from(idx.payload_size)
            .map_err(|_| Error::Parse(format!("index {i}: payload_size does not fit usize")))?;
        let end = start + size;
        let payload = &input[start..end];

        let entry = parse_payload(idx.tag, payload).map_err(|e| {
            Error::Parse(format!(
                "index {i} (tag '{}'): {e}",
                String::from_utf8_lossy(&idx.tag)
            ))
        })?;
        entries.push(entry);
    }

    Ok(RtxFile {
        index_count,
        entries,
    })
}

impl RtxFile {
    #[must_use]
    pub fn audio_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_audio()).count()
    }

    #[must_use]
    pub fn text_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_audio()).count()
    }
}

impl RtxAudioHeader {
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        let bytes_per_sample = (u32::from(self.audio_type.bits_per_sample()) / 8)
            * u32::from(self.audio_type.channels());
        if self.sample_rate == 0 || bytes_per_sample == 0 {
            return 0.0;
        }
        f64::from(self.audio_length) / (f64::from(self.sample_rate) * f64::from(bytes_per_sample))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rtx(payloads: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut index_entries: Vec<([u8; 4], u32, u32)> = Vec::new();

        for (tag, payload) in payloads {
            buf.extend_from_slice(*tag);
            let payload_len = u32::try_from(payload.len()).expect("payload length must fit u32");
            buf.extend_from_slice(&payload_len.to_be_bytes());
            let payload_offset = u32::try_from(buf.len()).expect("offset must fit u32");
            buf.extend_from_slice(payload);
            index_entries.push((**tag, payload_offset, payload_len));
        }

        let index_offset = u32::try_from(buf.len()).expect("index offset must fit u32");
        for (tag, off, size) in &index_entries {
            buf.extend_from_slice(tag);
            buf.extend_from_slice(&off.to_le_bytes());
            buf.extend_from_slice(&size.to_le_bytes());
        }

        buf.extend_from_slice(FOOTER_TAG);
        buf.extend_from_slice(&index_offset.to_le_bytes());
        buf.extend_from_slice(
            &u32::try_from(payloads.len())
                .expect("payload count must fit u32")
                .to_le_bytes(),
        );

        buf
    }

    fn make_text_payload(text: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(0); // kind
        buf.push(0); // subtype
        buf.extend_from_slice(
            &u16::try_from(text.len())
                .expect("text length must fit u16")
                .to_le_bytes(),
        );
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(text.as_bytes());
        buf
    }

    fn make_audio_payload(label: &str, type_id: u32, sample_rate: u32, pcm_len: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(0); // kind
        buf.push(1); // subtype
        buf.extend_from_slice(
            &u16::try_from(label.len())
                .expect("label length must fit u16")
                .to_le_bytes(),
        );
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(label.as_bytes());
        // 27-byte audio header
        buf.extend_from_slice(&type_id.to_le_bytes()); // type_id
        buf.extend_from_slice(&type_id.to_le_bytes()); // bit_depth
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.push(100); // level_0c
        buf.push(0); // loop_flag
        buf.extend_from_slice(&0u32.to_le_bytes()); // loop_offset
        buf.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // loop_end
        buf.extend_from_slice(&pcm_len.to_le_bytes()); // audio_length
        buf.push(0); // reserved_1a
        let pcm_size = usize::try_from(pcm_len).expect("pcm length must fit usize");
        buf.extend_from_slice(&vec![0x80u8; pcm_size]);
        buf
    }

    #[test]
    fn parse_single_text_entry() {
        let payload = make_text_payload("Hello world");
        let data = make_rtx(&[(b"txt1", &payload)]);
        let file = parse_rtx_file(&data).unwrap();
        assert_eq!(file.entries.len(), 1);
        assert_eq!(file.text_count(), 1);
        assert_eq!(file.audio_count(), 0);
        match &file.entries[0] {
            RtxEntry::Text { tag, text } => {
                assert_eq!(tag, b"txt1");
                assert_eq!(text, "Hello world");
            }
            RtxEntry::Audio { .. } => panic!("expected text entry"),
        }
    }

    #[test]
    fn parse_single_audio_entry() {
        let payload = make_audio_payload("Cyrus speaks", 1, 22050, 100);
        let data = make_rtx(&[(b"aud1", &payload)]);
        let file = parse_rtx_file(&data).unwrap();
        assert_eq!(file.entries.len(), 1);
        assert_eq!(file.audio_count(), 1);
        assert_eq!(file.text_count(), 0);
        match &file.entries[0] {
            RtxEntry::Audio {
                tag,
                label,
                header,
                pcm_data,
            } => {
                assert_eq!(tag, b"aud1");
                assert_eq!(label, "Cyrus speaks");
                assert_eq!(header.audio_type, AudioType::Mono16);
                assert_eq!(header.sample_rate, 22050);
                assert_eq!(pcm_data.len(), 100);
            }
            RtxEntry::Text { .. } => panic!("expected audio entry"),
        }
    }

    #[test]
    fn parse_mixed_entries() {
        let text = make_text_payload("line one");
        let audio = make_audio_payload("voice", 0, 11025, 50);
        let data = make_rtx(&[(b"tst1", &text), (b"vox1", &audio)]);
        let file = parse_rtx_file(&data).unwrap();
        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.text_count(), 1);
        assert_eq!(file.audio_count(), 1);
    }

    #[test]
    fn rejects_bad_footer_tag() {
        let mut data = make_rtx(&[]);
        let len = data.len();
        data[len - 12..len - 8].copy_from_slice(b"XXXX");
        assert!(parse_rtx_file(&data).is_err());
    }

    #[test]
    fn rejects_truncated_file() {
        let data = vec![0u8; 8];
        assert!(parse_rtx_file(&data).is_err());
    }
}
