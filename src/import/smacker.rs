//! Smacker header metadata; compressed video/audio decoding is external.
/// One movie's source dimensions, timing and vertical display flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmackerHeader {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub frame_micros: u32,
    pub double_height: bool,
    /// At least one of the seven audio stream descriptors has a sample rate.
    pub has_audio: bool,
}
impl SmackerHeader {
    /// Read the fixed header of an SMK2/SMK4 stream. See docs/formats/SMK.md.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 104 || !matches!(&bytes[..4], b"SMK2" | b"SMK4") {
            return Err("SMK: missing or invalid header".into());
        }
        let number = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        let width = number(4);
        let height = number(8);
        let frames = number(12);
        let timing = number(16) as i32;
        let frame_micros = if timing < 0 {
            timing.unsigned_abs().checked_mul(10)
        } else {
            (timing as u32).checked_mul(1000)
        }
        .filter(|v| *v > 0)
        .ok_or("SMK: invalid frame timing")?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 || frames == 0 {
            return Err("SMK: invalid dimensions/frame count".into());
        }
        Ok(Self {
            width,
            height,
            frames,
            frame_micros,
            double_height: number(20) & 6 != 0,
            has_audio: (72..100).step_by(4).any(|i| number(i) & 0x00ff_ffff != 0),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_timing_is_hundredths_of_a_millisecond() {
        let mut header = [0; 104];
        header[..4].copy_from_slice(b"SMK2");
        for (offset, value) in [
            (4, 640u32),
            (8, 240),
            (12, 6895),
            (16, (-6666i32) as u32),
            (20, 2),
        ] {
            header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            SmackerHeader::parse(&header).unwrap(),
            SmackerHeader {
                width: 640,
                height: 240,
                frames: 6895,
                frame_micros: 66660,
                double_height: true,
                has_audio: false,
            }
        );
        assert!(SmackerHeader::parse(&header[..24]).is_err());
        header[16..20].fill(0);
        assert!(SmackerHeader::parse(&header).is_err());
    }
}
