use crate::opts::{ConvertArgs, OutputFormat};
use color_eyre::Result;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, Rgba, RgbaImage};
use log::info;
use rgpre::import::{gxa, png::save_png};
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

pub(crate) fn handle_gxa_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let gxa_file =
        gxa::parse_gxa_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    let use_gif = !matches!(args.format, Some(OutputFormat::Png));

    if use_gif && gxa_file.frames.len() > 1 {
        std::fs::create_dir_all(output_path)?;

        let gif_name = "animation.gif";
        let gif_path = output_path.join(gif_name);
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(100));

        let frames: Vec<Frame> = gxa_file
            .frames
            .iter()
            .map(|f| {
                let img = rgba_to_image(f.width, f.height, &f.rgba);
                Frame::from_parts(img, 0, 0, delay)
            })
            .collect();

        let file = std::fs::File::create(&gif_path)?;
        let mut encoder = GifEncoder::new_with_speed(file, 10);
        encoder.set_repeat(Repeat::Infinite)?;
        encoder.encode_frames(frames)?;

        info!(
            "  {} frames -> {} (animated gif)",
            gxa_file.frames.len(),
            gif_path.display()
        );

        let source_name = args
            .file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let metadata = json!({
            "source": source_name,
            "title": gxa_file.title,
            "frame_count": gxa_file.frame_count,
            "file": gif_name,
        });
        let json_path = output_path.join("metadata.json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&metadata)?)?;
        info!("Metadata written to {}", json_path.display());

        return Ok(());
    }

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
            args.compress_textures,
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
