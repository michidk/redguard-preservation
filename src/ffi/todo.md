# FFI Refactor

Comprehensive list for the FFI interface overhaul targeting game-engine consumability.

## Struct Changes

- [x] **`RgmdSubmeshHeader`: `material_type` → `textured: u8`** — Boolean semantics (0 = solid color, 1 = textured). Clearer intent.
- [x] **`RgmdSubmeshHeader`: `color_index` → `color_r, color_g, color_b: u8`** — Engines need resolved RGB, not palette indices. Repack: move `texture_id`/`image_id` down, shrink padding to 1 byte. Same 16-byte size.
- [x] **`RgmdHeader`: `radius: u32` → `f32`, scaled** — Positions are divided by 20.0 but radius isn't. Inconsistent coordinate space. Convert to `f32` and apply the same scale as vertices.
- [x] **`RgmdHeader`: `version[4]` → RGMD format version** — Now carries RGMD format version (1.0.0.0) instead of source file version.
- [x] **`RgmdHeader`: `frame_count`** — Set to 1 for models (only frame 0 serialized), 0 for terrain.
- [x] **`RobSegmentHeader`: `has_model: u8`** — Already a clear bool name; no rename needed.
- [x] **`RgplPlacement`: document `object_type` values** — 0 = mesh, 1 = flat sprite, 2 = rope link.
- [x] **`RgplPlacement`: clarify `texture_id`/`image_id` purpose** — Used for flat sprite billboards; 0 for mesh/rope placements.
- [x] **`RgplLight`: document `color[3]` space** — Linear RGB, 0.0–1.0.
- [x] **`RgplLight`: add light type field** — `light_type: u8` added (0 = point). Struct now 64 bytes.

## Function Signature Changes

- [x] **`rg_parse_model_data`: add `assets_dir` parameter** — Required for palette resolution to populate the new `color_r/g/b` fields on solid-color submeshes.
- [x] **`rg_parse_rob_data`: add `assets_dir` parameter** — Same reason as above.

## Bug Fixes

- [x] **Terrain serializer drops G/B channels** — Fixed: `SolidColor(rgb)` now stores all three channels.
- [x] **Terrain serializer maps `PaletteTexture` to black** — Fixed: `PaletteTexture(rgb)` now uses its resolved RGB.
- [x] **`rg_gxa_frame_count` missing `clear_last_error()`** — Fixed: now calls `clear_last_error()` on success.

## Documentation (README)

- [x] **Generate `rgpre.h` C header** — Hand-maintained header at `src/ffi/rgpre.h`.
- [x] **Document thread safety** — All functions callable from any thread. `rg_last_error()` is per-thread. Texture cache is internally synchronized.
- [x] **Add buffer parsing walkthrough** — Pseudocode for RGMD, RGPL.
- [x] **Document coordinate system** — Right-handed Y-up, scaled coords.
- [x] **Document `transform[16]` matrix layout** — Column-major (translation in elements 12–14).
- [x] **Document UV origin convention** — Top-left, V increases downward.
- [x] **Document winding order** — CCW front faces.
- [x] **Document string encoding** — Null-terminated ASCII, zero-padded.
- [x] **Add build instructions** — `cargo build --release`, output per platform.
- [x] **Document `radius` field** — Bounding sphere radius in scaled coordinates.

## New API

- [x] **Add subtitle/dialogue text extraction** — `rg_get_rtx_subtitle(file_path, entry_index)` returns UTF-8 subtitle text.
