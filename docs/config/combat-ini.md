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
| `in_combat_threshold` | `200` | Distance threshold to enter combat state, in units × 256 |
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

## `[tauntNN]` Sections

Each `[tauntNN]` section defines one combat voice line or audio cue. There are 379 sections with indices spanning 00–599, with large gaps between combatant blocks.

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
| `rtx_label` | string | Label key into `ENGLISH.RTX` for the voice line; may be a 4-character code or a `&NNN` numeric reference |
| `animation` | integer | Animation group ID to play alongside the voice line; `0` in all shipped entries |

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
