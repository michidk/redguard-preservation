//! Installed actor-pool settings from SYSTEM.INI, as specified in engine/SOUP.md.

/// Fresh initialization uses this capacity when assigning runtime actor identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoupConfig {
    pub actor_pool_capacity: u32,
    /// Effective load-time defaults, truncated to their actor attribute bytes.
    pub jump_time: u8,
    pub jump_height: u8,
    /// Only the original numeric value one disables family links.
    pub disable_family: bool,
}

impl SoupConfig {
    /// Reads the required signed value and applies the engine's minimum of ten.
    /// Duplicate relevant keys are rejected because their precedence is unsupported.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut section = "";
        let mut jump_time = None;
        let mut jump_height = None;
        let mut disable_family = None;
        let mut capacity = None;
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                section = name.trim();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if section.eq_ignore_ascii_case("system")
                && key.trim().eq_ignore_ascii_case("max_active_objects")
            {
                if capacity.is_some() {
                    return Err(
                        "SYSTEM.INI [system] max_active_objects: duplicate key unsupported".into(),
                    );
                }
                capacity = Some(value.trim().parse::<i32>().map_err(|_| {
                    "SYSTEM.INI [system] max_active_objects: expected signed integer".to_owned()
                })?);
            } else {
                let target = match (
                    section.to_ascii_lowercase().as_str(),
                    key.trim().to_ascii_lowercase().as_str(),
                ) {
                    ("system", "jump_time") => &mut jump_time,
                    ("system", "jump_height") => &mut jump_height,
                    ("debug", "disable_family") => &mut disable_family,
                    _ => continue,
                };
                if target.is_some() {
                    return Err(format!(
                        "SYSTEM.INI [{section}] {}: duplicate key unsupported",
                        key.trim()
                    ));
                }
                let value = match value.trim().to_ascii_lowercase().as_str() {
                    "true" | "yes" | "on" => 1,
                    "false" | "no" | "off" | "" => 0,
                    number => number.parse::<i32>().map_err(|_| {
                        format!(
                            "SYSTEM.INI [{section}] {}: expected signed integer or switch",
                            key.trim()
                        )
                    })?,
                };
                *target = Some(value);
            }
        }
        let capacity =
            capacity.ok_or("SYSTEM.INI [system] max_active_objects: missing required value")?;
        Ok(Self {
            actor_pool_capacity: capacity.max(10) as u32,
            jump_time: jump_time.filter(|value| *value >= 1).unwrap_or(6) as u8,
            jump_height: jump_height.filter(|value| *value >= 1).unwrap_or(80) as u8,
            disable_family: disable_family == Some(1),
        })
    }

    /// Applies RAAT load-time defaults; model-path message-group assignment is separate.
    pub fn initialize_attributes(&self, attributes: &mut [u8; 256]) {
        if (attributes[31] as i8) <= 0 {
            attributes[31] = self.jump_time;
        }
        if attributes[32] == 0 {
            attributes[32] = self.jump_height;
        }
        if attributes[35] == 0 {
            attributes[35] = 2;
        }
        if attributes[34] == 0 {
            attributes[34] = 10;
        }
        if self.disable_family {
            attributes[25] = 0;
            attributes[26] = 0;
        }
        attributes[115] = attributes[6];
        attributes[119] = attributes[1];
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

#[cfg(test)]
mod actor_defaults_tests {
    use super::SoupConfig;

    #[test]
    fn load_defaults_preserve_positive_bytes_and_copy_reserved_fields() {
        let config = SoupConfig::parse("[system]\nmax_active_objects=32\njump_time=258\njump_height=259\n[debug]\ndisable_family=1\n").unwrap();
        for byte in 0..=255u8 {
            let mut attributes = [byte; 256];
            attributes[1] = 12;
            attributes[6] = 34;
            config.initialize_attributes(&mut attributes);
            assert_eq!(
                attributes[31],
                if byte == 0 || byte >= 128 { 2 } else { byte }
            );
            assert_eq!(attributes[32], if byte == 0 { 3 } else { byte });
            assert_eq!(attributes[34], if byte == 0 { 10 } else { byte });
            assert_eq!(attributes[35], if byte == 0 { 2 } else { byte });
            assert_eq!((attributes[25], attributes[26]), (0, 0));
            assert_eq!((attributes[115], attributes[119]), (34, 12));
            assert_eq!(attributes[125], byte, "model-path assignment is separate");
            assert_eq!(attributes[44], byte);
        }
    }

    #[test]
    fn configuration_defaults_and_family_exact_one_are_retained() {
        for token in ["-1", "0", "2", "255", "no", "off", "false", ""] {
            let config = SoupConfig::parse(&format!("[system]\nmax_active_objects=32\njump_time=-1\njump_height=0\n[debug]\ndisable_family={token}\n")).unwrap();
            assert_eq!((config.jump_time, config.jump_height), (6, 80));
            let mut attributes = [0; 256];
            attributes[25] = 9;
            attributes[26] = 24;
            config.initialize_attributes(&mut attributes);
            assert_eq!((attributes[25], attributes[26]), (9, 24));
        }
        for token in ["1", "yes", "ON", "true"] {
            let config = SoupConfig::parse(&format!(
                "[system]\nmax_active_objects=32\n[debug]\ndisable_family={token}\n"
            ))
            .unwrap();
            assert!(config.disable_family);
        }
    }

    #[test]
    fn invalid_and_duplicate_defaults_report_the_setting() {
        for setting in ["jump_time", "jump_height"] {
            for values in [
                format!("{setting}=bad"),
                format!("{setting}=1\n{setting}=2"),
            ] {
                let error =
                    SoupConfig::parse(&format!("[system]\nmax_active_objects=32\n{values}\n"))
                        .unwrap_err();
                assert!(error.contains(setting));
            }
        }
    }
}
