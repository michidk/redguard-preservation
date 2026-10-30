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

Placed nonmoving actors initialize their model-instance translation from each
signed Q8 position component shifted arithmetically right by eight bits, then
converted to floating point. Fractional map units are discarded before the
conversion. Placed orientation initializes the same instance matrix from the
authored angles. The prelighting pass reuses this prepared translation and
matrix when binding the static model; it does not rebuild the transform from
the actor's current position or angles.

## Entry and restoration order

Prelighting runs after world-entry loading and before ordinary actor updates.
On fresh entry, the actor loader's attribute defaults, model-path selection,
placed orientation and initial group requests have already run. Ordinary script
entry, task and main continuations have not advanced for the new entry.

Revisiting a retained world restores its actor records before prelighting.
Eligibility therefore uses the restored attribute and model kind, and the
operation consumes the restored embedded model-instance transform. The saved
script and animation continuations are installed before this pass resumes
ordinary updates. Restoring a saved game follows the same ordering: restore
world and actor state, run prelighting, then continue ordinary updates. Retained
colors are rebuilt by the pass rather than treated as saved animation state.
