//! Hugh's 3D Information Program text reports. See docs/formats/MORE.md.

use crate::{Result, error::Error};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MoreEntry {
    pub filename: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoreFile<'a> {
    pub tool_version: &'a str,
    pub copyright: &'a str,
    pub entries: Vec<MoreEntry>,
    pub declared_count: usize,
    #[serde(skip)]
    pub image: &'a [u8],
}

/// Parse the ordered model report and validate its final file count.
pub fn parse_more_file(data: &[u8]) -> Result<MoreFile<'_>> {
    let invalid = || Error::Parse("MORE: invalid 3D information report".into());
    if !data.is_ascii()
        || data
            .iter()
            .any(|&b| b < 0x20 && !matches!(b, b'\t' | b'\r' | b'\n'))
    {
        return Err(invalid());
    }
    let mut lines = std::str::from_utf8(data).map_err(|_| invalid())?.lines();
    let tool_version = lines
        .next()
        .and_then(|s| s.strip_prefix("Hugh's Marvelous 3D Information Program, Version "))
        .filter(|s| valid_version(s))
        .ok_or_else(invalid)?;
    let copyright = lines
        .next()
        .filter(|s| s.starts_with("(C)"))
        .ok_or_else(invalid)?;
    if lines.next() != Some("") {
        return Err(invalid());
    }
    let mut entries = Vec::new();
    let mut declared_count = None;
    for line in lines.filter(|s| !s.is_empty()) {
        if declared_count.is_some() {
            return Err(Error::Parse("MORE: data after file count".into()));
        }
        if let Some(count) = line.strip_suffix(" files found.") {
            if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            declared_count = Some(count.parse::<usize>().map_err(|_| invalid())?);
            continue;
        }
        let (filename, version) = line.split_once('\t').ok_or_else(invalid)?;
        if !filename.to_ascii_lowercase().ends_with(".3d")
            || filename.len() <= 3
            || filename
                .bytes()
                .any(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'\\'))
            || !version.strip_prefix('v').is_some_and(valid_version)
        {
            return Err(invalid());
        }
        entries.push(MoreEntry {
            filename: filename.to_owned(),
            version: version.to_owned(),
        });
    }
    let declared_count = declared_count.ok_or_else(invalid)?;
    if declared_count != entries.len() {
        return Err(Error::Parse("MORE: file count mismatch".into()));
    }
    Ok(MoreFile {
        tool_version,
        copyright,
        entries,
        declared_count,
        image: data,
    })
}

fn valid_version(s: &str) -> bool {
    s.split_once('.').is_some_and(|(major, minor)| {
        !major.is_empty()
            && !minor.is_empty()
            && major
                .bytes()
                .chain(minor.bytes())
                .all(|b| b.is_ascii_digit())
    })
}
