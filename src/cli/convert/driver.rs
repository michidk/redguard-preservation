use super::ensure_parent_dir;
use color_eyre::Result;
use log::info;
use rgpre::import::miles_driver::parse_miles_driver;
use std::path::Path;

pub(crate) fn handle_driver_convert(file: &Path, output: &Path) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let driver = parse_miles_driver(&bytes)?;
    let json = serde_json::to_string_pretty(&driver)?;
    ensure_parent_dir(output)?;
    std::fs::write(output, json)?;
    info!("Exported Miles driver metadata: {}", output.display());
    Ok(())
}
