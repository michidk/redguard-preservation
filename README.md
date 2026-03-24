# redguard-preservation

Preserving *The Elder Scrolls Adventures: Redguard* (1998) — reverse-engineered file format specifications, engine documentation, and a Rust CLI for parsing and converting game assets.

## Conversion Matrix

| Input Type | Input Extension(s) | Output File(s) |
|---|---|---|
| Model | `.3d`, `.3dc` | `.glb` |
| ROB archive | `.rob` | `.glb` |
| RGM scene | `.rgm` | `.glb` + actor metadata `.json` |
| WLD world | `.wld` | `.glb` + actor metadata `.json` (scene mode), or map `.png` set |
| Font | `.fnt` | bitmap `.png` + BMFont `.fnt` + glyph metadata `.json`, or `.ttf` |
| Visibility octree | `.pvo` | `.json` |
| Palette | `.col` | swatch `.png` + palette metadata `.json` |
| Sound effects bank | `.sfx` | extracted `.wav` files (directory output) |
| Dialogue audio | `.rtx` | extracted `.wav` files + `index.json` metadata (directory output) |
| Texture bank | `TEXBSI.###` | extracted `.png` files + metadata `.json` (directory output) |
| GXA bitmap archive | `.gxa` | extracted `.png` frames + metadata `.json` (directory output) |

The `scan` command recursively detects known Redguard files in a directory tree.

## Documentation

The [`docs/`](docs/README.md) directory is organized into three sections:

- [**File Formats**](docs/formats/README.md) — binary format specifications for models, textures, audio, maps, palettes, and other game assets
- [**Engine Details**](docs/engine/README.md) — reverse-engineered engine internals: cheat system, item attachment, and SOUP scripting
- [**Configuration**](docs/config/README.md) — text-based INI files shipped with the game (surface types, combat, keys, etc.)

## Repository Layout

- `src/` - Rust source
- `src/import/` - format importers/parsers
- `src/gltf/` - GLTF/GLB conversion (builder, primitives, terrain, texture cache)
- `src/ffi/` - C-compatible FFI layer for native plugin use (Unity, etc.)
- `src/cli/` - CLI command handlers (`cli/convert/` for per-format converters)
- `src/error.rs` - shared error types
- `docs/` - format specifications and engine notes
- `tests/` - integration tests

## Quick Start

Requirements:

- Rust stable toolchain
- Cargo

Build:

```bash
cargo build
```

Show CLI help:

```bash
rgpre --help
```

## CLI Commands

- `read` (`r`) - parse an input file and print decoded structure
- `convert` (`c`) - export supported inputs to output formats
- `scan` (`s`) - recursively scan a directory for known Redguard files

## Usage Examples

Read a model:

```bash
rgpre read 3dart/LHBM4.3DC
```

Read a ROB archive:

```bash
rgpre read 3dart/BELLTOWR.ROB
```

Convert model to GLB:

```bash
rgpre convert 3dart/LHBM4.3DC -o output/LHBM4.glb
```

Convert ROB archive to GLB:

```bash
rgpre convert 3dart/BELLTOWR.ROB -o output/BELLTOWR.glb
```

Convert RGM scene (palette auto-resolved from `WORLD.INI`):

```bash
rgpre convert maps/ISLAND.RGM --assets . -o output/ISLAND_scene.glb
```

Convert WLD world to GLB terrain + companion RGM placement (also writes JSON sidecar metadata):

```bash
rgpre convert maps/ISLAND.WLD --assets . -o output/ISLAND_world.glb
```

Convert WLD terrain only:

```bash
rgpre convert maps/ISLAND.WLD --assets . --terrain-only -o output/ISLAND_terrain.glb
```

Convert TEXBSI texture bank to PNGs (requires `--filetype bsi` since extension is numeric):

```bash
rgpre convert fxart/TEXBSI.302 --filetype bsi --palette fxart/ISLAND.COL -o output/TEXBSI_302/
```

Convert with all animation frames:

```bash
rgpre convert fxart/TEXBSI.302 --filetype bsi --palette fxart/ISLAND.COL --all-frames -o output/TEXBSI_302/
```

Scan a directory:

```bash
rgpre scan 3dart
```

## Native Plugin (FFI)

The library builds as a C-compatible shared library (`cdylib`) alongside the CLI binary. This lets game engines like Unity load it as a native plugin and call the conversion functions directly at runtime.

**Build output:** `librgpre.so` (Linux), `rgpre.dll` (Windows), `librgpre.dylib` (macOS)

### Architecture

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

All structured output buffers use `#[repr(C)]` structs (defined in `src/ffi/types.rs`) that can be directly cast via `Marshal.PtrToStructure<T>()` or `MemoryMarshal.Cast<byte, T>()` on the C# side. No manual byte parsing is needed — both sides share the same memory layout.

### Memory and Errors

```c
typedef struct ByteBuffer {
    uint8_t* data;
    int32_t len;
} ByteBuffer;

void rg_free_buffer(ByteBuffer* buffer);
ByteBuffer* rg_last_error(void);
```

### Binary Struct Types

All structured buffers use `#[repr(C)]` layouts defined in `src/ffi/types.rs`. C# equivalents use `[StructLayout(LayoutKind.Sequential)]`. Sizes include explicit padding — no hidden compiler-inserted gaps.

**Texture output** (`rg_decode_texture`, `rg_decode_gxa`):

```c
typedef struct TextureHeader {  // 16 bytes
    int32_t width;
    int32_t height;
    int32_t frame_count;
    int32_t rgba_size;
    // followed by rgba_size bytes of RGBA pixel data
} TextureHeader;

typedef struct AllFramesHeader {  // 12 bytes
    int32_t width;
    int32_t height;
    int32_t frame_count;
    // followed by frame_count frames, each prefixed by int32_t rgba_size
} AllFramesHeader;
```

**RGMD** (`rg_parse_model_data`, `rg_parse_rob_data`, `rg_parse_wld_terrain_data`):

```c
typedef struct RgmdHeader {  // 28 bytes
    uint8_t  magic[4];       // "RGMD"
    uint8_t  version[4];
    int32_t  submesh_count;
    int32_t  frame_count;
    int32_t  total_vertex_count;
    int32_t  total_index_count;
    uint32_t radius;
} RgmdHeader;

typedef struct RgmdSubmeshHeader {  // 16 bytes
    uint8_t  material_type;   // 0 = solid color, 1 = textured
    uint8_t  color_index;     // palette index (solid) or 0 (textured)
    uint16_t texture_id;      // TEXBSI id (textured) or 0 (solid)
    uint8_t  image_id;        // TEXBSI image (textured) or 0 (solid)
    uint8_t  _pad[3];
    int32_t  vertex_count;
    int32_t  index_count;
} RgmdSubmeshHeader;

typedef struct RgmdVertex {  // 32 bytes
    float position[3];
    float normal[3];
    float uv[2];
} RgmdVertex;
// Followed by index_count × uint32_t indices.
```

**RGPL** (`rg_parse_rgm_placements`):

```c
typedef struct RgplHeader {  // 12 bytes
    uint8_t magic[4];        // "RGPL"
    int32_t placement_count;
    int32_t light_count;
} RgplHeader;

typedef struct RgplPlacement {  // 132 bytes
    uint8_t  model_name[32];
    uint8_t  source_id[32];
    float    transform[16];
    uint16_t texture_id;
    uint8_t  image_id;
    uint8_t  object_type;
} RgplPlacement;

typedef struct RgplLight {  // 60 bytes
    uint8_t name[32];
    float   color[3];
    float   position[3];
    float   range;
} RgplLight;
```

**ROB** (`rg_parse_rob_data`):

```c
typedef struct RobHeader {  // 4 bytes
    int32_t segment_count;
} RobHeader;

typedef struct RobSegmentHeader {  // 16 bytes
    uint8_t segment_name[8];
    uint8_t has_model;
    uint8_t _pad[3];
    int32_t model_data_size;  // 0 if no model
    // if has_model == 1, followed by model_data_size bytes of RGMD data
} RobSegmentHeader;
```

### GLB Export

```c
ByteBuffer* rg_convert_model_from_path(const char* file_path, const char* assets_dir);
ByteBuffer* rg_convert_rgm_from_path(const char* file_path, const char* assets_dir);
ByteBuffer* rg_convert_wld_from_path(const char* file_path, const char* assets_dir);
```

`assets_dir` should be the game root containing `3dart/`, `fxart/`, `maps/`, and `WORLD.INI`. WLD conversion auto-discovers the companion RGM file.

### RGM Section Access

Extract raw section bytes from RGM files for direct use by AnimStore, ScriptStore, and ScriptedObject:

```c
int32_t rg_rgm_section_count(const char* file_path, const char* section_tag);
ByteBuffer* rg_get_rgm_section(const char* file_path, const char* section_tag, int32_t section_index);
```

`section_tag` is a 4-character string (e.g. `"RAHD"`, `"RAAN"`, `"RAGR"`, `"RAST"`, `"RASB"`, `"RASC"`, `"RAVA"`, `"RAAT"`, `"RANM"`, `"RALC"`, `"RAEX"`, `"RAVC"`, `"RAHK"`). Returns the raw section payload bytes.

### Scene Data Functions

Return pre-transformed mesh data for direct engine consumption (RGMD binary format). Vertices are scaled and flipped to match the GLTF coordinate convention (`-x/20, -y/20, z/20`), faces are fan-triangulated, and geometry is grouped by submesh/material:

```c
ByteBuffer* rg_parse_model_data(const char* file_path);
ByteBuffer* rg_parse_rob_data(const char* file_path);
ByteBuffer* rg_parse_wld_terrain_data(const char* file_path);
ByteBuffer* rg_parse_rgm_placements(const char* file_path);
```

### Texture Functions

```c
ByteBuffer* rg_decode_texture(const char* assets_dir, uint16_t texture_id, uint8_t image_id);
ByteBuffer* rg_decode_texture_all_frames(const char* assets_dir, uint16_t texture_id, uint8_t image_id);
int32_t rg_texbsi_image_count(const char* assets_dir, uint16_t texture_id);
int32_t rg_gxa_frame_count(const char* file_path);
ByteBuffer* rg_decode_gxa(const char* file_path, int32_t frame);
```

Texture functions resolve palette data from `WORLD.INI` in `assets_dir` and load texture banks on demand. The resolved `WORLD.INI`, palette, and TEXBSI directory index are cached per `assets_dir` for the lifetime of the loaded library — the first call pays the full I/O cost, subsequent calls with the same `assets_dir` reuse the cached state (including previously-parsed TEXBSI banks).

`image_id` is the TEXBSI image identifier from model/placement data, not an array index into TEXBSI entries.

### Audio Functions

```c
ByteBuffer* rg_convert_sfx_to_wav(const char* file_path, int32_t effect_index);
int32_t rg_sfx_effect_count(const char* file_path);
ByteBuffer* rg_convert_rtx_entry_to_wav(const char* file_path, int32_t entry_index);
int32_t rg_rtx_entry_count(const char* file_path);
```

### Other Functions

```c
ByteBuffer* rg_convert_fnt_to_ttf(const char* file_path);
```

## Development Checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
