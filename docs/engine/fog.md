# Glide fog and clipping

Scene visibility and sky blending use separate clipping ranges and fog tables.

## Clipping distances

**Verified:** the scene uses `SYSTEM.INI`'s `[xngine]` `front_plane` and
`back_plane`, in engine units. A nonzero `world_back_plane[N]` replaces the
system far distance on world load. The installed GOG configuration uses 7 and
3800, and none of its 29 worlds declares a world override.
The [sky plane](sky.md#glide-sky-plane) independently uses near/far 1 and 65000.

## Fog lookup

**Verified:** the sky supplies reciprocal positive camera-space depth to Glide.
The sky table is active during that draw; the scene table is restored afterwards.
Fog color is `world_fogrgb[N]`. Scene densities come from [FOG.INI](../config/fog-ini.md).

The Glide table's entry `i` corresponds to camera depth
`2^(3 + floor(i / 4)) / (8 - (i modulo 4))` in engine units.
For positive depth `d`, let `e = floor(log2(d))`. The continuous table coordinate
is `4*e + 8*(1 - 2^e/d)`, clamped to the interval 0 through 63.
Interpolation between adjacent entries is linear in reciprocal depth, rather
than distance. This is a camera-depth curve, rather than radial fog.

Glide uploads each density byte with a delta byte equal to
`4*(next density - current density) modulo 256`. The last delta is zero.
The interpolated density is the current byte plus one quarter of the delta
byte times the fractional coordinate. The hardware quantizes that fraction
and result, and can dither them. For monotonic adjacent differences below 64,
this has the usual adjacent-value interpolation curve. The shipped scene
table's transition from entry 62 to 63 wraps its delta; this lies beyond the
installed scene far plane.

**Verified for the software Voodoo oracle:** the RGB blend weight is
`(interpolated density + 1) / 256`. Blending operates on encoded RGB channel
values toward the fog color, before framebuffer quantization. Density 255
replaces RGB completely. Density zero has a 1/256 blend weight. Fog does not
replace alpha. This channel operation differs from mixing linear-light RGB.
Fixed-point rounding, fog dither and framebuffer RGB565 precision can change
individual pixels around the continuous curve.

| Depth | Table index | Scene density | Sky density |
|---|---|---|---|
| 256 | 32 | 0 | 0 |
| 512 | 36 | 6 | 32 |
| 1024 | 40 | 14 | 64 |
| 2048 | 44 | 38 | 96 |
| 4096 | 48 | 255 | 128 |
| 32768 | 60 | 255 | 224 |
| 52428.8 | 63 | 0 | 255 |

**Verified:** the sky table is zero through entry 32, then rises by eight per
entry through 240 at entry 62, followed by 255 at entry 63. Its density reaches
255 while approaching the finite sky boundary, so the texture blends into the
same fog RGB that fills uncovered pixels.

## Evidence and limits

Installed `SYSTEM.INI`, all 29 `WORLD.INI` entries and the 18-record `FOG.INI`
verify the configuration samples. Glide's published table-depth conversion and
table-upload representation establish the coordinate mapping. The software
Voodoo renderer used by the project's original-game runner establishes the
blend normalization and channel operation. Exact pixel agreement with physical
Voodoo hardware has not been measured.

## External References

- [Glide table-depth conversion](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gu.c)
- [Glide table upload](https://github.com/sezero/glide/blob/master/glide2x/sst1/glide/src/gglide.c)
- [Software Voodoo pixel operations](https://github.com/joncampbell123/dosbox-x/blob/master/src/hardware/voodoo_data.h)
