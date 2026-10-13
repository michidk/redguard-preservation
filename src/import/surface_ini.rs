//! Texture surface assignments and animation sound remapping; see `docs/config/surface-ini.md`.

/// Surface codes in the executable's table order.
pub const SURFACE_NAMES: [&str; 12] = [
    "unknown",
    "water",
    "deepwater",
    "scapewater",
    "scapedeepwater",
    "lava",
    "sand",
    "wood",
    "tile",
    "scape",
    "rock",
    "gloop",
];

/// Parsed texture assignments and twelve independent animation sound tables.
///
/// Surface codes index [`SURFACE_NAMES`]. Sound IDs are zero-based effect-bank
/// indices, not frequencies or durations. See `docs/config/surface-ini.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceIni {
    pub assignments: SurfaceAssignments,
    sounds: [[u16; 256]; 12],
}

impl SurfaceIni {
    /// Parse `[surfaces]` and any named sound sections from installed text.
    ///
    /// Missing/empty sound sections retain identity. Repeated sound keys and
    /// sections overwrite earlier values, unlike first-match texture assignments.
    /// Both sound tokens must start with a decimal digit. Their decimal prefixes
    /// are read as nonnegative signed 32-bit integers; overflow is rejected.
    /// Sources must fit in 0..=255; targets retain their low 16 bits.
    /// Unknown sections are ignored. Errors include file, section, line and key.
    pub fn parse(content: &str) -> Result<Self, String> {
        let assignments = SurfaceAssignments::parse(content)?;
        let mut sounds = [std::array::from_fn(|id| id as u16); 12];
        let mut active = None;
        for (line_number, line) in records(content) {
            if let Some(header) = line.strip_prefix('[') {
                let header = header.strip_suffix(']').ok_or_else(|| {
                    format!("SURFACE.INI line {line_number}: invalid section header {line:?}")
                })?;
                active = SURFACE_NAMES
                    .iter()
                    .position(|name| header.eq_ignore_ascii_case(name));
                continue;
            }
            let Some(surface) = active else { continue };
            // The shared game INI reader separates '=' before reading tokens.
            let separated = line.replace('=', " = ");
            let mut tokens = separated.split_ascii_whitespace();
            let key = tokens.next().unwrap_or_default();
            let error = |detail: &str| {
                format!(
                    "SURFACE.INI [{}] line {line_number}, key {key:?}: {detail}",
                    SURFACE_NAMES[surface]
                )
            };
            let source = sound_number(key)
                .filter(|value| *value < 256)
                .ok_or_else(|| error("sound source must start with a digit and be in 0..255"))?;
            let mut value = tokens.next().unwrap_or_default();
            if value == "=" {
                value = tokens.next().unwrap_or_default();
            }
            let target = sound_number(value).ok_or_else(|| {
                error("sound target must start with a digit and fit in 0..2147483647")
            })?;
            sounds[surface][source as usize] = target as u16;
        }
        Ok(Self {
            assignments,
            sounds,
        })
    }

    /// Raw configured row, including the `unknown` row that animation bypasses.
    /// Missing entries map each source ID to itself. Invalid surface codes return
    /// `None`; valid codes are 0..=11 in [`SURFACE_NAMES`] order.
    pub fn sound_table(&self, surface: u8) -> Option<&[u16; 256]> {
        self.sounds.get(usize::from(surface))
    }

    /// Resolve an animation PlaySound effect using an actor's assigned surface.
    ///
    /// Surface zero bypasses remapping. Returns `None` for surface codes outside
    /// 0..=11, including the original engine's out-of-bounds code 12. The caller
    /// must validate a decoded animation operand before converting it to `u8`.
    /// The returned ID still needs checking against the loaded sound bank.
    /// Script, combat and speech sounds must not use this remapping.
    pub fn animation_sound(&self, surface: u8, sound: u8) -> Option<u16> {
        let row = self.sound_table(surface)?;
        Some(if surface == 0 {
            u16::from(sound)
        } else {
            row[usize::from(sound)]
        })
    }
}

fn sound_number(token: &str) -> Option<i32> {
    token
        .as_bytes()
        .first()
        .filter(|byte| byte.is_ascii_digit())?;
    decimal_prefix(token)
}

fn records(content: &str) -> impl Iterator<Item = (usize, &str)> {
    content.lines().enumerate().filter_map(|(index, line)| {
        let line = line.split(';').next().unwrap_or_default().trim();
        (!line.is_empty()).then_some((index + 1, line))
    })
}

/// Ordered texture assignments. Sound-remap sections are outside this parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceAssignments {
    entries: Vec<(u16, u8)>,
}

impl SurfaceAssignments {
    /// Parse `[surfaces]`, preserving first-match and zero-key termination semantics.
    /// Numeric tokens use the original decimal-prefix conversion; overflow is rejected.
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut entries = Vec::new();
        let mut found = false;
        for (line_number, line) in records(content) {
            if line.starts_with('[') {
                if found {
                    break;
                }
                found = line.eq_ignore_ascii_case("[surfaces]");
                continue;
            }
            if !found {
                continue;
            }
            let error = |detail| {
                format!("SURFACE.INI [surfaces] line {line_number}, record {line:?}: {detail}")
            };
            let fields: Vec<_> = line.split(',').map(str::trim).collect();
            if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
                return Err(error("expected set, index, type"));
            }
            let set = decimal_prefix(fields[0])
                .filter(|v| (0..512).contains(v))
                .ok_or_else(|| error("texture set outside 0..511"))?;
            let index = decimal_prefix(fields[1])
                .filter(|v| (0..128).contains(v))
                .ok_or_else(|| error("texture index outside 0..127"))?;
            let surface = SURFACE_NAMES
                .iter()
                .position(|name| fields[2].eq_ignore_ascii_case(name))
                .ok_or_else(|| error("unknown surface type"))?;
            entries.push((((set as u16) << 7) | index as u16, surface as u8));
        }
        if !found {
            return Err("SURFACE.INI: missing [surfaces]".into());
        }
        Ok(Self { entries })
    }

    /// Resolve a packed model-face texture key `(set << 7) | image`.
    /// Zero is the unknown surface, including for an unmatched key.
    pub fn surface(&self, texture: u16) -> u8 {
        self.entries
            .iter()
            .take_while(|(key, _)| *key != 0)
            .find(|(key, _)| *key == texture)
            .map_or(0, |(_, surface)| *surface)
    }
}

fn decimal_prefix(token: &str) -> Option<i32> {
    let (negative, digits) = match token.as_bytes().first() {
        Some(b'-') => (true, &token[1..]),
        Some(b'+') => (false, &token[1..]),
        _ => (false, token),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .try_fold(0_i32, |value, digit| {
            value.checked_mul(10)?.checked_add(i32::from(digit - b'0'))
        })?;
    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_texture_lookup_matches_original_rules() {
        let assignments = SurfaceAssignments::parse(
            "; comment\n[surfaces]\n42,3,WOOD\n42,3,rock\n42,all,sand\n43,+2suffix,tile\n0,0,lava\n44,1,gloop\n[wood]\n4=89\n",
        ).unwrap();
        assert_eq!(assignments.surface((42 << 7) | 3), 7);
        assert_eq!(assignments.surface(42 << 7), 6);
        assert_eq!(assignments.surface((42 << 7) | 1), 0);
        assert_eq!(assignments.surface((43 << 7) | 2), 8);
        assert_eq!(assignments.surface(0), 0);
        assert_eq!(assignments.surface((44 << 7) | 1), 0);
    }

    #[test]
    fn invalid_assignments_report_line_and_field() {
        for (line, expected) in [
            ("512,0,wood", "set"),
            ("1,128,wood", "index"),
            ("1,-1,wood", "index"),
            ("1,0,missing", "type"),
            ("1,,wood", "expected"),
            ("999999999999,0,wood", "set"),
        ] {
            let error = SurfaceAssignments::parse(&format!("[surfaces]\n{line}")).unwrap_err();
            assert!(
                error.contains("line 2") && error.contains(expected),
                "{error}"
            );
        }
        assert!(SurfaceAssignments::parse("[wood]\n4=89").is_err());
        assert_eq!(
            SurfaceAssignments::parse("[surfaces]")
                .unwrap()
                .surface(128),
            0
        );
    }
}
