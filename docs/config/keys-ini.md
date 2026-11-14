# KEYS.INI

Input binding configuration mapping keyboard scancodes, mouse buttons, and joystick axes to game actions.

Shipped path: `KEYS.INI` (game root directory).

## Sections

### `[input]`

Action-to-scancode bindings. Two binding slots exist per directional/action key (index `[0]` for keyboard, index `[1]` for joystick/gamepad).

| Key | Default | Description |
|---|---|---|
| `next_key` | `52` (period) | Next item in inventory. |
| `prev_key` | `51` (comma) | Previous item in inventory. |
| `inventory_key` | `23` (I) | Open the [inventory screen](../engine/hud.md#inventory-screen). |
| `log_key` | `38` (L) | Open logbook. |
| `quick_sword_key` | `16` (Q) | Quick-draw sword. |
| `quick_health_key` | `35` (H) | Quick-use health potion. |
| `map_key` | `50` (M) | Open map. |
| `key_up[0]` | `17` (W) | Move forward (keyboard). |
| `key_down[0]` | `31` (S) | Move backward (keyboard). |
| `key_left[0]` | `30` (A) | Turn left (keyboard). |
| `key_right[0]` | `32` (D) | Turn right (keyboard). |
| `key_a[0]` | `42` (L Shift) | Walk/run modifier (keyboard). |
| `key_b[0]` | `57` (Space) | Jump (keyboard). |
| `key_c[0]` | `56` (Alt) | View / Look (keyboard). |
| `key_d[0]` | `18` (E) | Activate / Attack (keyboard). |
| `key_up[1]` | `138` | Move forward (joystick). |
| `key_down[1]` | `139` | Move backward (joystick). |
| `key_left[1]` | `136` | Turn left (joystick). |
| `key_right[1]` | `137` | Turn right (joystick). |
| `key_a[1]` | `135` | Walk/run modifier (joystick button 4). |
| `key_b[1]` | `133` | Jump (joystick button 2). |
| `key_c[1]` | `134` | View / Look (joystick button 3). |
| `key_d[1]` | `132` | Activate / Attack (joystick button 1). |
| `user_key_up` | `0` | User-remapped forward key. |
| `user_key_down` | `0` | User-remapped backward key. |
| `user_key_left` | `0` | User-remapped left key. |
| `user_key_right` | `0` | User-remapped right key. |
| `user_key_a` | `0` | User-remapped walk/run modifier. |
| `user_key_b` | `0` | User-remapped jump key. |
| `user_key_c` | `0` | User-remapped view key. |
| `user_key_d` | `0` | User-remapped activate/attack key. |

A value of `0` in user remap fields means no override (use default binding).

The shipped file footer contains `NEXT_KEY = 37` and `PREV_KEY = 38` inside `[defined]`. These do not override bindings in `[input]`. Numeric configuration lookup returns the first matching key in the requested section.

### `[misc]`

| Key | Default | Description |
|---|---|---|
| `waiting_message` | `HIT KEY` | Prompt inside the Controls page capture brackets. |

### `[defined]`

Scancode-to-name lookup table used by the key binding UI. Maps scancode indices 0–139 to human-readable names.

Scancodes 0–127 are keyboard keys (standard PC AT scancodes):

| Range | Keys |
|---|---|
| 0 | `empty` (no key) |
| 1 | `escape` |
| 2–11 | `1` through `0` |
| 12–13 | `minus`, `plus` |
| 14 | `backspace` |
| 15 | `tab` |
| 16–25 | `q` through `p` |
| 26–27 | `l bracket`, `r bracket` |
| 28 | `enter` |
| 29 | `ctrl` |
| 30–38 | `a` through `l` |
| 39–41 | `:`, `"`, `tildy` |
| 42 | `l shift` |
| 43 | `\` |
| 44–50 | `z` through `m` |
| 51–53 | `comma`, `period`, `slash` |
| 54 | `r shift` |
| 55 | `prt scr` |
| 56 | `alt` |
| 57 | `space` |
| 58 | `caps lock` |
| 59–68 | `f1` through `f10` |
| 69–70 | `num lock`, `scrl lock` |
| 71–73 | `home`, `kbd up`, `page up` |
| 74 | `num minus` |
| 75 | `kbd left` |
| 77 | `kbd right` |
| 78 | `num plus` |
| 79–81 | `end`, `kbd down`, `page down` |
| 82–83 | `insert`, `delete` |
| 87–88 | `f11`, `f12` |
| 76, 84–86, 89–127 | `unkn NN` (unused scancodes) |

Scancodes 128–139 are mouse/joystick inputs:

| Scancode | Name |
|---|---|
| 128 | `left mouse` |
| 129 | `right mouse` |
| 130 | `mouse x` (axis) |
| 131 | `mouse y` (axis) |
| 132–135 | `button 1` through `button 4` (joystick) |
| 136–137 | `joy left`, `joy right` |
| 138–139 | `joy up`, `joy down` |

## Input state

Every input update fills one state table indexed by input code, then each
action reads the state at its primary code OR its secondary code (the `[0]` and
`[1]` bindings). Codes below 128 are keyboard scancodes written by the keyboard
interrupt. Codes 128 through 139 are written by the pointer and joystick polls,
which run at the start of each update:

| Code | Value |
|---|---|
| 128, 129 | Left and right mouse button **release**: nonzero for one update, when the button goes from pressed to released, and zero otherwise. A held button never reads as pressed. |
| 132–135 | Joystick buttons 1 through 4: nonzero while the button is pressed. |
| 136 | Joystick X reading is more than `joy_tolerance` below the calibrated center. |
| 137 | Joystick X reading is more than `joy_tolerance` above the calibrated center. |
| 138 | Joystick Y reading is more than `joy_tolerance` below the calibrated center. |
| 139 | Joystick Y reading is more than `joy_tolerance` above the calibrated center. |

### Joystick polling

The joystick is enabled only when the DOS joystick BIOS service reports a
nonzero axis value for stick A or stick B at startup. When enabled, one axis
reading taken at startup becomes the calibrated center for X and Y; it is never
recalibrated. When disabled, codes 132 through 139 always read zero.

An axis reading is a loop count, not a calibrated unit. The poll starts the
gameport one-shot timers with a write to port `0x201`, then re-reads that port
up to 6144 times. Each axis reads as the number of iterations completed when its
timer bit (bit 0 for X, bit 1 for Y) dropped. A poll that times out reports 0 for
both axes, including an axis that had already finished. The count therefore
scales with the stick's resistance and with the CPU's port-read time. Only the
first stick's axes are used. Each poll also records the change of each axis
reading since the previous poll.

Per update, with `tolerance` = `joy_tolerance`, `x` and `y` the readings and
`cx` and `cy` the calibrated centers, each comparison strict:

| Code | Reads 1 when |
|---|---|
| 136 | `x < cx - tolerance` |
| 137 | `x > cx + tolerance` |
| 138 | `y < cy - tolerance` |
| 139 | `y > cy + tolerance` |

Buttons 1 through 4 (codes 132 through 135) are the inverted high nibble of the
gameport port, bit 4 upward: buttons 1 and 2 of stick A, then buttons 1 and 2 of
stick B.

The game therefore uses at most two axes and four buttons of a controller.
[SideWinder profiles](../formats/SIDEWINDER.md) do not change this; they act
through keyboard input.

For an ordinary keyboard key that passes the interrupt handler's special-key
handling, a make event writes 1 to its 16-bit state slot and a break event
writes 0. A make writes the slot even when the key was already held.

Consumers can clear a key's slot, as the [candle mode key](../engine/hud.md#mode-key)
does when it changes mode. The slot can therefore be zero while the physical
key remains held. A later accepted make event sets it again without requiring
an intervening release; a release clears an unconsumed make.

Controls capture reads the same table: it accepts a keyboard scancode, a mouse
button release (128 or 129) or a joystick button (132 through 135).

## GOG release

The GOG release runs the game in DOSBox, so the gameport is DOSBox's emulation,
selected by `joysticktype` in the `[joystick]` section of the DOSBox
configuration. The game launch of the GOG `2.0.0.5` installer loads
`dosbox_redguard.conf` and `dosbox_redguard_single.conf`; the first sets
`joysticktype=none`, which disables the gameport emulation, so the game has no
joystick input. The `joysticktype=auto` in `DOSBOX/dosbox.conf` is not part of
that launch, and the Graphic Mode Setup tool changes only display settings.

With `none`, the bundled DOSBox build also creates no controller bindings in its
keyboard mapper, so a controller cannot be mapped to keys there either. With
`4axis`, DOSBox emulates one joystick with four buttons, all of which the game
reads. With `2axis` it emulates two two-button joysticks, so one host controller
provides only buttons 1 and 2. Once emulation is enabled, further controller
buttons reach the game only as keys assigned in DOSBox's keyboard mapper
(`mapperfile=mapper.txt`, which the installer does not include).

## Action semantics

The shipped manual, User's Guide page 1, identifies Shift as Walk/Run and
Ctrl as Activate/Attack. These correspond to `key_a` and `key_d`, respectively,
in the GOG keyboard layout. The example bindings above describe one layout;
read the installed file rather than assuming those codes are universal.

## External References

- [UESP: Redguard:Controls](https://en.uesp.net/wiki/Redguard:Controls)
