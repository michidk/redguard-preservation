//! Validate all art assets in a player's install without copying them into the repository.
use rgpre::import::{
    bsi, model3d,
    palette::Palette,
    rob,
    software_texture::{self, PaletteLookupTable, SoftwareTextureEntry},
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: check_edition_assets ART_DIRECTORY")?,
    );
    let software = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("TEXTURE.")
        });
    let mut cache = rgpre::gltf::TextureCache::new_for_edition(root.clone(), None, software);
    let strict_references = std::env::args().any(|arg| arg == "--references");
    let mut missing_references = std::collections::BTreeSet::new();
    let mut files = 0;
    let mut frames = 0;
    let mut failures = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        let bytes = std::fs::read(path)?;
        let result: Result<(), Box<dyn std::error::Error>> = (|| {
            if name.starts_with("TEXTURE.") {
                let bank_id = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .filter(|extension| extension.len() == 3)
                    .and_then(|extension| extension.parse::<u16>().ok());
                for (image_id, entry) in software_texture::parse_texture_bank(&bytes)?
                    .entries
                    .into_iter()
                    .enumerate()
                {
                    if let SoftwareTextureEntry::Image(image) = entry {
                        let cached = if image.frames.is_empty() {
                            None
                        } else {
                            bank_id.and_then(|bank_id| {
                                cache.get_all_frames_by_image_id(
                                    bank_id,
                                    u8::try_from(image_id).ok()?,
                                )
                            })
                        };
                        for (frame, pixels) in image.frames.iter().enumerate() {
                            assert_eq!(
                                pixels.len(),
                                usize::from(image.width) * usize::from(image.height)
                            );
                            let mut expected = image.rgba(frame, None)?;
                            let stride = usize::from(image.width) * 4;
                            expected = expected
                                .chunks_exact(stride)
                                .rev()
                                .flatten()
                                .copied()
                                .collect();
                            if bank_id.is_some()
                                && cached
                                    .as_ref()
                                    .and_then(|info| info.frames.get(frame))
                                    .and_then(|frame| frame.as_ref())
                                    != Some(&expected)
                            {
                                return Err(format!(
                                    "cache pixel mismatch: image {image_id}, frame {frame}"
                                )
                                .into());
                            }
                            frames += 1;
                        }
                    }
                }
            } else if name.starts_with("TEXBSI.") || name.ends_with(".BSI") {
                for image in bsi::parse_bsi_file(&bytes)?.images {
                    for frame in 0..usize::from(image.frame_count) {
                        let rgba = image
                            .decode_frame_rgba(frame, None)
                            .ok_or("missing image frame")?;
                        assert_eq!(
                            rgba.len(),
                            usize::from(image.width) * usize::from(image.height) * 4
                        );
                        frames += 1;
                    }
                }
            } else if name.ends_with(".COL") {
                if bytes.is_empty() {
                    return Ok(());
                }
                Palette::parse(&bytes)?;
            } else if name.starts_with("SHADE.") || name.starts_with("HAZE.") {
                PaletteLookupTable::parse(&bytes)?;
            } else if name.ends_with(".3D") || name.ends_with(".3DC") || name.ends_with(".ROB") {
                let models = if name.ends_with(".ROB") {
                    rob::parse_rob_with_models(&bytes)?.1
                } else {
                    vec![model3d::parse_3d_file(&bytes)?]
                };
                for model in models {
                    if model.vertex_coords.len() != model.header.num_vertices as usize
                        || model.face_data.len() != model.header.num_faces as usize
                    {
                        return Err("incomplete base geometry".into());
                    }
                    for face in &model.face_data {
                        for vertex in &face.face_vertices {
                            if vertex.vertex_index >= model.header.num_vertices {
                                return Err("face vertex outside model".into());
                            }
                        }
                        if let model3d::TextureData::Texture {
                            texture_id,
                            image_id,
                        } = face.texture_data
                            && cache.get_image_dimensions(texture_id, image_id).is_none()
                        {
                            missing_references.insert((path.to_path_buf(), texture_id, image_id));
                        }
                    }
                    for frame in 0..(model.header.num_frames as usize).max(1) {
                        model.at_frame(frame)?;
                        frames += 1;
                    }
                }
            } else {
                return Ok(());
            }
            files += 1;
            Ok(())
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }
    println!(
        "{files} assets, {frames} decoded frames, {} failures",
        failures.len()
    );
    for failure in &failures {
        eprintln!("{failure}");
    }
    for (path, bank, image) in &missing_references {
        eprintln!(
            "Unresolved authored texture: {} bank {bank}, image {image}",
            path.display()
        );
    }
    println!(
        "{} unresolved authored texture references",
        missing_references.len()
    );
    if strict_references && !missing_references.is_empty() {
        return Err("texture reference coverage failed".into());
    }
    if !failures.is_empty() {
        return Err("asset coverage failed".into());
    }
    Ok(())
}
