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

## Loaded-image lifetime

The loaded-image cache identifies names with a six-digit base-40 key. Its digit
alphabet, in ascending order, is `0123456789abcdefghijklmnopqrstuvwxyz~_#%`.
ASCII uppercase letters are converted to lowercase. Conversion stops at the
first null byte, period, character outside that alphabet, or after six
characters. Missing trailing digits are filled with `%`. The first character
is the most significant digit. Characters after the stopping point do not
distinguish cache entries, and the source bank path is not part of the key.

Looking up an already loaded image reuses its texture record and increments its
reference count. It does not reset the current frame or animation deadline,
and does not add the active resource group's ownership bit.

Loading a TEXBSI bank visits every image record in the bank. For each already
loaded image, it increments the reference count and adds the selected group's
ownership bit without resetting animation state. Newly decoded images receive
the selected group's ownership bit. Bank ownership therefore includes images
that have not been requested by a model or terrain polygon. Repeated records
increment references again while retaining the same ownership bit.

Images can belong to multiple resource groups. Releasing a group clears that
group's ownership bit and releases its reference. A record retained by another
group keeps its animation state. When no ownership bits remain, group cleanup
forces final release even if repeated lookups within that group increased the
reference count. Final release frees every frame and clears the image record.
A subsequently loaded image receives the initial frame and deadline state.

Book-menu setup saves the selected resource group and selects group 30 for
menu loading. Menu cleanup restores the saved selection. Its release argument
controls group release: zero retains group 30, and any nonzero value releases
it. Closing the book menu requests release; preparing its contents first runs
cleanup without release. Releasing menu ownership preserves an image's animation
state when another resource group still owns it.

The banks referenced by `MENU.ROB`, `INVENTRY.ROB` and the configured book-page
surfaces contain only single-frame texture images. Their image keys do not
match any animated image in the shipped TEXBSI banks. Shipped TEXBSI image
names consist of six ASCII letters or digits.

World-resource cleanup releases group 31 when world resources are loaded, then
clears the loaded state. Repeating cleanup with that state already clear does
not release the group again. World transitions perform this cleanup before
loading the destination world's resources.

## Model polygon admission

Before model polygons are prepared, draw submission classifies a bound around
the model origin transformed into scaled camera space. Let `(x, y, z)` be that
origin, `r` the model header radius in engine units, and `(sx, sy)` the
[projection scales](projection.md). Its depth interval is `[z-r, z+r]`; its
horizontal and vertical intervals are `[x-hx, x+hx]` and `[y-hy, y+hy]`, where
`hx = 1.415 * (r*sx)` and `hy = 1.415 * (r*sy)`. The decimal multiplier is
`1.415`. The scaled radii, multiplied side radii and interval endpoints are
stored in single precision.

Submission rejects a depth interval whose upper endpoint is at or before the
near plane, or whose lower endpoint is at or beyond the far plane. It rejects
a side interval whose upper endpoint is at or below `-z`, or whose lower
endpoint is at or above `z`. Otherwise it records each plane crossed by an
interval. An endpoint exactly on a plane does not count as crossing it.
No crossed planes selects the unclipped polygon preparation path; any crossed
plane selects the clipped path. A rejected bound does not enter the draw list.

Ordinary model drawing visits the authored polygons in order. A polygon passes
its facing test only when the vector from the local-space camera position to
its first authored vertex has a strictly negative dot product with the
selected frame's face normal. A zero result rejects the polygon. Both shaded
and unshaded drawing use this test. Solid-color polygons do not bind a texture.

Queued-model draw flag `0x04` selects a separate explicit-frame path. This path
prepares only the model's first polygon, bypasses the ordinary facing test and
uses the instance's explicit texture-frame selection. Ordinary drawing requests
automatic frame selection. Shipped ROB segment attribute bytes use only
`0x00`, `0x02` and `0x40`; none selects the explicit-frame path when copied
unchanged into the queued-model draw flags.

Both paths prepare a polygon before binding its texture. They bind only when
preparation returns at least three vertices. Earlier object rejection still
applies.

The clipped preparation path transforms vertices into camera space with the
horizontal and vertical [projection scales](projection.md) already applied.
It classifies each vertex against the near and far depth planes and the four
side planes `x = -z`, `x = z`, `y = -z`, and `y = z`. Points exactly on a
plane are inside. A polygon whose vertices are all outside the same plane is
rejected immediately. Otherwise, clipping visits the near, far, left, right,
negative-Y and positive-Y planes in that order, as needed.

Preparation returns the remaining vertex count. It does not require projected
area or covered pixels: coincident vertices can still satisfy the count check.
The unclipped preparation path retains the input vertex count. Thus texture
binding admission is distinct from both an object's visibility bound and the
eventual rasterized coverage.

## Terrain triangle admission

Terrain submits the cells in its [camera window](water.md#terrain-window).
Each cell tests its two triangles separately. The dot product of the vector
from the camera to the cell's anchor vertex with that triangle's stored face
normal must be strictly negative. A zero result rejects the triangle.

An admitted triangle copies its three prepared camera-space vertices and
classifies them against the same six clipping planes used for model polygons.
A common outside plane rejects it. Otherwise it clips as needed and requires
at least three remaining vertices before binding the cell's texture with an
automatic frame request. The texture request precedes polygon drawing.
Clipping boundaries are inclusive, and coincident vertices can still produce
an automatic request. Rejected or back-facing terrain triangles do not advance
their texture image.

## Export metadata

Textured glTF material names identify the resolved TEXBSI or TEXTURE bank,
original image name and image ID.

Animated glTF materials carry `extras.rgpre_texture_animation` with version 1.
It contains the original `image_name`, signed header `delay_ms`, and `textures`,
an ordered array of glTF texture indices including frame zero. The base-color
texture remains frame zero for consumers without animation support. Static
images omit this metadata.

Every frame uses the same palette, row orientation and sampler as frame zero.
Model materials retain alpha masking when any exported frame contains
transparent pixels. Terrain materials and every exported terrain frame remain
opaque, including palette index zero, whose RGB comes from the selected palette.
Terrain and model uses of the same image have separate exported textures.
Runtime model render attributes can impose a different chroma policy. Consumers sharing one loaded original image must share its playback
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
