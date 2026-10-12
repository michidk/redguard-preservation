//! Installed actor-pool settings from SYSTEM.INI, as specified in engine/SOUP.md.

/// Fresh initialization uses this capacity when assigning runtime actor identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoupConfig {
    pub actor_pool_capacity: u32,
}

impl SoupConfig {
    /// Reads the required signed value and applies the engine's minimum of ten.
    /// Duplicate relevant keys are rejected because their precedence is unsupported.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut system = false;
        let mut capacity = None;
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                system = section.trim().eq_ignore_ascii_case("system");
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if system && key.trim().eq_ignore_ascii_case("max_active_objects") {
                if capacity.is_some() {
                    return Err(
                        "SYSTEM.INI [system] max_active_objects: duplicate key unsupported".into(),
                    );
                }
                capacity = Some(value.trim().parse::<i32>().map_err(|_| {
                    "SYSTEM.INI [system] max_active_objects: expected signed integer".to_owned()
                })?);
            }
        }
        let capacity =
            capacity.ok_or("SYSTEM.INI [system] max_active_objects: missing required value")?;
        Ok(Self {
            actor_pool_capacity: capacity.max(10) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SoupConfig;

    #[test]
    fn pool_capacity_uses_its_section_and_the_engine_minimum() {
        for (value, expected) in [(32, 32), (0, 10), (-1, 10), (9, 10), (65535, 65535)] {
            let text = format!(
                "[other]\nmax_active_objects=123\n[SyStEm]\n MAX_ACTIVE_OBJECTS = {value} ; note\n"
            );
            assert_eq!(
                SoupConfig::parse(&text).unwrap().actor_pool_capacity,
                expected
            );
        }
    }

    #[test]
    fn missing_malformed_and_duplicate_capacity_are_contextual_errors() {
        for text in [
            "[other]\nmax_active_objects=32\n",
            "[system]\nmax_active_objects=bad\n",
            "[system]\nmax_active_objects=2147483648\n",
            "[system]\nmax_active_objects=32\nMAX_ACTIVE_OBJECTS=64\n",
        ] {
            assert!(
                SoupConfig::parse(text)
                    .unwrap_err()
                    .contains("SYSTEM.INI [system] max_active_objects")
            );
        }
    }
}
