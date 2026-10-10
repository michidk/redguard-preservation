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

## Integer precision and framebuffer dithering

**Verified for the software Voodoo 1 oracle:** table fog uses an unsigned
16-bit reciprocal-depth encoding. For depth greater than one, let
`t = floor(2^32 / depth)`. Values of `t` below 65536 select 65535. Otherwise,
with `e` the count of leading zero bits in the 32-bit `t`, the encoding is
`min(65535, ((e << 12) | ((~t >> (19-e)) & 4095)) + 1)`.
Depth at or below one selects zero. The table index is the encoding divided
by 1024; its interpolation fraction is bits 2 through 9.

The density is the table byte plus
`floor(delta_byte * fraction_byte / 1024)`. Voodoo 1 does not apply fog-density
dithering; the fog-dither switch is a Voodoo 2 feature. For incoming encoded
8-bit channel `c`, fog channel `f` and density `d`, the resulting channel is
`clamp(c + floor((f-c)*(d+1)/256), 0, 255)`.

Glide initializes framebuffer dithering to 4 by 4. The shipped game has no
call to change that mode. The threshold matrix, indexed by framebuffer Y then
X modulo four, is:

```
 0  8  2 10
12  4 14  6
 3 11  1  9
15  7 13  5
```

For threshold `b` and encoded channel `c`, the stored red/blue five-bit value
is `floor((2*c - floor(c/16) + floor(c/128) + b)/16)`. The green six-bit value
is `floor((4*c - floor(c/16) + floor(c/64) + b)/16)`. Display conversion repeats
high bits into the low bits: five-bit `v` becomes `(v << 3) | (v >> 2)`, and
six-bit `v` becomes `(v << 2) | (v >> 4)`.

These operations describe the reference rasterizer's precision. They do not
establish identical upstream lighting, texture filtering, triangle coverage,
monitor gamma, or physical Voodoo output.

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
