pub(crate) mod cht;
pub(crate) mod col;
pub(crate) mod fnt;
pub(crate) mod gxa;
pub(crate) mod pvo;
pub(crate) mod rtx;
pub(crate) mod sfx;
pub(crate) mod texbsi;
pub(crate) mod wld;

use super::utils::{
    auto_resolve_palette, parse_file, resolve_asset_root_from_input, resolve_filetype,
};
use crate::cli::filetype::FileTypeCliExt;
use crate::opts::{ConvertArgs, OutputFormat};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use color_eyre::eyre::bail;
use log::{info, warn};
use rgpre::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf, to_glb,
};
use rgpre::import::FileType;
use rgpre::import::{palette::Palette, registry};
use std::path::{Path, PathBuf};

pub(super) fn resolve_asset_root(args: &ConvertArgs) -> PathBuf {
    args.assets
        .clone()
        .or_else(|| args.asset_path.clone())
        .or_else(|| args.asset_dir.clone())
        .unwrap_or_else(|| resolve_asset_root_from_input(&args.file))
}

fn default_fnt_output_for_format(file: &Path, format: OutputFormat) -> PathBuf {
    let mut path = file.to_path_buf();
    if format == OutputFormat::Ttf {
        path.set_extension("ttf");
    } else {
        path.set_extension("png");
    }
    path
}

pub(super) fn ensure_parent_dir(path: &Path) -> Result<(), color_eyre::eyre::Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).wrap_err_with(|| {
            format!(
                "failed to create output directory '{}'; choose a writable location",
                parent.display()
            )
        })?;
    }
    Ok(())
}

fn resolve_output_path(args: &ConvertArgs, filetype: FileType) -> PathBuf {
    if filetype != FileType::Fnt {
        let default = filetype.default_output_path(&args.file);
        return match args.output.clone() {
            Some(out) => resolve_dir_output(out, &default),
            None => default,
        };
    }

    let default = match args.format {
        Some(fmt @ (OutputFormat::Ttf | OutputFormat::Bitmap)) => {
            default_fnt_output_for_format(&args.file, fmt)
        }
        _ => filetype.default_output_path(&args.file),
    };
    match args.output.clone() {
        Some(out) => resolve_dir_output(out, &default),
        None => default,
    }
}

/// If `output` is an existing directory on disk, append the default filename;
/// otherwise return `output` unchanged.
fn resolve_dir_output(output: PathBuf, default_path: &Path) -> PathBuf {
    if output.is_dir() {
        output.join(default_path.file_name().unwrap_or(default_path.as_os_str()))
    } else {
        output
    }
}

pub(super) fn load_palette(
    args: &ConvertArgs,
    asset_root: &Path,
    filetype: FileType,
) -> Result<Option<Palette>> {
    match args.palette.as_ref() {
        Some(path) => {
            let data = std::fs::read(path)?;
            let palette = Palette::parse(&data).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            Ok(Some(palette))
        }
        None => auto_resolve_palette(asset_root, &args.file, filetype),
    }
}

fn build_texture_cache(palette: Option<&Palette>, asset_root: PathBuf) -> Option<TextureCache> {
    palette.map_or_else(
        || {
            warn!(
                "No palette provided (and none found in WORLD.INI); texture export is disabled and materials will be plain white"
            );
            None
        },
        |pal| {
            Some(TextureCache::new(
            asset_root,
            Some(Palette { colors: pal.colors }),
            ))
        },
    )
}

fn convert_rgm(
    args: &ConvertArgs,
    output_path: &Path,
    registry: &registry::Registry,
    palette: Option<&Palette>,
    texture_cache: &mut Option<TextureCache>,
) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let (rgm_file, positioned_models, lights) =
        rgpre::import::rgm::parse_rgm_with_models(&file_content, registry)?;

    if positioned_models.is_empty() && lights.is_empty() {
        bail!("No positioned models or lights found to convert.");
    }

    info!(
        "Converting {} positioned models and {} lights with transformations",
        positioned_models.len(),
        lights.len()
    );

    let (root, buffer) = convert_positioned_models_to_gltf(
        &positioned_models,
        &lights,
        palette,
        texture_cache.as_mut(),
        args.compress_textures,
    )?;
    let glb_data = to_glb(&root, &buffer)?;
    ensure_parent_dir(output_path)?;
    std::fs::write(output_path, &glb_data)?;
    info!("Successfully converted to: {}", output_path.display());

    let json_path = output_path.with_extension("json");
    let metadata = rgpre::import::rgm::export_rgm_metadata_json(&rgm_file);
    let json_bytes = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(&json_path, json_bytes)?;
    info!("Actor metadata exported to: {}", json_path.display());

    Ok(())
}

fn convert_models(
    args: &ConvertArgs,
    output_path: &Path,
    filetype: FileType,
    registry: Option<&registry::Registry>,
    palette: Option<&Palette>,
    texture_cache: &mut Option<TextureCache>,
) -> Result<()> {
    let models = parse_file(&args.file, Some(filetype), registry)?;

    if filetype == FileType::Rob {
        warn!(
            "ROB conversion exports segment geometry only (no scene instance placement). Full area/object placement requires RGM scene data."
        );
    }

    if models.is_empty() {
        bail!("No 3D models found to convert.");
    }

    let (root, buffer) = convert_models_to_gltf(
        &models,
        palette,
        texture_cache.as_mut(),
        args.compress_textures,
    )?;
    let glb_data = to_glb(&root, &buffer)?;
    ensure_parent_dir(output_path)?;
    std::fs::write(output_path, glb_data)?;
    info!("Successfully converted to: {}", output_path.display());

    Ok(())
}

fn warn_irrelevant_flags(args: &ConvertArgs, filetype: FileType) {
    if args.terrain_only && filetype != FileType::Wld {
        warn!(
            "--terrain-only has no effect for {} files (WLD only)",
            filetype.display_name()
        );
    }
    if !args.terrain_textures && filetype != FileType::Wld {
        warn!(
            "--terrain-textures has no effect for {} files (WLD only)",
            filetype.display_name()
        );
    }
    if let Some(fmt) = args.format {
        let valid = match fmt {
            OutputFormat::Bitmap | OutputFormat::Ttf => filetype == FileType::Fnt,
            OutputFormat::Frames | OutputFormat::Gif => {
                matches!(filetype, FileType::Bsi | FileType::Gxa)
            }
            OutputFormat::Png => {
                matches!(filetype, FileType::Bsi | FileType::Gxa | FileType::Col)
            }
            OutputFormat::Json => filetype == FileType::Col,
        };
        if !valid {
            warn!(
                "--format {fmt:?} has no effect for {} files",
                filetype.display_name()
            );
        }
    }
    let produces_images = !matches!(
        filetype,
        FileType::Cht | FileType::Pvo | FileType::Sfx | FileType::Rtx
    );
    if args.compress_textures && !produces_images {
        warn!(
            "--compress-textures has no effect for {} files (image/GLB output only)",
            filetype.display_name()
        );
    }
}

#[allow(clippy::needless_pass_by_value)]
// CLI handlers take owned args by clap design for consistent command dispatch.
pub fn handle_convert_command(args: ConvertArgs) -> Result<()> {
    let filetype = resolve_filetype(&args.file)?;
    warn_irrelevant_flags(&args, filetype);
    let asset_root = resolve_asset_root(&args);

    if args.asset_path.is_some() || args.asset_dir.is_some() {
        warn!("--asset-path/--asset-dir are deprecated aliases; prefer --assets");
    }

    let output_path = resolve_output_path(&args, filetype);

    info!("Converting file: {}", args.file.display());
    info!("Requested output path: {}", output_path.display());

    if let Some(result) = filetype.handle_direct_convert(&args, &output_path) {
        return result;
    }

    let registry = if filetype == FileType::Rgm {
        info!(
            "Creating registry from assets root: {}",
            asset_root.display()
        );
        Some(registry::scan_dir(asset_root.clone())?)
    } else {
        None
    };

    let palette = load_palette(&args, &asset_root, filetype)?;
    let mut texture_cache = build_texture_cache(palette.as_ref(), asset_root);

    if filetype == FileType::Rgm {
        let registry = registry.ok_or_else(|| {
            color_eyre::eyre::eyre!("internal error: registry is required for RGM files")
        })?;
        return convert_rgm(
            &args,
            &output_path,
            &registry,
            palette.as_ref(),
            &mut texture_cache,
        );
    }

    convert_models(
        &args,
        &output_path,
        filetype,
        registry.as_ref(),
        palette.as_ref(),
        &mut texture_cache,
    )
}
