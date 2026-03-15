# Water Wave Animation

Per-frame vertex displacement system that animates water surfaces on the terrain grid. Water cells are identified by texture index, then their height values are replaced with a sine-table lookup producing radial concentric ripples.

## Water Tile Detection

A grid cell is classified as water when **all four corner vertices** have a texture index (lower 6 bits of [Map 3](../formats/WLD.md#texture-selection-map-3)) in the set **{0, 5, 30, 31}**. The check runs per-cell inside the wave renderer — only cells passing all four corners receive wave displacement. Non-water cells retain their static [height-table](../formats/WLD.md#height-lookup-table) value.

## Wave Parameters

Three values configure the wave system per world, set via [`world_wave[N]`](../config/world-ini.md) in `WORLD.INI`:

| Parameter | INI position | Console command | Description |
|---|---|---|---|
| Amplitude | 1st | `fxwaveamp` | Vertical scale of wave displacement. Multiplies the sine-table value. |
| Speed | 2nd | `fxwavespeed` | Rate of phase advance per frame. Higher values = faster ripple animation. |
| Spatial frequency | 3rd | `fxwavefreq` | Controls ripple density. Multiplies the squared-distance term in the phase calculation. |

The setup function stores amplitude and spatial frequency directly; speed passes through a conversion function before storage.

## Displacement Formula

For each water vertex at grid position (x, z), the engine computes:

```
distance_sq = x_offset² + z_offset²
phase       = (distance_sq * spatial_freq + frame_count * speed) & 0x7FF
wave_offset = sine_table[phase] * amplitude
bias        = amplitude * centering_constant

vertex.y    = wave_offset + height_table[heightmap_byte] - bias
```

Where:
- **x_offset, z_offset** — grid-relative coordinates from the center of the visible terrain window
- **frame_count** — global frame counter, advances each tick
- **`& 0x7FF`** — wraps the phase to 2048 entries (the sine table length)
- **height_table** — the same 128-entry [height lookup table](../formats/WLD.md#height-lookup-table) used for all terrain
- **bias** — centers the oscillation so waves ripple symmetrically around the base water level

The squared-distance term produces **concentric circular ripples** radiating outward. This is not a planar wave — the phase depends on radial distance from the grid center, so ripples form rings rather than parallel lines.

## Sine Lookup Table

The wave animation indexes a **2048-entry float table** allocated at runtime. The table is addressed as:

```
value = table[(phase & 0x7FF) * 4]    (byte offset; effectively table[phase & 0x7FF] as float)
```

The table stores one full period of a periodic waveform across 2048 samples. Multiple engine systems share this table — it is also used for camera rotation interpolation and sky animation, confirming it is a general-purpose sine/cosine lookup rather than a water-specific waveform.

## Water Level Initialization

The terrain height table has two initialization modes:

| Mode | Formula | When used |
|---|---|---|
| Default | `height[i] = -ABS(source[i])` | No water level specified |
| Water-relative | `height[i] = water_level - ABS(source[i])` | Water level parameter is non-zero |

When a non-zero water level is provided, a secondary rendering flag is set that enables the wave displacement pass. The same 128-entry source table is used in both modes — only the sign/offset changes.

## Rendering Pipeline

The wave renderer runs each frame as part of the terrain update:

```
1. Build vertex grid
   └─ 33×33 vertices, each with X, Y (height), Z, texture index
   └─ stride: 76 bytes per vertex, 2584 bytes per row

2. Wave displacement
   ├─ Clear dirty flags
   ├─ For each grid cell:
   │   ├─ Check all 4 corners for water texture indices {0, 5, 30, 31}
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

Each vertex in the 33×33 grid occupies 76 bytes:

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

- [WLD § Water Tiles](../formats/WLD.md#water-tiles) — texture-index detection criteria for water cells
- [WLD § Height Lookup Table](../formats/WLD.md#height-lookup-table) — the 128-entry height table shared by terrain and water base heights
- [WORLD.INI § Water](../config/world-ini.md#water) — `world_wave[N]` INI parameter format
- [SURFACE.INI](../config/surface-ini.md) — surface-type definitions including `[water]`, `[deepwater]`, `[scapewater]` sound sections
- [Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences) — software vs Glide renderer behavior (wave rendering may differ between renderers)
