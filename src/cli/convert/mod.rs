mod cht;
mod col;
mod fnt;
mod pvo;
mod rtx;
mod sfx;
mod texbsi;
mod wld;

use super::utils::{
    auto_resolve_palette, parse_file, resolve_asset_root_from_input, resolve_filetype,
};
use crate::opts::{ConvertArgs, FontOutputMode};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use color_eyre::eyre::bail;
use log::{info, warn};
use redguard_preservation::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf, to_glb,
};
use redguard_preservation::import::FileType;
use redguard_preservation::import::{palette::Palette, registry};
use std::path::{Path, PathBuf};

fn resolve_asset_root(args: &ConvertArgs) -> PathBuf {
    args.assets
        .clone()
        .or_else(|| args.asset_path.clone())
        .or_else(|| args.asset_dir.clone())
        .unwrap_or_else(|| resolve_asset_root_from_input(&args.file))
}

fn default_output_for(file: &Path, filetype: FileType) -> PathBuf {
    let mut path = file.to_path_buf();
    match filetype {
        FileType::Fnt | FileType::Col => {
            path.set_extension("png");
        }
        FileType::Cht | FileType::Pvo => {
            path.set_extension("json");
        }
        FileType::Wld => {
            path.set_extension("png");
        }
        FileType::Sfx | FileType::Rtx => {
            path.set_extension("");
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            path.set_file_name(format!("{stem}_wav"));
        }
        FileType::Bsi => {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            path.set_file_name(format!("{stem}_{ext}"));
        }
        FileType::Model3d | FileType::Model3dc | FileType::Rob | FileType::Rgm => {
            path.set_extension("glb");
        }
    }
    path
}

fn default_fnt_output_for_mode(file: &Path, mode: FontOutputMode) -> PathBuf {
    let mut path = file.to_path_buf();
    match mode {
        FontOutputMode::Bitmap => {
            path.set_extension("png");
        }
        FontOutputMode::Ttf => {
            path.set_extension("ttf");
        }
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

pub(crate) fn handle_convert_command(args: ConvertArgs) -> Result<()> {
    let filetype = resolve_filetype(&args.file, args.filetype)?;
    let asset_root = resolve_asset_root(&args);

    if args.asset_path.is_some() || args.asset_dir.is_some() {
        warn!("--asset-path/--asset-dir are deprecated aliases; prefer --assets");
    }

    let output_path = if filetype == FileType::Fnt {
        match (args.output.clone(), args.font_output) {
            (Some(path), _) => path,
            (None, Some(mode)) => default_fnt_output_for_mode(&args.file, mode),
            (None, None) => default_output_for(&args.file, filetype),
        }
    } else {
        args.output
            .clone()
            .unwrap_or_else(|| default_output_for(&args.file, filetype))
    };

    info!("Converting file: {}", args.file.display());
    info!("Requested output path: {}", output_path.display());

    match filetype {
        FileType::Cht => return cht::handle_cht_convert(&args, &output_path),
        FileType::Col => return col::handle_col_convert(&args, &output_path),
        FileType::Pvo => return pvo::handle_pvo_convert(&args, &output_path),
        FileType::Fnt => return fnt::handle_fnt_convert(&args, &output_path),
        FileType::Wld => return wld::handle_wld_convert(&args, &output_path),
        FileType::Sfx => return sfx::handle_sfx_convert(&args, &output_path),
        FileType::Rtx => return rtx::handle_rtx_convert(&args, &output_path),
        FileType::Bsi => return texbsi::handle_texbsi_convert(&args, &output_path),
        FileType::Rgm | FileType::Model3d | FileType::Model3dc | FileType::Rob => {}
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

    let palette = match args.palette.as_ref() {
        Some(p) => {
            let data = std::fs::read(p)?;
            Some(Palette::parse(&data).map_err(|e| color_eyre::eyre::eyre!("{e}"))?)
        }
        None => auto_resolve_palette(&asset_root, &args.file, filetype)?,
    };

    let mut texture_cache = if let Some(pal) = palette.as_ref() {
        Some(TextureCache::new(
            asset_root.clone(),
            Some(Palette { colors: pal.colors }),
        ))
    } else {
        warn!(
            "No palette provided (and none found in WORLD.INI); texture export is disabled and materials will be plain white"
        );
        None
    };

    if filetype == FileType::Rgm {
        let registry = registry.ok_or_else(|| {
            color_eyre::eyre::eyre!("internal error: registry is required for RGM files")
        })?;
        let file_content = std::fs::read(&args.file)?;
        let (rgm_file, positioned_models, lights) =
            redguard_preservation::import::rgm::parse_rgm_with_models(&file_content, &registry)?;

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
            palette.as_ref(),
            texture_cache.as_mut(),
            args.compress_textures,
        )?;
        let glb_data = to_glb(&root, &buffer)?;
        ensure_parent_dir(&output_path)?;
        std::fs::write(&output_path, &glb_data)?;
        info!("Successfully converted to: {}", output_path.display());

        let json_path = output_path.with_extension("json");
        let metadata = redguard_preservation::import::rgm::export_rgm_metadata_json(&rgm_file);
        let json_bytes = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&json_path, json_bytes)?;
        info!("Actor metadata exported to: {}", json_path.display());
    } else {
        let models = parse_file(&args.file, Some(filetype), registry.as_ref())?;

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
            palette.as_ref(),
            texture_cache.as_mut(),
            args.compress_textures,
        )?;
        let glb_data = to_glb(&root, &buffer)?;
        ensure_parent_dir(&output_path)?;
        std::fs::write(&output_path, glb_data)?;
        info!("Successfully converted to: {}", output_path.display());
    }

    Ok(())
}
