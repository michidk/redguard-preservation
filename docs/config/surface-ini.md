# SURFACE.INI

Text configuration mapping texture keys to surface classes and animation sound IDs to effect IDs.

The file lives in the game root directory as `surface.ini`.

The engine reads this file at startup and requires a `surfaces` section. It
validates surface type values, texture set and index bounds, and parts of each
sound section.

## File Structure

Top-level sections:

1. `[surfaces]` section: texture-set/index to surface-type assignments
2. Sound remap sections: `[unknown]`, `[water]`, `[deepwater]`, `[scapewater]`, `[scapedeepwater]`, `[lava]`, `[sand]`, `[wood]`, `[tile]`, `[scape]`, `[rock]`, `[gloop]`

### Surface Assignment Records (`[surfaces]`)

- `set, index, type`
- `set, all, type`

Where:

- `set`: texture set id, range 0..511
- `index`: texture index inside the set
- `type`: case-insensitive name from the surface-class table below

The shipped comments describe `all` as a wildcard, but the executable's loader
converts the index with its decimal integer reader. `all` therefore becomes
index zero; it does not expand to every texture. Numeric indices must be in
`0..127`. The reader accepts an optional sign and an initial sequence of decimal
digits; a token without initial digits produces zero.

Assignments retain file order. Each lookup key is `(set << 7) | index`, and the
first matching assignment wins. A zero key terminates lookup before matching,
so an assignment to set zero, index zero hides itself and every later assignment.
An unmatched key returns surface zero (`unknown`).

For v4/v5 model faces, solid-color materials resolve to packed surface key zero,
and therefore to surface zero. Both palette-indexed and directly encoded solid
colors follow this rule; their color value does not select a texture assignment.

### Surface classes

The same class codes select sound rows and describe the actor's ground state.

| Code | Name |
|---|---|
| 0 | `unknown` |
| 1 | `water` |
| 2 | `deepwater` |
| 3 | `scapewater` |
| 4 | `scapedeepwater` |
| 5 | `lava` |
| 6 | `sand` |
| 7 | `wood` |
| 8 | `tile` |
| 9 | `scape` |
| 10 | `rock` |
| 11 | `gloop` |

### Sound remap records

Each named sound section contains `<source> = <target>` records. The equals
sign is optional. Names are case-insensitive; semicolons start comments.

Each class has an independent 256-entry table initialized to identity.
Missing or empty sections keep that identity table. Repeated sections are
processed in file order, and later sound assignments overwrite earlier ones.
Water and terrain-water classes have separate tables; neither inherits the
other's entries.

Both numeric tokens must begin with a decimal digit. Conversion uses the
initial decimal digits, ignoring a following suffix. Source IDs must be in
`0..255`; invalid sources cause a configuration error. Target IDs are stored
as unsigned 16-bit values, retaining the low 16 bits of the converted integer.
They are not restricted to the animation source range or to the loaded sound
bank's size. Tokens after the target on the same line are ignored.

Only animation PlaySound commands use these rows. Surface zero bypasses the
lookup even if `[unknown]` contains remaps. Script sounds, combat hit sounds
and speech do not use the remap. See [sound playback](../engine/sound.md#surfaceini-remap)
for lookup bounds and the ground-state source.

## Terrain surface selection

Terrain starts with the assignment for texture set 302 and the low-six-bit
image index from WLD Map 3. Fixed engine blend-mask rules can replace this
base class with sand, rock, `scapewater` or `scapedeepwater`. These rules are
not configurable records in `SURFACE.INI`. The file has no CFG mask paths or
palette-remap tables. See [terrain material lookup and coverage](../engine/player.md#terrain-material-blend-coverage).

## Rust API

`rgpre::import::surface_ini::SurfaceIni::parse` reads the assignments and
sound rows. Its `assignments.surface(packed_texture)` preserves first-match
texture lookup and zero-key termination. `sound_table(surface)` exposes the
configured row; `animation_sound(surface, source)` also applies the surface-zero
bypass. Both sound accessors return `None` for class codes outside `0..11`.
Animation sources are `u8`; resolved bank indices are `u16`.

The parser rejects numeric overflow beyond a nonnegative signed 32-bit integer
for sound tokens. Errors carry the file, section, line and offending key.
`SurfaceAssignments::parse` remains available for assignment-only consumers
and does not validate sound sections.

## Relationships to Other Formats

- [WLD](../formats/WLD.md) terrain Map 3 supplies image indices for terrain surface queries.
- [TEXBSI](../formats/TEXBSI.md) supplies the texture images identified by assignment keys.

## External References

- [UESP: Mod:World Files](https://en.uesp.net/wiki/Mod:World_Files)
