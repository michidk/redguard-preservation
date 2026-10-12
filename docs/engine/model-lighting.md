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

## Export metadata

GLTF model primitives retain per-vertex face inputs in the `rgpre_lighting`
extras object. Version 1 has `version: 1` and a `vertices` array in exported
vertex order. Each entry is `[brightness, solid]`, where brightness is the
unsigned face byte and solid is 1 for palette-color faces or 0 for textured
faces. Material grouping and polygon triangulation retain these inputs for
every emitted vertex. Terrain primitives omit this metadata.
