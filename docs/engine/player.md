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

## Remaining coverage

Complete animation eligibility, collision, startup and stop movement, timer-mode
selection, jumping, gravity, combat, swimming, climbing, and follow-camera behavior
are not specified here. Reproducing only the arithmetic above does not establish
controller parity.

## Related documentation

- [Keyboard bindings](../config/keys-ini.md)
- [System configuration](../config/system-ini.md)
- [RGM markers and actor data](../formats/RGM.md)
