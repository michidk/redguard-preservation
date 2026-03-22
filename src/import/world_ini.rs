/// Parser for `WORLD.INI` — the game's world/level database.
///
/// Extracts the per-world palette mapping so the CLI can auto-resolve
/// `--palette` when converting RGM or WLD files.
use std::collections::HashMap;

/// A single world entry from `WORLD.INI`.
#[derive(Debug, Clone)]
pub struct WorldEntry {
    /// World index (`N` in `world_map[N]`).
    pub index: u32,
    /// RGM scene path (from `world_map[N]`), backslash-separated, as-is from the INI.
    pub map: String,
    /// Optional WLD terrain path (from `world_world[N]`).
    pub world: Option<String>,
    /// COL palette path (from `world_palette[N]`).
    pub palette: String,
}

/// Parsed `WORLD.INI` contents.
#[derive(Debug, Clone)]
pub struct WorldIni {
    pub entries: Vec<WorldEntry>,
}

/// Extract the filename (last path component) from a WORLD.INI path value,
/// normalizing backslash separators.
#[must_use]
fn extract_filename(ini_path: &str) -> &str {
    let normalized = ini_path.trim();
    normalized.rsplit(['\\', '/']).next().unwrap_or(normalized)
}

/// Extract the file stem (filename without extension) from a WORLD.INI path,
/// lowercased for case-insensitive matching.
#[must_use]
fn stem_lower(ini_path: &str) -> String {
    let filename = extract_filename(ini_path);
    match filename.rsplit_once('.') {
        Some((stem, _)) => stem.to_ascii_lowercase(),
        None => filename.to_ascii_lowercase(),
    }
}

impl WorldIni {
    /// Parses the text content of a `WORLD.INI` file.
    ///
    /// Only extracts `world_map[N]`, `world_world[N]`, and `world_palette[N]`
    /// keys. All other keys are ignored. Malformed lines (including the known
    /// typos in the shipped file) are silently skipped.
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let mut maps: HashMap<u32, String> = HashMap::new();
        let mut worlds: HashMap<u32, String> = HashMap::new();
        let mut palettes: HashMap<u32, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();

            if let Some(idx) = parse_indexed_key(&key, "world_map") {
                maps.insert(idx, value);
            } else if let Some(idx) = parse_indexed_key(&key, "world_world") {
                worlds.insert(idx, value);
            } else if let Some(idx) = parse_indexed_key(&key, "world_palette") {
                palettes.insert(idx, value);
            }
        }

        let mut entries: Vec<WorldEntry> = maps
            .into_iter()
            .filter_map(|(idx, map)| {
                let palette = palettes.remove(&idx)?;
                Some(WorldEntry {
                    index: idx,
                    map,
                    world: worlds.remove(&idx),
                    palette,
                })
            })
            .collect();

        entries.sort_by_key(|e| e.index);
        Self { entries }
    }

    /// Returns all world entries whose RGM map stem matches `file_stem`
    /// (case-insensitive).
    #[must_use]
    pub fn find_by_map_stem(&self, file_stem: &str) -> Vec<&WorldEntry> {
        let needle = file_stem.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|e| stem_lower(&e.map) == needle)
            .collect()
    }

    /// Returns all world entries whose WLD terrain stem matches `file_stem`
    /// (case-insensitive).
    #[must_use]
    pub fn find_by_world_stem(&self, file_stem: &str) -> Vec<&WorldEntry> {
        let needle = file_stem.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|e| e.world.as_ref().is_some_and(|w| stem_lower(w) == needle))
            .collect()
    }
}

/// Parses `prefix[N]` and returns `N`, or `None` if the key doesn't match.
#[must_use]
fn parse_indexed_key(key: &str, prefix: &str) -> Option<u32> {
    let rest = key.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('[')?;
    let rest = rest.strip_suffix(']')?;
    rest.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_INI: &str = "\
[world]
start_world=0
world_map[0]=MAPS\\start.rgm
world_world[0]=MAPS\\hideout.WLD
world_palette[0]=3DART\\sunset.COL
";

    #[test]
    fn parse_single_entry() {
        let ini = WorldIni::parse(MINIMAL_INI);
        assert_eq!(ini.entries.len(), 1);
        let e = &ini.entries[0];
        assert_eq!(e.index, 0);
        assert_eq!(e.map, "MAPS\\start.rgm");
        assert_eq!(e.world.as_deref(), Some("MAPS\\hideout.WLD"));
        assert_eq!(e.palette, "3DART\\sunset.COL");
    }

    const MULTI_INI: &str = "\
[world]
world_map[0]=MAPS\\start.rgm
world_world[0]=MAPS\\hideout.WLD
world_palette[0]=3DART\\sunset.COL
world_map[1]=MAPS\\ISLAND.rgm
world_world[1]=MAPS\\ISLAND.WLD
world_palette[1]=3DART\\island.COL
world_map[27]=MAPS\\island.rgm
world_world[27]=MAPS\\ISLand.WLD
world_palette[27]=3DART\\nightsky.COL
world_map[28]=MAPS\\ISLAND.rgm
world_world[28]=MAPS\\ISLAND.WLD
world_palette[28]=3DART\\sunset.COL
";

    #[test]
    fn parse_multiple_entries_sorted() {
        let ini = WorldIni::parse(MULTI_INI);
        assert_eq!(ini.entries.len(), 4);
        assert_eq!(ini.entries[0].index, 0);
        assert_eq!(ini.entries[1].index, 1);
        assert_eq!(ini.entries[2].index, 27);
        assert_eq!(ini.entries[3].index, 28);
    }

    #[test]
    fn find_by_map_stem_case_insensitive() {
        let ini = WorldIni::parse(MULTI_INI);
        let matches = ini.find_by_map_stem("island");
        assert_eq!(matches.len(), 3, "ISLAND.rgm appears in worlds 1, 27, 28");
        assert_eq!(matches[0].index, 1);
        assert_eq!(matches[1].index, 27);
        assert_eq!(matches[2].index, 28);
    }

    #[test]
    fn find_by_map_stem_unique() {
        let ini = WorldIni::parse(MULTI_INI);
        let matches = ini.find_by_map_stem("start");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].palette, "3DART\\sunset.COL");
    }

    #[test]
    fn find_by_world_stem() {
        let ini = WorldIni::parse(MULTI_INI);
        let matches = ini.find_by_world_stem("island");
        assert_eq!(matches.len(), 3, "ISLAND.WLD appears in worlds 1, 27, 28");
    }

    #[test]
    fn find_by_world_stem_hideout() {
        let ini = WorldIni::parse(MULTI_INI);
        let matches = ini.find_by_world_stem("hideout");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].index, 0);
    }

    #[test]
    fn missing_palette_skips_entry() {
        let ini = WorldIni::parse("world_map[5]=MAPS\\observe.rgm\n");
        assert!(ini.entries.is_empty(), "no palette → no entry");
    }

    #[test]
    fn comments_and_blanks_ignored() {
        let content = "\
; this is a comment
# so is this

[world]
world_map[2]=MAPS\\catacomb.rgm
world_palette[2]=3DART\\catacomb.COL
";
        let ini = WorldIni::parse(content);
        assert_eq!(ini.entries.len(), 1);
        assert_eq!(ini.entries[0].index, 2);
    }

    #[test]
    fn extract_filename_handles_backslash() {
        assert_eq!(extract_filename("3DART\\island.COL"), "island.COL");
        assert_eq!(extract_filename("MAPS\\ISLAND.rgm"), "ISLAND.rgm");
        assert_eq!(extract_filename("island.COL"), "island.COL");
    }

    #[test]
    fn stem_lower_normalizes() {
        assert_eq!(stem_lower("3DART\\ISLAND.COL"), "island");
        assert_eq!(stem_lower("MAPS\\start.rgm"), "start");
    }

    #[test]
    fn no_world_field_still_parses() {
        let content = "\
world_map[3]=MAPS\\PALACE.rgm
world_palette[3]=3DART\\palace00.COL
";
        let ini = WorldIni::parse(content);
        assert_eq!(ini.entries.len(), 1);
        assert!(ini.entries[0].world.is_none());
    }
}
