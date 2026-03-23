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

### Memory and Errors

```c
typedef struct ByteBuffer {
    uint8_t* data;
    int32_t len;
} ByteBuffer;

void rg_free_buffer(ByteBuffer* buffer);
ByteBuffer* rg_last_error(void);
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

Texture functions resolve palette data from `WORLD.INI` in `assets_dir` and load texture banks on demand.

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
