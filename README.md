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
| Sound effects bank | `.sfx` | extracted `.wav` files + `index.json` metadata (directory output) |
| Dialogue audio | `.rtx` | extracted `.wav` files + `index.json` metadata (directory output) |
| Texture bank | `TEXBSI.###` | extracted `.png` or `.gif` files + `metadata.json` (directory output) |
| GXA bitmap archive | `.gxa` | extracted `.png` frames or animated `.gif` + `metadata.json` (directory output) |
| Cheat states | `.cht` | `.json` |

The `scan` command recursively detects known Redguard files in a directory tree.

## Convert Flag Matrix

Not all flags apply to every format. The CLI warns when a flag has no effect for the given file type.

| Flag | 3D/3DC/ROB | RGM | WLD | FNT | TEXBSI | GXA | COL | SFX | RTX | CHT/PVO |
|---|---|---|---|---|---|---|---|---|---|---|
| `--format png` | — | — | — | — | frame 0 only (default) | per-frame PNGs (default) | — | — | — | — |
| `--format frames` | — | — | — | — | all frames as PNGs | — | — | — | — | — |
| `--format gif` | — | — | — | — | animated GIF | animated GIF | — | — | — | — |
| `--format bitmap` | — | — | — | PNG atlas + BMFont (default) | — | — | — | — | — | — |
| `--format ttf` | — | — | — | TrueType font | — | — | — | — | — | — |
| `--terrain-only` | — | — | skip RGM placement | — | — | — | — | — | — | — |
| `--terrain-textures` | — | — | enable/disable texturing | — | — | — | — | — | — | — |
| `--compress-textures` | smaller GLB | smaller GLB | smaller GLB/PNG | smaller PNG | smaller PNGs | smaller PNGs | smaller PNG | — | — | — |
| `--resolve-names` | — | — | — | — | — | — | — | — | text-based filenames | — |
| `--palette` | face colors | face colors | terrain colors | — | decode colors | — | — | — | — | — |
| `--assets` | — | model lookup | model + texture lookup | — | — | — | — | — | — | — |

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

Convert TEXBSI texture bank to PNGs (auto-detected by `TEXBSI.*` stem):

```bash
rgpre convert fxart/TEXBSI.302 --palette fxart/ISLAND.COL -o output/TEXBSI_302/
```

Convert with all animation frames as separate PNGs:

```bash
rgpre convert fxart/TEXBSI.302 --palette fxart/ISLAND.COL --format frames -o output/TEXBSI_302/
```

Convert animated textures to GIF:

```bash
rgpre convert fxart/TEXBSI.302 --palette fxart/ISLAND.COL --format gif -o output/TEXBSI_302/
```

Scan a directory:

```bash
rgpre scan 3dart
```

## Native Plugin (FFI)

The library builds as a C-compatible shared library (`cdylib`) alongside the CLI binary. This lets game engines like Unity load it as a native plugin and call the conversion functions directly at runtime.

See [`src/ffi/README.md`](src/ffi/README.md) for the full API reference — architecture, C struct definitions, function signatures, memory management, and error handling.
