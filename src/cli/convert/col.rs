use crate::opts::{ConvertArgs, OutputFormat};
use color_eyre::Result;
use log::info;
use rgpre::import::{palette::Palette, palette_export};
use std::path::Path;

pub(crate) fn handle_col_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let palette = Palette::parse(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    let paths = palette_export::export_col_palette(&palette, output_path, args.compress_textures)
        .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    match args.format {
        Some(OutputFormat::Json) => {
            std::fs::remove_file(&paths.png_path).ok();
            info!(
                "Palette metadata exported to: {}",
                paths.json_path.display()
            );
        }
        Some(OutputFormat::Png) => {
            std::fs::remove_file(&paths.json_path).ok();
            info!("Palette swatch exported to: {}", paths.png_path.display());
        }
        _ => {
            info!("Palette swatch exported to: {}", paths.png_path.display());
            info!(
                "Palette metadata exported to: {}",
                paths.json_path.display()
            );
        }
    }

    Ok(())
}
