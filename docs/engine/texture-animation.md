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
