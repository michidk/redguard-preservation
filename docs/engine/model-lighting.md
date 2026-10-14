# Glide model lighting

Model lighting supplies encoded RGB at each polygon vertex before rasterization.

## Texture transparency

The queued-model draw flags come from the ROB segment's attribute byte for
ordinary archive model submission. Bit 0 selects point filtering instead of
bilinear; bit 1 enables chroma-key rejection for textured faces. See
[ROB segment attributes](../formats/ROB.md#segment-attributes-0x10).

The software Voodoo 1 pipeline compares the filtered texture RGB with the
chroma-key RGB before applying vertex-light modulation. A matching RGB is
rejected. Filtering uses the encoded palette colors, including the key color;
it does not interpolate a transparent alpha mask and apply an alpha cutoff.

Display initialization selects indexed palette texture mode. Each texture's
chroma key is the selected palette's entry-zero RGB, copied without channel
quantization. Palette selection uploads the complete 256-entry RGB table to
texture unit zero. Palette-zero texels retain their RGB through filtering even
when the material enables chroma rejection.

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

## World prelighting

World prelighting is enabled by default. The `npl` command-line option disables
it. The world-entry pass requires both a nonempty static-object list and a
nonempty admitted map-light list. It registers the map lights and bakes retained
face-vertex colors for the static instances. With either list empty, this pass
does not bake those instances.

For world-entry prelighting, spatial cells are allocated in first-occurrence
order: static placements first, then admitted map lights. The cell key is
`((z & 0xfe00) >> 2) + ((x & 0xffff) >> 9)` using the low coordinate bits.
Lights prepend to their cell's list. Collection concatenates cells in ascending
allocation order, retaining reverse insertion order within each cell. Light
registration and model selection preserve this order for accumulation.

Ordinary world drawing registers the map-light list only in editor mode.
Outside that mode, the retained colors carry the baked map-light contributions;
the map lights are not reapplied by that draw loop. Other registered local lights
can still contribute through the retained-color shading path.

## Export metadata

Textured GLTF materials retain their effective palette-zero RGB in
`rgpre_texture` extras with `version: 1` and `chroma_key_rgb: [r, g, b]`.
Embedded PNGs preserve those RGB bytes at alpha-zero texels. The alpha marker
supports generic viewers; engine renderers use the RGB key after filtering.

Positioned-model GLTF nodes retain the selected ROB segment attribute byte in
`rgpre_render`, with `version: 1` and `rob_attributes`. The attribute is null
for standalone model files and generated geometry. The loader retains it from
the same successful archive segment as the geometry, including fallback
selection and cached loads. Node metadata keeps instance attributes separate
when exported instances share one mesh.

MPSF nodes additionally carry `rgpre_flat` with `version: 1`, `texture_id`,
`image_id`, source-image `dimensions: [width, height]`, `texture_scale`,
submission `flags`, and integer `alpha`. Dimensions and scale are null when
texture data is unavailable. Scale includes the zero-to-one default and the
BSI fixed-point conversion. Flags include the centered-anchor base flag; alpha
includes the non-wrapping attribute increment or the default 255. See
[flat placement and attributes](../formats/RGM.md#mpsf-flat-objects).

The flat metadata identifies a separate engine drawing path. Its dimensions
and scale define camera-facing geometry; the generated mesh is a generic-viewer
fallback, not authored model geometry. ROB model lighting and polygon-admission
metadata do not describe flat drawing. Malformed FLAT sections produce placement
diagnostics instead of flat nodes with assumed defaults.

GLTF model primitives retain per-vertex face inputs in the `rgpre_lighting`
extras object. Version 3 has `version: 3`, the model header radius as
`radius_fixed` in 1/256 engine units, and a `vertices` array in exported
vertex order. `solid_rgb` holds the resolved palette RGB for solid-color
primitives and is null for textured primitives. Retained-color baking applies
this RGB before integer rounding. Each entry is `[brightness, solid]`, where
brightness is the unsigned face byte and solid is 1 for palette-color faces or 0 for textured
faces. Material grouping and polygon triangulation retain these inputs for
every emitted vertex. Terrain primitives omit this metadata.

## Local-light records

`MPOB` object kind 6 defines a runtime local light. With `objects_load` enabled, its
signed world ID must match the current world; its activation byte does not
gate this branch. The loader initializes the light as locked and enabled,
with zero positional offset and no intensity or radius modulation. Current
and starting intensity, radius and signed RGB come from the authored record.

These objects enter spatial-cell lists separately from `MPSL` map lights.
Outside editor mode, world drawing submits their enabled, positive-radius
lights from the active object list before drawing static models. Their current
signed RGB and radius contribute through the local-light calculation, including
when a static model already has retained colors. They are not part of the
world-entry map-light bake.

`RgmFile::object_lights(world_id)` returns these admitted authored records in
file order, before runtime script changes and active-region selection.

## Runtime object-light region

The normal world loop selects spatial lists using the target camera's integer
X/Z coordinates before camera smoothing. Each axis starts at `target - 4608`.
If that value is negative, the start is zero; otherwise, when `target + 4608`
exceeds 65535, the start is 56319. Shift each start right by nine bits to obtain
the first cell coordinate.

Selection visits an 18-by-18 cell square in row order, omitting its four corners.
The grid has 128 cells per row. Within a cell, object lights follow reverse
insertion order. Lists are reused while the first selected cell remains the
same. The final rendered camera position can differ from the target used for
this selection.

## Map-light records

`RgmFile::prelight_lights(world_id)` returns the world-entry accumulation order
when static loading is enabled. `RgmFile::local_lights(world_id)` returns
admitted authored records in file order with signed
RGB, radius and positions in engine coordinates. RGM inspection metadata retains
these values and the activation and world fields. Placement extraction does not
convert them to standard punctual lights: their signed contribution and custom
falloff require engine shading.
