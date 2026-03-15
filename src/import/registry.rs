use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::import::FileType;

/// Represents a file entry in the registry with its metadata
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
}

impl FileEntry {
    /// Creates a registry entry for a discovered file.
    pub fn new(name: String, path: PathBuf, file_type: FileType) -> Self {
        Self {
            name,
            path,
            file_type,
        }
    }
}

/// Registry for managing files by their model names
///
/// The registry uses the model name (file stem without extension) as the key,
/// allowing easy lookup of files regardless of their file extension or path.
pub struct Registry {
    pub root_path: PathBuf,
    pub files: HashMap<String, FileEntry>,
}

impl Registry {
    /// Creates an empty registry rooted at `root_path`.
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            files: HashMap::new(),
        }
    }

    /// Returns a sort key used to prioritize duplicate file sources.
    pub fn source_rank_key(&self, path: &Path) -> (usize, usize, String) {
        let rel = path.strip_prefix(&self.root_path).unwrap_or(path);
        let depth = rel.components().count();
        let normalized = rel.to_string_lossy().to_ascii_lowercase();
        let source_rank = if normalized.contains("fxart/") || normalized.contains("fxart\\") {
            0usize
        } else if normalized.contains("maps/") || normalized.contains("maps\\") {
            1usize
        } else if normalized.contains("3dart/") || normalized.contains("3dart\\") {
            2usize
        } else {
            3usize
        };
        (source_rank, depth, normalized)
    }

    /// Add a file to the registry using its path and file type
    ///
    /// The file will be indexed by its name (file stem without extension).
    /// If a file with the same name already exists, it will be overwritten.
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, file_type: FileType) {
        let path_ref = path.as_ref();
        let name = path_ref
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let file_entry = FileEntry::new(name.clone(), path_ref.to_path_buf(), file_type);
        if let Some(existing) = self.files.get(&name) {
            // When types differ (e.g. ROB vs COL) or same-type duplicates from
            // different paths, keep both using synthetic keys so all ROBs remain
            // searchable for embedded segment lookups.
            if existing.path != file_entry.path
                && (existing.file_type != file_entry.file_type
                    || existing.file_type == FileType::Rob)
            {
                let mut suffix = 1usize;
                loop {
                    let candidate = format!("{name}#{suffix}");
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        self.files.entry(candidate)
                    {
                        entry.insert(file_entry);
                        return;
                    }
                    suffix += 1;
                }
            }

            let existing_rank = self.source_rank_key(&existing.path);
            let new_rank = self.source_rank_key(path_ref);
            if new_rank < existing_rank {
                self.files.insert(name, file_entry);
            }
        } else {
            self.files.insert(name, file_entry);
        }
    }

    /// Get a file entry by its model name
    pub fn get_file_by_name(&self, name: &str) -> Option<&FileEntry> {
        self.files.get(name)
    }

    /// Get a file entry by its path (extracts the name from the path)
    pub fn get_file_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&FileEntry> {
        let path_ref = path.as_ref();
        let name = path_ref.file_stem().unwrap_or_default().to_string_lossy();
        self.files.get(name.as_ref())
    }

    /// Get all model names in the registry
    pub fn get_all_names(&self) -> Vec<&String> {
        self.files.keys().collect()
    }

    /// Check if a model name exists in the registry
    pub fn has_model(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }
}

/// Recursively scans a directory and indexes recognized Redguard files.
pub fn scan_dir<P: AsRef<Path>>(path: P) -> Result<Registry, std::io::Error> {
    let path = path.as_ref();
    let mut registry = Registry::new(path.to_path_buf());

    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && let Some(file_type) = FileType::from_path(entry.path())
        {
            registry.add_file(entry.path(), file_type);
        }
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_rank_prefers_fxart_over_3dart() {
        let reg = Registry::new(PathBuf::from("/root"));
        let fxart = reg.source_rank_key(Path::new("/root/fxart/FOO.ROB"));
        let maps = reg.source_rank_key(Path::new("/root/maps/FOO.ROB"));
        let art3d = reg.source_rank_key(Path::new("/root/3dart/FOO.ROB"));
        assert!(fxart < maps, "fxart should rank before maps");
        assert!(maps < art3d, "maps should rank before 3dart");
    }

    #[test]
    fn duplicate_rob_from_different_dirs_both_kept() {
        let mut reg = Registry::new(PathBuf::from("/root"));
        reg.add_file("/root/3dart/FOO.ROB", FileType::Rob);
        reg.add_file("/root/fxart/FOO.ROB", FileType::Rob);
        let has_primary = reg.files.contains_key("FOO");
        let has_dup = reg.files.contains_key("FOO#1");
        assert!(has_primary, "primary ROB key must exist");
        assert!(has_dup, "duplicate ROB must be kept under synthetic key");
    }

    #[test]
    fn different_type_same_stem_both_kept() {
        let mut reg = Registry::new(PathBuf::from("/root"));
        reg.add_file("/root/fxart/ISLAND.ROB", FileType::Rob);
        reg.add_file("/root/fxart/ISLAND.COL", FileType::Col);
        let has_primary = reg.files.contains_key("ISLAND");
        let has_dup = reg.files.contains_key("ISLAND#1");
        assert!(has_primary, "primary entry must exist");
        assert!(
            has_dup,
            "different-type same-stem must be kept under synthetic key"
        );
    }

    #[test]
    fn same_type_same_path_not_duplicated() {
        let mut reg = Registry::new(PathBuf::from("/root"));
        reg.add_file("/root/fxart/FOO.ROB", FileType::Rob);
        reg.add_file("/root/fxart/FOO.ROB", FileType::Rob);
        assert!(reg.files.contains_key("FOO"));
        assert!(
            !reg.files.contains_key("FOO#1"),
            "identical path should not create duplicate"
        );
    }
}
