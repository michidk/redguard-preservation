//! Glide terrain inputs from WORLD.INI, per engine/terrain-lighting.md.

use crate::{Result, error::Error};

/// Encoded RGB strengths and a normalized direction in engine coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldLighting {
    pub ambient: [f32; 3],
    pub directional: [f32; 3],
    pub direction: [f32; 3],
}

impl WorldLighting {
    /// Read one world's terrain lighting. Missing entries use engine defaults.
    /// Malformed channels and angles outside the rotation table are rejected.
    pub fn parse(content: &str, world: u32) -> Result<Self> {
        let value = |name: &str| {
            let key = format!("{name}[{world}]");
            content
                .lines()
                .filter_map(|line| line.split_once('='))
                .filter(|(name, _)| name.trim().eq_ignore_ascii_case(&key))
                .map(|(_, value)| value.trim())
                .next_back()
        };
        let error = |key: &str| Error::Parse(format!("WORLD.INI world {world}: invalid {key}"));
        let channels = |key: &str, fallback: &str, count: usize| -> Result<[f32; 3]> {
            let values = value(key)
                .unwrap_or(fallback)
                .split(',')
                .map(|v| v.trim().parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| error(key))?;
            if values.len() != count {
                return Err(error(key));
            }
            Ok([values[0] as f32, values[1] as f32, values[2] as f32])
        };
        let read_angle = |key: &str| -> Result<u16> {
            let angle = value(key)
                .unwrap_or("0")
                .parse::<u16>()
                .map_err(|_| error(key))?;
            if angle >= 2048 {
                return Err(error(key));
            }
            Ok(angle)
        };
        let angle = read_angle("world_sunangle")?;
        let skew = read_angle("world_sunskew")?;
        #[allow(clippy::approx_constant)]
        // The engine uses this decimal full-period constant when building the float table.
        let sample =
            |index: u16| ((f64::from(index) * 6.2831852 / 2048.0).sin() as f32 * 16000.0).trunc();
        let direction = glam::Vec3::from([sample(angle), sample(angle + 512), sample(skew)])
            .normalize_or_zero()
            .to_array();
        Ok(Self {
            ambient: channels("world_ambientrgb", "128,128,128", 3)?,
            directional: channels("world_sunrgb", "128,128,100,0", 4)?,
            direction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_installed_day_inputs() {
        let defaults = WorldLighting::parse("", 1).unwrap();
        assert_eq!(defaults.ambient, [128.0; 3]);
        assert_eq!(defaults.directional, [128.0, 128.0, 100.0]);
        assert_eq!(defaults.direction, [0.0, 1.0, 0.0]);
        let input = "world_ambientrgb[1]=64,64,64\nworld_sunrgb[1]=255,255,255,2\nworld_sunangle[1]=256\nworld_sunskew[1]=512";
        let day = WorldLighting::parse(input, 1).unwrap();
        assert_eq!(day.ambient, [64.0; 3]);
        assert_eq!(day.directional, [255.0; 3]);
        for (actual, expected) in day
            .direction
            .into_iter()
            .zip([0.49998432, 0.49998432, 0.7071289])
        {
            assert!((actual - expected).abs() < 0.000001);
        }
        assert_eq!(WorldLighting::parse(input, 2).unwrap(), defaults);
        assert_eq!(
            WorldLighting::parse("world_ambientrgb[1]=1,2,3\nWORLD_AMBIENTRGB[1]=4,5,6", 1)
                .unwrap()
                .ambient,
            [4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn invalid_fields_report_world_and_key() {
        for (key, value) in [
            ("world_ambientrgb", "1,2"),
            ("world_sunrgb", "1,2,3"),
            ("world_sunangle", "2048"),
            ("world_sunskew", "-1"),
        ] {
            let error = WorldLighting::parse(&format!("{key}[27]={value}"), 27)
                .unwrap_err()
                .to_string();
            assert!(error.contains("world 27") && error.contains(key));
        }
    }
}
