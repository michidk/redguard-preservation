# FOG.INI

Glide scene fog density table for models and terrain.

## Selection and format

**Verified:** the Glide renderer first reads `fxart/FOG<world ID>.INI`, then
falls back to `fxart/FOG.INI`. If neither exists, all 64 entries remain zero.
This selection is independent of the software renderer's `world_haze` field.
The installed GOG data contains only the fallback file.

Comma-separated index/value pairs fill a zero-initialized 64-entry table.
Each value fills its index through the index before the next pair. This file
expansion does not interpolate values. Semicolon comments are ignored.

- Index is a table position from 0 through 63.
- Value is a density byte from 0 through 255.
- The final index `63` has no value and remains zero.

## Shipped values

**Verified:** the installed fallback contains these 18 index records.

| Index | Value |
|---|---|
| 0 | 0 |
| 33 | 1 |
| 34 | 2 |
| 35 | 4 |
| 36 | 6 |
| 37 | 8 |
| 38 | 10 |
| 39 | 12 |
| 40 | 14 |
| 41 | 16 |
| 42 | 18 |
| 43 | 22 |
| 44 | 38 |
| 45 | 76 |
| 46 | 136 |
| 47 | 196 |
| 48 | 255 |
| 63 | End marker, remains zero |

Entries 0 through 32 are zero; entries 48 through 62 are 255.
The shipped comment says "46 is the final value for 3800 render distance".
The actual density at distance 3800 uses entries 47 and 48, as described in
[Glide fog and clipping](../engine/fog.md). Changing the clipping distance
does not rescale this table.

## External References

- [UESP: Redguard:Glide Differences](https://en.uesp.net/wiki/Redguard:Glide_Differences)
- [Glide fog and clipping](../engine/fog.md)
