use crate::{Result, error::Error};
use std::collections::BTreeMap;

pub(super) struct Section<'a> {
    pub(super) file: &'static str,
    pub(super) name: String,
    pub(super) values: BTreeMap<String, (&'a str, usize)>,
}

pub(super) fn sections<'a>(
    text: &'a str,
    file: &'static str,
) -> Result<BTreeMap<String, Section<'a>>> {
    let mut sections = BTreeMap::new();
    let mut active: Option<String> = None;
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line_number = index + 1;
        let line = line.split(';').next().unwrap_or_default().trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let invalid = || Error::Parse(format!("{file} line {line_number}: invalid INI record"));
        if line.starts_with('[') {
            let name = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(invalid)?
                .trim()
                .to_ascii_lowercase();
            sections.entry(name.clone()).or_insert_with(|| Section {
                file,
                name: name.clone(),
                values: BTreeMap::new(),
            });
            active = Some(name);
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(invalid)?;
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(invalid());
        }
        let section = active.as_ref().ok_or_else(invalid)?;
        sections
            .get_mut(section)
            .unwrap()
            .values
            .entry(key)
            .or_insert((value.trim(), line_number));
    }
    Ok(sections)
}

pub(super) fn required<'a, 'b>(
    sections: &'b BTreeMap<String, Section<'a>>,
    file: &str,
    name: &str,
) -> Result<&'b Section<'a>> {
    sections
        .get(name)
        .ok_or_else(|| Error::Parse(format!("{file}: missing [{name}]")))
}

impl Section<'_> {
    pub(super) fn get<T: IniValue>(&self, key: &str) -> Result<Option<T>> {
        let Some(&(value, line)) = self.values.get(key) else {
            return Ok(None);
        };
        if value.is_empty() {
            return Ok(None);
        }
        T::parse(value).map(Some).ok_or_else(|| {
            Error::Parse(format!(
                "{} [{}] {key}, line {line}: expected {}, got {value:?}",
                self.file,
                self.name,
                T::EXPECTED
            ))
        })
    }
}

pub(super) trait IniValue: Sized {
    const EXPECTED: &'static str;
    fn parse(value: &str) -> Option<Self>;
}

impl IniValue for i32 {
    const EXPECTED: &'static str = "a signed 32-bit integer";
    fn parse(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

impl IniValue for u32 {
    const EXPECTED: &'static str = "an unsigned 32-bit integer";
    fn parse(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

impl IniValue for f32 {
    const EXPECTED: &'static str = "a finite 32-bit float";
    fn parse(value: &str) -> Option<Self> {
        value.parse::<f32>().ok().filter(|value| value.is_finite())
    }
}

impl IniValue for bool {
    const EXPECTED: &'static str = "0/1, no/yes, off/on, or false/true";
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "1" | "yes" | "on" | "true" => Some(true),
            "0" | "no" | "off" | "false" => Some(false),
            _ => None,
        }
    }
}

impl IniValue for String {
    const EXPECTED: &'static str = "text";
    fn parse(value: &str) -> Option<Self> {
        Some(value.to_owned())
    }
}

impl IniValue for Vec<u32> {
    const EXPECTED: &'static str = "comma-separated unsigned 32-bit integers";
    fn parse(value: &str) -> Option<Self> {
        value
            .split(',')
            .map(|token| token.trim().parse().ok())
            .collect()
    }
}

impl IniValue for [i32; 2] {
    const EXPECTED: &'static str = "two comma-separated signed 32-bit integers";
    fn parse(value: &str) -> Option<Self> {
        let (first, second) = value.split_once(',')?;
        Some([first.trim().parse().ok()?, second.trim().parse().ok()?])
    }
}

macro_rules! ini_section {
    ($(#[$meta:meta])* indexed $name:ident { $($field:ident: $ty:ty => $key:literal),* $(,)? }) => {
        ini_section! { $(#[$meta])* $name { $($field: $ty => $key,)* } }
        impl $name {
            fn is_field(key: &str) -> bool {
                matches!(key, $($key)|*)
            }
        }
    };
    ($(#[$meta:meta])* $name:ident { $($field:ident: $ty:ty => $key:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
        pub struct $name {
            $(#[serde(rename = $key)] pub $field: Option<$ty>,)*
        }

        impl $name {
            pub(super) fn parse(section: Option<&super::ini::Section<'_>>) -> crate::Result<Self> {
                let Some(section) = section else { return Ok(Self::default()); };
                Ok(Self { $($field: section.get($key)?,)* })
            }
        }
    };
}

pub(super) use ini_section;

macro_rules! ini_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $code:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
        pub enum $name {
            $($variant,)*
            Unknown(i32),
        }

        impl super::ini::IniValue for $name {
            const EXPECTED: &'static str = "a signed 32-bit type code";
            fn parse(value: &str) -> Option<Self> {
                Some(match value.parse::<i32>().ok()? {
                    $($code => Self::$variant,)*
                    code => Self::Unknown(code),
                })
            }
        }
    };
}

pub(super) use ini_enum;
