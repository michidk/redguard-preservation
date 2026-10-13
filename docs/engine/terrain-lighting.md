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

## Texture combination

The terrain draw selects Glide's texture-times-iterated-RGB
color-combine mode. Lighting therefore modulates texture colors before scene
fog. Vertex RGB is interpolated across screen space; texture coordinates use
perspective correction. Terrain does not pass through a scene exposure or
filmic tone-mapping stage.
