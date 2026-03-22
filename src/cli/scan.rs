//! Scan command handler

use crate::opts::ScanArgs;
use color_eyre::Result;
use rgpre::import::{FileType, registry};
use std::collections::{BTreeMap, HashMap};

/// Count files by their type
fn count_files_by_type(registry: &registry::Registry) -> HashMap<FileType, usize> {
    let mut file_type_counts = HashMap::new();

    for entry in registry.files.values() {
        *file_type_counts.entry(entry.file_type).or_default() += 1;
    }

    file_type_counts
}

/// Group file types by their display name for reporting
fn group_file_types_by_display_name(
    file_type_counts: &HashMap<FileType, usize>,
) -> BTreeMap<&'static str, (Vec<&'static str>, usize)> {
    let mut type_groups: BTreeMap<&str, (Vec<&str>, usize)> = BTreeMap::new();

    for file_type in FileType::all() {
        if let Some(&count) = file_type_counts.get(file_type)
            && count > 0
        {
            let entry = type_groups
                .entry(file_type.display_name())
                .or_insert((vec![], 0));
            entry.0.extend_from_slice(file_type.extensions());
            entry.1 += count;
        }
    }

    type_groups
}

/// Print the scan results in a formatted way
fn print_scan_results(total_files: usize, type_groups: &BTreeMap<&str, (Vec<&str>, usize)>) {
    println!("Scan complete! Found {total_files} total files");

    if type_groups.is_empty() {
        println!("\nNo recognized files found in the specified directory.");
        return;
    }

    println!("\nRecognized file types:");
    for (display_name, (extensions, count)) in type_groups {
        let extension_str = extensions.join(", ");
        println!("  {display_name:<15} ({extension_str:<4}): {count} files");
    }
}

#[allow(clippy::needless_pass_by_value)]
// CLI handlers take owned args by clap design for consistent command dispatch.
pub fn handle_scan_command(args: ScanArgs) -> Result<()> {
    let registry = registry::scan_dir(&args.dir)?;
    let file_type_counts = count_files_by_type(&registry);
    let total_files = registry.files.len();

    let type_groups = group_file_types_by_display_name(&file_type_counts);
    print_scan_results(total_files, &type_groups);

    Ok(())
}
