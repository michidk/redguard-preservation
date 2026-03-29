use crate::cli::convert::ensure_parent_dir;
use color_eyre::Result;
use log::info;
use rgpre::import::cht;
use serde_json::json;
use std::path::Path;

pub(crate) fn handle_cht_convert(file: &Path, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(file)?;
    let parsed = cht::parse_cht_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    let cheats: serde_json::Map<String, serde_json::Value> = parsed
        .named_cheats()
        .iter()
        .map(|e| {
            let name = e.name.unwrap_or("unknown").to_string();
            let value = if e.value > 1 {
                json!(e.value)
            } else {
                json!(e.is_on())
            };
            (name, value)
        })
        .collect();

    let output = json!(cheats);

    let json_text = serde_json::to_string_pretty(&output)
        .map_err(|e| color_eyre::eyre::eyre!("failed to serialize CHT JSON: {e}"))?;
    ensure_parent_dir(output_path)?;
    std::fs::write(output_path, json_text)?;
    info!(
        "Successfully converted CHT to JSON: {}",
        output_path.display()
    );
    Ok(())
}
