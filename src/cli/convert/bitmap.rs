use super::{ensure_parent_dir, rgba_to_image, save_rgba_png};
use color_eyre::{Result, eyre::bail};
use image::{
    Delay, Frame,
    codecs::gif::{GifEncoder, Repeat},
};
use log::info;
use rgpre::import::{FileType, anm, lbm, pcx};
use serde_json::json;
use std::path::Path;

pub(crate) fn handle_bitmap_convert(
    file: &Path,
    output: &Path,
    filetype: FileType,
    compress: bool,
) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let image = match filetype {
        FileType::Pcx => pcx::parse_pcx_file(&bytes)?.image,
        FileType::Lbm => lbm::parse_lbm_file(&bytes)?.image,
        _ => bail!("Unsupported bitmap format: {filetype:?}"),
    };
    ensure_parent_dir(output)?;
    save_rgba_png(
        output,
        u32::from(image.width),
        u32::from(image.height),
        &image.rgba(),
        compress,
    )?;
    info!(
        "Exported {}x{} bitmap: {}",
        image.width,
        image.height,
        output.display()
    );
    Ok(())
}

pub(crate) fn handle_anm_convert(file: &Path, output: &Path) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let animation = anm::parse_anm_file(&bytes)?;
    std::fs::create_dir_all(output)?;
    let gif = std::fs::File::create(output.join("animation.gif"))?;
    let mut encoder = GifEncoder::new_with_speed(gif, 10);
    let looping = animation.loop_delta.is_some() && animation.last_delta_valid;
    if looping {
        encoder.set_repeat(Repeat::Infinite)?;
    }
    let delay = Delay::from_numer_denom_ms(1000, u32::from(animation.frames_per_second));
    for frame in &animation.frames {
        encoder.encode_frame(Frame::from_parts(
            rgba_to_image(frame.width, frame.height, &frame.rgba()),
            0,
            0,
            delay,
        ))?;
    }
    let first = &animation.frames[0];
    let metadata = json!({
        "source": file.file_name().unwrap_or_default().to_string_lossy(),
        "width": first.width, "height": first.height,
        "frame_count": animation.frames.len(),
        "declared_frame_count": animation.declared_frame_count,
        "record_count": animation.records.len(),
        "frames_per_second": animation.frames_per_second,
        "looping": looping,
        "pages": animation.pages,
        "file": "animation.gif",
    });
    std::fs::write(
        output.join("index.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    info!(
        "Exported {} animation frames: {}",
        animation.frames.len(),
        output.display()
    );
    Ok(())
}
