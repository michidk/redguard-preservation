mod buffer;
pub mod scene;

use self::buffer::{
    clear_last_error, into_ffi_result, last_error_message, run_on_large_stack, set_last_error,
};
use crate::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf,
    convert_wld_scene_to_gltf, to_glb,
};
use crate::import::{FileType, palette::Palette, registry, rgm, rob, wld, world_ini::WorldIni};
use crate::model3d;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;

pub use self::buffer::ByteBuffer;

pub(crate) fn i32_to_usize(value: i32, name: &str) -> crate::Result<usize> {
    usize::try_from(value)
        .map_err(|_| crate::error::Error::Parse(format!("{name} must be >= 0, got {value}")))
}

pub(crate) unsafe fn read_bytes<'a>(
    data: *const u8,
    len: i32,
    name: &str,
) -> crate::Result<&'a [u8]> {
    let len = i32_to_usize(len, name)?;
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(crate::error::Error::Parse(format!(
            "{name} pointer is null but length is {len}"
        )));
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    Ok(bytes)
}

unsafe fn read_array<'a, T>(data: *const T, count: usize, name: &str) -> crate::Result<&'a [T]> {
    if count == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(crate::error::Error::Parse(format!(
            "{name} pointer is null but count is {count}"
        )));
    }
    let items = unsafe { std::slice::from_raw_parts(data, count) };
    Ok(items)
}

unsafe fn read_c_str(ptr: *const c_char, name: &str) -> crate::Result<String> {
    if ptr.is_null() {
        return Err(crate::error::Error::Parse(format!("{name} is null")));
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str()
        .map(|s| s.to_owned())
        .map_err(|e| crate::error::Error::Parse(format!("{name} is not valid UTF-8: {e}")))
}

const WORLD_INI_NAMES: [&str; 2] = ["WORLD.INI", "world.ini"];

fn find_world_ini(asset_root: &Path) -> Option<PathBuf> {
    for name in &WORLD_INI_NAMES {
        let path = asset_root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn find_palette_on_disk(asset_root: &Path, ini_palette_path: &str) -> Option<PathBuf> {
    let filename = ini_palette_path
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_else(|| ini_palette_path.trim());
    let filename_lower = filename.to_ascii_lowercase();

    for dir_name in &["fxart", "3dart", "FXART", "3DART"] {
        let dir = asset_root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }

        let exact = dir.join(filename);
        if exact.is_file() {
            return Some(exact);
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            if entry.file_name().to_string_lossy().to_ascii_lowercase() == filename_lower {
                return Some(entry.path());
            }
        }
    }
    None
}

fn auto_resolve_palette(
    asset_root: &Path,
    input_file: &Path,
    file_type: FileType,
) -> Option<Palette> {
    if !matches!(file_type, FileType::Rgm | FileType::Wld) {
        return None;
    }

    let ini_path = find_world_ini(asset_root)?;
    let content = std::fs::read_to_string(ini_path).ok()?;
    let world_ini = WorldIni::parse(&content);

    let file_stem = input_file.file_stem()?.to_str().unwrap_or("");
    let matches = match file_type {
        FileType::Rgm => world_ini.find_by_map_stem(file_stem),
        FileType::Wld => world_ini.find_by_world_stem(file_stem),
        _ => return None,
    };
    let entry = matches.first()?;
    let palette_path = find_palette_on_disk(asset_root, &entry.palette)?;
    let bytes = std::fs::read(palette_path).ok()?;
    Palette::parse(&bytes).ok()
}

fn build_path_texture_cache(
    asset_root: &Path,
    file_path: &Path,
    file_type: FileType,
) -> (Option<Palette>, Option<TextureCache>) {
    let palette = auto_resolve_palette(asset_root, file_path, file_type);
    match palette {
        Some(pal) => {
            let cache = TextureCache::new(
                asset_root.to_path_buf(),
                Some(Palette { colors: pal.colors }),
            );
            (Some(pal), Some(cache))
        }
        None => (None, None),
    }
}

#[allow(clippy::type_complexity)]
unsafe fn extract_texbsi_data(
    palette_data: *const u8,
    palette_len: i32,
    texbsi_ids: *const u16,
    texbsi_datas: *const *const u8,
    texbsi_lens: *const i32,
    texbsi_count: i32,
) -> crate::Result<(Option<Palette>, HashMap<u16, Vec<u8>>)> {
    let palette = if palette_data.is_null() {
        None
    } else {
        let bytes = unsafe { read_bytes(palette_data, palette_len, "palette_data") }?;
        Some(Palette::parse(bytes)?)
    };

    let count = i32_to_usize(texbsi_count, "texbsi_count")?;
    if count == 0 {
        return Ok((palette, HashMap::new()));
    }

    let ids = unsafe { read_array(texbsi_ids, count, "texbsi_ids") }?;
    let datas = unsafe { read_array(texbsi_datas, count, "texbsi_datas") }?;
    let lens = unsafe { read_array(texbsi_lens, count, "texbsi_lens") }?;

    let mut texbsi_map = HashMap::with_capacity(count);
    for idx in 0..count {
        let data = unsafe { read_bytes(datas[idx], lens[idx], &format!("texbsi_datas[{idx}]")) }?;
        texbsi_map.insert(ids[idx], data.to_vec());
    }

    Ok((palette, texbsi_map))
}

fn wld_texbsi_id(wld_file: &wld::WldFile) -> u16 {
    u16::from_le_bytes([
        wld_file.sections[0].header[6],
        wld_file.sections[0].header[7],
    ])
}

/// # Safety
///
/// `buffer` must be either null or a pointer previously returned by this FFI module.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_free_buffer(buffer: *mut ByteBuffer) {
    if buffer.is_null() {
        return;
    }
    let owned = unsafe { Box::from_raw(buffer) };
    owned.destroy();
}

/// # Safety
///
/// The returned pointer must be released by calling `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_last_error() -> *mut ByteBuffer {
    match last_error_message() {
        Some(err) => Box::into_raw(Box::new(ByteBuffer::from_vec(err.into_bytes()))),
        None => ptr::null_mut(),
    }
}

/// # Safety
///
/// All pointer/length arguments must describe readable memory for the given lengths.
/// The returned pointer must be freed with `rg_texture_cache_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_texture_cache_create(
    palette_data: *const u8,
    palette_len: i32,
    texbsi_ids: *const u16,
    texbsi_datas: *const *const u8,
    texbsi_lens: *const i32,
    texbsi_count: i32,
) -> *mut TextureCache {
    let result = (|| -> crate::Result<TextureCache> {
        let (palette, texbsi_map) = unsafe {
            extract_texbsi_data(
                palette_data,
                palette_len,
                texbsi_ids,
                texbsi_datas,
                texbsi_lens,
                texbsi_count,
            )
        }?;
        run_on_large_stack(move || Ok(TextureCache::from_data(texbsi_map, palette)))
    })();
    match result {
        Ok(cache) => {
            clear_last_error();
            Box::into_raw(Box::new(cache))
        }
        Err(err) => {
            set_last_error(err);
            ptr::null_mut()
        }
    }
}

/// # Safety
///
/// `cache` must be null or a pointer returned by `rg_texture_cache_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_texture_cache_free(cache: *mut TextureCache) {
    if cache.is_null() {
        return;
    }
    let _ = unsafe { Box::from_raw(cache) };
}

/// # Safety
///
/// `file_path` and `assets_dir` must be valid null-terminated UTF-8 strings.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_model_from_path(
    file_path: *const c_char,
    assets_dir: *const c_char,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let file_path = PathBuf::from(file_path);
        let assets_dir = PathBuf::from(assets_dir);

        let file_type = FileType::from_path(&file_path).ok_or_else(|| {
            crate::error::Error::Parse(format!(
                "unsupported model path extension: {}",
                file_path.display()
            ))
        })?;

        let model_bytes = std::fs::read(&file_path)?;
        let (palette, mut texture_cache) =
            build_path_texture_cache(&assets_dir, &file_path, file_type);

        run_on_large_stack(move || {
            let models = match file_type {
                FileType::Model3d | FileType::Model3dc => {
                    vec![model3d::parse_3d_file(&model_bytes)?]
                }
                FileType::Rob => {
                    let (_, models) = rob::parse_rob_with_models(&model_bytes)?;
                    models
                }
                _ => {
                    return Err(crate::error::Error::Parse(format!(
                        "rg_convert_model_from_path supports .3d/.3dc/.rob, got: {}",
                        file_path.display()
                    )));
                }
            };

            let (root, buffer) =
                convert_models_to_gltf(&models, palette.as_ref(), texture_cache.as_mut(), false)?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `file_path` and `assets_dir` must be valid null-terminated UTF-8 strings.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_rgm_from_path(
    file_path: *const c_char,
    assets_dir: *const c_char,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let file_path = PathBuf::from(file_path);
        let assets_dir = PathBuf::from(assets_dir);

        let rgm_bytes = std::fs::read(&file_path)?;
        let registry = registry::scan_dir(&assets_dir)?;
        let (palette, mut texture_cache) =
            build_path_texture_cache(&assets_dir, &file_path, FileType::Rgm);

        run_on_large_stack(move || {
            let (_, positioned_models, lights) = rgm::parse_rgm_with_models(&rgm_bytes, &registry)?;
            let (root, buffer) = convert_positioned_models_to_gltf(
                &positioned_models,
                &lights,
                palette.as_ref(),
                texture_cache.as_mut(),
                false,
            )?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `rgm_data` must point to readable bytes of length `rgm_len`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_rgm_metadata(rgm_data: *const u8, rgm_len: i32) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let rgm_bytes = unsafe { read_bytes(rgm_data, rgm_len, "rgm_data") }?;
        run_on_large_stack(move || {
            let metadata = rgm::export_rgm_runtime_metadata_json(rgm_bytes)?;
            Ok(serde_json::to_vec(&metadata)?)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `file_path` and `assets_dir` must be valid null-terminated UTF-8 strings.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_wld_from_path(
    file_path: *const c_char,
    assets_dir: *const c_char,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let file_path = unsafe { read_c_str(file_path, "file_path") }?;
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let file_path = PathBuf::from(file_path);
        let assets_dir = PathBuf::from(assets_dir);

        let wld_bytes = std::fs::read(&file_path)?;
        let wld_file = wld::parse_wld_file(&wld_bytes)?;
        let texbsi_id = wld_texbsi_id(&wld_file);

        let rgm_upper = file_path.with_extension("RGM");
        let rgm_lower = file_path.with_extension("rgm");
        let companion_rgm = if rgm_upper.is_file() {
            Some(rgm_upper)
        } else if rgm_lower.is_file() {
            Some(rgm_lower)
        } else {
            None
        };

        let positioned_models = if let Some(rgm_path) = companion_rgm {
            let registry = registry::scan_dir(&assets_dir)?;
            let rgm_bytes = std::fs::read(rgm_path)?;
            let (_, models, _) = rgm::parse_rgm_with_models(&rgm_bytes, &registry)?;
            models
        } else {
            Vec::new()
        };

        let (palette, mut texture_cache) =
            build_path_texture_cache(&assets_dir, &file_path, FileType::Wld);

        run_on_large_stack(move || {
            let (root, buffer) = convert_wld_scene_to_gltf(
                &wld_file,
                texbsi_id,
                &positioned_models,
                palette.as_ref(),
                texture_cache.as_mut(),
                false,
            )?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}
