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

All asset inputs are file paths (`const char*`) plus scalar arguments (`i32`, `u16`, `u8`). No asset byte arrays cross the FFI boundary. Results are returned as `ByteBuffer*` pointers that the caller must free with `rg_free_buffer`. On error, buffer-returning functions return `NULL`, count functions return `-1`, and the message is available via `rg_last_error()`.

All structured output buffers use `#[repr(C)]` structs (defined in [`types.rs`](types.rs)) that can be directly cast via `Marshal.PtrToStructure<T>()` or `MemoryMarshal.Cast<byte, T>()` on the C# side. No manual byte parsing is needed — both sides share the same memory layout.

## Thread Safety

All functions are safe to call from any thread. Multiple threads may call FFI functions concurrently.

`rg_last_error()` is thread-local — each thread has its own error slot, so concurrent calls won't clobber each other's error state.

The internal texture cache is protected by a `Mutex`. The first `rg_decode_texture` call for a given `assets_dir` pays the full I/O cost (parse `WORLD.INI`, load palette, scan for `TEXBSI.*` files); subsequent calls reuse the cached state. The lock is held during cache population, so parallel first-calls to different `assets_dir` values proceed independently, but parallel first-calls to the *same* `assets_dir` will serialize.

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

## RGM Section Access

Extracts raw section bytes from RGM files for direct use by AnimStore, ScriptStore, and ScriptedObject. `section_tag` is a 4-character string (e.g. `"RAHD"`, `"RAAN"`, `"RAGR"`, `"RAST"`, `"RASB"`, `"RASC"`, `"RAVA"`, `"RAAT"`, `"RANM"`, `"RALC"`, `"RAEX"`, `"RAVC"`, `"RAHK"`). Returns the raw section payload bytes.

## Scene Data Functions

Return pre-transformed mesh data for direct engine consumption (RGMD binary format). Vertices are in right-handed Y-up coordinates (see Conventions above), faces are fan-triangulated with CCW winding, and geometry is grouped by submesh/material. `assets_dir` is used to resolve the palette for solid-color materials. Solid-color submeshes carry resolved RGB values; no separate palette lookup is needed on the engine side.

## Texture Functions

Texture functions resolve palette data from `WORLD.INI` in `assets_dir` and load texture banks on demand. The resolved `WORLD.INI`, palette, and TEXBSI directory index are cached per `assets_dir` for the lifetime of the loaded library — the first call pays the full I/O cost, subsequent calls with the same `assets_dir` reuse the cached state (including previously-parsed TEXBSI banks).

`image_id` is the TEXBSI image identifier from model/placement data, not an array index into TEXBSI entries.

## Audio Functions

RTX files interleave audio and text entries. Use `rg_convert_rtx_entry_to_wav` for audio entries and `rg_get_rtx_subtitle` for subtitle text. `rg_get_rtx_subtitle` returns a UTF-8 byte buffer (no null terminator).

## Other Functions

`rg_convert_fnt_to_ttf` converts a bitmap FNT file to a TrueType font in memory.
