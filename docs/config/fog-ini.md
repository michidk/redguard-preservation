# FOG.INI

Fog density ramp table defining distance-based fog intensity for the terrain renderer.

Shipped path: `fxart/FOG.INI` (inside the Glide asset directory).

## Format

Comma-separated `index,value` pairs defining a piecewise fog density curve. The engine interpolates between entries to fill a 64-entry fog table.

- `index`: fog table position (0–63), corresponding to distance bands.
- `value`: fog intensity (0–255), where 0 = fully clear and 255 = fully opaque.

The final entry (`63`) has no value — it marks the end of the table and uses the last specified density.

## Shipped Values

| Index | Value | Description |
|---|---|---|
| 0 | 0 | No fog at close range. |
| 33 | 1 | Fog begins at index 33. |
| 34–48 | 2–255 | Rapid ramp from barely visible to fully opaque. |
| 63 | — | End marker (holds value 255 from index 48). |

From the shipped comments: "46 is the final value for 3800 render distance" — this ties the fog ramp to the `back_plane=3800` setting in [SYSTEM.INI](system-ini.md) `[xngine]`.

## External References

- [UESP: Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences)
