# Native Plugin (FFI)

The library builds as a C-compatible shared library (`cdylib`) alongside the CLI binary. This lets game engines like Unity load it as a native plugin and call the conversion functions directly at runtime.

**Build:** `cargo build --release`

**Output:** `target/release/librgpre.so` (Linux), `target/release/rgpre.dll` (Windows), `target/release/librgpre.dylib` (macOS)

**C header:** [`rgpre.h`](rgpre.h) — all struct definitions and function signatures.

## Architecture

```
Unity C# (or any FFI consumer)
  |
  |  file path + scalar parameters
  v
char*  --P/Invoke--> Rust native plugin --> ByteBuffer*
                      (file I/O + parsing)      |
  <--- copy bytes <-----------------------------+
  |
  v
  rg_free_buffer(buf)
```

All asset inputs are file paths (`const char*`) plus scalar arguments (`i32`, `u16`, `u8`), or an opaque `RgWorldHandle*` returned by `rg_open_world`. No asset byte arrays cross the FFI boundary. Results are returned as `ByteBuffer*` pointers that the caller must free with `rg_free_buffer`. On error, buffer-returning functions return `NULL`, count functions return `-1`, and the message is available via `rg_last_error()`.

All structured output buffers use `#[repr(C)]` structs (defined in [`types.rs`](types.rs)) that can be directly cast via `Marshal.PtrToStructure<T>()` or `MemoryMarshal.Cast<byte, T>()` on the C# side. No manual byte parsing is needed — both sides share the same memory layout.

## Thread Safety

All functions are safe to call from any thread. Multiple threads may call FFI functions concurrently.

`rg_last_error()` is thread-local — each thread has its own error slot, so concurrent calls won't clobber each other's error state.

Each `RgWorldHandle` owns its resolved palette, world metadata, and `TextureCache` instance. Parallel calls against different world handles do not share texture decode state.

## Conventions

**Coordinate system:** Right-handed, Y-up. Matches glTF. Positions are in scaled units (original engine units ÷ 20). The engine's Y axis is negated during conversion; triangle winding is reversed to preserve CCW front faces.

**Winding order:** Counter-clockwise (CCW) front faces. Standard OpenGL/glTF convention.

**UV origin:** Top-left. V increases downward. Matches glTF/DirectX convention. Model/ROB UVs are derived from raw fixed-point deltas divided by `16 × texture_dimension`, with V flipped (`1 − v`) to convert from the engine's bottom-left origin. Terrain UVs are hardcoded per-tile 0–1 coordinates with rotation variants.

**Transform matrices:** `float[16]` in column-major order (translation in elements 12–14). Matches glTF/OpenGL convention.

**String fields:** `model_name[32]`, `source_id[32]`, `segment_name[8]`, `name[32]` are null-terminated ASCII, zero-padded after the terminator.

## Memory and Errors

Every buffer-returning function allocates a `ByteBuffer` (see [`rgpre.h`](rgpre.h)). The caller must free it with `rg_free_buffer`. On failure, `NULL` is returned and the error message is available via `rg_last_error()`.

## Binary Struct Types

All structured buffers use `#[repr(C)]` layouts defined in [`types.rs`](types.rs). C# equivalents use `[StructLayout(LayoutKind.Sequential)]`. Sizes include explicit padding — no hidden compiler-inserted gaps. See [`rgpre.h`](rgpre.h) for the complete C definitions.

Reading an RGMD buffer:

```
read RgmdHeader
for i in 0..submesh_count:
    read RgmdSubmeshHeader
    read vertex_count × RgmdVertex
    read index_count × uint32_t (indices)
```

Reading an RGPL buffer:

```
read RgplHeader
for i in 0..placement_count:
    read RgplPlacement
for i in 0..light_count:
    read RgplLight
```

## GLB Export

Converts 3D/3DC, ROB, RGM, and WLD files to in-memory GLB. `assets_dir` should be the game root containing `3dart/`, `fxart/`, `maps/`, and `WORLD.INI`. WLD conversion auto-discovers the companion RGM file.

## World Handle API

Use `rg_world_count` to enumerate available worlds, then `rg_open_world(assets_dir, world_id)` to create a native world context. The handle resolves the `WORLD.INI` entry, palette, scene paths, and texture cache once and keeps them together so Unity cannot accidentally mix one world's scene with another world's palette.

`rg_get_world_descriptor` returns a fixed-size `RgWorldDescriptor` struct containing the world id, whether a WLD exists, the terrain TEXBSI id when available, and the raw `WORLD.INI` paths for the RGM, WLD, and palette entries.

After opening a handle, Unity can request terrain (`rg_get_world_terrain`), placements (`rg_get_world_placements`), RGM section payloads (`rg_rgm_section_count_world`, `rg_get_rgm_section_world`), and decoded TEXBSI textures (`rg_decode_texture_world`, `rg_decode_texture_all_frames_world`) without passing palette names or re-parsing `WORLD.INI`.

Release the handle with `rg_close_world` when finished.

## Scene Data Functions

Return pre-transformed mesh data for direct engine consumption (RGMD binary format). Vertices are in right-handed Y-up coordinates (see Conventions above), faces are fan-triangulated with CCW winding, and geometry is grouped by submesh/material. `assets_dir` is used to resolve the palette for solid-color materials. Solid-color submeshes carry resolved RGB values; no separate palette lookup is needed on the engine side.

## Texture Functions

Texture decode in the FFI API is world-handle based. Open a world first, then decode TEXBSI images through that handle so the correct palette and cache are always used for that world.

`image_id` is the TEXBSI image identifier from model/placement data, not an array index into TEXBSI entries.

## Audio Functions

RTX files interleave audio and text entries. Use `rg_convert_rtx_entry_to_wav` for audio entries and `rg_get_rtx_subtitle` for subtitle text. `rg_get_rtx_subtitle` returns a UTF-8 byte buffer (no null terminator).

## Other Functions

`rg_convert_fnt_to_ttf` converts a bitmap FNT file to a TrueType font in memory.
