# SOUP property descriptors

Script property operands (`Object.name`, for example `Player.hide` or `Me.pos.vx`) are resolved through an engine descriptor table. Each descriptor names one property and gives the byte offset of its field and the field's width. The `[refs]` names of [SOUP386.DEF](SOUPDEF.md) select descriptors by name, case-insensitively.

The table has 226 descriptors in the order below. Offsets are hexadecimal byte offsets from the start of the actor record. The tag column is the descriptor's type letter: `b`, `w` and `d` are one-, two- and four-byte values. An uppercase tag (or `x`, `o`, `A`) marks a field that holds a pointer to, or the body of, a sub-record. The dotted names that follow such an entry (`anim.*`, `size.*`) address fields inside that sub-record, and their offsets are relative to the sub-record rather than the actor.

Several descriptors name state that other documents describe by function: `fall_flag` is the falling-state byte, `extra_flag` the living-state byte, `torch_timer` the burning-torch byte, `id` the low-word runtime identifier, `attribute` the pointer to the 256-byte attribute table, `origin` the retained fall origin Y, `under_surface` the support surface code, and `pos.vx`, `pos.vy` and `pos.vz` the local Q8 position. `grip[0]`, `grip[1]`, `scabbard[0]` and `scabbard[1]` are the four signed words the loader copies from RAEX before attachment queries.

| Name | Offset | Tag | Width |
|---|---|---|---|
| `address` | `0x0` | `x` | 4 |
| `next` | `0x0` | `O` | 4 |
| `list` | `0x4` | `O` | 4 |
| `parent` | `0x8` | `O` | 4 |
| `linked` | `0xc` | `O` | 4 |
| `grid` | `0x10` | `d` | 4 |
| `id` | `0x14` | `w` | 2 |
| `name` | `0x16` | `A` | 4 |
| `ftype` | `0x1a` | `b` | 1 |
| `sflags` | `0x1b` | `b` | 1 |
| `runtime_local_list` | `0x1c` | `D` | 4 |
| `runtime_locals` | `0xc1` | `b` | 1 |
| `extra` | `0x21` | `D` | 4 |
| `object_data` | `0x25` | `o` | 156 |
| `static_data` | `0xc1` | `b` | 1 |
| `loaded_matrix` | `0xc2` | `b` | 1 |
| `data` | `0xc3` | `B` | 4 |
| `header` | `0xc7` | `B` | 4 |
| `pos.vx` | `0xcb` | `d` | 4 |
| `pos.vy` | `0xcf` | `d` | 4 |
| `pos.vz` | `0xd3` | `d` | 4 |
| `slave_pos.vx` | `0xd7` | `d` | 4 |
| `slave_pos.vy` | `0xdb` | `d` | 4 |
| `slave_pos.vz` | `0xdf` | `d` | 4 |
| `last_slave_pos.vx` | `0xe3` | `d` | 4 |
| `last_slave_pos.vy` | `0xe7` | `d` | 4 |
| `last_slave_pos.vz` | `0xeb` | `d` | 4 |
| `start_pos.vx` | `0xef` | `d` | 4 |
| `start_pos.vy` | `0xf3` | `d` | 4 |
| `start_pos.vz` | `0xf7` | `d` | 4 |
| `last_pos.vx` | `0xfb` | `d` | 4 |
| `last_pos.vy` | `0xff` | `d` | 4 |
| `last_pos.vz` | `0x103` | `d` | 4 |
| `last_valid_flag` | `0x107` | `b` | 1 |
| `last_valid_pos.vx` | `0x108` | `d` | 4 |
| `last_valid_pos.vy` | `0x10c` | `d` | 4 |
| `last_valid_pos.vz` | `0x110` | `d` | 4 |
| `angle.vx` | `0x120` | `d` | 4 |
| `angle.vy` | `0x124` | `d` | 4 |
| `angle.vz` | `0x128` | `d` | 4 |
| `slave_angle.vx` | `0x12c` | `d` | 4 |
| `slave_angle.vy` | `0x130` | `d` | 4 |
| `slave_angle.vz` | `0x134` | `d` | 4 |
| `last_slave_angle.vx` | `0x138` | `d` | 4 |
| `last_slave_angle.vy` | `0x13c` | `d` | 4 |
| `last_slave_angle.vz` | `0x140` | `d` | 4 |
| `start_angle.vx` | `0x144` | `d` | 4 |
| `start_angle.vy` | `0x148` | `d` | 4 |
| `start_angle.vz` | `0x14c` | `d` | 4 |
| `last_angle.vx` | `0x150` | `d` | 4 |
| `last_angle.vy` | `0x154` | `d` | 4 |
| `last_angle.vz` | `0x158` | `d` | 4 |
| `move.vx` | `0x15c` | `d` | 4 |
| `move.vy` | `0x160` | `d` | 4 |
| `move.vz` | `0x164` | `d` | 4 |
| `ov.vx` | `0x168` | `d` | 4 |
| `ov.vy` | `0x16c` | `d` | 4 |
| `ov.vz` | `0x170` | `d` | 4 |
| `size` | `0x174` | `Z` | 4 |
| `location` | `0x1a5` | `D` | 4 |
| `yangle_change` | `0x1a9` | `w` | 2 |
| `rope` | `0x1ab` | `R` | 4 |
| `rope_id` | `0x1af` | `b` | 1 |
| `rope_wait` | `0x1b0` | `b` | 1 |
| `slide_flag` | `0x1b1` | `b` | 1 |
| `fall_flag` | `0x1b2` | `b` | 1 |
| `jump_flag` | `0x1b3` | `b` | 1 |
| `extra_flag` | `0x1b4` | `b` | 1 |
| `extra_time` | `0x1b5` | `b` | 1 |
| `action_flag` | `0x1b6` | `d` | 4 |
| `action` | `0x1ba` | `d` | 4 |
| `action_time` | `0x1be` | `b` | 1 |
| `arms_time` | `0x1bf` | `b` | 1 |
| `jump_time` | `0x1c0` | `b` | 1 |
| `jump_height` | `0x1c1` | `b` | 1 |
| `origin` | `0x1c2` | `d` | 4 |
| `origin_time` | `0x1c6` | `b` | 1 |
| `origin_accum` | `0x1c7` | `d` | 4 |
| `last_direction` | `0x1cb` | `b` | 1 |
| `move_counter` | `0x1cc` | `b` | 1 |
| `last_collide` | `0x1cd` | `b` | 1 |
| `last_collide_count` | `0x1d1` | `b` | 1 |
| `under_object` | `0x1d4` | `O` | 4 |
| `under_surface` | `0x1dc` | `b` | 1 |
| `surface_time` | `0x1dd` | `b` | 1 |
| `ai_type` | `0x1de` | `b` | 1 |
| `ai_mode` | `0x1df` | `b` | 1 |
| `ai_delay` | `0x1e0` | `b` | 1 |
| `ai_var1` | `0x1e1` | `w` | 2 |
| `ai_var2` | `0x1e3` | `w` | 2 |
| `ai_length` | `0x1e5` | `d` | 4 |
| `node_map` | `0x1e9` | `N` | 4 |
| `node` | `0x1ed` | `d` | 4 |
| `dnode` | `0x1f1` | `d` | 4 |
| `fdnode` | `0x1f9` | `d` | 4 |
| `weapon_drawn` | `0x1fd` | `b` | 1 |
| `torch_timer` | `0x1fe` | `b` | 1 |
| `hand_item` | `0x200` | `b` | 1 |
| `hand_pos.vx` | `0x201` | `d` | 4 |
| `hand_pos.vy` | `0x205` | `d` | 4 |
| `hand_pos.vz` | `0x209` | `d` | 4 |
| `hand_angle.vx` | `0x20d` | `d` | 4 |
| `hand_angle.vy` | `0x211` | `d` | 4 |
| `hand_angle.vz` | `0x215` | `d` | 4 |
| `grip[0]` | `0x219` | `w` | 2 |
| `grip[1]` | `0x21b` | `w` | 2 |
| `scabbard[0]` | `0x21d` | `w` | 2 |
| `scabbard[1]` | `0x21f` | `w` | 2 |
| `sphere_object` | `0x221` | `S` | 4 |
| `fsphere_heap` | `0x229` | `D` | 4 |
| `fspheres` | `0x22d` | `w` | 2 |
| `active` | `0x237` | `b` | 1 |
| `hide` | `0x238` | `b` | 1 |
| `dead` | `0x239` | `b` | 1 |
| `invisible` | `0x23a` | `b` | 1 |
| `combat` | `0x23b` | `C` | 4 |
| `light` | `0x23f` | `L` | 4 |
| `flat` | `0x243` | `F` | 4 |
| `anim` | `0x247` | `M` | 4 |
| `anim_list` | `0x24b` | `W` | 4 |
| `frame` | `0x24f` | `w` | 2 |
| `handle` | `0x251` | `w` | 2 |
| `last_frame` | `0x253` | `w` | 2 |
| `last_handle` | `0x255` | `w` | 2 |
| `panic_group` | `0x257` | `w` | 2 |
| `message_index` | `0x259` | `b` | 1 |
| `broadcast_index` | `0x25a` | `b` | 1 |
| `anchor` | `0x25b` | `b` | 1 |
| `hook_data` | `0x25c` | `B` | 4 |
| `script_data` | `0x260` | `B` | 4 |
| `script_int` | `0x264` | `B` | 4 |
| `script_pc` | `0x268` | `B` | 4 |
| `script_return` | `0x26c` | `B` | 4 |
| `script_rv` | `0x270` | `b` | 1 |
| `script_ret_rv` | `0x271` | `b` | 1 |
| `attribute` | `0x272` | `B` | 4 |
| `local_index` | `0x276` | `d` | 4 |
| `string_index` | `0x27a` | `d` | 4 |
| `disable_script` | `0x27e` | `b` | 1 |
| `task[0]` | `0x27f` | `B` | 4 |
| `task[1]` | `0x283` | `B` | 4 |
| `task[2]` | `0x287` | `B` | 4 |
| `task[3]` | `0x28b` | `B` | 4 |
| `task[4]` | `0x28f` | `B` | 4 |
| `task[5]` | `0x293` | `B` | 4 |
| `task_variable[0]` | `0x297` | `d` | 4 |
| `task_variable[1]` | `0x29b` | `d` | 4 |
| `task_variable[2]` | `0x29f` | `d` | 4 |
| `task_variable[3]` | `0x2a3` | `d` | 4 |
| `task_variable[4]` | `0x2a7` | `d` | 4 |
| `task_variable[5]` | `0x2ab` | `d` | 4 |
| `task_status[0]` | `0x2af` | `b` | 1 |
| `task_status[1]` | `0x2b0` | `b` | 1 |
| `task_status[2]` | `0x2b1` | `b` | 1 |
| `task_status[3]` | `0x2b2` | `b` | 1 |
| `task_status[4]` | `0x2b3` | `b` | 1 |
| `task_status[5]` | `0x2b4` | `b` | 1 |
| `task_pause[0]` | `0x2b5` | `b` | 1 |
| `task_pause[1]` | `0x2b6` | `b` | 1 |
| `task_pause[2]` | `0x2b7` | `b` | 1 |
| `task_pause[3]` | `0x2b8` | `b` | 1 |
| `task_pause[4]` | `0x2b9` | `b` | 1 |
| `task_pause[5]` | `0x2ba` | `b` | 1 |
| `task_timer[0]` | `0x2bb` | `w` | 2 |
| `task_timer[1]` | `0x2bd` | `w` | 2 |
| `task_timer[2]` | `0x2bf` | `w` | 2 |
| `task_timer[3]` | `0x2c1` | `w` | 2 |
| `task_timer[4]` | `0x2c3` | `w` | 2 |
| `task_timer[5]` | `0x2c5` | `w` | 2 |
| `task_variable2[0]` | `0x2c7` | `d` | 4 |
| `task_variable2[1]` | `0x2cb` | `d` | 4 |
| `task_variable2[2]` | `0x2cf` | `d` | 4 |
| `task_variable2[3]` | `0x2d3` | `d` | 4 |
| `task_variable2[4]` | `0x2d7` | `d` | 4 |
| `task_variable2[5]` | `0x2db` | `d` | 4 |
| `task_variable3[0]` | `0x2df` | `d` | 4 |
| `task_variable3[1]` | `0x2e3` | `d` | 4 |
| `task_variable3[2]` | `0x2e7` | `d` | 4 |
| `task_variable3[3]` | `0x2eb` | `d` | 4 |
| `task_variable3[4]` | `0x2ef` | `d` | 4 |
| `task_variable3[5]` | `0x2f3` | `d` | 4 |
| `task_variable4[0]` | `0x2f7` | `d` | 4 |
| `task_variable4[1]` | `0x2fb` | `d` | 4 |
| `task_variable4[2]` | `0x2ff` | `d` | 4 |
| `task_variable4[3]` | `0x303` | `d` | 4 |
| `task_variable4[4]` | `0x307` | `d` | 4 |
| `task_variable4[5]` | `0x30b` | `d` | 4 |
| `anim.address` | `0x0` | `M` | 4 |
| `anim.current` | `0x0` | `M` | 2 |
| `anim.next` | `0x2` | `M` | 2 |
| `anim.next_speed` | `0x4` | `M` | 1 |
| `anim.frame` | `0x5` | `M` | 2 |
| `anim.speed` | `0x7` | `M` | 1 |
| `anim.speed_count` | `0x8` | `M` | 1 |
| `anim.stop_flag` | `0xa` | `M` | 1 |
| `anim.halt_flag` | `0xb` | `M` | 1 |
| `anim.hold_flag` | `0xc` | `M` | 1 |
| `anim.rep_count` | `0xd` | `M` | 1 |
| `anim.rewind_flag` | `0xe` | `M` | 1 |
| `anim.loop_count` | `0xf` | `M` | 1 |
| `anim.sound_handle` | `0x10` | `M` | 1 |
| `anim.real_frame` | `0x11` | `M` | 1 |
| `anim.max_group` | `0x12` | `M` | 1 |
| `anim.pos.vx` | `0x13` | `M` | 2 |
| `anim.pos.vy` | `0x15` | `M` | 2 |
| `anim.pos.vz` | `0x17` | `M` | 2 |
| `anim.angle.vx` | `0x19` | `M` | 2 |
| `anim.angle.vy` | `0x1b` | `M` | 2 |
| `anim.angle.vz` | `0x1d` | `M` | 2 |
| `anim.last_angle.vx` | `0x1f` | `M` | 2 |
| `anim.last_angle.vy` | `0x21` | `M` | 2 |
| `anim.last_angle.vz` | `0x23` | `M` | 2 |
| `size.address` | `0x0` | `Z` | 4 |
| `size.orientation` | `0x0` | `Z` | 1 |
| `size.size.vx` | `0x1` | `Z` | 4 |
| `size.size.vy` | `0x5` | `Z` | 4 |
| `size.size.vz` | `0x9` | `Z` | 4 |
| `size.pos.vx` | `0xd` | `Z` | 4 |
| `size.pos.vy` | `0x11` | `Z` | 4 |
| `size.pos.vz` | `0x15` | `Z` | 4 |
| `size.psize.vx` | `0x19` | `Z` | 4 |
| `size.psize.vy` | `0x1d` | `Z` | 4 |
| `size.psize.vz` | `0x21` | `Z` | 4 |
| `size.msize.vx` | `0x25` | `Z` | 4 |
| `size.msize.vy` | `0x29` | `Z` | 4 |
| `size.msize.vz` | `0x2d` | `Z` | 4 |

## Extra-time counter

`extra_time` is a receiver-owned unsigned byte. Script reads return values from
0 through 255; writes retain the low eight bits and change only that field.
Actor allocation clears it with the rest of the actor record.

An admitted ordinary actor update leaves zero and 255 unchanged. Other values
lose the nominal tick count, saturating at zero. This stage follows the early
action and ledge work and precedes the ordinary motion-vector clear and the
subsequent animation advance. Actors excluded from ordinary processing do not
reach this countdown stage.

Successful torch-lighting and torch-dousing entry request their animation first,
then assign their living state and clear `extra_time`. A refused entry leaves
it unchanged.

## Stored playback default

`panic_group` is the receiver actor's stored playback default, separate from
the installed `PanicGroup` attribute. Reads sign-extend its 16-bit word; writes
retain the low 16 bits. A property write changes only the stored default. It
does not request a group, replace the current or pending group, advance commands
or dispatch animation sounds. Later request fallback and default comparisons
consume the changed word. Ordinary default selection can overwrite it.

Actor allocation clears this word, including reuse of pooled records. Actor
animation-control initialization also sets it to zero. The word belongs to the
actor record even when no playback allocation exists. Object save records retain
it in the packed actor image, and object restoration copies it back independently
of the optional playback trailer.

## Packed movement counter

`move_counter` is the actor's unsigned packed movement-counter byte. Raw property
writes retain the low eight bits and do not themselves request or advance
animation or translate the actor. The forward startup phase occupies bits 4 and
5. A forward amount query below phase three performs its support admission before
requesting animation. After animation admission it advances that phase, preserving
the other bits. Phase three keeps the full amount and remains saturated.
The byte clears with actor allocation, including reuse, and remains part of
the saved actor body.

## Vertical-motion timer

`origin_time` is the receiver actor's retained queued/falling vertical-motion
timer byte. Property reads are unsigned and writes retain the low eight bits.
The vertical-motion consumers interpret that same byte as signed: queued motion
uses it as the factor index and advances it with byte wrapping, while falling
uses factor zero when the signed timer is nonpositive. A positive falling timer
selects its factor and decreases by the sum of the two fine-tick increments,
with byte wrapping. Model-controlled vertical motion skips these arithmetic
updates. A property write does not initialize a fall or move the actor.

Actor allocation clears the byte, including reused pooled records. Jump entry
initializes it to zero; ordinary fall entry initializes it to 30 when it records
the fall origin. The timer remains part of the saved actor body, independently
of playback allocation. It is distinct from the rope regrab countdown and
`extra_time`.

## Accumulated displacement

`origin_accum` is the receiver actor's signed 32-bit accumulated displacement,
shared by sliding and the pre-script queued/falling jump displacement. Property
access retains the entire word and does not itself move the actor. Allocation
clears it, including pooled reuse, and the saved actor body retains it. Jump
initialization clears it; the first slide also clears it before adding the slide
distance. Later slide responses add their distance with 32-bit wrapping.

The pre-script jump displacement compares this accumulator with its motion's cap
as a signed value. Below the cap, the candidate sum of accumulator and step wraps
at 32 bits before the signed cap comparison. A clamped step is the cap minus the
accumulator with 32-bit wrapping. When model-controlled motion does not suppress
displacement, the selected step is added to the accumulator with 32-bit wrapping;
only a positive step translates the actor. At or above the cap, the step is zero.
Slide termination compares this same signed accumulator with the slide-hurt
distance threshold.

## Pending-action timer

`action_time` is the receiver actor's unsigned byte timer for pending-action
promotion, separate from the state-17 action-transition counter. Property reads
are unsigned and writes retain the low eight bits. Allocation clears it, including
pooled reuse, and the saved actor body retains it. The pending-action branch of
`SetAction` clears it when it sets the pending directional bits.

The late jump-window stage adds the low byte of the nominal-tick increment with
byte wrapping. A result greater than 36 merges the pending action word into the
accepted word, clears the pending word and resets `action_time` to zero. A result
of 36 does not promote. This timer update precedes the queued-motion and current
animation gates that clear the directional low bits and decide jump entry. A
raw property write does not perform promotion or start a jump.

## Arms timer

`arms_time` is the receiver actor's unsigned byte action-transition counter.
Property reads are unsigned and writes retain the low eight bits. Allocation
clears it, including pooled reuse, and the saved actor body retains it. The
admitted state-17 transition assigns six after requesting its transition group.

The ordinary actor update consumes this byte using the signed-byte action-counter
rule in SOUP.md: zero is unchanged; the early-clear test precedes wrapping tick
subtraction and negative-result clamping. A raw property write does not request
a transition group or change living state. This field is separate from the
pending-action promotion timer `action_time`.

## Pending directional action word

`action_flag` is the receiver actor's pending directional action word. Property
access preserves all 32 bits; the script integer carries the same bit pattern.
Allocation clears the word, including pooled reuse, and the saved actor body
retains it. A raw write does not reset `action_time` or execute an action.

The pending-action branch of `SetAction` sets its low two bits. The late
jump-window promotion merges the entire pending word into the accepted `action`
word, then clears the pending word. The later motion and animation gates clear
only the low two directional bits, retaining the other bits. These words are
separate from the queued-motion and falling-motion bytes.

## Accepted directional action word

`action` is the receiver actor's accepted directional action word, separate from
pending `action_flag`. Property access preserves all 32 bits through the script
integer's bit pattern. Allocation clears it, including pooled reuse, and the
saved actor body retains it. A raw write does not execute an action or reset
the promotion timer.

The late jump-window stage consumes accepted low bits independently of whether
the pending word was promoted on that update. A queued motion or a current group
other than running or torch-running clears the low two bits of both words.
Otherwise accepted bit zero selects running motion eight before bit one can
select motion nine; consumption clears the low two bits in both words before
jump admission. All higher bits remain unchanged. Promotion merges the entire
pending word into the accepted word before these gates and consumption.

## Runtime script-control property

`disable_script` is the receiver actor's runtime script-control byte, shared by
the enable and disable control natives. Property reads are unsigned and writes
retain the low eight bits. The masks retain the independent script, hook, task,
object and combat meanings documented in SOUP.md. The descriptor is distinct
from `sflags` and from the actor's attribute table.

Actor allocation clears this byte, including pooled reuse, and the saved actor
body retains it. A raw property write stores the complete byte; control natives
modify only their own mask and preserve the other bits. Ordinary stage admission
reads this same byte.

## Collision-metadata property

`sflags` is the receiver actor's retained collision-metadata byte. Reads are
unsigned and writes retain the low eight bits. It is independent of the runtime
script-control byte `disable_script` and the actor's attribute table.

Allocation clears the byte, including reuse of pooled actors, and the saved actor
body retains it. On the static model-load path, a matching named ROB segment
supplies its collision byte. The MPOB static-model fallback performs the same
copy. Without a matching segment, that copy is skipped rather than assigning a
new value. Live static face candidates retain this actor byte as support
metadata. See [actor model initialization](animation.md) for the model-load
lifecycle.

## AI lifecycle-selector property

`ai_type` is the receiver actor's lifecycle-selector byte, shared by
`MyAiType` and `SetAiType`. Reads are unsigned and writes retain the low eight
bits. A raw property write replaces only this byte; it does not change the
adjacent AI mode, health, the killed flag, or script-control masks.

Actor allocation clears the byte, including pooled reuse, and the saved actor
body retains it. Combat entry uses selector six, while the ordinary `IsDead`
predicate tests selector eight. The selected player's independent death override
can also make `IsDead` succeed. The selector is distinct from the `dead` property.
See [receiver AI type](SOUP.md#receiver-ai-type-query) and
[dead actor default selection](animation.md#dead-actor-default-selection).

## AI mode property

`ai_mode` is the receiver actor's unsigned AI-mode byte, shared by `MyAiMode`
and `SetAiMode`. Writes retain the low eight bits and replace only this byte.
The adjacent `ai_type` lifecycle selector, the independent `dead` property,
health, and script-control masks are unchanged. A raw mode write does not
invoke a lifecycle transition.

Actor allocation clears the byte, including pooled reuse, and the saved actor
body retains it. `endCombat` clears this mode while setting the lifecycle
selector to one. See [AI mode query and setter](SOUP.md#receiver-ai-mode-query)
and [AI lifecycle-selector property](#ai-lifecycle-selector-property).

## AI delay property

`ai_delay` is the receiver actor's unsigned AI-delay byte. Reads return values
from zero through 255 and writes retain the low eight bits. It is separate
from the adjacent AI mode and lifecycle selector and from `extra_time`. Actor
allocation clears it, including pooled reuse, and the saved actor body retains it.

The AI delay update retains zero and 255. For any other value, it compares the
unsigned byte with the shared nominal tick count. A smaller value becomes zero;
otherwise the low byte of the tick count is subtracted. The update then
continues into lifecycle-selector dispatch even when the delay was zero or 255.
`GoToLocation` invokes this update after selecting lifecycle state four and
storing the requested location word.

One AI admission branch requires delay zero as well as a mode other than eleven
before invoking its candidate service. In the successful path-selection branch,
a delay of 255 becomes 64; other delay values remain unchanged. The temporary
AI-state save and restore pair retains this byte alongside the mode, length and
AI-variable state. Raw property writes invoke none of those services.

## Position and angle history properties

`last_pos.vx`, `last_pos.vy` and `last_pos.vz` are the actor's previous local position (the triple
`SetPosition` writes alongside the current position), `start_pos.*` its saved start position (the
triple `SetStartPosition` restores from and `DistanceFromStart` measures against) and `last_angle.*`
the angle history that `SetRotation` stores with the local angles. Each is a four-byte field read and
written whole, without a matrix refresh or any movement service; a write to `start_pos` therefore
changes what `DistanceFromStart` reports, and a write to `last_pos` or `last_angle` changes only the
retained history.

## Stable-position and creation-angle properties

`last_valid_flag` is a byte and `last_valid_pos.vx`, `last_valid_pos.vy` and `last_valid_pos.vz` are
three four-byte fields: the actor's [stable ordinary position](player.md#stable-ordinary-position)
record, a validity byte followed by the Q8 local position it holds. The stable recovery runs only
while the byte is nonzero and restores the position from the record. Raw writes change only the
fields, so a script can arm a recovery by setting the position and then the flag, or disarm it by
clearing the flag.

`start_angle.vx`, `start_angle.vy` and `start_angle.vz` are the actor's creation angles, four-byte
fields. The orientation setter that actor creation, the player placement and the player swap use
masks the angles to 0 through 2047 and stores them in the current angles, the angle history and
this triple before it rebuilds the matrix; [`ResetRotation`](SOUP.md#resetrotation) restores a
component from it. `SetRotation` does not change it. A property write stores the whole value with
no matrix refresh.

## Motion triple properties

`move.vx`, `move.vy` and `move.vz` are the actor's retained motion components, three four-byte fields
read and written whole. The fall start and obstacle interruption clear them. The rope release service
stores the jump-off speed in `move.vz` (the installed jump-off speed, plus the node's measured speed
for a non-player receiver, negative for a backward jump) and queued motion 24 reads its sign and
magnitude to displace the actor, so a script write to `move.vz` while that motion runs changes the
jump. A property write moves no actor and runs no service.

## Slave pose properties

`slave_pos.vx`, `slave_pos.vy` and `slave_pos.vz` are the actor's attached position and
`slave_angle.vx`, `slave_angle.vy` and `slave_angle.vz` its attached angles, the pose the family
update composes for an actor attached to a master; `last_slave_pos.*` and `last_slave_angle.*` hold
the values the previous family update published. Each is a four-byte field read and written whole,
without a matrix refresh.

Every family update copies the current attached position and angles into the `last_slave_*` fields,
then stores the new composed position and the composed angles (masked to 0 through 2047) in the
`slave_*` fields. An actor with a nonzero `attr_slave` takes its effective position and angles for
queries from `slave_pos` and `slave_angle` and every other actor from its local position and angles,
so a property write changes what distance and placement queries see for an attached actor until the
next family update replaces it, and changes nothing for an unattached one.

## Task slot word properties

`task_variable[n]`, `task_variable2[n]`, `task_variable3[n]` and `task_variable4[n]` (n from 0 through 5)
are the four 32-bit words that task slot n of the receiver retains for its native handler. The task
resume pass copies them into the shared native command scratch before the handler runs and copies the
scratch back afterwards, so a property write to a slot is what the next resumption of that slot sees,
and a property read shows the state saved by its last resumption. The words are:

- `task_variable`: the first native continuation word (the loaded image resource of `AnimateGXA`).
- `task_variable2`: the pause countdown in nominal ticks that point-walk handlers decrement before
  doing any other work.
- `task_variable3`: the third word, the second native continuation word of handlers such as
  `FxPhase`; point-walk handlers also hold the 32-bit command value here, which the wrapper
  copies to and from the actor's AI length.
- `task_variable4`: the packed command word (low 16 bits), phase byte (bits 16 to 23) and counter
  byte (bits 24 to 31).

Each is read and written whole. A property write moves no handler and clears no slot; a slot that is
empty or paused keeps whatever the resume pass last stored for it.

## Task slot control properties

`task_status[n]` and `task_pause[n]` are bytes and `task_timer[n]` a 16-bit word of the receiver's task
slot n (n from 0 through 5), kept in parallel arrays beside the slot's call word. A property read
zero-extends a byte and sign-extends the timer; a write keeps the low byte or word.

- `task_status` is nonzero while the slot blocks its thread: allocation stores one for a blocking task
  and zero for a multitask. The task pass adds the status of every unpaused occupied slot, so a status
  of any nonzero value blocks.
- `task_pause` pauses the slot while nonzero. `TaskPause` stores one, `TaskResume` zero, and the task
  pass skips a paused slot without invoking its handler or advancing its timer.
- `task_timer` is the invocation counter. Allocation stores one; each pass invocation derives its
  elapsed input from the counter times the frame scale shifted right by eight, then increments it with
  16-bit wrap.

A slot whose call word is zero is empty and is skipped whatever its three bytes hold. Completion of a
handler clears the call word and the status and leaves the pause byte and the timer as they are.
`TaskStop` clears the call word, status and pause of every matching slot and leaves the timer.
Allocation overwrites all three. A property write changes only the field: it removes no call, runs no
handler and cancels no native state.

## Message selection properties

`message_index` and `broadcast_index` are the receiver actor's unsigned selection bytes for its
message channel and for the broadcast channel: the cursors the message natives (`ReceiveMessage`,
`DeleteMessage`, `IsMessageWaiting`, `GetMessageSender` and its identifier form) and the broadcast
natives (`ReceiveBroadcast`, `DeleteBroadcast`, `IsBroadcastWaiting`, `GetBroadcastSender` and
its identifier form, `ReplyBroadcastSender`) read and advance. Property reads are unsigned and
writes retain the low eight bits, with no clamp to the entry count: a stored value outside the
queue is used unchecked by the natives. Actor allocation clears both bytes, including pooled reuse.
A raw write moves no cursor service and changes no entry.

## AI record properties

`ai_var1` and `ai_var2` are two-byte fields and `ai_length` a four-byte field in the actor record's
AI block next to `ai_delay`. A property read sign-extends a two-byte field and returns a four-byte
field whole; a write keeps the low sixteen bits of a two-byte field and all of a four-byte one.
Actor allocation zeroes the record and then sets `ai_var1` to minus one (the word `0xffff`),
leaving `ai_var2` and `ai_length` zero, including for pooled reuse. `GoToLocation` stores the low
sixteen bits of its location argument in `ai_var1`, and its sentinel test reads that word back as a
signed value.

`node`, `dnode` and `fdnode` are the actor's held current, next and final-destination walk-node
indices, and `node_map` the walk map they index; the previous-next record sits between `dnode` and
`fdnode`. Allocation sets the three indices to minus one and `node_map` to null. Assigning the
current node first copies a held next node into the previous-next record; the next-node assignment
and the walk code move the held values between the fields. A raw property write stores the value
without claiming or releasing a node reservation.

The `HeaderHook` diagnostic prints `node`, `dnode`, `ai_length` as signed integers and both
variables as signed sixteen-bit values. The temporary AI-state save and restore pair retains
`ai_length` and `ai_var1` with the mode and delay. Raw property writes invoke no AI service.
