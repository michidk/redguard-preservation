use crate::opts::{ConvertArgs, OutputFormat};
use color_eyre::Result;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, Rgba, RgbaImage};
use log::info;
use rayon::prelude::*;
use rgpre::import::{bsi, bsi::BsiImage, palette::Palette, png::save_png};
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

fn rgba_to_image(width: u16, height: u16, rgba: &[u8]) -> RgbaImage {
    RgbaImage::from_fn(u32::from(width), u32::from(height), |x, y| {
        let i = (y * u32::from(width) + x) as usize * 4;
        Rgba([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
    })
}

fn save_animated_gif(image: &BsiImage, palette: Option<&Palette>, path: &Path) -> Result<()> {
    let delay_ms = image.anim_delay.unsigned_abs().max(1);
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(
        u64::from(delay_ms),
    ));

    let mut frames = Vec::with_capacity(image.frame_count as usize);
    for f in 0..image.frame_count as usize {
        let rgba = image
            .decode_frame_rgba(f, palette)
            .unwrap_or_else(|| image.decode_rgba(palette));
        let img = rgba_to_image(image.width, image.height, &rgba);
        frames.push(Frame::from_parts(img, 0, 0, delay));
    }

    let file = std::fs::File::create(path)?;
    let mut encoder = GifEncoder::new_with_speed(file, 10);
    encoder.set_repeat(Repeat::Infinite)?;
    encoder.encode_frames(frames)?;
    Ok(())
}

fn export_image_png(
    image: &BsiImage,
    palette: Option<&Palette>,
    output_path: &Path,
    compress: bool,
    export_all_frames: bool,
) -> Result<(String, Vec<String>)> {
    let png_name = format!("{}.png", image.name);
    let png_path = output_path.join(&png_name);
    let rgba = image.decode_rgba(palette);
    save_rgba_png(
        &png_path,
        u32::from(image.width),
        u32::from(image.height),
        &rgba,
        compress,
    )?;

    let mut frame_files: Vec<String> = vec![png_name.clone()];

    if export_all_frames && image.frame_count > 1 {
        for f in 1..image.frame_count as usize {
            if let Some(frame_rgba) = image.decode_frame_rgba(f, palette) {
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

    Ok((png_name, frame_files))
}

fn export_image_gif(
    image: &BsiImage,
    palette: Option<&Palette>,
    output_path: &Path,
    compress: bool,
) -> Result<(String, Vec<String>)> {
    if image.frame_count > 1 {
        let gif_name = format!("{}.gif", image.name);
        let gif_path = output_path.join(&gif_name);
        save_animated_gif(image, palette, &gif_path)?;
        Ok((gif_name.clone(), vec![gif_name]))
    } else {
        let png_name = format!("{}.png", image.name);
        let png_path = output_path.join(&png_name);
        let rgba = image.decode_rgba(palette);
        save_rgba_png(
            &png_path,
            u32::from(image.width),
            u32::from(image.height),
            &rgba,
            compress,
        )?;
        Ok((png_name.clone(), vec![png_name]))
    }
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

    let use_gif = !matches!(
        args.format,
        Some(OutputFormat::Png) | Some(OutputFormat::Frames)
    );
    let export_all_frames = args.format == Some(OutputFormat::Frames);
    let compress = args.compress_textures;

    let image_metadata: Vec<serde_json::Value> = bsi_file
        .images
        .par_iter()
        .filter(|image| image.width > 0 && image.height > 0)
        .map(|image| {
            let (primary_file, frame_files) = if use_gif {
                export_image_gif(image, palette.as_ref(), output_path, compress)?
            } else {
                export_image_png(
                    image,
                    palette.as_ref(),
                    output_path,
                    compress,
                    export_all_frames,
                )?
            };

            if frame_files.len() > 1 {
                info!(
                    "  [{}] {}x{} frames={} -> {} (+{} frames)",
                    image.name,
                    image.width,
                    image.height,
                    image.frame_count,
                    output_path.join(&primary_file).display(),
                    frame_files.len() - 1,
                );
            } else if use_gif && image.frame_count > 1 {
                info!(
                    "  [{}] {}x{} frames={} -> {} (animated gif)",
                    image.name,
                    image.width,
                    image.height,
                    image.frame_count,
                    output_path.join(&primary_file).display(),
                );
            } else {
                info!(
                    "  [{}] {}x{} -> {}",
                    image.name,
                    image.width,
                    image.height,
                    output_path.join(&primary_file).display(),
                );
            }

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
            entry.insert("file".into(), json!(primary_file));
            if frame_files.len() > 1 {
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

    let json_path = output_path.join("index.json");
    let json_text = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(&json_path, json_text)?;

    let total_files: usize = image_metadata
        .iter()
        .map(|e| {
            e.get("frames")
                .and_then(|f| f.as_array())
                .map_or(1, |a| a.len())
        })
        .sum();
    info!(
        "Extracted {} images ({} files) to {}",
        image_metadata.len(),
        total_files,
        output_path.display()
    );
    info!("Metadata written to {}", json_path.display());

    Ok(())
}
