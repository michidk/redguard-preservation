//! Installed defaults for book-menu preferences; see `docs/engine/menu.md`.
use std::collections::BTreeMap;

/// SYSTEM.INI preferences expressed in the menu's units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSettings {
    pub subtitles: bool,
    pub movie_interlace: bool,
    pub voice: bool,
    /// Rounded from the engine's 0..256 volume using a factor of 12.8.
    pub sound_volume: u8,
    pub music_volume: u8,
    /// Held-direction repeat delay in BIOS timer ticks.
    pub repeat_ticks: u32,
}

impl MenuSettings {
    /// Parse required settings from their original sections; the first occurrence wins.
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut section = String::new();
        let mut fields = BTreeMap::new();
        for line in content.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].to_ascii_lowercase();
            } else if let Some((key, value)) = line.split_once('=') {
                fields
                    .entry((section.clone(), key.trim().to_ascii_lowercase()))
                    .or_insert(value.trim());
            }
        }
        let number = |section: &str, key: &str, max: u32| -> Result<u32, String> {
            let context = format!("SYSTEM.INI [{section}] {key}");
            let value = fields
                .get(&(section.into(), key.into()))
                .ok_or_else(|| format!("{context}: missing value"))?;
            value
                .parse::<u32>()
                .ok()
                .filter(|n| *n <= max)
                .ok_or_else(|| format!("{context}: expected 0..{max}, got {value:?}"))
        };
        let repeat_ticks = number("dialog", "menu_traverse_delay", u32::MAX)?;
        if repeat_ticks == 0 {
            return Err("SYSTEM.INI [dialog] menu_traverse_delay: must be positive".into());
        }
        Ok(Self {
            subtitles: number("dialog", "dialog_print_text", 1)? != 0,
            movie_interlace: number("screen", "smk_interlace", 1)? != 0,
            voice: number("dialog", "dialog_use_speech", 1)? != 0,
            sound_volume: (f64::from(number("system", "volume", 256)?) / 12.8).round() as u8,
            music_volume: (f64::from(number("system", "redbook_volume", 256)?) / 12.8).round()
                as u8,
            repeat_ticks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CONFIG: &str = "[dialog]\nmenu_traverse_delay=4\ndialog_print_text=1\ndialog_use_speech=0\n[screen]\nsmk_interlace=1\n[system]\nvolume=255\nredbook_volume=200\n";
    #[test]
    fn installed_preferences_use_menu_units_and_sections() {
        assert_eq!(
            MenuSettings::parse(CONFIG).unwrap(),
            MenuSettings {
                subtitles: true,
                movie_interlace: true,
                voice: false,
                sound_volume: 20,
                music_volume: 16,
                repeat_ticks: 4,
            }
        );
        for text in [
            CONFIG.replace("volume=255", "volume=257"),
            CONFIG.replace("delay=4", "delay=0"),
            CONFIG.replace("text=1", "text=bad"),
        ] {
            assert!(
                MenuSettings::parse(&text)
                    .unwrap_err()
                    .contains("SYSTEM.INI")
            );
        }
    }
}
