# Glide terrain lighting

Terrain receives an RGB lighting value at each vertex, after its normals are
computed and before rasterization.

## World inputs

World loading sets the Glide ambient channels from
`world_ambientrgb[N]`. If that entry is absent, the channels default to
`128,128,128`.

The directional-light channel strengths are the first three components of
`world_sunrgb[N]`, defaulting to `128,128,100` when the entry is absent.
The fourth component does not multiply those channels in the terrain lighting
calculation. World loading constructs the direction using `world_sunangle[N]`
and `world_sunskew[N]` and the engine's trigonometric tables. The direction
setter normalizes a nonzero vector. A zero vector disables directional lighting.

These are the Glide terrain inputs. The `world_sun[N]` vector and intensity,
`world_ambient[N]`, and `world_ambientfx[N]` are not operands of this terrain
vertex calculation. The visible sun image has separate tint inputs described
in [sky.md](sky.md).

The installed ISLAND day entry uses ambient RGB `64,64,64` and directional RGB
`255,255,255`. Its `world_sunrgb` fourth component is 2, which does not double
those directional RGB values.

## Vertex calculation

Let `A` be the ambient RGB vector, `S` the directional RGB
strength, `D` the normalized light direction, and `N` the supplied terrain
vertex normal, expressed in the same renderer coordinate system.

The directional weight is `max(0, -dot(D, N))`. With directional lighting
disabled, the weight is zero. Each output channel is
`clamp(A[channel] + weight * S[channel], 0, 255)`.

The routine consumes the supplied normal without normalizing it again.
It retains floating-point RGB values at this stage. It does not apply
material roughness, specular lighting, inverse-square attenuation, or the
model renderer's local-light list to this terrain calculation.

For ambient `64,64,64`, directional strength `255,220,200`, and direction
`0,1,0`, a normal of `0,-1,0` gives `255,255,255` after clamping. A normal of
`0,1,0` gives `64,64,64`. A normal of `-0.6,-0.8,0` gives approximately
`255,240,224`.

## Terrain updates

Rebuilding the camera's terrain grid recomputes face normals,
then vertex normals, then vertex lighting. The wave update repeats these
stages for dirty terrain entries after changing heights. Unmarked entries retain
their earlier normals and RGB, including the omitted diagonal at water edges.
The admission set and rebuild lifetime are specified in
[water.md](water.md#dirty-normals-and-retained-lighting).

## Direction and exported coordinates

The direction before normalization is formed from the shared
float rotation table. Its components are `sin(angle) * 16000`,
`sin(angle + 512) * 16000`, and `sin(skew) * 16000`, each truncated toward zero
to an integer. The table sample at index `n` is
`sin(n * 6.2831852 / 2048)`, stored as a 32-bit float. Missing angles are zero.

For ISLAND day, angle 256 and skew 512 produce the integer vector
`11313,11313,16000`, whose normalized direction is approximately
`0.499984324,0.499984324,0.707128882`.

For the interior terrain rows, source row `r` corresponds to engine grid
row `256-r`. The GLTF terrain position uses negative source X, positive
height, and negative source row, with 20 engine units per exported unit.
Consequently direction and normal components transform as `[-X,-Y,+Z]`.
The Z origin translation does not affect directions or normals.

## Shading normals

Each terrain cell is split along the diagonal joining its
negative-X, positive-Z corner to its positive-X, negative-Z corner.
Each triangle's cross product is normalized separately. A vertex normal is
the normalized sum of the six incident triangle normals. Triangle areas
therefore do not weight the sum.

Both normalization stages use a quantized reciprocal square root. For a
positive squared length stored as a 32-bit float, let `e` be its unbiased
binary exponent and `m` the upper seven fraction bits. Select index
`i = m + 128 * (e & 1)`. The 256-entry table stores the upper seven fraction
bits of the 32-bit float reciprocal square root of
`(1 + m/128) * 2^(i/128)`. Lower fraction bits are zero.
Combine these table bits with exponent `floor((253-e)/2)` when the table
entry is nonzero, or `floor((254-e)/2)` when it is zero. Multiply the vector
by the resulting float. These exponents are biased float exponents.

A flat face normal is `0,-1,0`; the sum of six such faces normalizes to
`0,-0.99609375,0`. That supplied magnitude is retained by the lighting
calculation. With ISLAND day inputs, the resulting flat-terrain RGB is
approximately `190.99797,190.99797,190.99797`.

## Polygon preparation

Terrain triangles are clipped before projection and Glide submission. In the
projection-scaled camera coordinates used by the terrain path, the accepted
volume is `near <= z <= far`, `-z <= x <= z`, and `-z <= y <= z`.
Points on a plane are inside. Planes are processed in near, far, left, right,
top, bottom order.

At a crossing edge, camera position, RGB and the two texture coordinates use
the same linear edge parameter. Each generated component is stored as a 32-bit
float. Near/far intersections set depth to the corresponding plane. Left/right
intersections derive depth from the stored X coordinate; top/bottom derive it
from Y. RGB is not perspective-weighted during this clipping step.

The surviving polygon is submitted as a triangle fan, anchored at its first
vertex. Projection follows clipping. Each projected screen coordinate is
biased by `786432` and stored as a 32-bit float before Glide receives it.
The unbiased X coordinate has an additional 32-bit float store before this
addition; Y remains in the wider intermediate precision until the biased store.
The float spacing at that magnitude is `1/16` pixel. The common bias cancels
from the gradient differences and is discarded by the software rasterizer's
signed 16-bit fixed-coordinate conversion. Triangle color gradients therefore
use these quantized projected positions.

Each fan triangle receives its own Glide color-gradient setup. Clipping a
previously established screen-space color plane does not reproduce this
camera-space RGB interpolation and subsequent per-triangle setup.

## Screen-space color interpolation

Glide triangle setup orders vertices by screen Y. For input vertices `0`, `1`,
and `2`, the following table specifies the order, including equal-Y cases:

| Condition | Ordered vertices |
| --- | --- |
| `y0 < y1` and `y1 <= y2` | `0, 1, 2` |
| `y0 < y2 < y1` | `0, 2, 1` |
| `y2 <= y0 < y1` | `2, 0, 1` |
| `y2 <= y1 <= y0` | `2, 1, 0` |
| `y1 <= y0 < y2` | `1, 0, 2` |
| `y1 < y2 <= y0` | `1, 2, 0` |

This is not a stable sort. In particular, a top horizontal edge selects the
later of its two input vertices as the color anchor. The anchor matters after
fixed-point conversion, even when an unquantized affine plane is unchanged.

For ordered vertices `A`, `B`, and `C`, define screen-coordinate differences
`AB = A - B` and
`BC = B - C`, and determinant `D = AB.x * BC.y - BC.x * AB.y`.
A zero determinant produces no triangle. The four coordinate differences
divided by `D` are stored as 32-bit floats. For each color channel, setup
writes the channel at `A` and these gradients, also stored as 32-bit floats:

```text
dLdx = (LA - LB) * (BC.y / D) - (LB - LC) * (AB.y / D)
dLdy = (LB - LC) * (AB.x / D) - (LA - LB) * (BC.x / D)
```

In the software Voodoo 1 renderer, screen coordinates are converted to signed
16-bit fixed point with four fractional bits. Starting color and each gradient
are converted separately to signed 24-bit fixed point with twelve fractional
bits. These conversions truncate toward zero before narrowing.

The terrain color path enables subpixel adjustment. Let `ax` and `ay` be the
fixed-point coordinates of `A`, and let `S`, `Gx`, and `Gy` be a channel's
fixed-point start and gradients. Before rasterization, the start becomes:

```text
S = S + floor(((8 - (ax & 15)) * Gx + (8 - (ay & 15)) * Gy) / 16)
```

For integer screen pixel coordinates `(x, y)`, the channel accumulator is:

```text
I = S + (x - floor(ax / 16)) * Gx + (y - floor(ay / 16)) * Gy
```

The channel integer is obtained by arithmetic right shift of `I` by twelve
bits. The Voodoo 1 color conversion keeps the low twelve bits, maps `4095`
to zero and `256` to `255`, and otherwise keeps the low eight bits.
A constant channel has zero gradients and remains constant across the triangle.
Quantizing an interpolated floating-point channel only at the fragment does
not reproduce the separately quantized start, gradients and subpixel origin.

## Texture combination

The terrain draw selects Glide's texture-times-iterated-RGB
color-combine mode. Lighting therefore modulates texture colors before scene
fog. Vertex RGB is interpolated across screen space; texture coordinates use
perspective correction. Terrain does not pass through a scene exposure or
filmic tone-mapping stage.

The ordinary terrain pass uses bilinear minification and magnification.
Terrain texture binding disables chroma-key rejection. Palette index zero
therefore remains an opaque color on terrain.

In the software Voodoo 1 renderer, texture modulation operates on encoded
8-bit RGB channels. The interpolated lighting channel is converted to an
integer before combination. For texture channel `T` and lighting channel `L`,
the result is `floor(T * (L + 1) / 256)`, before fog.
