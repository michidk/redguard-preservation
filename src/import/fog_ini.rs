//! Glide scene fog table decoding. See docs/config/fog-ini.md.

use crate::{Result, error::Error};

/// Decode comma-separated index/density pairs into the 64-entry Glide table.
/// Each density fills up to the next index. The valueless terminal index 63
/// remains zero. Semicolon comments and whitespace are ignored.
pub fn parse_fog_ini(text: &str) -> Result<[u8; 64]> {
    let tokens: Vec<_> = text
        .lines()
        .flat_map(|line| line.split(';').next().unwrap_or_default().split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let invalid =
        || Error::Parse("FOG.INI: expected increasing index/density pairs ending with 63".into());
    if tokens.len() < 3 || tokens.len() % 2 != 1 || tokens.last() != Some(&"63") {
        return Err(invalid());
    }
    let mut table = [0; 64];
    let mut previous = None;
    for pair in tokens[..tokens.len() - 1].as_chunks::<2>().0 {
        let index = pair[0].parse::<usize>().map_err(|_| invalid())?;
        let density = pair[1].parse::<u8>().map_err(|_| invalid())?;
        if index >= 63 {
            return Err(invalid());
        }
        if let Some((start, value)) = previous {
            if index <= start {
                return Err(invalid());
            }
            table[start..index].fill(value);
        }
        previous = Some((index, density));
    }
    if let Some((start, value)) = previous {
        table[start..63].fill(value);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::parse_fog_ini;

    #[test]
    fn fills_bands_without_interpolating_or_filling_the_end_marker() {
        let table = parse_fog_ini("; table\n0,0,\n33,1, ; first band\n36,8,\n48,255,\n63").unwrap();
        assert_eq!(&table[..33], &[0; 33]);
        assert_eq!(&table[33..36], &[1; 3]);
        assert_eq!(&table[36..48], &[8; 12]);
        assert_eq!(&table[48..63], &[255; 15]);
        assert_eq!(table[63], 0);
    }

    #[test]
    fn rejects_invalid_indices_densities_and_terminal_markers() {
        for text in [
            "",
            "63",
            "0,0",
            "0,0,62",
            "0,0,63,255",
            "0,256,63",
            "0,-1,63",
            "33,1,32,2,63",
            "0,1,0,2,63",
            "64,1,63",
            "bad,1,63",
        ] {
            assert!(
                parse_fog_ini(text)
                    .unwrap_err()
                    .to_string()
                    .contains("FOG.INI"),
                "{text}"
            );
        }
    }
}
