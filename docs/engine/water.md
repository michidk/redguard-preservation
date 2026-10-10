# Water Wave Animation

Per-frame vertex displacement system that animates water surfaces on the terrain grid. Selected terrain vertices receive a shared rotation-table displacement producing radial concentric ripples.

## Water vertex selection

Wave displacement tests a vertex and the three corners toward
negative grid X and positive engine Z. The selected vertex must have texture
index **{0, 30, 31}**; the other three may have **{0, 5, 30, 31}**. Each index is
masked to its lower six bits. Only the selected vertex moves. Texture 5 can
border an animated vertex but is never itself selected for displacement.

The engine reverses WLD source rows when loading the terrain. In the raw
row-major grid returned by `WldFile::combined_map`, the selected vertex is
`(x, row)` and its neighbours are `(x-1, row)`, `(x, row-1)` and
`(x-1, row-1)`. The first raw row and first column have no such complete cell.

## Wave parameters and enablement

`world_wave[N]` contains amplitude, speed and spatial frequency,
in that order. The world loader parses integers and converts them to floats.
Terrain setup truncates speed and frequency toward zero into integers.
Amplitude remains floating point. There is no additional speed multiplier.
A zero amplitude disables displacement; `world_scapeshift` does not enable it.

| Parameter | Console command | Meaning |
|---|---|---|
| Amplitude | `fxwaveamp` | Vertical displacement scale in engine units. |
| Speed | `fxwavespeed` | Phase advance per BIOS timer tick. |
| Spatial frequency | `fxwavefreq` | Multiplier for squared grid distance. |

## Displacement and clock

For selected engine-grid vertex `(x, z)`, width `W`, height `H`,
integer speed `S`, integer frequency `F`, amplitude `A`, and BIOS tick count `T`:

```
distance_sq = (x - floor(W/2))² + (z - floor(H/2))²
phase       = (distance_sq * F + T * S) & 2047
height      = base_height + wave_table[phase] * A - A * 0.5
```

The BIOS timer count is independent of rendered frames. The standard PC BIOS
clock advances approximately 18.2065 times per second. The absolute phase
depends on that clock's initial value, not on scene loading. The spatial origin
is the **whole terrain midpoint**, not the camera or the visible-window center.

`base_height` is the [height-table](../formats/WLD.md#height-lookup-table)
value, `world_scapeshift - abs(source_height)`, in engine coordinates with
positive Y down. Exporters using positive Y up must negate the displacement
and apply their engine-unit scale. The half-amplitude subtraction is a vertical
bias; the range for positive amplitude is approximately `[-1.5*A, 0.5*A]`.
It does not center the oscillation on the base height.

## Wave lookup table

The shared rotation table contains samples of
`sin(n * 6.2831852 / 2048)`, stored as 32-bit floats. Water addresses the table
starting at sample 512, so `wave_table[p]` is the float sample
`sin((p + 512) * 6.2831852 / 2048)`. The 2048 wave phases therefore start near
one, zero, minus one, and zero at phases 0, 512, 1024, and 1536 respectively.
The decimal full-period constant is part of the table construction.

**Inferred:** for amplitude 24 and zero base height, engine heights at those
four phases are approximately 12, -12, -36, and -12. At speed 28, phase advances
by 28 per BIOS tick; its continuous period is approximately 4.02 seconds.

## Terrain window

The engine fills a 34-by-34 storage grid around its terrain
camera-grid coordinates, from `camera-17` through `camera+16`. The wave loop
selects the inner 32-by-32 vertices, from `camera-16` through `camera+15` on
each axis, and reads the four corners described above from that storage.
This is an original rendering window, not a moving ripple origin.

## Rendering Pipeline

The wave renderer runs each frame as part of the terrain update:

```
1. Build vertex grid
   └─ 34×34 storage vertices, each with X, Y (height), Z, texture index
   └─ stride: 76 bytes per vertex, 2584 bytes per row

2. Wave displacement
   ├─ Clear dirty flags
   ├─ For each grid cell:
   │   ├─ Check the selected corner {0, 30, 31} and its three neighbours {0, 5, 30, 31}
   │   ├─ If water: replace vertex.y with sine_table[phase] * amplitude + height - bias
   │   └─ Set dirty flags on affected cells and neighbors
   └─ Recompute normals on displaced geometry:
       ├─ Face normals   (cross products per triangle)
       ├─ Vertex normals  (average adjacent face normals)
       └─ Smooth normals  (per-vertex lighting pass)

3. Lighting
   └─ Per-vertex RGB from ambient + directional dot-product, clamped to [0, 255]

4. Rasterize
```

The normal recomputation after displacement ensures water surfaces receive correct per-frame lighting as waves move — the normals tilt with the displaced geometry rather than remaining flat.

## Terrain Vertex Layout

Each storage vertex occupies 76 bytes:

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| 0x00 | 4 | `f32` | x | World X position (`grid_x * 256.0`) |
| 0x04 | 4 | `f32` | y | Height (from lookup table, or wave-displaced) |
| 0x08 | 4 | `f32` | z | World Z position (`grid_z * 256.0`) |
| 0x18–0x30 | | | face_normals | Two face normals per cell (upper/lower triangle) |
| 0x30–0x3C | | | vertex_normal | Averaged vertex normal for lighting |
| 0x48 | 4 | `u32` | texture_index | Terrain texture ID (lower 6 bits used for water detection) |

Grid stride: 76 bytes between adjacent X-axis vertices, 2584 bytes (34 × 76) between rows.

## Console Commands

Three runtime console commands allow tuning wave parameters without restarting:

| Command | Syntax | Effect |
|---|---|---|
| `fxwaveamp` | `fxwaveamp <value>` | Set wave amplitude |
| `fxwavespeed` | `fxwavespeed <value>` | Set wave speed |
| `fxwavefreq` | `fxwavefreq <value>` | Set spatial frequency |

These modify the same globals as the INI parameters and take effect on the next frame.

## External References

- [PC BIOS timer ticks](https://www.delorie.com/djgpp/doc/rbinter/id/80/22.html) — standard BIOS timer clock

- [WLD § Water Tiles](../formats/WLD.md#water-tiles) — texture-index detection criteria for water cells
- [WLD § Height Lookup Table](../formats/WLD.md#height-lookup-table) — the 128-entry height table shared by terrain and water base heights
- [WORLD.INI § Water](../config/world-ini.md#water) — `world_wave[N]` INI parameter format
- [SURFACE.INI](../config/surface-ini.md) — surface-type definitions including `[water]`, `[deepwater]`, `[scapewater]` sound sections
- [Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences) — software vs Glide renderer behavior (wave rendering may differ between renderers)
