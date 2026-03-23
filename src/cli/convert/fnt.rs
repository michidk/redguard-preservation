use crate::opts::{ConvertArgs, FontOutputMode};
use color_eyre::Result;
use log::{info, warn};
use rgpre::import::fnt_export;
use std::path::Path;

pub(super) fn handle_fnt_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let out_ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);

    let mode = args.font_output.unwrap_or({
        if matches!(out_ext.as_deref(), Some("ttf")) {
            FontOutputMode::Ttf
        } else {
            FontOutputMode::Bitmap
        }
    });

    if mode == FontOutputMode::Ttf {
        if !matches!(out_ext.as_deref(), Some("ttf")) {
            warn!("--font-output ttf selected; overriding output extension to .ttf");
        }

        let ttf_output = if matches!(out_ext.as_deref(), Some("ttf")) {
            output_path.to_path_buf()
        } else {
            output_path.with_extension("ttf")
        };

        fnt_export::export_fnt_ttf(&args.file, &ttf_output)
            .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
        info!(
            "Successfully converted FNT to TTF: {}",
            ttf_output.display()
        );
        return Ok(());
    }

    let paths = fnt_export::export_fnt_bitmap(&args.file, output_path)
        .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
    info!("Successfully converted FNT to {}", paths.png_path.display());
    info!("Wrote BMFont text to {}", paths.bmfont_path.display());
    info!("Wrote glyph metadata to {}", paths.json_path.display());
    Ok(())
}
