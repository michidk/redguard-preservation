mod buffer;
pub mod scene;

use self::buffer::{
    clear_last_error, into_ffi_result, last_error_message, run_on_large_stack, set_last_error,
};
use crate::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf,
    convert_wld_scene_to_gltf, to_glb,
};
use crate::import::{FileType, palette::Palette, registry::Registry, rgm, rob, wld};
use crate::model3d;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
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

fn normalize_model_name(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    let stem = file_name.split('.').next().unwrap_or(file_name).trim();
    stem.to_ascii_uppercase()
}

fn parse_model_file_type(discriminant: u8) -> crate::Result<FileType> {
    match discriminant {
        3 => Ok(FileType::Model3d),
        4 => Ok(FileType::Model3dc),
        5 => Ok(FileType::Rob),
        _ => Err(crate::error::Error::Parse(format!(
            "unsupported model file type discriminant: {discriminant}"
        ))),
    }
}

unsafe fn build_registry_from_raw(
    model_names: *const *const u8,
    model_datas: *const *const u8,
    model_lens: *const i32,
    model_types: *const u8,
    model_count: i32,
) -> crate::Result<Registry> {
    let count = i32_to_usize(model_count, "model_count")?;
    if count == 0 {
        return Ok(Registry::from_data(HashMap::new()));
    }

    let names = unsafe { read_array(model_names, count, "model_names") }?;
    let datas = unsafe { read_array(model_datas, count, "model_datas") }?;
    let lens = unsafe { read_array(model_lens, count, "model_lens") }?;
    let types = unsafe { read_array(model_types, count, "model_types") }?;

    let mut entries: HashMap<String, (Vec<u8>, FileType)> = HashMap::with_capacity(count);
    for idx in 0..count {
        let name_ptr = names[idx];
        if name_ptr.is_null() {
            return Err(crate::error::Error::Parse(format!(
                "model_names[{idx}] is null"
            )));
        }
        let c_name = unsafe { CStr::from_ptr(name_ptr.cast::<c_char>()) };
        let model_name = normalize_model_name(&c_name.to_string_lossy());
        if model_name.is_empty() {
            return Err(crate::error::Error::Parse(format!(
                "model_names[{idx}] resolves to empty model name"
            )));
        }

        let file_type = parse_model_file_type(types[idx])?;
        let data = unsafe { read_bytes(datas[idx], lens[idx], &format!("model_datas[{idx}]")) }?;

        entries.insert(model_name, (data.to_vec(), file_type));
    }

    Ok(Registry::from_data(entries))
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

fn extract_palette(cache: &Option<&mut TextureCache>) -> Option<Palette> {
    match cache {
        Some(tc) => tc.palette().map(|p| Palette { colors: p.colors }),
        None => None,
    }
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
/// `model_data` must point to readable bytes of length `model_len`.
/// `texture_cache` may be null or a valid pointer from `rg_texture_cache_create`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_model_to_glb(
    model_data: *const u8,
    model_len: i32,
    texture_cache: *mut TextureCache,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let model_bytes = unsafe { read_bytes(model_data, model_len, "model_data") }?;
        let texture_cache = unsafe { texture_cache.as_mut() };
        run_on_large_stack(move || {
            let model = model3d::parse_3d_file(model_bytes)?;
            let palette = extract_palette(&texture_cache);
            let (root, buffer) = convert_models_to_gltf(
                std::slice::from_ref(&model),
                palette.as_ref(),
                texture_cache,
                false,
            )?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `rob_data` must point to readable bytes of length `rob_len`.
/// `texture_cache` may be null or a valid pointer from `rg_texture_cache_create`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_rob_to_glb(
    rob_data: *const u8,
    rob_len: i32,
    texture_cache: *mut TextureCache,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let rob_bytes = unsafe { read_bytes(rob_data, rob_len, "rob_data") }?;
        let texture_cache = unsafe { texture_cache.as_mut() };
        run_on_large_stack(move || {
            let (_, models) = rob::parse_rob_with_models(rob_bytes)?;
            let palette = extract_palette(&texture_cache);
            let (root, buffer) =
                convert_models_to_gltf(&models, palette.as_ref(), texture_cache, false)?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `rgm_data` must point to readable bytes of length `rgm_len`.
/// Model arrays must all have `model_count` entries when `model_count > 0`.
/// `texture_cache` may be null or a valid pointer from `rg_texture_cache_create`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_rgm_to_glb(
    rgm_data: *const u8,
    rgm_len: i32,
    texture_cache: *mut TextureCache,
    model_names: *const *const u8,
    model_datas: *const *const u8,
    model_lens: *const i32,
    model_types: *const u8,
    model_count: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let rgm_bytes = unsafe { read_bytes(rgm_data, rgm_len, "rgm_data") }?;
        let registry = unsafe {
            build_registry_from_raw(
                model_names,
                model_datas,
                model_lens,
                model_types,
                model_count,
            )
        }?;
        let texture_cache = unsafe { texture_cache.as_mut() };
        run_on_large_stack(move || {
            let (_, positioned_models, lights) = rgm::parse_rgm_with_models(rgm_bytes, &registry)?;
            let palette = extract_palette(&texture_cache);
            let (root, buffer) = convert_positioned_models_to_gltf(
                &positioned_models,
                &lights,
                palette.as_ref(),
                texture_cache,
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
/// `wld_data` must point to readable bytes of length `wld_len`.
/// If `rgm_data` is non-null, it must point to readable bytes of length `rgm_len`.
/// Model arrays must all have `model_count` entries when `model_count > 0`.
/// `texture_cache` may be null or a valid pointer from `rg_texture_cache_create`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_convert_wld_to_glb(
    wld_data: *const u8,
    wld_len: i32,
    texture_cache: *mut TextureCache,
    rgm_data: *const u8,
    rgm_len: i32,
    model_names: *const *const u8,
    model_datas: *const *const u8,
    model_lens: *const i32,
    model_types: *const u8,
    model_count: i32,
) -> *mut ByteBuffer {
    let result = (|| -> crate::Result<Vec<u8>> {
        let wld_bytes = unsafe { read_bytes(wld_data, wld_len, "wld_data") }?;
        let rgm_context = if rgm_data.is_null() {
            None
        } else {
            let rgm_bytes = unsafe { read_bytes(rgm_data, rgm_len, "rgm_data") }?;
            let registry = unsafe {
                build_registry_from_raw(
                    model_names,
                    model_datas,
                    model_lens,
                    model_types,
                    model_count,
                )
            }?;
            Some((rgm_bytes, registry))
        };
        let texture_cache = unsafe { texture_cache.as_mut() };
        run_on_large_stack(move || {
            let wld_file = wld::parse_wld_file(wld_bytes)?;

            let positioned_models = match rgm_context {
                Some((rgm_bytes, ref registry)) => {
                    let (_, models, _) = rgm::parse_rgm_with_models(rgm_bytes, registry)?;
                    models
                }
                None => Vec::new(),
            };

            let texbsi_id = wld_texbsi_id(&wld_file);
            let palette = extract_palette(&texture_cache);
            let (root, buffer) = convert_wld_scene_to_gltf(
                &wld_file,
                texbsi_id,
                &positioned_models,
                palette.as_ref(),
                texture_cache,
                false,
            )?;

            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}
