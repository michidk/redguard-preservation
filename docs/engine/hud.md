# In-game HUD

The Glide build draws the in-game overlay (candle, active item, prompt text,
compass, power-up icons and logbook icon) over the 3D scene on a virtual
640×480 screen, scaled to the output size.

## Shared rules

- A HUD bitmap is drawn with its top-left corner at the configured anchor
  plus the frame's draw offsets (the signed fields at frame-record offsets
  `0x06` and `0x08`, see [GXA](../formats/GXA.md#frame-record-header)). Every
  frame of the HUD atlases has offsets `(0, 0)`, so each INI coordinate is the
  bitmap's top-left corner. The power-up slots and the logbook icon use the
  anchor alone.
- Palette index 0 is transparent.
- HUD text uses `FONTS\REDSEL.FNT` with the `[3dfx] font_sel` pair (shipped
  `255,255`), the same glyph, shadow and colour rules as an NPC
  [subtitle](subtitles.md#glyphs). Text is drawn as stored, ignores
  `[system] disable_text`, and wraps only when a glyph would cross x = 640.
- Timers below count game ticks, the unit of `rtx_pickup_override_time` and
  the subtitle timers. Animations marked *per rendered frame* advance once per
  drawn frame, independent of ticks; their on-screen rate in the original has
  not been observed.
- *Lockout* is the value last passed to the script function `LockoutPlayer`;
  the engine also resets it to 0, for example after player death. Each element
  below tests it differently.
- `attr_player_type` uses the SOUP386.DEF values `ply_cyrus` 0,
  `ply_gremlin` 1, `ply_crossbow` 2.

| Element | Bitmap | Frames | Size | Anchor keys | Shipped anchor |
|---|---|---:|---|---|---|
| Candle | `SYSTEM\SNUFF.GXA` | 81 | 50×98 | `candle_xco`, `candle_yco` | 12, 8 |
| Active item | ITEM.INI `bitmap_file` (`SYSTEM\PICKUPS.GXA`) | 66 | 119×83 | `pickup_xco`, `pickup_yco` | 12, 386 |
| Compass | `SYSTEM\COMPASS2.GXA` | 32 | 73×65 | `compass_xco`, `compass_yco` | 546, 396 |
| Power-up slots | `[system] game_bitmap` (`system\powerup.gxa`) | 2 | 39×61, 54×61 | `game_xco1/2`, `game_yco1/2` | 576, 6 and 576, 96 |
| Logbook icon | `SYSTEM\SCROLL.GXA` | 32 | 73×58 | `logbook_xco`, `logbook_yco` | 540, 20 |

All anchor keys are in [SYSTEM.INI](../config/system-ini.md) `[system]`.

## Candle

The candle shows the player's health. Maximum health is a fixed 100 (no INI
key). Health is `attr_health` (attribute index 3 in SOUP386.DEF), always
clamped to `0..100`.

### Level

With integer arithmetic and truncating division:

```text
d = ((100 - min(health, 100)) * 256) / 100
L = (d * 11) >> 8
```

| Level `L` | Health |
|---:|---|
| 0 | 91–100 |
| 1 | 82–90 |
| 2 | 73–81 |
| 3 | 64–72 |
| 4 | 55–63 |
| 5 | 46–54 |
| 6 | 37–45 |
| 7 | 27–36 |
| 8 | 18–26 |
| 9 | 9–17 |
| 10 | 1–8 |
| 11 (snuffed) | 0 |

### Frame layout

Levels 0–10 own six frames each: `6L` to `6L + 4` are the flicker cycle and
`6L + 5` is the transition frame between level `L` and `L + 1` (frames 0–65).
Frames 66–80 are the snuff-out animation.

### Animation

The candle keeps a shown level `S` (unset when a play session opens) and a
step counter `C` (initially 0). Every rendered frame of a running world,
whether or not the candle is visible:

1. If `S` is unset, `S = L`.
2. If `L < 11`: `C = C + 1`, wrapping to 0 above 4. Then
   - if `S < L`: show `6S + 5`, then `S = S + 1`, `C = 4`;
   - if `S > L`: `S = S - 1`, show `6S + 5`, `C = 4`;
   - otherwise show `6S + C`.
3. If `L = 11`: if `S = 11`, `C = C + 1`; otherwise `S = 11`, `C = 0` (no
   stepping through levels). `C` is clamped to 14 and frame `66 + C` is shown,
   holding frame 80.

The candle therefore steps one level per rendered frame through transition
frames and flickers in a five-frame loop. Frames on consecutive rendered
frames:

| Situation | Frames |
|---|---|
| Spawn at health 100 | 1, 2, 3, 4, 0, 1, 2, … |
| Level 0, health drops to 50 | 5, 11, 17, 23, 29, 30, 31, 32, 33, 34, 30, … |
| Level 5, health restored to 100 | 29, 23, 17, 11, 5, 0, 1, 2, … |
| Level 5, health drops to 0 | 66, 67, …, 80, 80, … |
| Snuffed, health set to 100 | 65, 59, 53, …, 11, 5, 0, 1, … |

### Visibility

`[screen] candle_mode` selects the visibility mode (shipped `2`):

| Mode | Candle |
|---:|---|
| 0 | Shown only while the show timer runs. |
| 1 | Always shown. |
| 2 | Always shown, with the health number below it. |

In every mode the candle is drawn only while lockout is exactly 0. The show
timer `T` (ticks) is set:

- to 25 when the player's health changes (damage, healing, health script
  functions) while `T` is below 50 (some health script functions only while
  `T` is at most 25);
- to 25 when the player draws or sheathes the sword;
- to 50 on every frame while the sword is drawn and lockout is 0;
- to 50 while the inventory screen is open (which draws the candle itself),
  and to 0 when it closes.

After the candle is drawn, a timer with `0 < T < 50` loses the frame's ticks,
stopping at 0. A value of 50 never decays; it is replaced by the events above.
`T` does not decay while lockout hides the candle, and it is stored in save
games. In mode 0 the candle is thus visible while the sword is drawn, for 25
ticks after a health change or a draw/sheathe, and in the inventory.

### Health number

In mode 2 the player's `attr_health` is drawn as a decimal integer. It is
centred, by the [subtitle centring rule](subtitles.md#glyphs), on
`x = candle_xco + width / 2 − 5` (integer half) with its top at
`y = candle_yco + height`; with shipped values, centred on x = 32 at y = 106.

### Mode key

F8 (scan code 66, not configurable in KEYS.INI) cycles `candle_mode`
0 → 1 → 2 → 0 and immediately writes the new value to SYSTEM.INI
`[SCREEN] CANDLE_MODE`. No notice is shown.

## Active item

The icon is frame `bitmap[x]` of ITEM.INI `bitmap_file` for the selected item
`x`, drawn at `(pickup_xco, pickup_yco)`. The HUD does not use
`bitmap_selected_file`. All 87 shipped items define `bitmap[x]`.
`pickbox.gxa` and `pickblob.gxa` are not used: no box or blob is drawn behind
the icon.

### Count

When the selected item's count is greater than 1, the count is drawn as a
decimal integer, left-aligned with its top-left at
`(icon_x + width − 32, icon_y + 8)`: (99, 394) with shipped values. At a new
game the gold count is 53 (`player_total[2] = 52` plus one from
`start_item_list`), so the gold icon shows `53`.

### Selection

- The initial selection is ITEM.INI `start_item_select` (shipped 1, the
  sabre), or the first item when absent.
- KEYS.INI `[input] prev_key` / `next_key` step once per press through items in
  ID order, wrapping, and skipping items with `hide[x] = 1` or a count of 0.
- Cycling is disabled while the sword is drawn or `attr_holding_weapon` is
  set; the icon stays visible. Drawing or sheathing does not change the
  selection.
- When the selected item's count drops below 1, the selection moves to the
  next valid item; with none, nothing is selected and nothing is drawn.

### Visibility

Evaluated each frame in this order:

1. Lockout 1: neither icon nor prompt is drawn, and the timers below hold.
2. Activation hold-off `A > 0`: `A` loses the frame's ticks, stopping at 0,
   and nothing is drawn.
3. A pending activation prompt replaces the icon (see below).
4. `attr_player_type` 1: nothing is drawn.
5. Otherwise the icon and count are drawn.

Lockout values other than 1 do not hide the icon.

## Prompt text

A single text line is drawn left-aligned from `x = pickup_xco` with its top at
`y = pickup_text_yco` (shipped 12, 436).

### Activation prompt

The script function `Activate(label)` (also used by `TorchActivate`) offers
RTX entry `label` as the prompt when lockout is not 1, no dialogue topic menu
is open, the sword is not drawn, no other object holds the activation, the
hold-off `A` is 0, and no prompt is already pending for the frame. Further
player-state conditions exist; which states they are is unknown. The prompt
text is the RTX entry's text.

- If the use key is held when `Activate` is called, the activation fires:
  the function returns true and `A` is set to 22 ticks, blanking the icon and
  prompt area. The script function `Deactivate` clears `A`.
- Otherwise the prompt is drawn in place of the icon.
- While a prompt is pending, holding the previous/next item key sets an
  override timer to `rtx_pickup_override_time` (shipped 24 ticks). While it
  runs, the prompt is suppressed, the icon is drawn and items can be cycled.
  It decays by ticks and is cleared when no prompt is pending.
- While the dialogue topic menu is open, and in some player states, a pending
  prompt draws nothing and still hides the icon.

### Item name

The selected item's `name[x]` text is drawn here only when the item has no
`bitmap[x]`, which no shipped item does. Cycling items does not show the item
name.

## Compass

The compass is drawn at `(compass_xco, compass_yco)` when lockout is not 1,
`attr_player_type` is not 1, the player owns item 0, and the compass display
flag is on.

The flag starts off, toggles each time item 0 is used, and is stored in save
games. Receiving item 0 does not set it. This differs from the ITEM.INI
comment that giving the player item 0 "will automatically turn on the compass
display"; whether a shipped script uses the item on pickup has not been
observed.

The frame is

```text
frame = ((-(heading + world_compass[N])) mod 2048) >> 6
```

where `heading` is the player [heading](player.md#coordinate-and-heading-units)
and `world_compass[N]` the current world's [WORLD.INI](../config/world-ini.md)
value, in the same 2048-unit circle. Shipped worlds 3 (2000), 7 (1024),
13 (1536), 15 (1536), 20 (512), 22 (1024) and 23 (1024) set it; it is 0
elsewhere. Turning right increases the frame index.

| Heading | `world_compass` | Frame |
|---:|---:|---:|
| 0 | 0 | 0 |
| 1985–2047 | 0 | 0 |
| 1984 | 0 | 1 |
| 1024 | 0 | 16 |
| 65 | 0 | 30 |
| 1–64 | 0 | 31 |
| 0 | 2000 | 0 |
| 1000 | 2000 | 17 |

Which compass direction frame 0 depicts is a property of the artwork and is
not established here.

## Power-up slots

Two slots show frames of `[system] game_bitmap` (`system\powerup.gxa`: frame 0
gauntlet, frame 1 shield) at `(game_xco1, game_yco1)` and
`(game_xco2, game_yco2)`, slot 1 then slot 2, while lockout is exactly 0.

- `ShowBitmap(id, frame)` requires `id` 0 or 1 and a valid frame; other values
  are a fatal error. If `frame` is already shown in either slot, nothing
  changes. Otherwise it uses slot 1 if empty, else slot 2 if empty, else the
  slot after the one last chosen (choosing either slot, including an empty
  one, makes the other slot next). The slot stores `(id, frame)`.
- `UnShowBitmap(id)` clears slot 1 if its id matches, otherwise slot 2 if its
  id matches.
- Slot contents are stored in save games.

Which shipped scripts call these functions has not been catalogued.

## Logbook icon

When a new logbook entry is written and the icon is not already playing,
`SCROLL.GXA` plays frames 0–31 once at `(logbook_xco, logbook_yco)`, one
frame per rendered frame, then disappears. It is not hidden by lockout.
Opening the logbook screen stops it.

## Toggle notices

Tab (scan code 15) toggles walk mode and F7 (scan code 65) toggles
auto-defend; neither key is configurable in KEYS.INI. Each writes SYSTEM.INI
`[CYRUS] WALK_MODE` or `[CYRUS] AUTO_DEFEND` and shows an RTX string for 24
ticks, centred on x = 320: walk mode (`#WON` / `#WOF`) with its top at
y = 10, auto-defend (`#DON` / `#DOF`) at y = 30.

## Draw order

After the 3D scene, later entries over earlier ones:

1. Compass
2. Candle, then its health number
3. Logbook icon
4. Walk-mode and auto-defend notices
5. Power-up slot 1, slot 2
6. Activation prompt, or item icon then count (or item name)
7. A further pair of bitmaps whose content is unknown
8. [Subtitles](subtitles.md), the dialogue topic menu and later layers
