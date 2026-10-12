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
| `0x40` | 4 | `u32` | field_40 | Stored control value. |
| `0x44` | 616 | `[u8; 616]` | mapping_data | Opaque mapping and macro data. |

Labels include `Use/Attack`, `Jump`, and `Inventory` in the Gamepad profile,
and `Button A`, `Button B`, and `Button C` in the Freestyle Pro profile.
Both profiles contain direction labels such as `Up + Right`.

An empty label does not remove its record. Bytes following the first NUL
remain part of the stored field. Text bytes map directly to Unicode code
points U+0000 through U+00FF without selecting a Windows code page.

These profiles are controller configuration files. They do not contain game
assets, save state, or swap memory. [KEYS.INI](../config/keys-ini.md) describes
the separate game keyboard bindings.

Modern controller input uses game actions and bindings directly. These profiles
remain references for the original SideWinder presets; their hardware-specific
mapping data is separate from the game's input configuration. See
[Bundled support resources](BUNDLED-RESOURCES.md).

## External References

- [Microsoft SideWinder controller families](https://en.wikipedia.org/wiki/Microsoft_SideWinder)
