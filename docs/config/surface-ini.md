# SURFACE.INI

Text-based INI configuration file that defines per-texture-index blend behavior and surface-type sound remapping for the terrain renderer.

Shipped sample path: `/Redguard/surface.ini` (e.g. `.../GOG Galaxy/Redguard/Redguard/surface.ini`).

Engine string analysis confirms the engine expects this file at startup, requires a `surfaces` section/header, and validates surface type values plus texture set/index bounds (with some sound-section validation).

## File Structure

Top-level sections:

1. `[surfaces]` section: texture-set/index to surface-type assignments
2. Sound remap sections: `[unknown]`, `[water]`, `[deepwater]`, `[scapewater]`, `[scapedeepwater]`, `[lava]`, `[sand]`, `[wood]`, `[tile]`, `[scape]`, `[rock]`, `[gloop]`

### Surface Assignment Records (`[surfaces]`)

- `set, index, type`
- `set, all, type`

Where:

- `set`: texture set id, documented range 0..511
- `index`: texture index inside the set
- `type`: case-insensitive token from `{WATER, DEEPWATER, LAVA, SAND, WOOD, TILE, ROCK, GLOOP}`

### Sound Remap Records

Each sound section contains entries of the form:

- `<sound_id> = <remap_sound_id>`

Behavior/constraints from shipped comments and runtime validation strings:

- Sound ids are 0..255 at animation-system call sites; remaps may point to other effect ids.
- Separate `water`/`scapewater` and `deepwater`/`scapedeepwater` sections are expected.
- Empty sections are valid (for example `[unknown]`, `[lava]`, `[scape]` in shipped sample).
- The file is parser-tolerant regarding token case (`WOOD` and `wood` both appear).

## Terrain Texture Blending

`SURFACE.INI` drives a pixel-level alpha-blending system for terrain tile transitions. The blend system uses a 64-entry type table (one per texture index) with these categories:

| Blend type | Behavior |
|---|---|
| None (default) | Tile rendered as fully opaque; no transition blending. |
| Full | Tile is always fully blended (alpha = 256). Used for index 0 (default/empty tile). |
| Gradient | Alpha ramps linearly across the tile based on pixel position. Used for indices 6 and 52. |
| Hard edge | Sharp alpha cutoff at a fixed distance into the tile. Used for index 31. |
| Custom pattern | Per-pixel alpha from a 256x256 lookup buffer, configured by `.CFG` files loaded via `SURFACE.INI`. Used for indices 5, 7, 30, and 32. |

Complete shipped-runtime taxonomy for terrain texture indices `0..63`:

- Explicit special indices are `{0, 5, 6, 7, 30, 31, 32, 52}`.
- Water-surface shortcut set is `{0, 5, 30, 31}` (all four tile corners in this set trigger water-surface rendering).
- All other indices in `0..63` use the default non-special blend/type behavior (no additional hardcoded special handling).

The file also defines a palette-remap table per texture type, mapping source palette indices to blended output indices.

This blending is a pixel-level renderer effect. It does not affect geometry, UVs, or material assignment — it only controls how adjacent terrain textures cross-fade at their shared edges.

## Relationships to Other Formats

- [WLD](../formats/WLD.md) terrain Map 3 texture indices are the primary consumer of the blend type table and surface-type assignments.
- [TEXBSI](../formats/TEXBSI.md) supplies the texture images referenced by the 64-entry terrain texture table.

## External References

- [UESP: Mod:World Files](https://en.uesp.net/wiki/Mod:World_Files)
