//! Installed ordinary-player settings from SYSTEM.INI.
use crate::{Result, error::Error};
use std::collections::BTreeMap;

/// Numeric inputs for ordinary grounded movement, in original map/heading units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerConfig {
    pub standing_height: i32,
    pub post_collide_height: u32,
    pub slide_range: i32,
    pub turn_speed: u32,
    pub turn_max_speed: u32,
    pub poly_push_units: u32,
}

impl PlayerConfig {
    /// Reads the required installed values without inventing missing defaults.
    /// Duplicate relevant keys are rejected because their precedence is unsupported.
    pub fn parse(text: &str) -> Result<Self> {
        let mut section = String::new();
        let mut values = BTreeMap::new();
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                section = name.trim().to_ascii_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let relevant = match section.as_str() {
                "system" => matches!(
                    key.as_str(),
                    "standing_height" | "post_collide_height" | "slide_range"
                ),
                "cyrus" => matches!(
                    key.as_str(),
                    "turn_speed" | "turn_max_speed" | "poly_push_units"
                ),
                _ => false,
            };
            if relevant && values.insert(key.clone(), value.trim()).is_some() {
                return Err(Error::Parse(format!(
                    "SYSTEM.INI [{section}] {key}: duplicate key unsupported"
                )));
            }
        }
        let signed = |key: &str| -> Result<i32> {
            values
                .get(key)
                .ok_or_else(|| {
                    Error::Parse(format!("SYSTEM.INI {key}: missing required player setting"))
                })?
                .parse()
                .map_err(|_| Error::Parse(format!("SYSTEM.INI {key}: expected signed integer")))
        };
        let unsigned = |key: &str| -> Result<u32> {
            u32::try_from(signed(key)?)
                .map_err(|_| Error::Parse(format!("SYSTEM.INI {key}: negative value unsupported")))
        };
        Ok(Self {
            standing_height: signed("standing_height")?,
            post_collide_height: unsigned("post_collide_height")?,
            slide_range: signed("slide_range")?,
            turn_speed: unsigned("turn_speed")?,
            turn_max_speed: unsigned("turn_max_speed")?,
            poly_push_units: unsigned("poly_push_units")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerConfig;
    const INPUT: &str = "[system]\nstanding_height=-2\npost_collide_height=50\nslide_range=-60000\n[cyrus]\nturn_speed=12\nturn_max_speed=48\npoly_push_units=16\n";
    #[test]
    fn parses_sections_and_preserves_original_units() {
        assert_eq!(
            PlayerConfig::parse(INPUT).unwrap(),
            PlayerConfig {
                standing_height: -2,
                post_collide_height: 50,
                slide_range: -60000,
                turn_speed: 12,
                turn_max_speed: 48,
                poly_push_units: 16,
            }
        );
        assert_eq!(
            PlayerConfig::parse(&INPUT.to_uppercase()).unwrap(),
            PlayerConfig::parse(INPUT).unwrap()
        );
        assert!(PlayerConfig::parse(&format!("[unrelated]\nturn_speed=9\n{INPUT}")).is_ok());
    }
    #[test]
    fn rejects_missing_duplicate_and_invalid_settings() {
        for input in [
            String::new(),
            INPUT.replace("[cyrus]", "[other]"),
            INPUT.replace("turn_speed=12", "turn_speed=-1"),
            INPUT.replace("slide_range=-60000", "slide_range=bad"),
            format!("{INPUT}turn_speed=8"),
        ] {
            assert!(
                PlayerConfig::parse(&input)
                    .unwrap_err()
                    .to_string()
                    .contains("SYSTEM.INI")
            );
        }
    }
}
