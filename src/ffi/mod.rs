//! C-compatible FFI layer for native plugin use (Unity, etc.).
//!
//! See [`README.md`](README.md) for the full API reference — architecture,
//! C struct definitions, function signatures, memory management, and error
//! handling.

mod buffer;
pub mod scene;
pub mod types;
pub mod world;

use self::buffer::{into_ffi_result, last_error_message, run_on_large_stack};
use crate::gltf::{
    TextureCache, convert_models_to_gltf, convert_positioned_models_to_gltf,
    convert_wld_scene_to_gltf, to_glb,
};
use crate::import::{FileType, palette::Palette, registry, rgm, rob, wld, world_ini::WorldIni};
use crate::model3d;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;

pub use self::buffer::ByteBuffer;

pub(crate) fn i32_to_usize(value: i32, name: &str) -> crate::Result<usize> {
    usize::try_from(value)
        .map_err(|_| crate::error::Error::Parse(format!("{name} must be >= 0, got {value}")))
}

pub(crate) unsafe fn read_c_str(ptr: *const c_char, name: &str) -> crate::Result<String> {
    if ptr.is_null() {
        return Err(crate::error::Error::Parse(format!("{name} is null")));
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str()
        .map(|s| s.to_owned())
        .map_err(|e| crate::error::Error::Parse(format!("{name} is not valid UTF-8: {e}")))
}

const WORLD_INI_NAMES: [&str; 2] = ["WORLD.INI", "world.ini"];

pub(crate) fn find_world_ini(asset_root: &Path) -> Option<PathBuf> {
    for name in &WORLD_INI_NAMES {
        let path = asset_root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn find_palette_on_disk(asset_root: &Path, ini_palette_path: &str) -> Option<PathBuf> {
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

pub(crate) fn auto_resolve_palette(
    asset_root: &Path,
    input_file: &Path,
    file_type: FileType,
) -> Option<Palette> {
    let ini_path = find_world_ini(asset_root)?;
    let content = std::fs::read_to_string(ini_path).ok()?;
    let world_ini = WorldIni::parse(&content);

    let file_stem = input_file.file_stem()?.to_str().unwrap_or("");
    let matches = match file_type {
        FileType::Rgm => world_ini.find_by_map_stem(file_stem),
        FileType::Wld => world_ini.find_by_world_stem(file_stem),
        _ => Vec::new(),
    };
    let entry = matches.first().copied()?;
    let palette_path = find_palette_on_disk(asset_root, &entry.palette)?;
    let bytes = std::fs::read(palette_path).ok()?;
    Palette::parse(&bytes).ok()
}

pub(crate) fn build_path_texture_cache(
    asset_root: &Path,
    file_path: &Path,
    file_type: FileType,
) -> (Option<Palette>, TextureCache) {
    let palette = auto_resolve_palette(asset_root, file_path, file_type);
    let cache = TextureCache::new(
        asset_root.to_path_buf(),
        palette.as_ref().map(|pal| Palette { colors: pal.colors }),
    );
    (palette, cache)
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
                convert_models_to_gltf(&models, palette.as_ref(), Some(&mut texture_cache), false)?;
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
                Some(&mut texture_cache),
                false,
            )?;
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
                Some(&mut texture_cache),
                false,
            )?;
            to_glb(&root, &buffer)
        })
    })();

    into_ffi_result(result)
}

use self::buffer::{clear_last_error, set_last_error};
use self::scene::{scan_rgm_sections, serialize_rgm_placements, serialize_terrain_primitives};
use self::types::RgWorldDescriptor;
use self::world::WorldHandle;
use crate::gltf::build_wld_unrolled_primitives;

/// # Safety
///
/// `assets_dir` must be a valid null-terminated UTF-8 string.
/// The returned handle must be freed with `rg_close_world`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_open_world(
    assets_dir: *const c_char,
    world_id: i32,
) -> *mut WorldHandle {
    let result = (|| -> crate::Result<WorldHandle> {
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let world_id = u32::try_from(world_id).map_err(|_| {
            crate::error::Error::Parse(format!("world_id must be >= 0, got {world_id}"))
        })?;
        WorldHandle::open(PathBuf::from(assets_dir), world_id)
    })();

    match result {
        Ok(handle) => {
            clear_last_error();
            Box::into_raw(Box::new(handle))
        }
        Err(err) => {
            set_last_error(err);
            ptr::null_mut()
        }
    }
}

/// # Safety
///
/// `assets_dir`, `rgm_path`, and `palette_path` must be valid null-terminated UTF-8 strings.
/// `wld_path` may be null.
/// The returned handle must be freed with `rg_close_world`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_open_world_explicit(
    assets_dir: *const c_char,
    rgm_path: *const c_char,
    wld_path: *const c_char,
    palette_path: *const c_char,
) -> *mut WorldHandle {
    let result = (|| -> crate::Result<WorldHandle> {
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let rgm_path = unsafe { read_c_str(rgm_path, "rgm_path") }?;
        let wld_path = if wld_path.is_null() {
            None
        } else {
            let value = unsafe { read_c_str(wld_path, "wld_path") }?;
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        };
        let palette_path = unsafe { read_c_str(palette_path, "palette_path") }?;

        WorldHandle::open_explicit(PathBuf::from(assets_dir), rgm_path, wld_path, palette_path)
    })();

    match result {
        Ok(handle) => {
            clear_last_error();
            Box::into_raw(Box::new(handle))
        }
        Err(err) => {
            set_last_error(err);
            ptr::null_mut()
        }
    }
}

/// # Safety
///
/// `world` must be either null or a pointer previously returned by `rg_open_world`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_close_world(world: *mut WorldHandle) {
    if world.is_null() {
        return;
    }
    let _ = unsafe { Box::from_raw(world) };
}

/// Returns the number of worlds defined in WORLD.INI.
///
/// # Safety
///
/// `assets_dir` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_world_count(assets_dir: *const c_char) -> i32 {
    let result = (|| -> crate::Result<i32> {
        let assets_dir = unsafe { read_c_str(assets_dir, "assets_dir") }?;
        let assets_dir = PathBuf::from(assets_dir);
        let ini_path = find_world_ini(&assets_dir).ok_or_else(|| {
            crate::error::Error::Parse(format!("WORLD.INI not found in: {}", assets_dir.display()))
        })?;
        let content = std::fs::read_to_string(ini_path)?;
        let world_ini = WorldIni::parse(&content);
        i32::try_from(world_ini.entries.len()).map_err(|_| {
            crate::error::Error::Parse(format!(
                "world count exceeds i32::MAX: {}",
                world_ini.entries.len()
            ))
        })
    })();

    match result {
        Ok(count) => {
            clear_last_error();
            count
        }
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

fn fixed_path_string<const N: usize>(s: &str) -> [u8; N] {
    let mut buf = [0u8; N];
    let bytes = s.as_bytes();
    let len = bytes.len().min(N - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_world_descriptor(world: *mut WorldHandle) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result: crate::Result<Vec<u8>> = {
        let handle = unsafe { &mut *world };

        let wld_bytes = handle.wld_bytes().ok();
        let texbsi_id = wld_bytes
            .and_then(|bytes| wld::parse_wld_file(bytes).ok())
            .map(|wld_file| {
                u16::from_le_bytes([
                    wld_file.sections[0].header[6],
                    wld_file.sections[0].header[7],
                ])
            })
            .unwrap_or(0);

        let descriptor = RgWorldDescriptor {
            world_id: i32::try_from(handle.world_id()).unwrap_or(-1),
            has_wld: if handle.wld_path_raw().is_some() {
                1
            } else {
                0
            },
            _pad: [0; 3],
            texbsi_id,
            _pad2: [0; 2],
            rgm_path: fixed_path_string::<64>(handle.rgm_path_raw()),
            wld_path: fixed_path_string::<64>(handle.wld_path_raw().unwrap_or("")),
            palette_path: fixed_path_string::<64>(handle.palette_path_raw()),
        };

        Ok(bytemuck::bytes_of(&descriptor).to_vec())
    };

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_world_terrain(world: *mut WorldHandle) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result = (|| -> crate::Result<Vec<u8>> {
        let handle = unsafe { &mut *world };
        let wld_bytes = handle.wld_bytes()?;
        let wld_file = wld::parse_wld_file(wld_bytes)?;
        let texbsi_id = u16::from_le_bytes([
            wld_file.sections[0].header[6],
            wld_file.sections[0].header[7],
        ]);

        run_on_large_stack(move || {
            let primitives = build_wld_unrolled_primitives(&wld_file, texbsi_id)?;
            serialize_terrain_primitives(primitives)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_world_placements(world: *mut WorldHandle) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result = (|| -> crate::Result<Vec<u8>> {
        let handle = unsafe { &mut *world };
        let rgm_bytes = handle.rgm_bytes()?;

        run_on_large_stack(move || {
            let (placements, lights) = rgm::extract_rgm_placements(rgm_bytes)?;
            serialize_rgm_placements(&placements, &lights)
        })
    })();

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture_world(
    world: *mut WorldHandle,
    texture_id: u16,
    image_id: u8,
) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result: crate::Result<Vec<u8>> = {
        let handle = unsafe { &mut *world };
        let cache = handle.texture_cache_mut();

        run_on_large_stack(move || {
            let (rgba, width, height, frame_count) = cache
                .get_image_rgba_with_frame_count(texture_id, image_id)
                .ok_or_else(|| {
                    crate::error::Error::Parse(format!(
                        "texture not found: TEXBSI.{texture_id:03} image {image_id}"
                    ))
                })?;

            let header = types::TextureHeader {
                width: i32::from(width),
                height: i32::from(height),
                frame_count: i32::from(frame_count),
                rgba_size: i32::try_from(rgba.len()).map_err(|_| {
                    crate::error::Error::Parse(format!(
                        "rgba_size exceeds i32::MAX: {}",
                        rgba.len()
                    ))
                })?,
            };

            let mut out =
                Vec::with_capacity(std::mem::size_of::<types::TextureHeader>() + rgba.len());
            out.extend_from_slice(bytemuck::bytes_of(&header));
            out.extend_from_slice(&rgba);
            Ok(out)
        })
    };

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_decode_texture_all_frames_world(
    world: *mut WorldHandle,
    texture_id: u16,
    image_id: u8,
) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result: crate::Result<Vec<u8>> = {
        let handle = unsafe { &mut *world };
        let cache = handle.texture_cache_mut();

        run_on_large_stack(move || {
            let info = cache
                .get_all_frames_by_image_id(texture_id, image_id)
                .ok_or_else(|| {
                    crate::error::Error::Parse(format!(
                        "texture not found: TEXBSI.{texture_id:03} image {image_id}"
                    ))
                })?;

            let header = types::AllFramesHeader {
                width: i32::from(info.width),
                height: i32::from(info.height),
                frame_count: i32::from(info.frame_count),
            };

            let mut out = Vec::new();
            out.extend_from_slice(bytemuck::bytes_of(&header));
            for frame in &info.frames {
                match frame {
                    Some(rgba) => {
                        let size = i32::try_from(rgba.len()).map_err(|_| {
                            crate::error::Error::Parse(format!(
                                "rgba_size exceeds i32::MAX: {}",
                                rgba.len()
                            ))
                        })?;
                        out.extend_from_slice(&size.to_le_bytes());
                        out.extend_from_slice(rgba);
                    }
                    None => {
                        out.extend_from_slice(&0_i32.to_le_bytes());
                    }
                }
            }
            Ok(out)
        })
    };

    into_ffi_result(result)
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// `section_tag` must be a valid 4-character null-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_rgm_section_count_world(
    world: *mut WorldHandle,
    section_tag: *const c_char,
) -> i32 {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return -1;
    }

    let result = (|| -> crate::Result<i32> {
        let handle = unsafe { &mut *world };
        let tag = read_section_tag(section_tag)?;
        let rgm_bytes = handle.rgm_bytes()?;
        let count = scan_rgm_sections(rgm_bytes, &tag).len();
        i32::try_from(count).map_err(|_| {
            crate::error::Error::Parse(format!("section count exceeds i32::MAX: {count}"))
        })
    })();

    match result {
        Ok(count) => {
            clear_last_error();
            count
        }
        Err(err) => {
            set_last_error(err);
            -1
        }
    }
}

fn read_section_tag(tag_ptr: *const c_char) -> crate::Result<[u8; 4]> {
    let tag_str = unsafe { read_c_str(tag_ptr, "section_tag") }?;
    let bytes = tag_str.as_bytes();
    if bytes.len() != 4 {
        return Err(crate::error::Error::Parse(format!(
            "section_tag must be exactly 4 bytes, got {}",
            bytes.len()
        )));
    }
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// # Safety
///
/// `world` must be a valid pointer returned by `rg_open_world`.
/// `section_tag` must be a valid 4-character null-terminated string.
/// The returned buffer must be freed with `rg_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_get_rgm_section_world(
    world: *mut WorldHandle,
    section_tag: *const c_char,
    section_index: i32,
) -> *mut ByteBuffer {
    if world.is_null() {
        set_last_error(crate::error::Error::Parse("world handle is null".into()));
        return ptr::null_mut();
    }

    let result = (|| -> crate::Result<Vec<u8>> {
        let handle = unsafe { &mut *world };
        let tag = read_section_tag(section_tag)?;
        let idx = i32_to_usize(section_index, "section_index")?;
        let rgm_bytes = handle.rgm_bytes()?;
        let sections = scan_rgm_sections(rgm_bytes, &tag);
        let payload = sections.get(idx).ok_or_else(|| {
            crate::error::Error::Parse(format!(
                "section '{}' index {} out of range (found {})",
                String::from_utf8_lossy(&tag),
                idx,
                sections.len()
            ))
        })?;
        Ok(payload.to_vec())
    })();

    into_ffi_result(result)
}
