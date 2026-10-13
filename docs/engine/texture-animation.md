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

The model draw path prepares a polygon before binding its texture. It binds only
when preparation returns at least three vertices. Earlier object and face
rejection still applies.

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
