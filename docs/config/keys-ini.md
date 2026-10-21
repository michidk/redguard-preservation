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

Controls capture reads the same table: it accepts a keyboard scancode, a mouse
button release (128 or 129) or a joystick button (132 through 135).

## Action semantics

The shipped manual, User's Guide page 1, identifies Shift as Walk/Run and
Ctrl as Activate/Attack. These correspond to `key_a` and `key_d`, respectively,
in the GOG keyboard layout. The example bindings above describe one layout;
read the installed file rather than assuming those codes are universal.

## External References

- [UESP: Redguard:Controls](https://en.uesp.net/wiki/Redguard:Controls)
