# WORLD.INI

World/level database defining map files, palettes, lighting, sky, weather, and PVO node maps for all game locations.

Shipped path: `/Redguard/WORLD.INI`. Referenced by `SYSTEM.INI` via `world_ini=WORLD.INI`. The file contains a single `[world]` section with four global keys followed by per-world-index entries for every game location.

## File Structure

Single section: `[world]`

### Global Keys

| Key | Example value | Description |
|---|---|---|
| `start_world` | `0` | World index loaded on new game start. |
| `start_marker` | `0` | Spawn marker index within the start world. |
| `test_map_delay` | `1` | Delay (seconds) between worlds during test-map cycling. |
| `test_map_order` | `0,1,2,1,...,-2` | Comma-separated world index sequence for the test-map loop. `-2` terminates the list. The shipped file includes a commented-out compact variant and an active interleaved variant that returns to world 1 between each location. |

### Per-World Keys

Each world is identified by an integer index `N`. Keys follow the pattern `key_name[N]=value`.

#### Core

| Key | Type | Description |
|---|---|---|
| `world_map[N]` | path | [RGM](../formats/RGM.md) scene file for this world. Present on every entry. |
| `world_world[N]` | path | [WLD](../formats/WLD.md) terrain file. Only present for outdoor worlds with a heightmap terrain mesh. |
| `world_redbook[N]` | integer | CD audio track number for background music. |
| `world_palette[N]` | path | [COL](../formats/COL.md) palette file. |
| `world_shade[N]` | integer | Shade table index. |
| `world_haze[N]` | integer | Haze/distance-fog table index. |
| `world_background[N]` | integer | Background fill color index. `0` = black, `2` = sky color, other values are palette indices. |
| `world_compass[N]` | integer | Compass heading offset (fixed-point). Omitted for most worlds; present where the player can see a compass. |
| `world_flash_filename[N]` | path | GXA file used for screen-flash transitions when entering or leaving this world. |

#### Lighting

| Key | Type | Description |
|---|---|---|
| `world_sun[N]` | x, y, z, intensity | Sun direction vector and intensity. The three components are large fixed-point integers defining the light direction; intensity is a scalar. |
| `world_ambient[N]` | integer | Global ambient light level. |
| `world_ambientfx[N]` | integer | Ambient effect intensity (affects dynamic lighting). |
| `world_ambientrgb[N]` | r, g, b | Ambient light color as three 0..255 components. |
| `world_sunangle[N]` | integer | Sun angle (fixed-point). Controls the horizontal rotation of the sun direction. |
| `world_sunskew[N]` | integer | Sun skew (fixed-point). Controls the vertical tilt of the sun direction. |
| `world_sunrgb[N]` | r, g, b, scale | Sun color as three 0..255 components plus a scale factor. |
| `world_fogrgb[N]` | r, g, b | Distance fog color as three 0..255 components. |

#### Sky

| Key | Type | Description |
|---|---|---|
| `world_sky[N]` | path | GXA skybox texture. Only present for outdoor worlds. |
| `world_skyfx[N]` | filename | BSI sky texture (scrolling sky layer). |
| `world_skyscale[N]` | integer | Scale factor for the sky layer. |
| `world_skylevel[N]` | integer | Vertical offset of the sky layer (negative = below horizon). |
| `world_skyspeed[N]` | integer | Scroll speed of the sky layer. |
| `world_sky_xrotate[N]` | integer | Sky rotation rate around the X axis. |
| `world_sky_yrotate[N]` | integer | Sky rotation rate around the Y axis. |

#### Sun Disc

| Key | Type | Description |
|---|---|---|
| `world_sunimg[N]` | filename | BSI texture for the rendered sun disc. |
| `world_sunimgrgb[N]` | r, g, b | Tint color for the sun disc texture. |
| `world_sunscale[N]` | integer | Size scale of the sun disc. |

#### Water

| Key | Type | Description |
|---|---|---|
| `world_wave[N]` | a, b, c | Wave animation parameters for water surfaces. Three integers: **a** = amplitude (vertical displacement scale), **b** = speed (phase advance per frame), **c** = spatial frequency (ripple density, multiplies the squared-distance term). See [Water Waves](../engine/water.md) for the displacement formula. |

#### PVO Visibility

| Key | Type | Description |
|---|---|---|
| `world_node_mapN[W]` | path | PVO `.noo` node map file, where `N` is a 1-based sequence index and `W` is the world index. Each world can have multiple node maps. See [PVO](../formats/pvo/format.md) for the octree format. |

#### Rain / Weather

Only world 6 (necrisle) uses these keys. No other world has weather effects.

| Key | Type | Description |
|---|---|---|
| `world_rain_delay[N]` | integer | Frames between rain drop spawns. |
| `world_rain_drops[N]` | integer | Maximum simultaneous rain drops. |
| `world_rain_start[N]` | integer | Vertical start height for rain drops (negative = above ground). |
| `world_rain_end[N]` | integer | Vertical end height where drops are removed. |
| `world_rain_sphereN[W]` | x, y, z, r | Sphere defining a rain zone: center coordinates plus radius. Multiple spheres (indexed 1..4 in world 6) define the areas where rain falls. |

## World Catalog

IDs 9, 10, and 16 have no entries in the file and are skipped.

Worlds 0, 1, 6, 27, 28, and 30 have `world_world` entries and use a WLD terrain mesh. All other worlds are indoor or dungeon locations with no terrain.

| ID | RGM File | WLD File | Palette | Location |
|---|---|---|---|---|
| 0 | `MAPS\start.rgm` | `MAPS\hideout.WLD` | `3DART\sunset.COL` | Starting hideout (exterior, sunset) |
| 1 | `MAPS\ISLAND.rgm` | `MAPS\ISLAND.WLD` | `3DART\island.COL` | Stros M'Kai island (daytime) |
| 2 | `MAPS\catacomb.rgm` | — | `3DART\catacomb.COL` | Catacombs |
| 3 | `MAPS\PALACE.rgm` | — | `3DART\palace00.COL` | Palace interior |
| 4 | `MAPS\caverns.rgm` | — | `3DART\REDcave.COL` | Caverns |
| 5 | `MAPS\observe.rgm` | — | `3DART\observat.COL` | Observatory |
| 6 | `MAPS\necrisle.rgm` | `MAPS\necrisle.WLD` | `3DART\necro.COL` | Necromancer's Isle (rain, rotating sky) |
| 7 | `MAPS\necrtowr.rgm` | — | `3DART\necro.COL` | Necromancer's Tower interior |
| 8 | `MAPS\drint.rgm` | — | `3DART\observat.COL` | Dwemer ruin interior |
| 11 | `MAPS\jailint.rgm` | — | `3DART\necro.COL` | Jail interior |
| 12 | `MAPS\temple.rgm` | — | `3DART\island.COL` | Temple |
| 13 | `MAPS\mguild.rgm` | — | `3DART\redcave.COL` | Mages Guild |
| 14 | `MAPS\vile.rgm` | — | `3DART\island.COL` | Vile Lair |
| 15 | `MAPS\tavern.rgm` | — | `3DART\island.COL` | Tavern |
| 17 | `MAPS\hideint.rgm` | `MAPS\hideout.WLD` | `3DART\hideout.COL` | Hideout interior (shares hideout WLD) |
| 18 | `maps\silver1.rgm` | — | `3DART\island.COL` | Silversmith (area 1) |
| 19 | `maps\silver2.rgm` | — | `3DART\island.COL` | Silversmith (area 2) |
| 20 | `maps\belltowr.rgm` | — | `3DART\island.COL` | Bell Tower |
| 21 | `maps\harbtowr.rgm` | — | `3DART\island.COL` | Harbor Tower |
| 22 | `maps\gerricks.rgm` | — | `3DART\island.COL` | Gerrick's |
| 23 | `maps\cartogr.rgm` | — | `3DART\island.COL` | Cartographer's |
| 24 | `maps\smden.rgm` | — | `3DART\island.COL` | Smuggler's Den |
| 25 | `maps\rollos.rgm` | — | `3DART\island.COL` | Rollo's |
| 26 | `maps\jffers.rgm` | — | `3DART\island.COL` | Jeffers' |
| 27 | `MAPS\island.rgm` | `MAPS\ISLand.WLD` | `3DART\nightsky.COL` | Island (night variant) |
| 28 | `MAPS\ISLAND.rgm` | `MAPS\ISLAND.WLD` | `3DART\sunset.COL` | Island (sunset variant) |
| 29 | `maps\brennans.rgm` | — | `3DART\island.COL` | Brennan's Farm |
| 30 | `MAPS\extpalac.rgm` | `MAPS\ISLAND.WLD` | `3DART\sunset.COL` | Palace exterior (sunset, uses island WLD) |
| 99 | `maps\brennans.rgm` | — | `3DART\island.COL` | Brennan's Farm (alternate entry) |

## Notes

**Typos in the shipped file.** Line 286 reads `orld_ambientfx[8]=128` (missing leading `w`) and line 498 reads `orld_sunangle[23]=256`. Both are present in the shipped file as-is; the engine likely ignores the malformed keys and falls back to defaults for those fields.

**World 6 rain zones.** The four `world_rain_sphere` entries for necrisle define overlapping spheres centered on different parts of the island. Sphere 2 has the largest radius (1000 units) and covers the main approach area.

**World 17 shares terrain with world 0.** Both the starting exterior (world 0) and the hideout interior (world 17) reference `MAPS\hideout.WLD`. The interior uses the same terrain data but a different RGM scene and palette.

**Worlds 27 and 28 are time-of-day variants of world 1.** All three share `MAPS\ISLAND.WLD` and the same PVO node maps (`islan001`..`islan004`, `lighths`). World 27 uses a night palette and sky; world 28 uses a sunset palette matching world 0.

## Redguard Preservation CLI

When converting RGM or WLD files without an explicit `--palette` flag, the CLI reads `WORLD.INI` from the asset root to auto-resolve the correct palette. If multiple world entries match the input file (e.g. `ISLAND.RGM` appears in worlds 1, 27, and 28), the first match is used and alternatives are logged. Use `--palette` to override.

## External References

- [UESP: Redguard:Console](https://en.uesp.net/wiki/Redguard:Console) — the `show world` console command displays the current world index at runtime
