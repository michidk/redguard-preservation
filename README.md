# redguard-preservation

[![Crates.io](https://img.shields.io/crates/v/redguard-preservation.svg)](https://crates.io/crates/redguard-preservation)

Preserving *The Elder Scrolls Adventures: Redguard* (1998) — reverse-engineered file format specifications, engine documentation, and a Rust CLI for parsing and converting game assets.

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
| Software texture bank | `TEXTURE.###` | animated `.gif` or frame `.png` files + `index.json` |
| Software lookup table | `SHADE.###`, `HAZE.###` | grayscale index `.png` |
| Texture bank | `TEXBSI.###` | animated `.gif` + static `.png` files + `index.json` (directory output) |
| GXA bitmap archive | `.gxa` | animated `.gif` + `index.json` (directory output) |
| Cheat states | `.cht` | `.json` |
| PCX image | `.pcx` | `.png` |
| IFF packed bitmap | `.lbm` | `.png` |
| Deluxe Paint animation | `.anm` | directory: `animation.gif` + `index.json` |

The `scan` command recursively detects known Redguard files in a directory tree. The CLI and Rust parsers focus on game assets. Hardware drivers, audio utilities, controller profiles, and development reports remain documented in [Bundled support resources](docs/formats/BUNDLED-RESOURCES.md).

## Rust inspection exports

`import::wld::export_wld_metadata`, `import::model3d::export_model_metadata`, and
`import::bsi::export_bsi_metadata` expose decoded file data as JSON. Model metadata
includes geometry, accumulated UVs in sixteenths of a texel, animation poses, and
separately parsed subobjects. WLD metadata retains every map plane and includes
decoded combined height, texture-index, and rotation grids. BSI metadata includes
palettes and frame byte lengths; pixel buffers remain available through the image API.

`gltf::model_position`, `gltf::model_normal`, and `gltf::wld_grid_position` use the
same coordinates as GLB exports. `PvoFile::visibility_ids` resolves a PLST leaf
reference to placed-object IDs: IDs below the world's MPSO count select that record,
while higher IDs require the runtime secondary object table.

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
| `convert pcx` | `.pcx` | `.png` | (none) |
| `convert lbm` | `.lbm` | `.png` | (none) |
| `convert anm` | `.anm` | `animation.gif` + `index.json` | (none) |

`rgpre convert system/SKY_61.PCX` and `rgpre convert system/gui.lbm` export PNG images. `rgpre convert system/gui.anm` writes `system/gui_anm/animation.gif` and timing metadata. The Rust entry points are `rgpre::import::pcx::parse_pcx_file`, `rgpre::import::lbm::parse_lbm_file`, and `rgpre::import::anm::parse_anm_file`.

¹ Exported TTF fonts are bitmap-traced vector outlines without hinting. They can be installed on Windows, used in game engines (Unity, Godot, etc.), and previewed at [fontdrop.info](https://fontdrop.info).

## Save-game loading

The read-only Rust API parses version `1.31` save containers and loads a slot directory:

```rust
use rgpre::import::savegame::{load_save_slot, parse_savegame_file};

let game = parse_savegame_file(&std::fs::read("SAVEGAME.SAV")?)?;
let slot = load_save_slot("SAVEGAME/SAVEGAME.002")?;
println!("{}: world {}", game.metadata.description, game.metadata.world_id);
```

`SaveGame` exposes decoded metadata and every chunk's original bytes in file
order. `SaveSlot` contains the main save, a map of uppercase `.TSG` filenames to
snapshots, and optional `LOGBOOK.TXT` bytes. Resource filename matching ignores
case. Missing main saves, malformed containers, duplicate resource names, and
non-regular resource files return errors with file context.

The loader only reads files. It does not follow the stored bitmap path, write
saves, convert files, or restore actors and scripts into a running engine.
State payloads remain opaque. Display strings use UTF-8 replacement characters
for invalid bytes; the original bytes remain in `SVGM`. See
[Save-game containers](docs/formats/SAVEGAME.md).

## Typed configuration parsers

The Rust library parses the gameplay configuration files into named structs:

| File | Entry point | Contents |
|---|---|---|
| `SYSTEM.INI` | `rgpre::import::system_ini::SystemIni::parse` | Display, engine, debug, gameplay, player, object-manager, renderer, camera, dialogue, and Glide settings. |
| `COMBAT.INI` | `rgpre::import::combat_ini::CombatIni::parse` | Global combat settings and attacks, defenses, and taunts indexed by section ID. |
| `SURFACE.INI` | `rgpre::import::surface_ini::SurfaceIni::parse` | Texture surface classes and animation sound remaps; checked lookup and identity defaults. |
| `ITEM.INI` | `rgpre::import::item_ini::ItemIni::parse` | Inventory settings and item definitions indexed by item ID. |

```rust
use rgpre::import::{combat_ini::CombatIni, item_ini::ItemIni, system_ini::SystemIni};

let system = SystemIni::parse(&std::fs::read_to_string("SYSTEM.INI")?)?;
let combat = CombatIni::parse(&std::fs::read_to_string("COMBAT.INI")?)?;
let items = ItemIni::parse(&std::fs::read_to_string("ITEM.INI")?)?;
```

These APIs require `[system]`, `[misc]`, and `[items]`, respectively. Other
sections and individual fields can be absent. Missing or empty fields are
`None`; explicit zero remains `Some(0)`. Signed engine units, floating-point
camera factors, paths, RTX labels, and sparse IDs retain their values. Documented
type codes have named enum variants, and undocumented codes use `Unknown(i32)`.

Section and key matching ignores case. The first duplicate key wins. Semicolon
comments, LF or CRLF line endings, and an initial UTF-8 BOM are accepted. Unknown
fields are ignored. Invalid INI records, malformed recognized values, overflowing
integers, non-finite floats, and invalid indexed identifiers return `Error::Parse`.
Parsing retains stored values without applying engine defaults or runtime clamps.

The narrower `rgpre::import::player_ini::PlayerConfig` API remains available for
ordinary grounded movement. It requires its six player settings and rejects
duplicate relevant keys.

## Documentation

The full documentation is published at **<https://michidk.github.io/redguard-preservation/>** (rendered with `mdBook`). The Markdown sources live under [`docs/`](docs/README.md) and are organised into three sections:

- [**File Formats**](https://michidk.github.io/redguard-preservation/formats/) ([source](docs/formats/README.md)) — binary format specifications for models, textures, audio, maps, palettes, and other game assets
- [**Engine Details**](https://michidk.github.io/redguard-preservation/engine/) ([source](docs/engine/README.md)) — reverse-engineered engine internals: cheat system, item attachment, SOUP scripting, sky renderer, water waves
- [**Configuration**](https://michidk.github.io/redguard-preservation/config/) ([source](docs/config/README.md)) — text-based INI files shipped with the game (surface types, combat, keys, etc.)

## Related projects

[Cyrus](https://github.com/michidk/cyrus) is a modern, faithful Rust
reimplementation of Redguard built on this project's research and the
`redguard-preservation` crate. It ships an asset viewer that opens every file
format documented here: models, worlds, textures, fonts, animations, scripts,
and audio. Use it [in the browser](https://michidk.github.io/cyrus/viewer/) or
download a native build from the
[latest Cyrus release](https://github.com/michidk/cyrus/releases/latest).

[![Cyrus asset viewer showing a textured character model](https://raw.githubusercontent.com/michidk/cyrus/main/dev/pages/viewer-model.webp)](https://michidk.github.io/cyrus/viewer/)

## Acknowledgements

[RGUnity](https://github.com/RGUnity/redguard-unity) inspired
redguard-preservation and provided foundational Redguard research used by this
project. RGUnity is a human-developed Unity reimplementation built without AI
assistance. Its contributors focus on meticulous fidelity and the small details
that make the original game behave correctly.

redguard-preservation uses AI-assisted development to document broad working
coverage quickly. This complements RGUnity's detail-first work. We are grateful
to its contributors for their research, implementation, and continued care for
Redguard.

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

Install the CLI from [crates.io](https://crates.io/crates/redguard-preservation):

```bash
cargo install redguard-preservation
```

To build from source, install the stable Rust toolchain, then run:

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
| `3dart/` | Software | v2.6, v2.7 | 18 COL palettes, including `art_pal.col` | Original CD only |
| `fxart/` | Glide (3dfx) | v4.0, v5.0 | 27 populated COL palettes; one empty placeholder | ✓ |

The library decodes software and Glide model geometry and animation frames. Textured exports use software `TEXTURE.*` or Glide `TEXBSI.*` banks. Software `SHADE.*` and `HAZE.*` lookup tables have a separate parser. See [CD and GOG differences](docs/README.md#gog-vs-original-cd-differences) for resource selection.

Model exports select their texture format from the model version. For scene or terrain exports from an install containing both art trees, use `convert rgm ... --software` or `convert wld ... --software`. Rust consumers can select the edition with `TextureCache::new_for_edition`, `registry::scan_dir_for_edition`, and `WorldHandle::open_for_edition`. The equivalent C entry point is `rg_open_world_for_edition`.

```sh
rgpre convert texture 3dart/TEXTURE.302 --palette 3dart/ISLAND.COL --format frames -o output/texture_302
rgpre convert shade 3dart/SHADE.000 -o output/shade_000
rgpre convert haze 3dart/HAZE.000 -o output/haze_000
rgpre convert rgm maps/START.RGM --assets . --software -o output/start.glb
```

Texture exports include `index.json` with record metadata. Without `--palette`, image exports display indices as grayscale. Lookup exports write `indices.png`, preserving each mapping byte as a grayscale value.

To check every model frame, texture frame and lookup table in a local art directory:

```sh
cargo run --example check_edition_assets -- /path/to/Redguard/3dart
cargo run --example check_edition_assets -- /path/to/Redguard/fxart
```

The `fxart/` Glide assets are included in GOG and use the newer model format.
Each scene uses its own palette, such as `ISLAND.COL` or `NECRO.COL`. Pass the
matching palette via `--palette` for textured exports.

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

## Export metadata and configuration APIs

### Model rendering metadata

Textured GLTF materials retain their effective palette-zero RGB in
`rgpre_texture` extras with `version: 1` and `chroma_key_rgb: [r, g, b]`.
Embedded PNGs preserve those RGB bytes at alpha-zero texels. The alpha marker
supports generic viewers; engine renderers use the RGB key after filtering.

Positioned-model GLTF nodes retain the selected ROB segment attribute byte in
`rgpre_render`, with `version: 1` and `rob_attributes`. The attribute is null
for standalone model files and generated geometry. The loader retains it from
the same successful archive segment as the geometry, including fallback
selection and cached loads. Node metadata keeps instance attributes separate
when exported instances share one mesh.

MPSF nodes additionally carry `rgpre_flat` with `version: 1`, `texture_id`,
`image_id`, source-image `dimensions: [width, height]`, `texture_scale`,
submission `flags`, and integer `alpha`. Dimensions and scale are null when
texture data is unavailable. Scale includes the zero-to-one default and the
BSI fixed-point conversion. Flags include the centered-anchor base flag; alpha
includes the non-wrapping attribute increment or the default 255. See
[flat placement and attributes](docs/formats/RGM.md#mpsf-flat-objects).

The flat metadata identifies a separate engine drawing path. Its dimensions
and scale define camera-facing geometry; the generated mesh is a generic-viewer
fallback, not authored model geometry. ROB model lighting and polygon-admission
metadata do not describe flat drawing. Malformed FLAT sections produce placement
diagnostics instead of flat nodes with assumed defaults.

GLTF model primitives retain per-vertex face inputs in the `rgpre_lighting`
extras object. Version 5 has `version: 5`, the model header radius as
`radius_fixed` in 1/256 engine units, and a `vertices` array in exported
vertex order. `solid_rgb` holds the resolved palette RGB for solid-color
primitives and is null for textured primitives. Retained-color baking applies
this RGB before integer rounding. Each entry is `[brightness, solid]`, where
brightness is the converted runtime byte, `(tex_hi * 4) & 255`, and solid is 1
for palette-color faces or 0 for textured faces. Version 3 retained the stored
byte and requires re-export for the runtime-brightness contract. Version 5 also
preserves all referenced authored corner normals when their count exceeds the
position count. Earlier exports can substitute face normals for those omitted
records and require re-export. Material grouping and polygon triangulation retain these inputs for
every emitted vertex. Terrain primitives omit this metadata.

Indexed solid-color primitives retain `rgpre_solid_palette` with `version: 1`
and the original eight-bit `index`. Different indices remain separate primitives
when their resolved RGB values match. Generic material colors remain unchanged;
engine renderers can distinguish world index zero before applying palette
conversion and lighting. Textured primitives omit this metadata.

Textured model primitives also retain `rgpre_texcoords` version 1 with a `pixels`
array in exported vertex order. Each pair is the decoded model coordinate divided
by 16, in original source-image pixel units, before dimension normalization or
vertical reversal. Negative and out-of-image coordinates are retained. Material
splitting and triangle fans duplicate the corresponding pairs. `TEXCOORD_0`
remains normalized and vertically reversed for generic GLTF viewers. Terrain and
solid-color primitives omit `rgpre_texcoords`. Generated flat fallback meshes may
carry these pairs, but their `rgpre_flat` node metadata defines their separate
runtime geometry and coordinate path.

### Texture animation and polygon metadata

Textured glTF material names identify the resolved TEXBSI or TEXTURE bank,
original image name and image ID.

Animated glTF materials carry `extras.rgpre_texture_animation` with version 1.
It contains the original `image_name`, signed header `delay_ms`, and `textures`,
an ordered array of glTF texture indices including frame zero. The base-color
texture remains frame zero for consumers without animation support. Static
images omit this metadata.

Every frame uses the same palette, row orientation and sampler as frame zero.
Model materials retain alpha masking when any exported frame contains
transparent pixels. Terrain materials and every exported terrain frame remain
opaque, including palette index zero, whose RGB comes from the selected palette.
Terrain and model uses of the same image have separate exported textures.
Runtime model render attributes can impose a different chroma policy. Consumers sharing one loaded original image must share its playback
state even when different materials reference separate exported copies.

Model glTF primitives also carry `extras.rgpre_admission` with version 1. Its
`radius_fixed` retains the model header radius without conversion. `polygons`
contains the primitive's authored polygons in source order. Each entry has
`source_face`, the zero-based model face index; `vertices`, indices into that
primitive's POSITION accessor in original polygon order; and `normal`, the
authored base-frame face normal in glTF export axes without normalization.
The normal is independent of the vertex normals used for lighting.

Material splitting and fan triangulation preserve these records for both static
and animated models. Coincident authored vertices remain separate polygon
corners. Animated consumers use the source face identity to select the current
frame's normal, and the vertex mapping to read the updated mesh positions.
A missing or nonfinite authored normal is `null`. If invalid source vertex
indices prevent a complete polygon mapping, `vertices` is `null`; consumers
must not infer faithful admission from the remaining raster triangles.
Terrain primitives omit this model-specific metadata.

### KEYS.INI parser

`import::keys_ini::KeysIni` exposes the eight keyboard bindings and user overrides, seven additional gameplay keys, and the installed key-name table. Section/key matching ignores case, and the first occurrence wins. `control_codes()` returns bindings in menu-action order 710 through 724.

### MENU.INI parser

`import::menu_ini::MenuPage::parse(content, page)` reads a page's optional texture slots, ordered text entries, widget positions/alignment, and inclusive slider bounds. Missing numeric element fields default to zero; malformed values and nonsequential text indices report the page and field. The first texture slot is required; a second may be omitted when no element references it. The parser rejects incomplete references and reversed slider bounds. `import::menu_movies::parse_menu_movies` separately reads cinematic names, forward skip points, and timed subtitles. Runtime save labels remain outside these parsers. Rendering and traversal rules are documented in [Book menu](docs/engine/menu.md).
