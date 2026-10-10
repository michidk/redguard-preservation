//! Data-defined book-menu pages; see `docs/config/menu-ini.md`.
use std::collections::BTreeMap;

/// Global menu flags relevant to cinematic visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuGeneral {
    pub force_movies: bool,
}

impl MenuGeneral {
    /// Missing force_movies is zero; the first case-insensitive occurrence wins.
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut general = false;
        let mut force_movies = false;
        let mut seen = false;
        for line in content.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if line.starts_with('[') {
                general = line.eq_ignore_ascii_case("[general]");
            } else if general
                && let Some((key, value)) = line.split_once('=')
                && key.trim().eq_ignore_ascii_case("force_movies")
                && !seen
            {
                seen = true;
                force_movies = match value.trim() {
                    "0" => false,
                    "1" => true,
                    _ => return Err("MENU.INI [general] force_movies: expected 0 or 1".into()),
                };
            }
        }
        Ok(Self { force_movies })
    }
}

/// A TEXBSI archive and image index used as a page surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuTexture {
    pub set: u16,
    pub index: u16,
}

/// Text and interaction fields from one sequentially indexed menu element.
/// Coordinates are texels on the selected page texture, not screen pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub text: String,
    pub position: [i32; 2],
    pub texture: usize,
    /// 0 = left, 1 = center, 2 = right.
    pub justify: u8,
    /// Widget/key-name output in page texels, with the same alignment codes.
    pub output_position: [i32; 2],
    pub output_justify: u8,
    /// Inclusive bounds; missing fields default to zero.
    pub slider: [i32; 2],
    pub selectable: bool,
    pub default: bool,
    pub grayed: bool,
    /// Engine-defined action ID, independent of the displayed label.
    pub action: i32,
}

/// One page's optional texture references and ordered entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuPage {
    pub textures: [Option<MenuTexture>; 2],
    pub entries: Vec<MenuEntry>,
}

impl MenuPage {
    /// Parse `[pageN]` from MENU.INI. Missing numeric element fields default to
    /// zero, as in the engine. Rejects invalid recognized values, missing texture
    /// references, and gaps in the text indices. Unrelated fields are ignored.
    pub fn parse(content: &str, page: usize) -> Result<Self, String> {
        let section = format!("[page{page}]");
        let context = format!("MENU.INI {section}");
        let mut fields = BTreeMap::new();
        let mut active = false;
        for line in content.lines() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if line.starts_with('[') {
                active = line.eq_ignore_ascii_case(&section);
            } else if active && let Some((key, value)) = line.split_once('=') {
                fields
                    .entry(key.trim().to_ascii_lowercase())
                    .or_insert(value.trim());
            }
        }
        let number = |name: &str, index: usize, required: bool| -> Result<i32, String> {
            let key = format!("{name}[{index}]");
            match fields.get(&key) {
                Some(value) => value
                    .parse()
                    .map_err(|_| format!("{context} {key}: invalid integer {value:?}")),
                None if required => Err(format!("{context}: missing {key}")),
                None => Ok(0),
            }
        };
        let bounded = |name: &str, index, maximum, required| -> Result<i32, String> {
            let value = number(name, index, required)?;
            if !(0..=maximum).contains(&value) {
                return Err(format!(
                    "{context} {name}[{index}]: expected 0..{maximum}, got {value}"
                ));
            }
            Ok(value)
        };
        let mut textures = [None; 2];
        for (index, texture) in textures.iter_mut().enumerate() {
            if index == 0
                || fields.contains_key(&format!("texture_set[{index}]"))
                || fields.contains_key(&format!("texture_index[{index}]"))
            {
                *texture = Some(MenuTexture {
                    set: bounded("texture_set", index, i32::from(u16::MAX), true)? as u16,
                    index: bounded("texture_index", index, i32::from(u16::MAX), true)? as u16,
                });
            }
        }
        let mut entries = Vec::new();
        while let Some(text) = fields.get(&format!("text[{}]", entries.len())) {
            let index = entries.len();
            let texture = bounded("texture", index, 1, false)? as usize;
            if textures[texture].is_none() {
                return Err(format!(
                    "{context} texture[{index}]: undefined texture slot {texture}"
                ));
            }
            let slider = [
                number("slider_min", index, false)?,
                number("slider_max", index, false)?,
            ];
            if slider[0] > slider[1] {
                return Err(format!(
                    "{context} slider_min[{index}]: exceeds slider_max[{index}]"
                ));
            }
            entries.push(MenuEntry {
                text: (*text).to_owned(),
                position: [
                    number("text_x", index, false)?,
                    number("text_y", index, false)?,
                ],
                texture,
                justify: bounded("justify", index, 2, false)? as u8,
                output_position: [
                    number("output_x", index, false)?,
                    number("output_y", index, false)?,
                ],
                output_justify: bounded("output_justify", index, 2, false)? as u8,
                slider,
                selectable: bounded("selectable", index, 1, false)? != 0,
                default: bounded("default", index, 1, false)? != 0,
                grayed: bounded("grayed", index, 1, false)? != 0,
                action: number("action", index, false)?,
            });
        }
        if entries.is_empty() {
            return Err(format!("{context}: missing text[0]"));
        }
        for key in fields.keys() {
            if let Some(index) = key.strip_prefix("text[").and_then(|s| s.strip_suffix(']')) {
                let index: usize = index
                    .parse()
                    .map_err(|_| format!("{context}: invalid key {key}"))?;
                if index >= entries.len() {
                    return Err(format!("{context}: nonsequential {key}"));
                }
            }
        }
        Ok(Self { textures, entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PAGE: &str = "[page0]\ntexture_set[0]=9\ntexture_set[1]=8\ntexture_index[0]=3\ntexture_index[1]=2\ntext[0]=Changed label\ntext_x[0]=71\ntext_y[0]=24\ntexture[0]=1\naction[0]=-2\nselectable[0]=1\ntext[1]=Other\n[page1]\ntext[0]=Different page\n";

    #[test]
    fn cinematic_visibility_flag_is_section_scoped() {
        assert!(
            MenuGeneral::parse("[general]\nforce_movies=1\n[page3]\nforce_movies=0")
                .unwrap()
                .force_movies
        );
        assert!(!MenuGeneral::parse("").unwrap().force_movies);
        assert!(MenuGeneral::parse("[general]\nforce_movies=bad").is_err());
    }

    #[test]
    fn reads_labels_layout_actions_and_zero_defaults_from_data() {
        let page = MenuPage::parse(PAGE, 0).unwrap();
        assert_eq!(
            page.textures,
            [
                Some(MenuTexture { set: 9, index: 3 }),
                Some(MenuTexture { set: 8, index: 2 })
            ]
        );
        assert_eq!(
            page.entries[0],
            MenuEntry {
                text: "Changed label".into(),
                position: [71, 24],
                texture: 1,
                justify: 0,
                output_position: [0, 0],
                output_justify: 0,
                slider: [0, 0],
                selectable: true,
                default: false,
                grayed: false,
                action: -2,
            }
        );
        assert_eq!(page.entries[1].position, [0, 0]);
    }

    #[test]
    fn single_surface_settings_page_preserves_widget_fields() {
        let text = "[page5]\ntexture_set[0]=289\ntexture_index[0]=15\ntext[0]=Control\noutput_x[0]=20\noutput_y[0]=-4\noutput_justify[0]=2\nslider_min[0]=-10\nslider_max[0]=10\n";
        let page = MenuPage::parse(text, 5).unwrap();
        assert_eq!(page.textures[1], None);
        assert_eq!(page.entries[0].output_position, [20, -4]);
        assert_eq!(page.entries[0].output_justify, 2);
        assert_eq!(page.entries[0].slider, [-10, 10]);
        for extra in ["texture[0]=1", "texture_set[1]=289"] {
            assert!(MenuPage::parse(&format!("{text}{extra}\n"), 5).is_err());
        }
        assert!(
            MenuPage::parse(&text.replace("slider_min[0]=-10", "slider_min[0]=11"), 5).is_err()
        );
    }

    #[test]
    fn rejects_missing_invalid_and_nonsequential_fields_with_context() {
        for text in [
            PAGE.replace("texture_index[0]=3", ""),
            PAGE.replace("texture[0]=1", "texture[0]=2"),
            PAGE.replace("text[1]=Other", "text[2]=Other"),
            PAGE.replace("action[0]=-2", "action[0]=bad"),
        ] {
            assert!(
                MenuPage::parse(&text, 0)
                    .unwrap_err()
                    .contains("MENU.INI [page0]")
            );
        }
    }
}
