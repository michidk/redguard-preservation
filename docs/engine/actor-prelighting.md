# Actor prelighting

World entry can retain map-light RGB on eligible static-model actors.

The world-entry prelighting pass requires prelighting to be enabled, a nonempty
static-placement list and a nonempty admitted map-light list. The `npl`
command-line option disables this pass. The same list requirements gate both
static placements and actors; actors alone do not enable it.

An actor qualifies when attribute 122, `attr_non_moving`, is nonzero and its
runtime model kind selects the static-model path. This includes the
[actor-definition shortcut](animation.md#placed-actor-initialization-and-eligibility)
and the placed actor's explicit static-model fallback; eligibility does not
depend on which loader selected that runtime kind. The static model must exist.
Actors using an animation model table do not qualify, even when they have only
one frame or do not currently move.

The pass prepares an instance for the actor's static model and uses the same
face-vertex prelighting operation as static placements. Its retained color
buffer is registered for world-level release together with the static-placement
buffers. The attribute is tested at prelighting time; it is not a test of
observed movement or current animation playback.
