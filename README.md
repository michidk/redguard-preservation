# redguard-preservation

Preserving *The Elder Scrolls Adventures: Redguard* (1998) — reverse-engineered file format specifications, engine documentation, and a Rust CLI for parsing and converting game assets.

[![Docs](https://img.shields.io/badge/docs-michidk.github.io-2EA44F?style=flat-square&logo=mdbook&logoColor=white)](https://michidk.github.io/redguard-preservation/)
[![CI](https://img.shields.io/github/actions/workflow/status/michidk/redguard-preservation/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/michidk/redguard-preservation/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/redguard-preservation?style=flat-square)](https://crates.io/crates/redguard-preservation)
[![License: MIT](https://img.shields.io/crates/l/redguard-preservation?style=flat-square)](Cargo.toml)

> **📖 Full documentation** — file format specifications, engine internals, INI references, and more — is published at **<https://michidk.github.io/redguard-preservation/>**. This README is the CLI / library / FFI quick reference.

## Conversion Matrix

| Input Type | Input Extension(s) | Output File(s) |
|---|---|---|
| Model | `.3d`, `.3dc` | `.glb` |
| ROB archive | `.rob` | `.glb` |
| RGM scene | `.rgm` | directory: `.glb` + scene/actors/navigation `.json` + per-actor `.soup`/`.json` scripts |
| WLD world | `.wld` | `.glb` + actor metadata `.json` (scene mode), or map `.png` set |
| Font | `.fnt` | bitmap `.png` + BMFont `.fnt` + glyph metadata `.json`, or `.ttf` |
| Visibility octree | `.pvo` | `.json` |
| Palette | `.col` | swatch `.png` + palette metadata `.json` |
| Sound effects bank | `.sfx` | extracted `.wav` files + `index.json` metadata (directory output) |
| Dialogue audio | `.rtx` | extracted `.wav` files + `index.json` metadata (directory output) |
| Texture bank | `TEXBSI.###` | animated `.gif` + static `.png` files + `index.json` (directory output) |
| GXA bitmap archive | `.gxa` | animated `.gif` + `index.json` (directory output) |
| Cheat states | `.cht` | `.json` |

The `scan` command recursively detects known Redguard files in a directory tree.

## Convert Subcommands

Each format has its own subcommand with scoped flags. `convert <FILE>` auto-detects the format and uses defaults.

| Subcommand | Input | Output | Format-specific flags |
|---|---|---|---|
| `convert texbsi` | `TEXBSI.###` | `.gif` / `.png` | `--format gif\|png\|frames`, `--palette`, `--compress-textures` |
| `convert gxa` | `.gxa` | `.gif` / `.png` | `--format gif\|png`, `--compress-textures` |
| `convert fnt` | `.fnt` | `.png` + `.fnt` + `.json`, or `.ttf` | `--format bitmap\|ttf`, `--compress-textures` |
| `convert col` | `.col` | `.png` + `.json` | `--format png\|json` (default: both), `--compress-textures` |
| `convert wld` | `.wld` | `.glb` + `.json`, or map `.png` set | `--assets`, `--palette`, `--terrain-only`, `--terrain-textures`, `--compress-textures` |
| `convert model` | `.3d`, `.3dc`, `.rob` | `.glb` | `--assets`, `--palette`, `--compress-textures` |
| `convert rgm` | `.rgm` | output directory | `--assets`, `--palette`, `--compress-textures` |
| `convert rtx` | `.rtx` | `.wav` + `index.json` | `--resolve-names` |
| `convert sfx` | `.sfx` | `.wav` + `index.json` | (none) |
| `convert cht` | `.cht` | `.json` | (none) |
| `convert pvo` | `.pvo` | `.json` | (none) |

¹ Exported TTF fonts are bitmap-traced vector outlines without hinting. They can be installed on Windows, used in game engines (Unity, Godot, etc.), and previewed at [fontdrop.info](https://fontdrop.info).

## Documentation

The full documentation is published at **<https://michidk.github.io/redguard-preservation/>** (rendered with `mdBook`). The Markdown sources live under [`docs/`](docs/README.md) and are organised into three sections:

- [**File Formats**](https://michidk.github.io/redguard-preservation/formats/) ([source](docs/formats/README.md)) — binary format specifications for models, textures, audio, maps, palettes, and other game assets
- [**Engine Details**](https://michidk.github.io/redguard-preservation/engine/) ([source](docs/engine/README.md)) — reverse-engineered engine internals: cheat system, item attachment, SOUP scripting, sky renderer, water waves
- [**Configuration**](https://michidk.github.io/redguard-preservation/config/) ([source](docs/config/README.md)) — text-based INI files shipped with the game (surface types, combat, keys, etc.)

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

- `read` (`r`) — parse an input file and print decoded structure
- `convert` (`c`) — export supported inputs to output formats
  - `convert <FILE>` — auto-detect format, use defaults
  - `convert <FORMAT> <FILE>` — format-specific subcommand with scoped flags
- `scan` (`s`) — recursively scan a directory for known Redguard files

## Asset Directories

Redguard shipped with two parallel sets of 3D assets for different renderers:

| Directory | Renderer | Model versions | Palette files | Included in GOG |
|---|---|---|---|---|
| `3dart/` | Software | v2.6, v2.7 | `art_pal.col` only | Original CD only |
| `fxart/` | Glide (3dfx) | v4.0, v5.0 | 27 `.COL` palettes (per-scene) | ✓ |

Both directories are supported. The `fxart/` Glide assets are recommended — they are included in the GOG release and use the newer model format. Each scene uses its own palette (e.g. `ISLAND.COL` for the overworld, `NECRO.COL` for the Necropolis). Pass the matching palette via `--palette` — without it, 3D exports will have no textures.

## Usage Examples

Auto-detect format (uses defaults):

```bash
rgpre convert 3dart/LHBM4.3DC -o output/LHBM4.glb
rgpre convert fxart/TEXBSI.302 -o output/TEXBSI_302/
```

Convert model with palette and asset root:

```bash
rgpre convert model fxart/BELLTOWR.ROB --palette fxart/ISLAND.COL --assets . -o output/BELLTOWR.glb
```

Convert RGM scene with palette and asset root:

```bash
rgpre convert rgm maps/BELLTOWR.RGM --palette fxart/ISLAND.COL --assets . -o output/BELLTOWR_scene.glb
```

Convert WLD world to GLB terrain + companion RGM placement:

```bash
rgpre convert wld maps/ISLAND.WLD --palette fxart/ISLAND.COL --assets . -o output/ISLAND_world.glb
```

Convert WLD terrain only:

```bash
rgpre convert wld maps/ISLAND.WLD --palette fxart/ISLAND.COL --assets . --terrain-only -o output/ISLAND_terrain.glb
```

Convert TEXBSI texture bank with palette:

```bash
rgpre convert texbsi fxart/TEXBSI.302 --palette fxart/ISLAND.COL -o output/TEXBSI_302/
```

Export all animation frames as separate PNGs:

```bash
rgpre convert texbsi fxart/TEXBSI.302 --palette fxart/ISLAND.COL --format frames -o output/TEXBSI_302/
```

Convert font to TrueType:

```bash
rgpre convert fnt input/FONT01.FNT --format ttf -o output/FONT01.ttf
```

Export COL palette as JSON only:

```bash
rgpre convert col fxart/ISLAND.COL --format json -o output/ISLAND.json
```

Extract RTX dialogue with resolved filenames:

```bash
rgpre convert rtx input/DIALOG.RTX --resolve-names -o output/dialog/
```

Scan a directory:

```bash
rgpre scan 3dart
```

## Native Plugin (FFI)

The library builds as a C-compatible shared library (`cdylib`) alongside the CLI binary. This lets game engines like Unity load it as a native plugin and call the conversion functions directly at runtime.

See [`src/ffi/README.md`](src/ffi/README.md) for the full API reference — architecture, C struct definitions, function signatures, memory management, and error handling.
