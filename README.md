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
  |  File.ReadAllBytes()
  v
byte[] --P/Invoke--> Rust native plugin --> ByteBuffer*
                      (parsing + conversion)     |
  <--- copy bytes <------------------------------+
  |
  v
  rg_free_buffer(buf)
```

The caller provides raw file bytes. Rust parses and converts them in memory. Results are returned as `ByteBuffer` pointers that the caller must free with `rg_free_buffer`.

### Memory Management

```c
// All buffers returned by rg_* functions must be freed by the caller:
void rg_free_buffer(ByteBuffer* buffer);

// On error, functions return NULL. Retrieve the error message with:
ByteBuffer* rg_last_error(void);  // NULL if no error; caller must free
```

### Texture Cache (Reusable Handle)

For textured conversions, create a texture cache once and reuse it:

```c
// Create from palette (.COL) + TEXBSI files
TextureCache* rg_texture_cache_create(
    const uint8_t* palette_data, int32_t palette_len,      // nullable
    const uint16_t* texbsi_ids, const uint8_t** texbsi_datas,
    const int32_t* texbsi_lens, int32_t texbsi_count
);
void rg_texture_cache_free(TextureCache* cache);
```

### GLB Export Functions

Produce standard glTF Binary files from game assets:

| Function | Input | Output |
|----------|-------|--------|
| `rg_convert_model_to_glb` | 3D/3DC bytes + texture cache | GLB bytes |
| `rg_convert_rob_to_glb` | ROB bytes + texture cache | GLB bytes |
| `rg_convert_rgm_to_glb` | RGM bytes + texture cache + model files | GLB bytes |
| `rg_convert_wld_to_glb` | WLD bytes + texture cache + optional RGM + models | GLB bytes |
| `rg_get_rgm_metadata` | RGM bytes | JSON bytes (scripts/animations + MPOB runtime object records) |

### Scene Data Functions

Return pre-transformed mesh data for direct engine consumption (RGMD binary format). Vertices are scaled and flipped to match the GLTF coordinate convention (`-x/20, -y/20, z/20`), faces are fan-triangulated, and geometry is grouped by submesh/material:

| Function | Input | Output |
|----------|-------|--------|
| `rg_parse_model_data` | 3D/3DC bytes + texture cache | RGMD binary (triangulated submeshes) |
| `rg_parse_rob_data` | ROB bytes + texture cache | Segment names + RGMD per segment |
| `rg_parse_wld_terrain_data` | WLD bytes | RGMD binary (terrain mesh) |
| `rg_parse_rgm_placements` | RGM bytes | RGPL binary (transforms, names, lights) |

### Texture Functions

| Function | Input | Output |
|----------|-------|--------|
| `rg_decode_texture` | texture cache + texture ID + image ID | RGBA pixels (width, height, frame_count, pixel data) |
| `rg_decode_texture_all_frames` | texture cache + texture ID + image ID | RGBA pixels for all animation frames |
| `rg_texbsi_image_count` | texture cache + texture ID | Image count (`i32`, -1 on error) |
| `rg_decode_gxa` | GXA bytes + frame index | RGBA pixels (width, height, frame_count, pixel data) |

`image_id` is the TEXBSI image identifier from model/placement data, not an array index into TEXBSI entries.

### Audio Functions

| Function | Input | Output |
|----------|-------|--------|
| `rg_convert_sfx_to_wav` | SFX bytes + effect index | WAV bytes |
| `rg_sfx_effect_count` | SFX bytes | Effect count (`i32`, -1 on error) |
| `rg_convert_rtx_entry_to_wav` | RTX bytes + entry index | WAV bytes (audio entries only) |
| `rg_rtx_entry_count` | RTX bytes | Entry count (`i32`, -1 on error) |
| `rg_rtx_metadata` | RTX bytes | JSON bytes (entry index with tags, types, durations) |

### Dependency Discovery Functions

Query which external assets a file references before loading them for conversion:

| Function | Input | Output |
|----------|-------|--------|
| `rg_rgm_dependencies` | RGM bytes | JSON bytes (`{"model_names": [...], "texbsi_ids": [...]}`) |
| `rg_model_dependencies` | 3D/3DC bytes | JSON bytes (`{"texbsi_ids": [...]}`) |
| `rg_rob_dependencies` | ROB bytes | JSON bytes (`{"texbsi_ids": [...]}`) |
| `rg_wld_dependencies` | WLD bytes | JSON bytes (`{"texbsi_ids": [...]}`) |

Typical two-phase workflow for RGM/WLD conversion:

1. `rg_rgm_dependencies` → learn which model files and TEXBSI banks are needed
2. `rg_model_dependencies` / `rg_rob_dependencies` on each loaded model → discover additional TEXBSI IDs from face data
3. Load all required TEXBSI + palette files, create texture cache, call conversion

For WLD with a companion RGM, call `rg_wld_dependencies` and `rg_rgm_dependencies` separately and union the TEXBSI IDs.

### Data / Config Functions

| Function | Input | Output |
|----------|-------|--------|
| `rg_parse_palette` | COL bytes | JSON bytes (`{"colors": [[r,g,b], ...]}`) |
| `rg_convert_pvo_to_json` | PVO bytes | JSON bytes (octree nodes, leaves, polygon indices) |
| `rg_convert_cht_to_json` | CHT bytes | JSON bytes (cheat name → value map) |
| `rg_convert_fnt_to_ttf` | FNT bytes | TTF font bytes |
| `rg_parse_ini` | INI bytes (WORLD.INI) | JSON bytes (`{"worlds": [...]}`) |

## Development Checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
