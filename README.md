# redguard-preservation

[![Crates.io](https://img.shields.io/crates/v/redguard-preservation.svg)](https://crates.io/crates/redguard-preservation)

Documentation and tools for preserving *The Elder Scrolls Adventures: Redguard* (1998), Bethesda's action-adventure game. The project reverse-engineers the game's file formats and engine, writes down what it finds, and provides software that reads and converts the game's assets.

## What is in this repository

| Part | What it gives you | Start here |
|---|---|---|
| **Documentation** | Reverse-engineered file format specifications, engine behavior, and INI references. | [Published docs](https://michidk.github.io/redguard-preservation/) |
| **`rgpre` command-line tool** | Inspect game files and convert them to open formats such as GLB, PNG, GIF, WAV, and JSON. | [Quick start](#quick-start) |
| **Rust library** | The same parsers as a crate, for your own tools. | [Using the Rust library](#using-the-rust-library) |
| **Native plugin** | The library as a C-compatible shared library, for engines such as Unity. | [Native plugin](#native-plugin-ffi) |

**You need your own copy of the game.** This repository contains no game files. The GOG release contains the Glide assets that most examples here use; the original CD also contains the software renderer's assets (see [Game editions](#game-editions)). Point the tools at the installed game directory.

## Quick start

Install the CLI from [crates.io](https://crates.io/crates/redguard-preservation) (requires Rust 1.85 or newer):

```bash
cargo install redguard-preservation
```

To build from source instead, run `cargo build` in a checkout; the binary is `target/debug/rgpre`.

The binary is called `rgpre`. It has three commands:

| Command | Alias | What it does |
|---|---|---|
| `read <FILE>` | `r` | Parse a file and print its decoded structure. |
| `scan <DIR>` | `s` | Recursively find known Redguard files in a directory. |
| `convert <FILE>` | `c` | Export a file to an open format. |

A first session, assuming the game is installed at `Redguard/`:

```bash
mkdir -p out
rgpre scan Redguard/fxart                                  # what is in here?
rgpre read Redguard/fxart/ISLAND.COL                       # print a palette's contents
rgpre convert Redguard/fxart/ISLAND.COL -o out/            # export it as PNG + JSON
rgpre convert Redguard/fxart/BELLTOWR.ROB \
  --palette Redguard/fxart/ISLAND.COL --assets Redguard \
  -o out/BELLTOWR.glb                                      # export a textured model
```

`convert <FILE>` detects the format from the file and uses defaults. `-o` takes a file path, or an existing directory in which the filename is derived. Formats that write many files (texture banks, sound banks, scenes) create their output directory themselves.

Models, worlds, and scenes need a palette (`--palette`) and an asset root (`--assets`) to get textures; without a palette they export as plain white. Both flags also read the `PALETTE` and `ASSETS` environment variables. Each scene uses its own palette, such as `ISLAND.COL` or `NECRO.COL`.

Run `rgpre --help` or `rgpre convert <FORMAT> --help` for the full option list.

## Converting files

`convert <FORMAT> <FILE>` selects a format explicitly and enables that format's flags. Every subcommand accepts `-o/--output`, and the image-producing ones accept `--compress-textures` (higher PNG compression).

| Subcommand | Input | Output | Format-specific flags |
|---|---|---|---|
| `convert model` | `.3d`, `.3dc`, `.rob` | `.glb` | `--assets`, `--palette` |
| `convert rgm` | `.rgm` scene | directory: `.glb`, scene/actors/navigation `.json`, and per-actor `.soup`/`.json` scripts | `--assets`, `--palette`, `--software` |
| `convert wld` | `.wld` world | `.glb` plus actor metadata `.json` (scene mode), or a set of map `.png` files | `--assets`, `--palette`, `--software`, `--terrain-only`, `--terrain-textures true\|false` |
| `convert texbsi` | `TEXBSI.###` texture bank | animated `.gif` and static `.png` files plus `index.json` (directory) | `--format gif\|png\|frames`, `--palette` |
| `convert texture` | `TEXTURE.###` software texture bank | animated `.gif` or frame `.png` files plus `index.json` | `--format gif\|png\|frames`, `--palette` |
| `convert shade`, `convert haze` | `SHADE.###`, `HAZE.###` software lookup tables | grayscale index `.png` | (none) |
| `convert gxa` | `.gxa` bitmap archive | animated `.gif` plus `index.json` (directory) | `--format gif\|png` |
| `convert fnt` | `.fnt` font | bitmap `.png` + BMFont `.fnt` + glyph `.json`, or `.ttf`¹ | `--format bitmap\|ttf` |
| `convert col` | `.col` palette | swatch `.png` + palette `.json` | `--format png\|json` (default: both) |
| `convert rtx` | `.rtx` dialogue audio | `.wav` files plus `index.json` (directory) | `--resolve-names` |
| `convert sfx` | `.sfx` sound effects bank | `.wav` files plus `index.json` (directory) | (none) |
| `convert cht` | `.cht` cheat states | `.json` | (none) |
| `convert pvo` | `.pvo` visibility octree | `.json` | (none) |
| `convert pcx` | `.pcx` image | `.png` | (none) |
| `convert lbm` | `.lbm` IFF packed bitmap | `.png` | (none) |
| `convert anm` | `.anm` Deluxe Paint animation | directory: `animation.gif` plus `index.json` | (none) |

¹ Exported TTF fonts are bitmap-traced vector outlines without hinting. They can be installed on Windows, used in game engines (Unity, Godot, etc.), and previewed at [fontdrop.info](https://fontdrop.info).

Without `--palette`, image exports show palette indices as grayscale. `rgpre convert system/gui.anm` writes `system/gui_anm/animation.gif` and timing metadata. `rgpre convert system/SKY_61.PCX` and `rgpre convert system/gui.lbm` export PNG images.

### More examples

```bash
# Model with palette and asset root
rgpre convert model fxart/BELLTOWR.ROB --palette fxart/ISLAND.COL --assets . -o output/BELLTOWR.glb

# Scene (RGM) with palette and asset root
rgpre convert rgm maps/BELLTOWR.RGM --palette fxart/ISLAND.COL --assets . -o output/BELLTOWR_scene.glb

# World (WLD): terrain plus the placed objects from the companion RGM
rgpre convert wld maps/ISLAND.WLD --palette fxart/ISLAND.COL --assets . -o output/ISLAND_world.glb

# World terrain only
rgpre convert wld maps/ISLAND.WLD --palette fxart/ISLAND.COL --assets . --terrain-only -o output/ISLAND_terrain.glb

# Texture bank, as an animated GIF per texture (default) or every frame as a PNG
rgpre convert texbsi fxart/TEXBSI.302 --palette fxart/ISLAND.COL -o output/TEXBSI_302/
rgpre convert texbsi fxart/TEXBSI.302 --palette fxart/ISLAND.COL --format frames -o output/TEXBSI_302/

# Font to TrueType
rgpre convert fnt fonts/FONT01.FNT --format ttf -o output/FONT01.ttf

# Palette as JSON only
rgpre convert col fxart/ISLAND.COL --format json -o output/ISLAND.json

# Dialogue audio, named by text instead of 4-character tags
rgpre convert rtx ENGLISH.RTX --resolve-names -o output/dialog/
```

## Game editions

Redguard shipped with two parallel sets of 3D assets, one per renderer:

| Directory | Renderer | Model versions | Palette files | Included in GOG |
|---|---|---|---|---|
| `fxart/` | Glide (3dfx) | v4.0, v5.0 | 27 populated COL palettes; one empty placeholder | ✓ |
| `3dart/` | Software | v2.6, v2.7 | 18 COL palettes, including `art_pal.col` | Original CD only |

The examples above use the Glide assets in `fxart/`, which both editions have. Model exports select their texture format from the model version. Software textures live in `TEXTURE.###` banks and software lookup tables in `SHADE.###` and `HAZE.###`.

For scene or terrain exports, the tools use software art only when the asset root has `TEXTURE.###` banks and no `TEXBSI.###` banks. An install containing both art trees defaults to Glide; add `--software` to `convert rgm` or `convert wld` to choose software instead:

```bash
rgpre convert texture 3dart/TEXTURE.302 --palette 3dart/ISLAND.COL --format frames -o output/texture_302
rgpre convert shade 3dart/SHADE.000 -o output/shade_000
rgpre convert rgm maps/START.RGM --assets . --software -o output/start.glb
```

Lookup exports write `indices.png`, preserving each mapping byte as a grayscale value. Texture exports include `index.json` with record metadata.

Rust consumers select the edition with `TextureCache::new_for_edition`, `registry::scan_dir_for_edition`, and `WorldHandle::open_for_edition`. The equivalent C entry point is `rg_open_world_for_edition`. See [CD and GOG differences](docs/README.md#gog-vs-original-cd-differences) for the full comparison.

## Using the Rust library

The package `redguard-preservation` builds a library named `rgpre` next to the CLI, and the CLI is a thin layer over it. Add the package to your `Cargo.toml` and import from `rgpre`. Parsers live under `rgpre::import`, for example `rgpre::import::pcx::parse_pcx_file`, `lbm::parse_lbm_file`, and `anm::parse_anm_file`. This section covers the APIs that go beyond "parse a file":

- [Inspection exports](#inspection-exports): decoded file data as JSON
- [Save-game loading](#save-game-loading)
- [Typed configuration parsers](#typed-configuration-parsers): `SYSTEM.INI`, `COMBAT.INI`, `SURFACE.INI`, `ITEM.INI`, `KEYS.INI`, `MENU.INI`
- [GLB export metadata](#model-rendering-metadata): the extra data attached to exported glTF nodes, materials, and primitives

### Inspection exports

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

### Save-game loading

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
State payloads remain opaque, except the inventory chunk, which
`parse_save_inventory` and `encode_save_inventory` decode and encode. Display strings use UTF-8 replacement characters
for invalid bytes; the original bytes remain in `SVGM`. See
[Save-game containers](docs/formats/SAVEGAME.md).

### Typed configuration parsers

The library parses the gameplay configuration files into named structs:

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

### KEYS.INI parser

`import::keys_ini::KeysIni` exposes the eight keyboard bindings and user overrides, seven additional gameplay keys, and the installed key-name table. Section/key matching ignores case, and the first occurrence wins. `control_codes()` returns bindings in menu-action order 710 through 724.

### MENU.INI parser

`import::menu_ini::MenuPage::parse(content, page)` reads a page's optional texture slots, ordered text entries, widget positions/alignment, and inclusive slider bounds. Missing numeric element fields default to zero; malformed values and nonsequential text indices report the page and field. The first texture slot is required; a second may be omitted when no element references it. The parser rejects incomplete references and reversed slider bounds. `import::menu_movies::parse_menu_movies` separately reads cinematic names, forward skip points, and timed subtitles. Runtime save labels remain outside these parsers. Rendering and traversal rules are documented in [Book menu](docs/engine/menu.md).

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


## Native plugin (FFI)

The library also builds as a C-compatible shared library (`cdylib`). Game engines like Unity can load it as a native plugin and call the conversion functions directly at runtime.

See [`src/ffi/README.md`](src/ffi/README.md) for the full API reference: architecture, C struct definitions, function signatures, memory management, and error handling.

## Documentation

The full documentation is published at **<https://michidk.github.io/redguard-preservation/>** (rendered with `mdBook`). The Markdown sources live under [`docs/`](docs/README.md) in three sections:

- [**File Formats**](https://michidk.github.io/redguard-preservation/formats/) ([source](docs/formats/README.md)): binary format specifications for models, textures, audio, maps, palettes, and other game assets
- [**Engine Details**](https://michidk.github.io/redguard-preservation/engine/) ([source](docs/engine/README.md)): reverse-engineered engine internals such as the cheat system, item attachment, SOUP scripting, sky renderer, and water waves
- [**Configuration**](https://michidk.github.io/redguard-preservation/config/) ([source](docs/config/README.md)): text-based INI files shipped with the game (surface types, combat, keys, etc.)

The CLI and Rust parsers focus on game assets. Hardware drivers, audio utilities, controller profiles, and development reports are documented in [Bundled support resources](docs/formats/BUNDLED-RESOURCES.md).

## Related projects

[Cyrus](https://github.com/michidk/cyrus) is a modern, faithful Rust
reimplementation of Redguard built on this project's research and the
`redguard-preservation` crate. It ships an asset viewer that opens every file
format documented here: models, worlds, textures, fonts, animations, scripts,
and audio. Use it [in the browser](https://michidk.github.io/cyrus/viewer/) or
download a native build from the
[latest Cyrus release](https://github.com/michidk/cyrus/releases/latest).

[![Cyrus asset viewer showing a textured character model](https://raw.githubusercontent.com/michidk/cyrus/main/dev/pages/viewer-model.webp)](https://michidk.github.io/cyrus/viewer/)

## Contributing

Commits and pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/), for example `docs(menu): document delete confirmation`. Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Tests run against the small files in `tests/fixtures/`. Some tests inspect a real install and are skipped unless the `REDGUARD_DIR` environment variable points at the game directory.

### Repository layout

- `src/` - Rust source
  - `src/import/` - format importers and parsers
  - `src/gltf/` - GLB conversion (builder, primitives, terrain, texture cache)
  - `src/ffi/` - C-compatible FFI layer for native plugin use (Unity, etc.)
  - `src/cli/` - CLI command handlers (`cli/convert/` for per-format converters)
  - `src/error.rs` - shared error types
- `docs/` - format specifications, engine notes, and configuration references (the mdBook source)
- `tests/` - integration tests and fixtures

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
