use crate::{Result, error::Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Encoded audio channel/bit-depth layout for an SFX effect.
pub enum AudioType {
    Mono8,
    Mono16,
    Stereo8,
    Stereo16,
}

impl AudioType {
    #[must_use]
    pub(crate) const fn from_type_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Mono8),
            1 => Some(Self::Mono16),
            2 => Some(Self::Stereo8),
            3 => Some(Self::Stereo16),
            _ => None,
        }
    }

    /// Returns the number of PCM channels for this audio type.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        match self {
            Self::Mono8 | Self::Mono16 => 1,
            Self::Stereo8 | Self::Stereo16 => 2,
        }
    }

    /// Returns bits per sample for this audio type.
    #[must_use]
    pub const fn bits_per_sample(&self) -> u16 {
        match self {
            Self::Mono8 | Self::Stereo8 => 8,
            Self::Mono16 | Self::Stereo16 => 16,
        }
    }
}

#[derive(Debug, Clone)]
/// Parsed sound effect entry from an SFX file.
pub struct SfxEffect {
    pub audio_type: AudioType,
    pub sample_rate: u32,
    pub loop_flag: i8,
    pub loop_offset: u32,
    pub loop_end: u32,
    pub pcm_data: Vec<u8>,
}

impl SfxEffect {
    /// Estimates effect duration in seconds from PCM byte length and format.
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        let bytes_per_sample = (u32::from(self.audio_type.bits_per_sample()) / 8)
            * u32::from(self.audio_type.channels());
        if self.sample_rate == 0 || bytes_per_sample == 0 {
            return 0.0;
        }
        let pcm_len = u32::try_from(self.pcm_data.len()).unwrap_or(u32::MAX);
        f64::from(pcm_len) / (f64::from(self.sample_rate) * f64::from(bytes_per_sample))
    }
}

#[derive(Debug, Clone)]
/// Parsed SFX file description and effect list.
pub struct SfxFile {
    pub description: String,
    pub effects: Vec<SfxEffect>,
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

/// Parses a Redguard SFX file from bytes.
pub fn parse_sfx_file(input: &[u8]) -> Result<SfxFile> {
    if input.len() < 48 {
        return Err(Error::Parse("SFX file too small for FXHD section".into()));
    }

    // FXHD: 4 bytes BE section_size + 32 bytes description + 4 bytes LE effect_count
    let _section_size = read_u32_be(input, 0)
        .ok_or_else(|| Error::Parse("failed to read FXHD section_size".into()))?;

    let description = String::from_utf8_lossy(&input[4..36])
        .trim_matches('\0')
        .to_string();

    let effect_count = usize::try_from(
        read_u32_le(input, 36)
            .ok_or_else(|| Error::Parse("failed to read FXHD effect_count".into()))?,
    )
    .map_err(|_| Error::Parse("FXHD effect_count does not fit usize".to_string()))?;

    // FXDT: starts at offset 40 with 4 bytes BE section_size
    let fxdt_offset = 40;
    if input.len() < fxdt_offset + 4 {
        return Err(Error::Parse("SFX file too small for FXDT header".into()));
    }
    let _fxdt_size = read_u32_be(input, fxdt_offset)
        .ok_or_else(|| Error::Parse("failed to read FXDT section_size".into()))?;

    let mut cursor = fxdt_offset + 4;
    let mut effects = Vec::with_capacity(effect_count);

    for i in 0..effect_count {
        if cursor + 27 > input.len() {
            return Err(Error::Parse(format!(
                "effect {i}: not enough bytes for 27-byte header at offset {cursor}"
            )));
        }

        let type_id = read_u32_le(input, cursor)
            .ok_or_else(|| Error::Parse(format!("effect {i}: failed to read type_id")))?;
        let _bit_depth = read_u32_le(input, cursor + 4);
        let sample_rate = read_u32_le(input, cursor + 8)
            .ok_or_else(|| Error::Parse(format!("effect {i}: failed to read sample_rate")))?;
        #[allow(clippy::cast_possible_wrap)]
        // Engine stores this as a signed flag byte in binary data.
        let loop_flag = input[cursor + 13] as i8;
        let loop_offset = read_u32_le(input, cursor + 14)
            .ok_or_else(|| Error::Parse(format!("effect {i}: failed to read loop_offset")))?;
        let loop_end = read_u32_le(input, cursor + 18)
            .ok_or_else(|| Error::Parse(format!("effect {i}: failed to read loop_end")))?;
        let data_length = usize::try_from(
            read_u32_le(input, cursor + 22)
                .ok_or_else(|| Error::Parse(format!("effect {i}: failed to read data_length")))?,
        )
        .map_err(|_| Error::Parse(format!("effect {i}: data_length does not fit usize")))?;

        cursor += 27;

        if cursor + data_length > input.len() {
            return Err(Error::Parse(format!(
                "effect {i}: PCM data extends beyond file (need {data_length} bytes at offset {cursor})"
            )));
        }

        let audio_type = AudioType::from_type_id(type_id)
            .ok_or_else(|| Error::Parse(format!("effect {i}: unknown audio type_id {type_id}")))?;

        let pcm_data = input[cursor..cursor + data_length].to_vec();
        cursor += data_length;

        effects.push(SfxEffect {
            audio_type,
            sample_rate,
            loop_flag,
            loop_offset,
            loop_end,
            pcm_data,
        });
    }

    Ok(SfxFile {
        description,
        effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_sfx(effect_count: u32, effects_data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        let fxhd_payload_size: u32 = 36;
        buf.extend_from_slice(&fxhd_payload_size.to_be_bytes());
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&effect_count.to_le_bytes());
        let fxdt_size = u32::try_from(effects_data.len()).expect("effects data must fit u32");
        buf.extend_from_slice(&fxdt_size.to_be_bytes());
        buf.extend_from_slice(effects_data);
        buf.extend_from_slice(b"END ");
        buf
    }

    fn make_effect_bytes(type_id: u32, sample_rate: u32, pcm_len: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&type_id.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // bit_depth
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.push(64); // unknown_0c
        buf.push(0); // loop_flag
        buf.extend_from_slice(&0u32.to_le_bytes()); // loop_offset
        buf.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // loop_end
        buf.extend_from_slice(&pcm_len.to_le_bytes());
        buf.push(0); // unknown_1a
        let pcm_size = usize::try_from(pcm_len).expect("pcm length must fit usize");
        buf.extend_from_slice(&vec![0x80u8; pcm_size]);
        buf
    }

    #[test]
    fn parse_single_effect() {
        let effect = make_effect_bytes(1, 22050, 100);
        let sfx = make_minimal_sfx(1, &effect);
        let file = parse_sfx_file(&sfx).unwrap();
        assert_eq!(file.effects.len(), 1);
        assert_eq!(file.effects[0].audio_type, AudioType::Mono16);
        assert_eq!(file.effects[0].sample_rate, 22050);
        assert_eq!(file.effects[0].pcm_data.len(), 100);
    }

    #[test]
    fn parse_multiple_effects() {
        let mut data = Vec::new();
        data.extend_from_slice(&make_effect_bytes(0, 11025, 50));
        data.extend_from_slice(&make_effect_bytes(3, 22050, 200));
        let sfx = make_minimal_sfx(2, &data);
        let file = parse_sfx_file(&sfx).unwrap();
        assert_eq!(file.effects.len(), 2);
        assert_eq!(file.effects[0].audio_type, AudioType::Mono8);
        assert_eq!(file.effects[1].audio_type, AudioType::Stereo16);
    }

    #[test]
    fn rejects_truncated_file() {
        let sfx = vec![0u8; 10];
        assert!(parse_sfx_file(&sfx).is_err());
    }
}
