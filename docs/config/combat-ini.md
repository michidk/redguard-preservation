# COMBAT.INI

Combat system configuration defining attack moves, defense moves, and combat dialogue (taunts) for all combatants.

Shipped sample path: `/Redguard/COMBAT.INI` (e.g. `.../GOG Galaxy/Redguard/Redguard/COMBAT.INI`).

The file is 54 KB and 3,728 lines, the largest INI in the game. Every move, defense, and voice line for every combatant is defined here, making the combat system fully data-driven.

## File Structure

The file contains 474 sections in four types:

| Section type | Count | Index range | Purpose |
|---|---|---|---|
| `[misc]` | 1 | — | Global combat parameters |
| `[attackNN]` | 89 | 00–88 | Attack move definitions |
| `[defendNN]` | 5 | 00–04 | Defense move definitions |
| `[tauntNN]` | 379 | 00–599 (with gaps) | Combat dialogue and voice lines |

The file opens with a block of commented-out constants defining animation group IDs and attack type enums, followed by the sections in the order listed above.

Section suffixes are numeric IDs. A missing section leaves a gap in the ID
space; later sections keep their suffixes. Numeric fields contain decimal
integers, including signed `arc_center` offsets. An omitted field is distinct
from an explicit zero in the file. Taunt labels retain their RTX spelling,
including leading `&` characters.

## Animation Groups

The header comments define the animation group ID table used by `animation` fields throughout the file:

| ID | Name |
|---|---|
| 1 | `anim_defend_low` |
| 2 | `anim_defend_right` |
| 3 | `anim_defend_left` |
| 4 | `anim_defend_high` |
| 5 | `anim_attack_1` |
| 6 | `anim_attack_2` |
| 7 | `anim_attack_3` |
| 8 | `anim_attack_thrust` |
| 9 | `anim_attack_lunge` |
| 10 | `anim_attack_1_end` |
| 11 | `anim_attack_2_end` |
| 12 | `anim_fight_disarm` |
| 13 | `anim_fight_low` |
| 14 | `anim_fight_jump_start` |
| 15 | `anim_fight_jump` |
| 16 | `anim_fight_fall` |
| 17 | `anim_fight_land` |
| 18 | `anim_fight_fall_attack` |
| 19 | `anim_fight_land_attack` |
| 20 | `anim_fight_hurt_1` |
| 21 | `anim_fight_hurt_2` |
| 22 | `anim_death_fight_stab` |
| 23 | `anim_death_fight_hard` |
| 24 | `anim_sheath_sword` |
| 25 | `anim_explore_hurt_1` |
| 26 | `anim_explore_hurt_2` |
| 27 | `anim_death_explore` |

## `[misc]` Section

Global parameters for the combat system.

| Key | Value (shipped) | Description |
|---|---|---|
| `sword_clank_1` | `57` | Sound effect ID for sword clash, variant 1 |
| `sword_clank_2` | `58` | Sound effect ID for sword clash, variant 2 |
| `non_engaged_dist` | `100` | Distance threshold for non-engaged state |
| `in_face_dist` | `100` | Distance threshold for in-face proximity |
| `non_engaged_spacing` | `500` | Spacing between non-engaged combatants, in engine angles |
| `in_combat_threshold` | `200` | Strict horizontal distance threshold for `isFighting`, shifted left eight bits at load |
| `player_can_die` | `1` | Whether the player can be killed (`1` = yes) |
| `cyrus_defend_interval` | `2` | Minimum frames between Cyrus defends |
| `node_timer` | `120` | Timer value for combat node transitions |
| `max_taunts` | `600` | Maximum taunt index (exclusive upper bound for taunt pool) |

## `[attackNN]` Sections

Each `[attackNN]` section defines one attack move. There are 89 sections (`attack00` through `attack88`).

### Attack Types

The `type` field selects the attack category:

| Value | Name |
|---|---|
| 0 | melee |
| 1 | missile |
| 2 | combo |
| 3 | stab |
| 4 | finishing |

### Elevation Values

The `elevation` field controls the vertical targeting zone:

| Value | Meaning |
|---|---|
| 0 | regular (default when omitted) |
| 1 | above |
| 2 | below |

### Field Schema

**Standard attack** (`type` 0, 1, 3, 4):

| Field | Type | Description |
|---|---|---|
| `type` | integer | Attack category (see enum above) |
| `arc` | integer | Horizontal hit arc width, in engine angle units |
| `arc_center` | integer | Horizontal offset of arc center from forward direction; omitted when centered (0) |
| `elevation` | integer | Vertical targeting zone (see enum above); omitted when regular (0) |
| `min_range` | integer | Minimum distance to target for hit to register |
| `max_range` | integer | Maximum distance to target for hit to register |
| `first_collide_frame` | integer | Animation frame on which collision detection begins |
| `collide_duration` | integer | Number of frames collision detection remains active |
| `damage` | integer | Hit point damage dealt on successful hit |
| `force` | integer | Knockback force applied to target |
| `defense` | integer | Defense animation group ID triggered on the target |
| `first_defend_frame` | integer | Animation frame on which the attacker can be defended against; omitted on some attacks |
| `defend_duration` | integer | Number of frames the attacker is vulnerable to defense; omitted on some attacks |
| `animation` | integer | Animation group ID (see animation group table above) |
| `col_vertex` | integer | Model vertex index used as the collision sphere origin; present only on large creature attacks |
| `col_sphere_size` | integer | Radius of the collision sphere; present only on large creature attacks |

**Combo attack** (`type` 2):

Combo sections reference other attack sections by index rather than defining collision geometry directly.

| Field | Type | Description |
|---|---|---|
| `type` | integer | Always `2` for combos |
| `num_attacks` | integer | Number of sub-attacks in the combo (2 or 3) |
| `attack00` | integer | Index of the first sub-attack section |
| `attack01` | integer | Index of the second sub-attack section |
| `attack02` | integer | Index of the third sub-attack section; present only when `num_attacks` = 3 |

### Attack Allocation by Combatant

The file comments identify which attack indices belong to each combatant:

| Combatant | Attack indices |
|---|---|
| Cyrus (early) | 00–14 |
| Skeleton | 15–17 |
| Golem | 18–22 |
| Zombie | 23–25 |
| Serpent | 26 |
| Troll | 27–29 |
| Guard (low attack) | 30 |
| Goblin | 31–34 |
| Dragon | 35–37 |
| Richton | 38–47 |
| Dram | 48–52 |
| Ogre | 53 |
| Pirate (initial, weaker) | 54–64 |
| Cyrus (later) | 65–74 |
| Vermai | 75–77 |
| Tavern thugs (no damage) | 78–88 |

Large creature attacks (Golem, Serpent, Dragon) use `col_vertex` and `col_sphere_size` to attach the collision sphere to a specific model vertex rather than the combatant's origin point.

## `[defendNN]` Sections

Each `[defendNN]` section defines one defense move. There are 5 sections (`defend00` through `defend04`), shared by Cyrus and guards.

### Field Schema

| Field | Type | Description |
|---|---|---|
| `arc` | integer | Horizontal arc width covered by the defense; omitted on `defend00` |
| `min_range` | integer | Minimum distance at which the defense is effective |
| `max_range` | integer | Maximum distance at which the defense is effective |
| `first_collide_frame` | integer | Animation frame on which the defense window opens |
| `collide_duration` | integer | Number of frames the defense window remains active |
| `animation` | integer | Animation group ID (see animation group table above) |

The five shipped defenses correspond to: default (`defend00`), low (`defend01`), right (`defend02`), left (`defend03`), and high (`defend04`).

## Runtime Attack Loading

The engine scans `attack00`, `attack01`, and subsequent numbered sections in
order. A section is admitted when either `damage` or `type` has a nonempty value.
The first section with neither value ends the scan; later sections are not loaded.
This runtime rule differs from parsing the INI as a collection of sparse sections.

A non-combo attack loads one move. A combo reads its referenced moves in the
order of `attack00`, `attack01`, and `attack02`, using `num_attacks` as the count.
Each attack reserves three zero-initialized motion slots. Loading fills the
authored motions and leaves unused slots zeroed.
Each move's numeric loader reads an omitted or empty value as zero. This applies
to motion type, ranges, collision and defence timing, arc, damage, force,
defence selector, collision vertex, collision sphere size, animation and
elevation. Raw INI parsing still distinguishes omission from an explicit zero.
The runtime motion record stores the converted value for every loaded field.

Each loaded move adds 97 with signed 32-bit wrapping to its configured
`animation` value to obtain the actor animation group. An omitted animation
therefore selects group 97. A single attack copies its move's minimum and maximum ranges.
A nonempty combo starts its minimum at 65,535 and maximum at -1, then lowers the
minimum and raises the maximum for each referenced move.

Each attack also retains a damage total, initialized to zero. A single attack adds
its move's damage; a combo adds each referenced move's damage in authored order.
The additions wrap as signed 32-bit values. A combo with no moves retains zero
damage. Defence pacing reads this accumulated total, rather than only the first
motion's damage.

## Runtime Defense Loading

Defense loading scans `defend00`, `defend01`, and subsequent numbered sections
in order. A section is admitted when its numeric `max_range` is nonzero. The
first zero value ends the table; later sections are not loaded. Each admitted
record loads its minimum and maximum ranges, first collision frame, collision
duration, arc and animation. The configured animation value is increased by 97
to obtain the actor animation group.

## Actor Move Selection

Combat initialization binds nine attack slots from actor attribute bytes 70
through 78. Each nonzero byte selects the attack record with that index; zero
leaves the corresponding slot empty. An empty attack slot does not prevent later
attack slots from being populated.

Four defense slots use an attribute cursor starting at byte 79. A nonzero byte
selects the defense record with that index and advances the cursor. A zero byte
leaves the slot empty without advancing the cursor, so all remaining defense
slots also stay empty. Defense selection therefore uses only the nonzero prefix
of attribute bytes 79 through 82. A zero selector does not select `defend00`.

Binding a nonzero defense slot marks its shared defense record available and
stores the actor animation group's signed frame count in that record. An absent
animation group supplies zero. The four-slot availability query succeeds when
one bound record has its availability marker equal to one.

## `[tauntNN]` Sections

Each `[tauntNN]` section defines one combat voice line or audio cue. There are 379 sections with indices spanning 00–599, with large gaps between combatant blocks.

An actor's taunt list begins at its [RAEX](../formats/RGM.md) `taunt_id` and
contains `attr_num_taunts` consecutive entries. A zero count creates no list.
The range is selected per actor; it is independent of which taunt sections are
present elsewhere in the file. Without RAEX, the first index is zero. Each actor
allocates exactly `attr_num_taunts` pointers and fills them from that range in
order. The per-actor pointer list has no additional sentinel entry.

The loader creates an empty record for each absent section and an additional
empty sentinel at `max_taunts`. Empty records have a zero label, type, and
animation. Present records copy the first four `rtx_label` bytes directly,
including a leading `&`. A nonzero animation value is increased by 152 before
storage; zero remains zero and requests no accompanying animation.

### Taunt Types

The `type` field identifies when the taunt fires:

| Value | Trigger |
|---|---|
| 0 | begin combat |
| 1 | attack |
| 2 | hurt by opponent |
| 3 | hit opponent |
| 4 | defend |
| 5 | misc, during move |
| 6 | death |
| 7 | after kill opponent |
| 8 | want to switch out |
| 9 | want to switch in |
| 10 | opponent is unarmed (male) |
| 11 | opponent is unarmed (female) |

### Field Schema

| Field | Type | Description |
|---|---|---|
| `type` | integer | Trigger condition (see enum above) |
| `rtx_label` | string | Four-byte label key into `ENGLISH.RTX`, preserved literally including `&NNN` spellings |
| `animation` | integer | Combat animation offset; nonzero values become group `152 + animation`, while `0` disables accompanying animation |

Combat taunt selection is skipped when the actor satisfies `IsDead`, including
the selected player's death override. Begin-combat writes lifecycle selector 6
before this check; the selected-player override remains independent.

Begin-combat, hurt, death, and opponent-unarmed selection scans the actor's
list in order. Only entries with the requested type and a nonzero RTX label
participate. Each matching entry becomes the current selection and consumes one
random draw. The scan stops when `floor(4 * draw / 32767) == 1`; otherwise the
last matching entry wins. Death selection retries with the hurt type only when
no death entry matched. The other four types have no such fallback.

### Taunt Allocation by Combatant

The file comments define the index ranges reserved for each combatant:

| Index range | Combatant |
|---|---|
| 0–49 | Cyrus |
| 50–149 | Guards |
| 150–154 | Skeletons |
| 155–159 | Zombies |
| 160–179 | Tavern Thug 1 |
| 180–199 | Tavern Thug 2 |
| 200–219 | Tavern Thug 3 (Dagoo) |
| 220–228 | Brennan |
| 229–234 | Golem |
| 235–239 | Serpent / Vermai |
| 240–249 | Troll |
| 250–259 | Goblin |
| 260–279 | Richton |
| 280–299 | Dram |
| 300–319 | Pirate 1 |
| 320–339 | Pirate 2 |
| 340–349 | Ngasta |
| 350–359 | Urik |
| 360–369 | Zombie |
| 370–389 | Vander |
| 390–409 | Island Thug 1 |
| 410–429 | Island Thug 2 |
| 430–449 | Island Thug 3 |
| 450–469 | Island Thug 4 |
| 470–489 | Island Thug 5 |
| 490–509 | Island Thug 6 |
| 510–529 | Pirate Hideout 1 |
| 530–549 | Pirate Hideout 2 |
| 550–569 | Pirate Hideout 3 |
| 570–589 | Pirate Hideout 4 |
| 580–587 | Dragon |
| 591–599 | Jail interior (reuses guard lines) |

Not all reserved slots are populated. Many combatants use only a subset of their allocated range, and some ranges overlap in the comments (Dragon at 580–587 falls inside Pirate Hideout 4 at 570–589).

## External References
