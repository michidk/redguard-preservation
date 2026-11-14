# SideWinder controller profiles

SWG and SWX files store Microsoft SideWinder controller preferences and control mappings.

`REDGUARD.SWG` is the profile for the Microsoft SideWinder Gamepad.
`redguard.swx` is the profile for the Microsoft SideWinder Freestyle Pro.
Both use the same version 3 binary layout.

## Overall Structure

```text
[Header — size, version, signature, device identifier, and settings]
[Control records — fixed-size labels and mapping data]
```

Integers use little-endian byte order. The header occupies 142 bytes, and
each control record occupies 684 bytes. Each shipped profile contains 27
control records and occupies 18,610 bytes.

## Header

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 4 | `u32` | file_size | Total file size, including the header. |
| `0x04` | 4 | `u32` | version | Profile version, `3`. |
| `0x08` | 4 | `u32` | field_08 | Zero in the shipped profiles. |
| `0x0C` | 10 | `[u8; 10]` | signature | ASCII `SideWinder`, without a terminating NUL. |
| `0x16` | 16 | `[u8; 16]` | device_id | Device identifier bytes. |
| `0x26` | 104 | `[u8; 104]` | settings | Opaque controller settings. |

The Gamepad identifier is `84 CE 48 C9 26 90 D0 11 9F FE 00 A0 C9 11 F5 AF`.
The Freestyle Pro identifier is `86 CE 48 C9 26 90 D0 11 9F FE 00 A0 C9 11 F5 AF`.

## Control Records

Offsets in this table are relative to the beginning of a control record.

| Offset | Size | Type | Name | Description |
|---|---|---|---|---|
| `0x00` | 64 | `[u8; 64]` | name | Label ending at the first NUL, or at the end of the field. |
| `0x40` | 4 | `u32` | control_mask | Controller input the record assigns. See [Record order](#record-order). |
| `0x44` | 4 | `u32` | field_44 | `0` or `1`. |
| `0x48` | 4 | `u32` | field_48 | `1` in every record that sends keys, `0` otherwise. |
| `0x4C` | 4 | `u32` | field_4c | Zero in the shipped profiles. |
| `0x50` | 1 | `u8` | key_count | Number of keys the record sends at once. |
| `0x51` | 2 × `key_count` | `[[u8; 2]]` | keys | Keys sent: a PC keyboard scancode followed by a prefix byte, `0xE0` for extended keys such as the arrow keys and `0x00` otherwise. |
| — | — | — | — | Remaining bytes up to `0x2AC` hold further mapping settings; `0x57` and `0x59` (`u16`) and `0x5D` (`u16`, `0xFFFF`) are nonzero in records that send keys. |

### Record order

Both profiles use the same 27-record order:

| Records | Assigned input | `control_mask` |
|---|---|---|
| 0–7 | Buttons A, B, C, X, Y, Z, Left Trigger, Right Trigger | One bit each, `0x0001` through `0x0080` |
| 8 | Start | `0x0100` |
| 9–17 | The same nine controls while the modifier button is held (`Button M` on the Gamepad, `Shift` on the Freestyle Pro) | Control bit plus `0x0200` |
| 18–25 | Up, Up + Right, Right, Down + Right, Down, Down + Left, Left, Up + Left | One bit each, `0x0001` through `0x0080` |
| 26 | Unlabelled | `0x0100` in the Gamepad profile, `0x0001` in the Freestyle Pro profile |

### Shipped mappings

`REDGUARD.SWG` (Gamepad), scancodes in decimal:

| Control | Label | Key |
|---|---|---|
| A | `Use/Attack` | 29 (Ctrl) |
| B | `Jump` | 56 (Alt) |
| C | `Inventory` | 23 (`I`) |
| X | `View/Defend` | 42 (Left Shift) |
| Y | `Walk` | 57 (Space) |
| Z | `Quick Sword` | 31 (`S`) |
| Left Trigger | `Last Item` | 51 (comma) |
| Right Trigger | `Next Item` | 52 (period) |
| Start | `Menu` | 1 (Escape) |
| M + Left Trigger | `Log` | 38 (`L`) |
| M + Right Trigger | (no label) | 50 (`M`) |
| D-pad directions | direction labels | Extended arrow keys; diagonals send two arrow keys |

The labels are text only; the keys determine the effect, through the game's
keyboard bindings. `redguard.swx` (Freestyle Pro) sends keys only for Button Y
(`I`), Button Z (`S`), Left Trigger (comma), Right Trigger (period), and Start
(Escape).

An empty label does not remove its record. Bytes following the first NUL
remain part of the stored field. Text bytes map directly to Unicode code
points U+0000 through U+00FF without selecting a Windows code page.

These profiles are controller configuration files. They do not contain game
assets, save state, or swap memory. [KEYS.INI](../config/keys-ini.md) describes
the separate game keyboard bindings.

## Use by the game

Neither `RGFX.EXE` nor the `REDGUARD.EXE` launcher opens these files. They are
read by Microsoft's SideWinder software on the host system, which sends PC
keyboard scancodes for controller input; see [Shipped
mappings](#shipped-mappings). The game receives those keystrokes through the
keyboard bindings in [KEYS.INI](../config/keys-ini.md), not through its joystick
input.

The GOG release runs `RGFX.EXE` in DOSBox, where the SideWinder software is not
involved, so these profiles have no effect. Controller input reaches the game
only through DOSBox's emulated gameport or keyboard mapper; see
[KEYS.INI: GOG release](../config/keys-ini.md#gog-release).

See [Bundled support resources](BUNDLED-RESOURCES.md) for the other support
files shipped with the game.

## External References

- [Microsoft SideWinder controller families](https://en.wikipedia.org/wiki/Microsoft_SideWinder)
