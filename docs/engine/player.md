# Player movement

Player movement combines held input, actor attributes, animation state, and a shared frame scale.

## Coordinate and heading units

Runtime positions have eight fractional bits. A marker coordinate of `40816`
becomes a runtime coordinate of `10448896`. The developer console's `object coord`
command displays both representations. Its start position retains the spawn
position while the current position changes with movement and ground contact.

Headings wrap after 2048 units per revolution. For ordinary ground movement with
the sword sheathed, forward at heading zero increases Z. Right turning decreases
heading; left turning increases it. Other player states must be checked separately.

## Ground-turn arithmetic

Left and right input have independent turn accumulators. Releasing one direction
resets its accumulator to zero. For ordinary movement, a held direction adds
`SYSTEM.INI` `turn_speed` while its accumulator is below `turn_max_speed`.
The addition is not clamped, so settings whose increment does not divide the
limit can overshoot it. The installed values 12 and 48 produce held-input
accumulators 12, 24, 36, 48, 48 on consecutive input updates.

Each direction is multiplied by the current frame scale and shifted right by
eight bits separately. Ordinary ground turning adds the scaled left amount and
subtracts the scaled right amount, then wraps the heading modulo 2048.
At scale 256, starting from heading zero, five right-input updates produce
headings 2036, 2012, 1976, 1928, 1880. These are arithmetic examples, not a
claim that input updates occur at a fixed wall-clock frequency.

Animation and player state control whether this heading update is allowed.
The arithmetic alone is insufficient to implement the player state machine.
Combat can select `attr_fight_turn_speed` and `attr_fight_turn_accel` instead of
the ordinary turn settings.

## Movement data and timing

Forward running and walking use the actor's `attr_forward_speed` and
`attr_forward_walk`. They are actor data, not universal controller constants.
Startup movement also depends on `attr_step_type` and animation progress.

The frame scale uses eight fractional bits, with 256 representing one nominal
update. The first twelve updates after a timing reset use 256. Subsequent timing
can use an FPS-derived scale with the configured normal, minimum and maximum
frame rates and smoothing settings. A nominal frame rate of 12 does not mean
the game runs a fixed 12 Hz simulation.

## Forward-walking startup

Ordinary forward walking keeps a startup phase from zero through three.
The selected walking speed is converted to runtime position units by multiplying
it by 256. For `attr_step_type = 1`, phases zero, one, and two use one quarter,
one third, and one half of that speed respectively, with integer division.
Phase three uses the full speed. A walking speed of 8 therefore gives startup
amounts 512, 682, 1024, and then 2048.

An eligible walking update advances the startup phase before testing whether
the current animation permits translation. Suppressed translation does not undo
that phase advance. Consequently the first visible displacement need not use
phase zero. Two observed ISLAND starts first translated at phase one and phase
two respectively. Resetting the phase whenever translation is suppressed would
change the observed startup behavior.

When animation permits translation, the startup amount is multiplied by the
frame scale and shifted right by eight bits before heading-based displacement.
One observed heading-zero sequence began at raw Z `10502144` and produced:

| Startup phase used | Frame scale | Raw Z displacement | Resulting raw Z |
| --- | --- | --- | --- |
| 1 | 161 | 428 | 10502572 |
| 2 | 161 | 644 | 10503216 |
| 3 | 153 | 1224 | 10504440 |

Each resulting position was retained as the previous position in the following
update in this case. This sample does not establish collision behavior elsewhere,
the complete animation eligibility rules, or the timing of phase reset on release.

## FPS-derived frame scale

The FPS-derived timing path keeps a measured FPS and a smoothed FPS. On each
update, the smoothed value moves toward the measured value by one. If their
initial difference exceeds five, it first moves by half that difference,
rounded down. Thus a measured change from 19 to 17 produces smoothed values
18, then 17 on successive updates.

`use_smooth_fps` selects the smoothed value instead of the measured value.
The selected FPS is clamped to `min_frame_rate` and `max_frame_rate`. The target
frame scale is the integer quotient of `normal_frame_rate * 256` divided by
that FPS. A zero configured normal rate is replaced with 12 during loading.

With `use_smooth_divisor` disabled, the scale becomes the target immediately.
With it enabled, the scale first moves one unit toward the target. If it is
still above the target, it becomes the target immediately. If it is below the
target, the remaining difference determines a second increase:

| Remaining difference | Additional increase |
| --- | --- |
| 1–5 | 1 |
| 6–10 | 5 |
| 11–20 | 10 |
| 21 or more | 20 |

An exact match receives no additional increase. This smoothing is asymmetric:
a falling target can take effect immediately, while a rising target can take
several updates.

An observed ordinary-walking sequence with normal rate 12 had measured FPS 19
and scale 161. When measured FPS became 17, the following three scales were
167, 178, and 180. The intermediate smoothed FPS values 18 and 17 give targets
170 and 180, reproducing that scale sequence. Earlier in the same sequence,
measured FPS rose from 18 to 19 and the scale changed directly from 170 to 161.

This describes the FPS-derived path after timing startup. It does not establish
selection of that path in every game state, alternate-timer behavior, or the
mapping from host elapsed time to original timer ticks.

## Remaining coverage

Complete animation eligibility, collision, other startup paths and stop movement, timer-mode
selection, jumping, gravity, combat, swimming, climbing, and follow-camera behavior
are not specified here. Reproducing only the arithmetic above does not establish
controller parity.

## Related documentation

- [Keyboard bindings](../config/keys-ini.md)
- [System configuration](../config/system-ini.md)
- [RGM markers and actor data](../formats/RGM.md)
