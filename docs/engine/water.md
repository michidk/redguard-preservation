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
`sin(float32(n * 6.2831852 / 2048))`, stored as 32-bit floats. The angle itself
is rounded to single precision before taking sine. The table contains 2,560
entries, including the extended quarter revolution used by cosine. Water addresses the table
starting at sample 512, so `wave_table[p]` is the float sample
`sin(float32((p + 512) * 6.2831852 / 2048))`. The 2048 wave phases therefore start near
one, zero, minus one, and zero at phases 0, 512, 1024, and 1536 respectively.
The decimal full-period constant is part of the table construction.

## Terrain window

The engine fills a 34-by-34 storage grid around its terrain
camera-grid coordinates, from `camera-17` through `camera+16`. The wave loop
selects the inner 32-by-32 vertices, from `camera-16` through `camera+15` on
each axis, and reads the four corners described above from that storage.
The center on each axis is the camera's engine coordinate divided by 256,
rounded to single precision and then to the nearest integer, with ties to even.
A change on either axis reloads the storage grid before displacement.

Terrain projection transforms the inner 32-by-32 vertices. Triangle submission
uses only those vertices, covering 31-by-31 cells with two triangles per cell.
The outer storage ring supplies neighboring heights for selection and normals;
it is not drawn. This is an original rendering window, not a moving ripple origin.

## Dirty normals and retained lighting

Each wave update clears the storage grid's dirty flags. Every selected vertex
marks six storage entries, even when its replacement height equals its current
height. In engine-grid coordinates, the marks relative to the selected vertex
are:

| Relative X | Relative Z | Flags added |
|---|---|---|
| 0 | 0 | 3 |
| -1 | 0 | 3 |
| -1 | +1 | 3 |
| 0 | +1 | 3 |
| +1 | 0 | 2 |
| 0 | -1 | 2 |

Overlapping marks combine with bitwise OR. Positive engine Z corresponds to
decreasing raw WLD row, so negate the relative Z column when using source rows.

Face-normal, vertex-normal and vertex-RGB updates all admit an entry when its
whole dirty value is nonzero. They do not select different work from individual
flag bits. The face pass replaces both triangle normals of an admitted cell.
The vertex pass sums the six stored incident face normals using the
[terrain normalization rules](terrain-lighting.md#shading-normals); the RGB
pass then uses the retained or replaced vertex normal.

Within the 34-by-34 storage grid, face entries cover X indices 0 through 32
and Z indices 1 through 33. A face entry at `(x,z)` stores the two triangles
of the cell with corners `(x,z-1)`, `(x+1,z-1)`, `(x,z)` and `(x+1,z)`.
Vertex-normal and RGB entries cover indices 1 through 32 on both axes.
An entry outside its pass's range is not updated by that pass.

The diagonal neighbor at relative `(+1,-1)` is not marked by the selected
vertex, although one of its incident faces can change. Unless another selected
vertex marks it, its vertex normal and RGB retain their previous values.
Recomputing every vertex normal from the displaced grid therefore produces
different water-edge lighting.

When the camera's terrain-grid coordinates change, the storage grid reloads
base heights and all three passes run over their full ranges without dirty
admission. Subsequent wave updates resume selective replacement. Retention
lasts across wave updates until an entry is admitted or a full rebuild replaces
it.

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
       └─ Vertex RGB      (lighting from retained or replaced vertex normals)

3. Lighting
   └─ Per-vertex RGB from ambient + directional dot-product, clamped to [0, 255]

4. Rasterize
```

The RGB inputs and directional calculation are specified in [terrain-lighting.md](terrain-lighting.md).

Admitted normals tilt with the displaced geometry. Unmarked water-edge entries
retain their earlier normals and lighting as specified above.

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
- [Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences) — software vs Glide renderer behavior
