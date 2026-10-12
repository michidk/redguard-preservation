//! Sectioned text editor state from STATE.RST. See docs/formats/RST.md.

use crate::{Result, error::Error};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RstEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RstSection {
    pub name: String,
    pub entries: Vec<RstEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RstFile<'a> {
    pub sections: Vec<RstSection>,
    #[serde(skip)]
    pub image: &'a [u8],
}

/// Parse section headers and key=value lines, preserving order and duplicate keys.
///
/// Values remain text, including embedded equals signs and semicolons. Legacy
/// text bytes map directly to U+0000..U+00FF; no code page is assumed.
pub fn parse_rst_file(data: &[u8]) -> Result<RstFile<'_>> {
    if data
        .iter()
        .any(|&b| b < 0x20 && !matches!(b, b'\t' | b'\r' | b'\n'))
    {
        return Err(Error::Parse("RST: invalid text control byte".into()));
    }
    let content: String = data.iter().map(|&b| char::from(b)).collect();
    let mut sections: Vec<RstSection> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim_start_matches([' ', '\t']);
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let invalid = || {
            Error::Parse(format!(
                "RST: invalid section or assignment on line {}",
                i + 1
            ))
        };
        if line.starts_with('[') {
            let name = line
                .trim_end_matches([' ', '\t'])
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .filter(|s| !s.is_empty() && !s.contains(['[', ']']))
                .ok_or_else(invalid)?;
            sections.push(RstSection {
                name: name.to_owned(),
                entries: Vec::new(),
            });
        } else {
            let (key, value) = line.split_once('=').ok_or_else(invalid)?;
            let key = key.trim_matches([' ', '\t']);
            if key.is_empty() {
                return Err(invalid());
            }
            sections
                .last_mut()
                .ok_or_else(invalid)?
                .entries
                .push(RstEntry {
                    key: key.to_owned(),
                    value: value.to_owned(),
                });
        }
    }
    if sections.is_empty() {
        return Err(Error::Parse("RST: no sections".into()));
    }
    Ok(RstFile {
        sections,
        image: data,
    })
}
