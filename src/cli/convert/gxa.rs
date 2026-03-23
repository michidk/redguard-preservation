use crate::opts::ConvertArgs;
use color_eyre::Result;
use image::{Rgba, RgbaImage};
use log::info;
use rgpre::import::gxa;
use serde_json::json;
use std::path::Path;

fn save_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        let i = (y * width + x) as usize * 4;
        Rgba([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
    });
    img.save(path)?;
    Ok(())
}

pub(super) fn handle_gxa_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let gxa_file =
        gxa::parse_gxa_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    std::fs::create_dir_all(output_path)?;

    let source_name = args
        .file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut frames_meta = Vec::with_capacity(gxa_file.frames.len());
    for (i, frame) in gxa_file.frames.iter().enumerate() {
        let png_name = format!("frame_{i:03}.png");
        let png_path = output_path.join(&png_name);
        save_rgba_png(
            &png_path,
            u32::from(frame.width),
            u32::from(frame.height),
            &frame.rgba,
        )?;
        frames_meta.push(json!({
            "index": i,
            "width": frame.width,
            "height": frame.height,
            "file": png_name,
        }));
        info!(
            "  [frame {i}] {}x{} -> {}",
            frame.width,
            frame.height,
            png_path.display()
        );
    }

    let metadata = json!({
        "source": source_name,
        "title": gxa_file.title,
        "frame_count": gxa_file.frame_count,
        "frames": frames_meta,
    });
    let json_path = output_path.join("metadata.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(&metadata)?)?;

    info!(
        "Extracted {} GXA frames to {}",
        gxa_file.frames.len(),
        output_path.display()
    );
    info!("Metadata written to {}", json_path.display());

    Ok(())
}
