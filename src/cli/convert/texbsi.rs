use crate::opts::ConvertArgs;
use color_eyre::Result;
use image::{DynamicImage, Rgba, RgbaImage};
use log::info;
use rayon::prelude::*;
use rgpre::import::{bsi, palette::Palette, png::save_png};
use serde_json::json;
use std::path::Path;

fn save_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8], compress: bool) -> Result<()> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        let i = (y * width + x) as usize * 4;
        Rgba([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
    });
    save_png(&DynamicImage::ImageRgba8(img), path, compress)?;
    Ok(())
}

pub(crate) fn handle_texbsi_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let bsi_file =
        bsi::parse_bsi_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    let palette = match args.palette.as_ref() {
        Some(p) => {
            let data = std::fs::read(p)?;
            Some(Palette::parse(&data).map_err(|e| color_eyre::eyre::eyre!("{e}"))?)
        }
        None => None,
    };

    std::fs::create_dir_all(output_path)?;

    let palette_name = args
        .palette
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());

    let source_name = args
        .file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let all_frames = args.all_frames;
    let compress = args.compress_textures;
    let image_metadata: Vec<serde_json::Value> = bsi_file
        .images
        .par_iter()
        .filter(|image| image.width > 0 && image.height > 0)
        .map(|image| {
            let png_name = format!("{}.png", image.name);
            let png_path = output_path.join(&png_name);
            let rgba = image.decode_rgba(palette.as_ref());
            save_rgba_png(
                &png_path,
                u32::from(image.width),
                u32::from(image.height),
                &rgba,
                compress,
            )?;

            let mut frame_files: Vec<String> = vec![png_name.clone()];

            if all_frames && image.frame_count > 1 {
                for f in 1..image.frame_count as usize {
                    if let Some(frame_rgba) = image.decode_frame_rgba(f, palette.as_ref()) {
                        let frame_name = format!("{}_frame{f:02}.png", image.name);
                        let frame_path = output_path.join(&frame_name);
                        save_rgba_png(
                            &frame_path,
                            u32::from(image.width),
                            u32::from(image.height),
                            &frame_rgba,
                            compress,
                        )?;
                        frame_files.push(frame_name);
                    }
                }
            }

            info!(
                "  [{}] {}x{} frames={} -> {}",
                image.name,
                image.width,
                image.height,
                image.frame_count,
                png_path.display(),
            );

            let mut entry = serde_json::Map::new();
            entry.insert("name".into(), json!(image.name));
            entry.insert("image_index".into(), json!(image.image_index));
            entry.insert("width".into(), json!(image.width));
            entry.insert("height".into(), json!(image.height));
            entry.insert("x_offset".into(), json!(image.x_offset));
            entry.insert("y_offset".into(), json!(image.y_offset));
            entry.insert("frame_count".into(), json!(image.frame_count));
            entry.insert("anim_delay_ms".into(), json!(image.anim_delay));
            entry.insert("tex_scale".into(), json!(image.tex_scale));
            entry.insert("is_animated".into(), json!(image.is_animated));
            entry.insert(
                "has_embedded_palette".into(),
                json!(image.palette.is_some()),
            );
            entry.insert("data_encoding".into(), json!(image.data_encoding));
            entry.insert("file".into(), json!(png_name));
            if image.frame_count > 1 {
                entry.insert("frames".into(), json!(frame_files));
            }

            Ok(json!(entry))
        })
        .collect::<Result<Vec<_>>>()?;

    let metadata = json!({
        "source": source_name,
        "palette": palette_name,
        "image_count": image_metadata.len(),
        "images": image_metadata,
    });

    let json_path = output_path.join("metadata.json");
    let json_text = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(&json_path, json_text)?;

    info!(
        "Extracted {} images to {}",
        image_metadata.len(),
        output_path.display()
    );
    info!("Metadata written to {}", json_path.display());

    Ok(())
}
