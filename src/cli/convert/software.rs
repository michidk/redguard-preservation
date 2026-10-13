use color_eyre::Result;
use rgpre::import::software_texture::{self, SoftwareTextureEntry};
use std::path::Path;

pub fn convert_lookup(file: &Path, output: &Path) -> Result<()> {
    let data = std::fs::read(file)?;
    std::fs::create_dir_all(output)?;
    let table = software_texture::PaletteLookupTable::parse(&data)?;
    let pixels: Vec<_> = table.rows.iter().flatten().copied().collect();
    image::save_buffer(
        output.join("indices.png"),
        &pixels,
        256,
        64,
        image::ColorType::L8,
    )?;
    Ok(())
}

pub fn convert_texture(args: &crate::opts::TexbsiArgs, output: &Path) -> Result<()> {
    use crate::opts::TexbsiFormat;
    use image::{
        Delay, Frame, RgbaImage,
        codecs::gif::{GifEncoder, Repeat},
    };
    let data = std::fs::read(&args.io.file)?;
    let bank = software_texture::parse_texture_bank(&data)?;
    let palette = args
        .palette
        .as_ref()
        .map(|path| {
            let bytes = std::fs::read(path)?;
            Ok::<_, color_eyre::eyre::Report>(rgpre::import::palette::Palette::parse(&bytes)?)
        })
        .transpose()?;
    std::fs::create_dir_all(output)?;
    let metadata: Vec<_> = bank.entries.iter().enumerate().map(|(index,entry)| match entry {
        SoftwareTextureEntry::SolidColor(color) => serde_json::json!({"image_index":index,"solid_color":color}),
        SoftwareTextureEntry::Image(image) => serde_json::json!({"image_index":index,"width":image.width,"height":image.height,"flags":image.flags,"x_offset":image.x_offset,"y_offset":image.y_offset,"frame_count":image.frames.len(),"frame_delay":image.frame_delay,"scale_x":image.scale_x,"scale_y":image.scale_y}),
    }).collect();
    std::fs::write(
        output.join("index.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"name":bank.name,"images":metadata}))?,
    )?;
    for (index, entry) in bank.entries.iter().enumerate() {
        match entry {
            SoftwareTextureEntry::SolidColor(color) => {
                let rgb = palette
                    .as_ref()
                    .map_or([*color; 3], |p| p.colors[usize::from(*color)]);
                super::save_rgba_png(
                    &output.join(format!("{index:03}.png")),
                    1,
                    1,
                    &[rgb[0], rgb[1], rgb[2], 255],
                    args.compress_textures,
                )?;
            }
            SoftwareTextureEntry::Image(texture) => {
                if matches!(args.format, TexbsiFormat::Gif) && texture.frames.len() > 1 {
                    let file = std::fs::File::create(output.join(format!("{index:03}.gif")))?;
                    let mut encoder = GifEncoder::new(file);
                    encoder.set_repeat(Repeat::Infinite)?;
                    for frame in 0..texture.frames.len() {
                        let rgba = texture.rgba(frame, palette.as_ref())?;
                        let image = RgbaImage::from_raw(
                            u32::from(texture.width),
                            u32::from(texture.height),
                            rgba,
                        )
                        .ok_or_else(|| {
                            color_eyre::eyre::eyre!("TEXTURE record {index}: invalid pixels")
                        })?;
                        encoder.encode_frame(Frame::from_parts(
                            image,
                            0,
                            0,
                            Delay::from_numer_denom_ms(u32::from(texture.frame_delay).max(1), 1),
                        ))?;
                    }
                } else {
                    for frame in 0..if matches!(args.format, TexbsiFormat::Png) {
                        texture.frames.len().min(1)
                    } else {
                        texture.frames.len()
                    } {
                        super::save_rgba_png(
                            &output.join(format!("{index:03}_{frame:03}.png")),
                            u32::from(texture.width),
                            u32::from(texture.height),
                            &texture.rgba(frame, palette.as_ref())?,
                            args.compress_textures,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
