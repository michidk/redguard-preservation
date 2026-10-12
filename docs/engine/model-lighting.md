# Glide model lighting

Model lighting supplies encoded RGB at each polygon vertex before rasterization.

## Ambient and directional contribution

Models share the ambient RGB, directional RGB and normalized world direction
specified in [terrain-lighting.md](terrain-lighting.md). The model transform
maps the world direction into model coordinates using the transpose of its
linear matrix. Shading consumes the supplied model normal without normalizing
it again.

Each face adds its unsigned brightness byte to all three ambient channels.
For ambient `A`, face brightness `B`, directional strength `S`, transformed
direction `D` and supplied normal `N`, the contribution in each channel is
`A + B + max(0, -dot(D, N)) * S`.

Local lights add to this contribution before each channel is clamped to
`0..255`. For a solid-color face, the clamped channels are then multiplied
by the corresponding palette RGB channel divided by 256. Textured faces
use the resulting RGB to modulate the texture. RGB interpolates in screen
space, while texture coordinates use perspective correction. There is no
physical material response, scene exposure or filmic tone mapping.

The runtime face's brightness is the byte following its vertex count. In
v4.0/v5.0 model files this is the field named `tex_hi` in the
[model format](../formats/models/3d.md). The packed runtime material marks
solid colors separately from texture references.

A model instance may retain precomputed per-face-vertex RGB. That RGB replaces
the ambient, face-brightness and directional contribution, including its solid
color tint. Local lights still add to it, followed by channel clamping; the
solid tint is not applied a second time.

## Local-light model selection

A local light carries a world position, signed RGB strengths and a radius in
engine coordinates. Registration retains its squared radius and an attenuation
numerator `(radius * radius + 3 * radius) / 256`.

For model origin `O`, model radius `R`, light position `L` and light radius `r`,
the light is selected when `dot(L - O, L - O) - R * R <= r * r`.
The boundary is inclusive. This differs from testing whether two spheres overlap.
The model radius is the stored model bound; the selection test does not scale it
by the instance transform.

Selected light positions are transformed into model coordinates by multiplying
`L - O` by the transpose of the model's linear matrix. This uses the matrix
entries directly, including any scale.

## Local-light vertex contribution

For a selected light at transformed model-space position `L`, let
`V = vertex_position - L` and `q = dot(V, V)`. Normalize `V` with the
quantized reciprocal square root described in
[terrain lighting](terrain-lighting.md), retaining the
supplied model normal's magnitude. Only a negative dot product between that
normalized vector and the supplied normal contributes light.

The distance calculation uses a separate quantized square-root table. For
unbiased exponent `e` of `q` and its upper seven fraction bits `m`, the index is
`m + 128 * (e & 1)`. Table entry `i` contains the upper seven fraction bits of
the single-precision square root of `(1 + (i % 128) / 128) * 2^(i / 128)`,
where division of the index by 128 is integer division. Combine those bits
with exponent `floor(e / 2) + 127` to obtain distance `d`.

The attenuation is `(radius * radius + 3 * radius) / (256 * (q + 3 * d))`.
A negative attenuation contributes nothing. Multiply a nonnegative attenuation
by the negated normal dot product, then multiply each signed RGB strength by
that weight and add it to the current channel. There is no per-vertex cutoff
at the light radius. A vertex coincident with the light contributes nothing
because its normalized delta and dot product are zero.

Accumulate local lights before clamping and solid-color tinting. Retained
instance RGB receives local contributions and clamping without a second tint.
The distance-table initializer selects single-precision x87 arithmetic with
round-to-nearest; intermediate arithmetic rounds to single precision. The
attenuation denominator evaluates as `(2 * d + q) + d`; its numerator evaluates
as `((2 * radius + radius * radius) + radius) / 256`.

## Retained instance colors

Prelighting traverses faces in authored order and vertices in each face's
order. It uses each face's brightness and material together with the vertex
position and supplied normal, through the same model-lighting calculation.
The resulting clamped, tinted RGB channels are rounded to integers with
round-to-nearest, ties to even, then packed as `0x00RRGGBB`. The retained
array contains one packed entry per face vertex, including repeated mesh
vertices shared by different faces. An existing instance buffer is reused
when prelighting is recalculated.

## Export metadata

GLTF model primitives retain per-vertex face inputs in the `rgpre_lighting`
extras object. Version 1 has `version: 1` and a `vertices` array in exported
vertex order. Each entry is `[brightness, solid]`, where brightness is the
unsigned face byte and solid is 1 for palette-color faces or 0 for textured
faces. Material grouping and polygon triangulation retain these inputs for
every emitted vertex. Terrain primitives omit this metadata.

## Local-light records

`RgmFile::local_lights(world_id)` returns admitted authored records with signed
RGB, radius and positions in engine coordinates. RGM inspection metadata retains
these values and the activation and world fields. Placement extraction does not
convert them to standard punctual lights: their signed contribution and custom
falloff require engine shading.
