use std::path::Path;

#[derive(Debug, Clone)]
pub struct SoupDef {
    pub functions: Vec<SoupFunction>,
    pub flags: Vec<SoupFlag>,
    pub references: Vec<String>,
    pub attributes: Vec<String>,
    pub anim_groups: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SoupFunction {
    pub name: String,
    pub param_count: u8,
    /// Installed declaration, independent of the call site's dispatch opcode.
    pub kind: SoupFunctionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoupFunctionKind {
    Task,
    Multitask,
    Function,
}

#[derive(Debug, Clone)]
pub struct SoupFlag {
    pub name: String,
    pub flag_type: String,
    /// Authored value used when initializing a fresh game's global state.
    pub initial_value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Functions,
    Refs,
    Equates,
    Flags,
}

fn clean_line(line: &str) -> &str {
    line.split(';').next().unwrap_or("").trim()
}

/// Parses installed definition metadata without substituting malformed values.
/// Errors identify the section and one-based source line.
pub fn parse_soup_def(content: &str) -> Result<SoupDef, String> {
    let mut soup = SoupDef {
        functions: vec![SoupFunction {
            name: "NullFunction".to_string(),
            param_count: 0,
            kind: SoupFunctionKind::Function,
        }],
        flags: Vec::new(),
        references: Vec::new(),
        attributes: Vec::new(),
        anim_groups: Vec::new(),
    };

    let mut section = Section::None;
    let mut in_auto = false;
    let mut auto_index: usize = 0;
    let mut attributes_done = false;
    let mut anim_groups_done = false;
    for (line_index, raw_line) in content.lines().enumerate() {
        let line = clean_line(raw_line);
        if line.is_empty() {
            continue;
        }

        let lower = line.to_ascii_lowercase();
        match lower.as_str() {
            "[functions]" => {
                section = Section::Functions;
                continue;
            }
            "[refs]" => {
                section = Section::Refs;
                continue;
            }
            "[equates]" => {
                section = Section::Equates;
                continue;
            }
            "[flags]" => {
                section = Section::Flags;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Functions => {
                let parts = line.split_whitespace().collect::<Vec<_>>();
                if parts.len() != 4 || !parts[2].eq_ignore_ascii_case("parms") {
                    return Err(format!(
                        "functions line {}: expected kind name parms count",
                        line_index + 1
                    ));
                }
                let kind = match parts[0].to_ascii_lowercase().as_str() {
                    "task" => SoupFunctionKind::Task,
                    "multitask" => SoupFunctionKind::Multitask,
                    "function" => SoupFunctionKind::Function,
                    _ => {
                        return Err(format!(
                            "functions line {}: unknown declaration {}",
                            line_index + 1,
                            parts[0]
                        ));
                    }
                };
                let param_count = parts[3].parse::<u8>().map_err(|_| {
                    format!(
                        "functions line {}: invalid parameter count {}",
                        line_index + 1,
                        parts[3]
                    )
                })?;
                soup.functions.push(SoupFunction {
                    name: parts[1].to_string(),
                    param_count,
                    kind,
                });
            }
            Section::Refs => {
                soup.references.push(line.to_string());
            }
            Section::Equates => {
                if lower == "auto" {
                    in_auto = true;
                    auto_index = 0;
                    continue;
                }
                if lower == "endauto" {
                    if in_auto && !soup.attributes.is_empty() {
                        if attributes_done {
                            anim_groups_done = true;
                        }
                        attributes_done = true;
                    }
                    in_auto = false;
                    continue;
                }
                if in_auto && !attributes_done {
                    let parts = line.split('=').collect::<Vec<_>>();
                    let name = parts[0].trim();
                    if parts.len() < 2 {
                        while soup.attributes.len() < auto_index {
                            soup.attributes
                                .push(format!("attr_{}", soup.attributes.len()));
                        }
                        soup.attributes.push(name.to_string());
                        auto_index += 1;
                    }
                }
                if in_auto && attributes_done && !anim_groups_done {
                    let parts = line.split('=').collect::<Vec<_>>();
                    let name = parts[0].trim();
                    if parts.len() < 2 {
                        while soup.anim_groups.len() < auto_index {
                            soup.anim_groups
                                .push(format!("anim_{}", soup.anim_groups.len()));
                        }
                        soup.anim_groups.push(name.to_string());
                        auto_index += 1;
                    }
                }
            }
            Section::Flags => {
                let parts = line.split_whitespace().collect::<Vec<_>>();
                if parts.len() != 3 {
                    return Err(format!(
                        "flags line {}: expected type name value",
                        line_index + 1
                    ));
                }
                if !matches!(
                    parts[0].to_ascii_lowercase().as_str(),
                    "bool" | "number" | "flipflop"
                ) {
                    return Err(format!(
                        "flags line {}: unknown type {}",
                        line_index + 1,
                        parts[0]
                    ));
                }
                let authored_value = match parts[2].to_ascii_lowercase().as_str() {
                    "false" => "0",
                    "true" => "1",
                    _ => parts[2],
                };
                let initial_value = authored_value.parse::<i32>().map_err(|_| {
                    format!(
                        "flags line {}: invalid initial value {}",
                        line_index + 1,
                        parts[2]
                    )
                })?;
                soup.flags.push(SoupFlag {
                    flag_type: parts[0].to_string(),
                    name: parts[1].to_string(),
                    initial_value,
                });
            }
            Section::None => {}
        }
    }

    Ok(soup)
}

fn find_case_insensitive_dir(base: &Path, target_name: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name()?.to_str()?;
        if name.eq_ignore_ascii_case(target_name) {
            return Some(path);
        }
    }
    None
}

fn find_case_insensitive_file(base: &Path, target_name: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name()?.to_str()?;
        if name.eq_ignore_ascii_case(target_name) {
            return Some(path);
        }
    }
    None
}

#[must_use]
pub fn try_load_soup_def(asset_root: &Path) -> Option<SoupDef> {
    let soup_dir = find_case_insensitive_dir(asset_root, "soup386")?;
    let def_path = find_case_insensitive_file(&soup_dir, "SOUP386.DEF")?;
    let content = std::fs::read_to_string(def_path).ok()?;
    parse_soup_def(&content).ok()
}

/// Loads the installed definition, retaining path and parse errors for runtime callers.
pub fn load_soup_def(asset_root: &Path) -> Result<SoupDef, String> {
    let soup_dir = find_case_insensitive_dir(asset_root, "soup386")
        .ok_or_else(|| format!("{}: soup386 directory missing", asset_root.display()))?;
    let path = find_case_insensitive_file(&soup_dir, "SOUP386.DEF")
        .ok_or_else(|| format!("{}: SOUP386.DEF missing", soup_dir.display()))?;
    let content =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse_soup_def(&content).map_err(|error| format!("{}: {error}", path.display()))
}
