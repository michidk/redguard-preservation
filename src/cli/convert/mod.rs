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
use crate::opts::{
    AutoConvertArgs, ColArgs, ConvertArgs, ConvertCommand, ConvertFileArgs, FntArgs, FntFormat,
    GxaArgs, GxaFormat, ModelArgs, RgmArgs, RtxArgs, TexbsiArgs, TexbsiFormat, WldArgs,
};
use color_eyre::Result;
use color_eyre::eyre::{WrapErr, bail};
use log::{info, warn};
use rgpre::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf, to_glb,
};
use rgpre::import::FileType;
use rgpre::import::{palette::Palette, registry};
use std::path::{Path, PathBuf};

fn resolve_output(io: &ConvertFileArgs, filetype: FileType) -> PathBuf {
    let default = filetype.default_output_path(&io.file);
    match io.output.clone() {
        Some(out) => resolve_dir_output(out, &default),
        None => default,
    }
}

fn resolve_fnt_output(args: &FntArgs) -> PathBuf {
    let default = if args.format == FntFormat::Ttf {
        let mut p = args.io.file.clone();
        p.set_extension("ttf");
        p
    } else {
        FileType::Fnt.default_output_path(&args.io.file)
    };
    match args.io.output.clone() {
        Some(out) => resolve_dir_output(out, &default),
        None => default,
    }
}

fn resolve_dir_output(output: PathBuf, default_path: &Path) -> PathBuf {
    if output.is_dir() {
        output.join(default_path.file_name().unwrap_or(default_path.as_os_str()))
    } else {
        output
    }
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

fn resolve_asset_root(assets: Option<&Path>, file: &Path) -> PathBuf {
    assets
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_asset_root_from_input(file))
}

fn load_palette(
    palette_path: Option<&Path>,
    asset_root: &Path,
    file: &Path,
    filetype: FileType,
) -> Result<Option<Palette>> {
    match palette_path {
        Some(path) => {
            let data = std::fs::read(path)?;
            let palette = Palette::parse(&data).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            Ok(Some(palette))
        }
        None => auto_resolve_palette(asset_root, file, filetype),
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

fn handle_model_convert(args: &ModelArgs) -> Result<()> {
    let filetype = resolve_filetype(&args.io.file)?;
    if !matches!(
        filetype,
        FileType::Model3d | FileType::Model3dc | FileType::Rob
    ) {
        bail!(
            "'model' subcommand expects .3d, .3dc, or .rob files; got {}",
            filetype.display_name()
        );
    }

    let output_path = resolve_output(&args.io, filetype);
    let asset_root = resolve_asset_root(args.assets.as_deref(), &args.io.file);
    let palette = load_palette(
        args.palette.as_deref(),
        &asset_root,
        &args.io.file,
        filetype,
    )?;
    let mut texture_cache = build_texture_cache(palette.as_ref(), asset_root);

    let models = parse_file(&args.io.file, Some(filetype), None)?;

    if filetype == FileType::Rob {
        warn!(
            "ROB conversion exports segment geometry only (no scene instance placement). Full area/object placement requires RGM scene data."
        );
    }

    if models.is_empty() {
        bail!("No 3D models found to convert.");
    }

    info!("Converting file: {}", args.io.file.display());
    info!("Requested output path: {}", output_path.display());

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

    Ok(())
}

fn handle_rgm_convert(args: &RgmArgs) -> Result<()> {
    let output_path = resolve_output(&args.io, FileType::Rgm);
    let asset_root = resolve_asset_root(args.assets.as_deref(), &args.io.file);
    let soup_def = rgpre::import::soup_def::try_load_soup_def(&asset_root);
    let rtx_labels = rgpre::import::rtx::try_load_rtx_labels(&asset_root);

    info!(
        "Creating registry from assets root: {}",
        asset_root.display()
    );
    let registry = registry::scan_dir(asset_root.clone())?;

    let palette = load_palette(
        args.palette.as_deref(),
        &asset_root,
        &args.io.file,
        FileType::Rgm,
    )?;
    let mut texture_cache = build_texture_cache(palette.as_ref(), asset_root);

    info!("Converting file: {}", args.io.file.display());
    info!("Requested output path: {}", output_path.display());

    let file_content = std::fs::read(&args.io.file)?;
    let (rgm_file, positioned_models, lights) =
        rgpre::import::rgm::parse_rgm_with_models(&file_content, &registry)?;

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

    let out_dir = output_path.with_extension("");
    std::fs::create_dir_all(&out_dir)?;

    let glb_name = output_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("output.glb"));
    let glb_path = out_dir.join(glb_name);
    std::fs::write(&glb_path, &glb_data)?;
    info!("Successfully converted to: {}", out_dir.display());

    let metadata = rgpre::import::rgm::export_rgm_metadata_json(&rgm_file, soup_def.as_ref());

    let write_json = |name: &str, value: &serde_json::Value| -> Result<()> {
        let path = out_dir.join(name);
        std::fs::write(&path, serde_json::to_string_pretty(value)?)?;
        Ok(())
    };

    if let serde_json::Value::Object(map) = &metadata {
        let pick = |keys: &[&str]| -> serde_json::Value {
            let mut obj = serde_json::Map::new();
            for &key in keys {
                if let Some(val) = map.get(key) {
                    obj.insert(key.to_string(), val.clone());
                }
            }
            serde_json::Value::Object(obj)
        };

        write_json(
            "scene.json",
            &pick(&[
                "mps_placements",
                "mpob_objects",
                "lights",
                "flat_sprites",
                "ropes",
                "markers",
                "collision_volumes",
                "mpsz_entries",
            ]),
        )?;
        write_json("actors.json", &pick(&["actors"]))?;
        write_json(
            "navigation.json",
            &pick(&["walk_node_maps", "ralc_locations"]),
        )?;
    }
    info!("Exported metadata to: {}", out_dir.display());

    let scripts = rgpre::import::rgm::disassemble_rgm_scripts(&rgm_file, soup_def.as_ref());
    if !scripts.is_empty() {
        let scripts_dir = out_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir)?;
        let source_name = args.io.file.file_name().map_or_else(
            || "unknown".to_string(),
            |n| n.to_string_lossy().to_string(),
        );
        for (name, script) in &scripts {
            let text = rgpre::import::rgm::script::format_soup_text(
                name,
                &source_name,
                script,
                rtx_labels.as_ref(),
            );
            std::fs::write(scripts_dir.join(format!("{name}.soup")), text)?;

            let meta_json = rgpre::import::rgm::script::script_metadata_json(script);
            let json_bytes = serde_json::to_string_pretty(&meta_json)?;
            std::fs::write(scripts_dir.join(format!("{name}.json")), json_bytes)?;
        }
        info!(
            "Exported {} script files to: {}",
            scripts.len(),
            scripts_dir.display()
        );
    }

    Ok(())
}

fn handle_subcommand(cmd: ConvertCommand) -> Result<()> {
    match cmd {
        ConvertCommand::Texbsi(args) => {
            let output = resolve_output(&args.io, FileType::Bsi);
            texbsi::handle_texbsi_convert(&args, &output)
        }
        ConvertCommand::Gxa(args) => {
            let output = resolve_output(&args.io, FileType::Gxa);
            gxa::handle_gxa_convert(&args, &output)
        }
        ConvertCommand::Fnt(args) => {
            let output = resolve_fnt_output(&args);
            fnt::handle_fnt_convert(&args, &output)
        }
        ConvertCommand::Col(args) => {
            let output = resolve_output(&args.io, FileType::Col);
            col::handle_col_convert(&args, &output)
        }
        ConvertCommand::Wld(args) => {
            let output = resolve_output(&args.io, FileType::Wld);
            wld::handle_wld_convert(&args, &output)
        }
        ConvertCommand::Model(args) => handle_model_convert(&args),
        ConvertCommand::Rgm(args) => handle_rgm_convert(&args),
        ConvertCommand::Rtx(args) => {
            let output = resolve_output(&args.io, FileType::Rtx);
            rtx::handle_rtx_convert(&args, &output)
        }
        ConvertCommand::Sfx(io) => {
            let output = resolve_output(&io, FileType::Sfx);
            sfx::handle_sfx_convert(&io.file, &output)
        }
        ConvertCommand::Cht(io) => {
            let output = resolve_output(&io, FileType::Cht);
            cht::handle_cht_convert(&io.file, &output)
        }
        ConvertCommand::Pvo(io) => {
            let output = resolve_output(&io, FileType::Pvo);
            pvo::handle_pvo_convert(&io.file, &output)
        }
    }
}

fn handle_auto_convert(auto: AutoConvertArgs) -> Result<()> {
    let file = auto.file.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "file argument required.\nUsage: convert <FILE> or convert <FORMAT> <FILE>"
        )
    })?;
    let filetype = resolve_filetype(&file)?;

    let io = ConvertFileArgs {
        file,
        output: auto.output,
    };
    let compress = auto.compress_textures;

    info!("Converting file: {}", io.file.display());

    match filetype {
        FileType::Bsi => {
            let args = TexbsiArgs {
                io,
                format: TexbsiFormat::Gif,
                palette: None,
                compress_textures: compress,
            };
            let output = resolve_output(&args.io, FileType::Bsi);
            info!("Requested output path: {}", output.display());
            texbsi::handle_texbsi_convert(&args, &output)
        }
        FileType::Gxa => {
            let args = GxaArgs {
                io,
                format: GxaFormat::Gif,
                compress_textures: compress,
            };
            let output = resolve_output(&args.io, FileType::Gxa);
            info!("Requested output path: {}", output.display());
            gxa::handle_gxa_convert(&args, &output)
        }
        FileType::Fnt => {
            let args = FntArgs {
                io,
                format: FntFormat::Bitmap,
                compress_textures: compress,
            };
            let output = resolve_fnt_output(&args);
            info!("Requested output path: {}", output.display());
            fnt::handle_fnt_convert(&args, &output)
        }
        FileType::Col => {
            let args = ColArgs {
                io,
                format: None,
                compress_textures: compress,
            };
            let output = resolve_output(&args.io, FileType::Col);
            info!("Requested output path: {}", output.display());
            col::handle_col_convert(&args, &output)
        }
        FileType::Wld => {
            let args = WldArgs {
                io,
                assets: None,
                palette: None,
                terrain_only: false,
                terrain_textures: true,
                compress_textures: compress,
            };
            let output = resolve_output(&args.io, FileType::Wld);
            info!("Requested output path: {}", output.display());
            wld::handle_wld_convert(&args, &output)
        }
        FileType::Model3d | FileType::Model3dc | FileType::Rob => {
            let args = ModelArgs {
                io,
                assets: None,
                palette: None,
                compress_textures: compress,
            };
            handle_model_convert(&args)
        }
        FileType::Rgm => {
            let args = RgmArgs {
                io,
                assets: None,
                palette: None,
                compress_textures: compress,
            };
            handle_rgm_convert(&args)
        }
        FileType::Rtx => {
            let args = RtxArgs {
                io,
                resolve_names: false,
            };
            let output = resolve_output(&args.io, FileType::Rtx);
            info!("Requested output path: {}", output.display());
            rtx::handle_rtx_convert(&args, &output)
        }
        FileType::Sfx => {
            let output = resolve_output(&io, FileType::Sfx);
            info!("Requested output path: {}", output.display());
            sfx::handle_sfx_convert(&io.file, &output)
        }
        FileType::Cht => {
            let output = resolve_output(&io, FileType::Cht);
            info!("Requested output path: {}", output.display());
            cht::handle_cht_convert(&io.file, &output)
        }
        FileType::Pvo => {
            let output = resolve_output(&io, FileType::Pvo);
            info!("Requested output path: {}", output.display());
            pvo::handle_pvo_convert(&io.file, &output)
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn handle_convert_command(args: ConvertArgs) -> Result<()> {
    match args.command {
        Some(cmd) => handle_subcommand(cmd),
        None => handle_auto_convert(args.auto),
    }
}
