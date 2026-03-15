use crate::cli::convert::ensure_parent_dir;
use crate::opts::ConvertArgs;
use color_eyre::Result;
use log::info;
use redguard_preservation::import::pvo;
use serde_json::json;
use std::path::Path;

pub(super) fn handle_pvo_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let parsed = pvo::parse_pvo_file(&file_content)?;

    let nodes = parsed
        .octr_nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            json!({
                "index": i,
                "byte_offset": node.byte_offset,
                "child_mask": format!("0x{:02X}", node.child_mask),
                "child_count": node.child_count(),
                "leaf_ref": if node.is_interior() {
                    "none".to_string()
                } else {
                    format!("0x{:08X}", node.leaf_ref)
                },
                "child_refs": node.child_refs.iter()
                    .map(|(octant, offset)| json!({
                        "octant": octant,
                        "offset": format!("0x{offset:08X}"),
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    let leaves = parsed
        .plst_leaves
        .iter()
        .map(|leaf| {
            json!({
                "byte_offset": leaf.byte_offset,
                "entries": leaf.entries.iter().map(|e| json!({
                    "count": e.count,
                    "mlst_start": e.mlst_start,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "header": {
            "depth": parsed.header.depth,
            "total_nodes": parsed.header.total_nodes,
            "leaf_nodes": parsed.header.leaf_nodes,
            "interior_nodes": parsed.header.interior_nodes(),
            "mlst_polygon_count": parsed.header.mlst_polygon_count,
            "cell_size": parsed.header.cell_size,
            "center_x": parsed.header.center_x,
            "center_y": parsed.header.center_y,
            "center_z": parsed.header.center_z,
        },
        "octr_nodes": nodes,
        "plst_leaves": leaves,
        "mlst_indices": parsed.mlst_indices,
    });

    let json_text = serde_json::to_string_pretty(&output)
        .map_err(|e| color_eyre::eyre::eyre!("failed to serialize PVO JSON: {e}"))?;
    ensure_parent_dir(output_path)?;
    std::fs::write(output_path, json_text)?;
    info!(
        "Successfully converted PVO to JSON: {}",
        output_path.display()
    );
    Ok(())
}
