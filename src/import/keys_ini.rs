//! Player input bindings from `KEYS.INI`; see `docs/config/keys-ini.md`.

/// Order of the eight directional/action bindings in [`KeysIni`].
pub const PLAYER_KEYS: [&str; 8] = ["up", "down", "left", "right", "a", "b", "c", "d"];

/// Raw input codes. Zero means unbound; 128 and above are device inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysIni {
    /// Keyboard slot, in [`PLAYER_KEYS`] order, before user overrides.
    pub keyboard: [u8; 8],
    /// Optional user overrides; zero or missing means no override.
    pub user: [u8; 8],
}

impl KeysIni {
    /// Parse the keyboard slot and user overrides in `[input]`.
    ///
    /// Requires all eight keyboard bindings. Ignores unrelated fields and lines
    /// without `=`, matching the existing WORLD.INI parser's handling of typos.
    /// Rejects malformed recognized values and codes outside the documented 0–139 range.
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut keyboard = [None; 8];
        let mut user = [0; 8];
        let mut input = false;
        for (line_number, line) in content.lines().enumerate() {
            let line = line.split(';').next().unwrap_or_default().trim();
            if line.starts_with('[') {
                input = line.eq_ignore_ascii_case("[input]");
                continue;
            }
            if !input {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            for (index, name) in PLAYER_KEYS.iter().enumerate() {
                let is_keyboard = key == format!("key_{name}[0]");
                if !is_keyboard && key != format!("user_key_{name}") {
                    continue;
                }
                let code = value.trim().parse::<u8>().ok().filter(|code| *code <= 139)
                    .ok_or_else(|| format!("KEYS.INI [input] {key}, line {}: expected input code 0..139, got {value:?}", line_number + 1))?;
                if is_keyboard {
                    keyboard[index] = Some(code);
                } else {
                    user[index] = code;
                }
            }
        }
        let mut result = Self {
            keyboard: [0; 8],
            user,
        };
        for (index, code) in keyboard.into_iter().enumerate() {
            result.keyboard[index] = code.ok_or_else(|| {
                format!("KEYS.INI [input]: missing key_{}[0]", PLAYER_KEYS[index])
            })?;
        }
        Ok(result)
    }

    /// Keyboard codes after applying nonzero user overrides.
    #[must_use]
    pub fn keyboard_codes(&self) -> [u8; 8] {
        std::array::from_fn(|i| {
            if self.user[i] == 0 {
                self.keyboard[i]
            } else {
                self.user[i]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = "[input]\nkey_up[0]=17\nkey_down[0]=31\nkey_left[0]=30\nkey_right[0]=32\nkey_a[0]=42\nkey_b[0]=57\nkey_c[0]=56\nkey_d[0]=18\n";

    #[test]
    fn bindings_and_overrides_are_section_scoped() {
        let text = format!(
            "{INPUT}USER_KEY_UP=72 ; remap\nuser_key_d=0\nkey_up[1]=138\nuser_key_d0\n[defined]\nuser_key_up=1\n"
        );
        assert_eq!(
            KeysIni::parse(&text).unwrap().keyboard_codes(),
            [72, 31, 30, 32, 42, 57, 56, 18]
        );
    }

    #[test]
    fn reports_missing_and_invalid_bindings() {
        assert!(
            KeysIni::parse("[input]\n")
                .unwrap_err()
                .contains("key_up[0]")
        );
        for value in ["-1", "140", "256", "no"] {
            let text = INPUT.replace("key_up[0]=17", &format!("key_up[0]={value}"));
            assert!(
                KeysIni::parse(&text)
                    .unwrap_err()
                    .contains("key_up[0], line 2")
            );
        }
    }
}
