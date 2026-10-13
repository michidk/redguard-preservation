# Texture animation

Glide texture binding advances animated TEXBSI images on demand.

Each loaded image retains a current frame and a next-update deadline. The frame
starts unset and the deadline starts at zero. Frames belonging to one image
share this state wherever the image is drawn.

An automatic frame request selects frame zero for a single-frame image. For an
animated image, it advances by one frame when the BIOS tick count has reached
the deadline, or when the current frame is unset. Advancing past the final
frame wraps to zero. The next deadline becomes the current tick count plus
the image's frame delay. The first automatic request therefore selects frame
zero, even if its previous deadline lies in the future.

The delay comes from the TEXBSI header's `anim_delay`: milliseconds multiplied
by 18.2 and divided by 1000, rounded to an integer tick count. A result of zero
becomes one tick. See the [TEXBSI header](../formats/TEXBSI.md#bhdr--image-header-26-bytes).

A late request advances only once. It does not catch up through all frames
whose durations elapsed. An image that is not bound does not advance merely
because time passes. Repeated bindings before the next deadline retain the
same frame.

An explicit frame request selects that frame and replaces the retained current
frame without changing the deadline. A later automatic request continues from
that selected frame. This state belongs to the loaded texture image, not to
an individual mesh instance.

## Export metadata

Animated glTF materials carry `extras.rgpre_texture_animation` with version 1.
It contains the original `image_name`, signed header `delay_ms`, and `textures`,
an ordered array of glTF texture indices including frame zero. The base-color
texture remains frame zero for consumers without animation support. Static
images omit this metadata.

Every frame uses the same palette, row orientation and sampler as frame zero.
The generic material retains alpha masking when any exported frame contains
transparent pixels. Runtime render attributes can impose a different chroma
policy. Consumers sharing one loaded original image must share its playback
state even when different materials reference separate exported copies.

Model glTF primitives also carry `extras.rgpre_admission` with version 1. Its
`radius_fixed` retains the model header radius without conversion. `polygons`
contains the primitive's authored polygons in source order. Each entry has
`source_face`, the zero-based model face index; `vertices`, indices into that
primitive's POSITION accessor in original polygon order; and `normal`, the
authored base-frame face normal in glTF export axes without normalization.
The normal is independent of the vertex normals used for lighting.

Material splitting and fan triangulation preserve these records for both static
and animated models. Coincident authored vertices remain separate polygon
corners. Animated consumers use the source face identity to select the current
frame's normal, and the vertex mapping to read the updated mesh positions.
A missing or nonfinite authored normal is `null`. If invalid source vertex
indices prevent a complete polygon mapping, `vertices` is `null`; consumers
must not infer faithful admission from the remaining raster triangles.
Terrain primitives omit this model-specific metadata.
