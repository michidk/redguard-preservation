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

## Palette ownership

Indexed textures use the palette table selected for the draw, including textures
with an embedded BSI `CMAP`. Reading that record does not select or upload a new
palette. World rendering uses the [converted world COL table](../formats/COL.md#glide-world-palette-conversion),
so its six-bit channel precision applies before model texture filtering as well
as terrain filtering. The book menu selects a separate interface palette and
restores the world palette when closing. Sky drawing temporarily selects its
own palette and restores the previously selected table afterward.

Indexed solid-color model faces resolve their color index through the selected
palette when the runtime model is prepared. The resolved RGB is stored in the
runtime face, rather than looked up again for every draw. Direct packed-color
faces follow their separate decoding path.

World entry installs and selects the converted destination palette before
preparing its ROB archive. Indexed solid faces in that archive therefore retain
the converted world RGB, including black palette index zero, before model
lighting. Loading-progress drawing temporarily selects the interface palette
and restores the renderer-selected palette before model preparation continues.

## Placed flat drawing

Ordinary MPSF flats use point-sampled textures and palette chroma-key rejection.
They use constant alpha, independent of texture alpha. An alpha input below
254 selects source-alpha and one-minus-source-alpha RGB blending; an input
of 254 or greater selects source replacement. The constant alpha channel is
the low byte of the rounded alpha input. Alpha-channel blend factors select
source replacement in both cases. The alpha input and its default come from
the [FLAT attributes](../formats/RGM.md#flat).

Their generated local face normal is `(0, 0, -1)`. The camera-facing instance
matrix also transforms local-light positions into the generated quad's local
coordinates before applying the ordinary vertex-light calculation.

These flats retain depth writes and the less-than depth test. Textured RGB
uses the ordinary vertex-light modulation path. Their submitted face brightness
is 255 when the alpha input is below 254; otherwise it comes from the current
flat-brightness input. This is a face-brightness contribution, not a
replacement for the subsequent lighting calculation.

### Flat image dimensions

A centered ordinary flat spans the source image width and height multiplied by
its texture scale and submission geometry scale. Its half extents use half of
those dimensions. Its bounding radius is the quantized source-image diagonal
multiplied by half the same scale. Neither geometry nor bound uses the reduced
texture allocation dimensions, and flat submission has no 64-texel size cutoff.

The texture descriptor rounds allocation dimensions up to powers of two, capped
at 256. Allocation sizes of 128 and 256 use the corresponding reduction factors
from `SYSTEM.INI` `[3dfx] text_scale`. The pair lists the 256-size factor first
and the 128-size factor second. Missing settings default to `4,2`. A factor of
one disables reduction.
Source dimensions and the image's texture scale remain separate from these
allocation and sampling parameters.

### Texture allocation and indexed upload

The Glide allocation descriptor supports power-of-two dimensions from 1 through
256. Its aspect ratio is limited to 8:1: when the requested pair is more
elongated, the smaller allocation dimension increases to one eighth of the
larger. For maximum allocation dimension `M`, both fixed sampling levels are
`8 - log2(M)`.

With unit horizontal and vertical sampling strides, the descriptor's two
coordinate scales are both `256 / M`. Setting source dimensions smaller than
the allocation preserves those scales, including non-power-of-two source
sizes. Source-image dimensions and allocation dimensions remain distinct.

The indexed upload path with unit strides copies source texels row by row.
Unused columns to the right and unused rows below are filled with palette
index zero. It does not stretch or repeat the source image into that padding.
A supplied source row pitch controls the input row stride; the default pitch
is the source width.

For BSI loading, each source axis is rounded up to a power of two before
applying its configured 128- or 256-size reduction factor. Each axis retains
that factor as its sampling stride. Aspect-ratio padding follows reduction.
For source dimension `D`, sampling stride `S`, and maximum reduced allocation
dimension `M`, the coordinate scale is `256 * floor(D / S) / (M * D)`, stored
as a 32-bit float. Source dimensions themselves are retained without reduction.
The fixed sampling level is `8 - log2(M)`.

Indexed reduction samples source columns and rows starting at zero and stepping
by their respective strides. It does not average neighboring texels. Remaining
allocation columns and rows are still filled with palette index zero.

### Texture address selection

Glide initialization sets texture unit zero to wrap in both directions. Ordinary
model binding retains that address mode. Wrapping uses the allocation dimensions,
including zero-filled padding, rather than the original source-image dimensions.

In the DOSBox-X Voodoo path, point filtering selects an integer texel from each
signed perspective coordinate `Q` by arithmetic shift right by `level + 18`,
then masks with the corresponding allocation dimension minus one. It does not
round to the nearest integer. Bilinear filtering instead selects the neighboring
pair and four-bit fractions using the
[fixed texture address rule](terrain-lighting.md#texture-coordinates-and-setup).
The texture's fixed level is the allocation level described above.

### Flat submission order

Ordinary placed flats prepend to the spatial list selected by the low sixteen
bits of their integer X/Z placement coordinates, shifted right by nine bits.
Their active list appends those cell lists in the normal
[runtime spatial selection order](#runtime-object-light-region). Within each
cell, this reverses file insertion order. Static models are submitted before
these flats. Bound rejection happens before queue insertion; polygon clipping
happens later, while drawing the queue.

The normal queue's alpha partition does not compare depth. For ordinary flats,
alpha below 254 selects the partitioned category. When that category is present,
a tail cursor starts at the final queue entry and skips selected entries while
it is greater than zero. A forward cursor visits indices strictly below
`queue_length - 1 - selected_count`. Whenever the forward entry is selected,
it swaps with the tail entry, then decrements the tail and again skips selected
entries while greater than zero. The resulting order is not stable and need not
place every selected entry after every other entry. With an opaque model prefix,
partitioning the flat suffix independently gives the same flat order.

### Flat framebuffer blending

Ordinary flat drawing enables four-by-four framebuffer dithering and alpha
dither subtraction. In the software Voodoo 1 renderer, constant-alpha blending
uses the same [integer destination reconstruction and blend arithmetic as the
sun](sky.md#sun-framebuffer-blending), with the flat's constant eight-bit alpha
in place of filtered texture alpha. Source RGB is the encoded, lit scene color.
The separately truncated source and destination terms are added before final
RGB565 framebuffer quantization. Depth testing and depth writes remain enabled
for these blended fragments.

## Draw-flag ownership

A newly initialized render instance has zero draw flags and constant alpha 255.
Animated actor group loading checks the first group's name against the loaded
ROB model lists. When that lookup succeeds, it copies the segment attribute
byte into the actor's render instance. It also replaces constant alpha when the
segment alpha is between 1 and 253 inclusive. A missing archive entry leaves
those initialized values intact. Later groups do not repeat this assignment.
These are group-loading assignments, not per-frame filtering defaults.

Ordinary static-model submission refreshes the instance's draw flags and constant
alpha from the selected archive model's prepared metadata before queueing it.
The archive loader copies the segment attribute byte into that metadata and
maps segment alpha 0, 254 and 255 to constant alpha 255. Other segment alpha
values are retained. Static-model name lookup searches the loaded ROB model
lists and reports an error when no entry exists; it does not use the standalone
animated-model file loader as a fallback.

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

The runtime face's brightness is the byte following its vertex count. Model
preparation shifts the stored face byte left by two bits and retains the low
eight bits. In v4.0/v5.0 model files the stored byte is the field named `tex_hi`
in the [model format](../formats/models/3d.md), so the runtime contribution is
`(tex_hi * 4) & 255`. Generated runtime faces supply their brightness directly
and do not pass through this file-model conversion. The packed runtime material
marks solid colors separately from texture references.

A model instance may retain precomputed per-face-vertex RGB. That RGB replaces
the ambient, face-brightness and directional contribution, including its solid
color tint. Local lights still add to it, followed by channel clamping; the
solid tint is not applied a second time.

## Polygon clipping

The clipped model preparation path computes each source corner's RGB before
clipping. It transforms positions into projection-scaled camera coordinates
and uses the same six-plane volume and plane order as
[terrain polygon preparation](terrain-lighting.md#polygon-preparation).

At a crossing edge, RGB and texture coordinates use the same linear edge
parameter as camera position. Generated components are stored as 32-bit
floats. RGB is not multiplied by depth or reciprocal depth during clipping.
Projection and screen-space RGB interpolation occur after this preparation.
Consequently, clipping depth-weighted RGB and dividing it by the new depth
is not equivalent to the model preparation rule.

## Polygon projection

The ordinary model submission path projects the surviving clipped polygon as
an ordered triangle fan. The first corner remains the anchor; each subsequent
pair of corners forms the next triangle.

For each corner, reciprocal camera depth is stored as a 32-bit float before
projection. Screen X and Y use that stored reciprocal, the viewport center
and the corresponding viewport scale. Screen X has a 32-bit float store before
adding the bias `786432`; Y retains the wider intermediate until the biased
store. Both biased screen coordinates are stored as 32-bit floats. This is the
same `1/16`-pixel quantization used by
[terrain projection](terrain-lighting.md#polygon-preparation).

The submitted corner retains its clipped RGB. Its vertex alpha is `255`.
Each texture coordinate is multiplied by the stored reciprocal depth, and
that product is stored as a 32-bit float for Glide. Reciprocal depth is also
submitted separately for perspective correction.

## Screen-space color setup

Ordinary model fans use the same Glide triangle color-gradient setup as
[terrain](terrain-lighting.md#screen-space-color-interpolation). The submitted
RGB values become separate starting-color and screen-gradient registers after
screen-Y ordering. Each fan triangle receives its own setup.

The installed Glide driver's iterated-color and texture-times-iterated-color
modes enable subpixel adjustment whenever the color-combine state is set.
This does not depend on the prior subpixel-enable bit. Window initialization
clears the RGB/depth clamp state; color-combine changes preserve that state.
The software Voodoo renderer's signed color conversion and subpixel adjustment
therefore apply to the default model path as well as terrain.

Ordinary textured model drawing selects texture-times-iterated-color. Its
encoded texture-byte modulation uses the same
[integer texture combination](terrain-lighting.md#texture-combination) as
terrain. Solid-color submission uses iterated RGB without a texture product.

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
extras object. Version 5 has `version: 5`, the model header radius as
`radius_fixed` in 1/256 engine units, and a `vertices` array in exported
vertex order. `solid_rgb` holds the resolved palette RGB for solid-color
primitives and is null for textured primitives. Retained-color baking applies
this RGB before integer rounding. Each entry is `[brightness, solid]`, where
brightness is the converted runtime byte, `(tex_hi * 4) & 255`, and solid is 1
for palette-color faces or 0 for textured faces. Version 3 retained the stored
byte and requires re-export for the runtime-brightness contract. Version 5 also
preserves all referenced authored corner normals when their count exceeds the
position count. Earlier exports can substitute face normals for those omitted
records and require re-export. Material grouping and polygon triangulation retain these inputs for
every emitted vertex. Terrain primitives omit this metadata.

Indexed solid-color primitives retain `rgpre_solid_palette` with `version: 1`
and the original eight-bit `index`. Different indices remain separate primitives
when their resolved RGB values match. Generic material colors remain unchanged;
engine renderers can distinguish world index zero before applying palette
conversion and lighting. Textured primitives omit this metadata.

Textured model primitives also retain `rgpre_texcoords` version 1 with a `pixels`
array in exported vertex order. Each pair is the decoded model coordinate divided
by 16, in original source-image pixel units, before dimension normalization or
vertical reversal. Negative and out-of-image coordinates are retained. Material
splitting and triangle fans duplicate the corresponding pairs. `TEXCOORD_0`
remains normalized and vertically reversed for generic GLTF viewers. Terrain and
solid-color primitives omit `rgpre_texcoords`. Generated flat fallback meshes may
carry these pairs, but their `rgpre_flat` node metadata defines their separate
runtime geometry and coordinate path.

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

## Script light state and cleanup

Actor-attached light records retain a lock flag, an enabled flag and an
unsigned byte inactivity counter. During light maintenance, a nonzero lock
forces the enabled flag on.

`LightOff` returns zero. If the actor has no light record, it does not allocate
one. Otherwise it clears the lock, enabled flag and inactivity counter without
changing the retained offsets, intensity, radius, colors or modulation settings.
It does not immediately release the record.

While a record is disabled, maintenance adds the low byte of the elapsed tick
count to its inactivity counter with byte wrapping. It releases the record and
clears the actor's association only when the resulting unsigned counter exceeds
32. Enabled records do not accumulate this inactivity counter.

`Light(radius, intensity)` enables the record and replaces both the starting and
current radius and intensity. `LightRadius` and `LightIntensity` each replace
both copies of their respective value, enable the record and clear its inactivity
counter. These setters store the low sixteen bits of their arguments.

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
