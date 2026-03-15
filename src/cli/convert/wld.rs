use crate::cli::utils::{auto_resolve_palette, resolve_asset_root_from_input};
use crate::opts::ConvertArgs;
use color_eyre::Result;
use log::{info, warn};
use redguard_preservation::gltf::{TextureCache, convert_wld_scene_to_gltf, to_glb};
use redguard_preservation::import::{FileType, palette::Palette, registry, wld};
use std::path::Path;

pub(super) fn handle_wld_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let out_ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    if matches!(out_ext.as_deref(), Some("glb")) {
        let bytes = std::fs::read(&args.file)?;
        let wld_file = wld::parse_wld_file(&bytes).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

        const ENGINE_TERRAIN_TEXBSI_ID: u16 = 302;

        let declared_texbsi_id = if wld_file.sections[0].header.len() >= 8 {
            Some(u16::from_le_bytes([
                wld_file.sections[0].header[6],
                wld_file.sections[0].header[7],
            ]))
        } else {
            None
        };

        if let Some(declared) = declared_texbsi_id
            && declared != ENGINE_TERRAIN_TEXBSI_ID
        {
            warn!(
                "WLD section header declares TEXBSI.{declared:03}, but original engine terrain path hard-wires TEXBSI.{ENGINE_TERRAIN_TEXBSI_ID:03}; using engine behavior"
            );
        }

        let texbsi_id = ENGINE_TERRAIN_TEXBSI_ID;

        let asset_root = args
            .assets
            .clone()
            .or_else(|| args.asset_path.clone())
            .or_else(|| args.asset_dir.clone())
            .unwrap_or_else(|| resolve_asset_root_from_input(&args.file));

        let palette = match args.palette.as_ref() {
            Some(path) => {
                let data = std::fs::read(path)?;
                Some(Palette::parse(&data).map_err(|e| color_eyre::eyre::eyre!("{e}"))?)
            }
            None => auto_resolve_palette(&asset_root, &args.file, FileType::Wld)?,
        };

        let mut texture_cache = if args.terrain_textures {
            Some(TextureCache::new(
                asset_root.clone(),
                palette.as_ref().map(|pal| Palette { colors: pal.colors }),
            ))
        } else {
            info!("Terrain textures disabled (--terrain-textures false)");
            None
        };

        let rgm_upper = args.file.with_extension("RGM");
        let rgm_lower = args.file.with_extension("rgm");
        let rgm_path = if args.terrain_only {
            None
        } else if rgm_upper.is_file() {
            Some(rgm_upper)
        } else if rgm_lower.is_file() {
            Some(rgm_lower)
        } else {
            None
        };

        let positioned_models = if let Some(rgm_file) = rgm_path {
            let registry = registry::scan_dir(asset_root)?;
            let rgm_bytes = std::fs::read(&rgm_file)?;
            let (rgm_parsed, models, _lights) =
                redguard_preservation::import::rgm::parse_rgm_with_models(&rgm_bytes, &registry)
                    .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            info!(
                "Loaded {} positioned models from companion scene '{}'",
                models.len(),
                rgm_file.display()
            );

            let json_path = output_path.with_extension("json");
            let metadata =
                redguard_preservation::import::rgm::export_rgm_metadata_json(&rgm_parsed);
            let json_bytes = serde_json::to_string_pretty(&metadata)?;
            super::ensure_parent_dir(&json_path)?;
            std::fs::write(&json_path, &json_bytes)?;
            info!("Actor metadata exported to: {}", json_path.display());

            models
        } else if args.terrain_only {
            info!("Terrain-only mode enabled (--terrain-only)");
            Vec::new()
        } else {
            warn!("No companion RGM found for WLD; exporting terrain only");
            Vec::new()
        };

        let (root, buffer) = convert_wld_scene_to_gltf(
            &wld_file,
            texbsi_id,
            &positioned_models,
            palette.as_ref(),
            texture_cache.as_mut(),
            args.compress_textures,
        )
        .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

        let glb = to_glb(&root, &buffer).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
        super::ensure_parent_dir(output_path)?;
        std::fs::write(output_path, glb)?;
        info!(
            "Successfully exported WLD scene GLB: {}",
            output_path.display()
        );
        return Ok(());
    }

    if !matches!(out_ext.as_deref(), Some("png")) {
        warn!("WLD map export writes PNG unless output is .glb; overriding extension to .png");
    }

    let png_output = if matches!(out_ext.as_deref(), Some("png")) {
        output_path.to_path_buf()
    } else {
        output_path.with_extension("png")
    };

    let outputs = wld::export_wld_maps_pngs(&args.file, &png_output)
        .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
    info!("Successfully exported WLD map PNGs:");
    info!("  map1 (height): {}", outputs.map1_path.display());
    info!("  map2: {}", outputs.map2_path.display());
    info!("  map3: {}", outputs.map3_path.display());
    info!("  map4: {}", outputs.map4_path.display());
    Ok(())
}
