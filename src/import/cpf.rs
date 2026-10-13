//! Reader and writer for REDGUARD.CPF console preferences.
use crate::{Result, error::Error};

/// Size of a complete version 2.0 console preference file.
pub const CPF_FILE_SIZE: usize = 40;

/// Raw persisted settings; interpretation and loading defaults belong to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpfFile {
    /// Font selector, normally zero or one.
    pub font: u32,
    /// Stored shadow setting, normally zero or one.
    pub shadow: u32,
    /// GUI color indices in edit, report, screen and paper order.
    pub colors: [u32; 4],
    /// Hexadecimal dump line count; the engine clamps values below one on load.
    pub hex_lines: i32,
    /// Zero enables paper artwork; one selects a solid background.
    pub paper_disabled: u32,
    /// Palette entry for the solid background.
    pub background: u8,
    /// Screen limit with eight fractional bits.
    pub screen_limit: i32,
}

/// Decode exactly one version 2.0 file without normalizing any stored values.
pub fn parse_cpf_file(data: &[u8]) -> Result<CpfFile> {
    if data.len() != CPF_FILE_SIZE {
        return Err(Error::Parse(format!(
            "CPF file must be exactly {CPF_FILE_SIZE} bytes, got {}",
            data.len()
        )));
    }
    if &data[..3] != b"2.0" {
        return Err(Error::Parse("CPF signature must be 2.0".into()));
    }
    let word = |offset| {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    };
    Ok(CpfFile {
        font: word(3),
        shadow: word(7),
        colors: [word(11), word(15), word(19), word(23)],
        hex_lines: word(27) as i32,
        paper_disabled: word(31),
        background: data[35],
        screen_limit: word(36) as i32,
    })
}

/// Encode every stored field, retaining raw values that a host may not support.
#[must_use]
pub fn encode_cpf_file(file: &CpfFile) -> [u8; CPF_FILE_SIZE] {
    let mut bytes = [0; CPF_FILE_SIZE];
    bytes[..3].copy_from_slice(b"2.0");
    for (offset, value) in [
        (3, file.font),
        (7, file.shadow),
        (11, file.colors[0]),
        (15, file.colors[1]),
        (19, file.colors[2]),
        (23, file.colors[3]),
        (27, file.hex_lines as u32),
        (31, file.paper_disabled),
        (36, file.screen_limit as u32),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[35] = file.background;
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    const BYTES: [u8; CPF_FILE_SIZE] = [
        b'2', b'.', b'0', 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 13, 0, 0, 0, 9, 0, 0, 0, 15, 0, 0, 0,
        255, 255, 255, 255, 1, 0, 0, 0, 199, 0, 44, 1, 0,
    ];

    #[test]
    fn reads_and_writes_the_documented_layout_without_normalizing() {
        let expected = CpfFile {
            font: 1,
            shadow: 2,
            colors: [3, 13, 9, 15],
            hex_lines: -1,
            paper_disabled: 1,
            background: 199,
            screen_limit: 300 * 256,
        };
        assert_eq!(parse_cpf_file(&BYTES).unwrap(), expected);
        assert_eq!(encode_cpf_file(&expected), BYTES);
    }

    #[test]
    fn rejects_incomplete_trailing_and_wrong_version_data() {
        for length in 0..CPF_FILE_SIZE {
            assert!(parse_cpf_file(&BYTES[..length]).is_err());
        }
        let mut trailing = BYTES.to_vec();
        trailing.push(0);
        assert!(parse_cpf_file(&trailing).is_err());
        let mut invalid = BYTES;
        invalid[0] = b'3';
        assert!(parse_cpf_file(&invalid).is_err());
    }
}
