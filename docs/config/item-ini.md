# ITEM.INI

Item database defining all collectible objects, weapons, potions, keys, and quest items with their properties, models, and scripted behaviors.

Shipped sample path: `/Redguard/ITEM.INI` (e.g. `.../GOG Galaxy/Redguard/Redguard/ITEM.INI`).

The file is data-driven: every game object from the compass to the soul sword is defined here with associated 3D models, inventory bitmaps, and AI scripts. Item 0 is always the compass, a special case — giving the player this item automatically turns on the compass display. Any item index declared as 0 in a weapon or hand-object field is treated as "no object" for this reason.

## File Structure

The file contains a single `[items]` section with two parts:

1. A header block with global inventory settings.
2. Per-item field blocks indexed by item ID (e.g. `name[0]`, `type[1]`).

### `[items]` Header Fields

| Field | Value | Description |
|---|---|---|
| `bitmap_file` | `SYSTEM\PICKUPS.GXA` | GXA texture atlas for unselected inventory icons. |
| `bitmap_selected_file` | `SYSTEM\PICKUPSS.GXA` | GXA texture atlas for selected inventory icons. |
| `start_item_list` | `1, 2, 4, 18` | Item IDs the player starts the game with. |
| `start_item_select` | `1` | Item ID selected by default at game start. |
| `additional_length` | `8` | Extra inventory display length (slots). |
| `weapon_sphere_size` | `10` | Collision sphere radius for weapon hit detection. |
| `default_weapon_item` | `1` | Item ID used as the default weapon (the sabre). |
| `torch_time` | `60` | Torch burn duration in seconds (tolerance of +/- 3 seconds). |

### Per-Item Field Schema

All fields are optional. Missing fields are treated as unused. The index `x` in each field name is the item ID used in all script commands.

#### General fields

| Field | Description |
|---|---|
| `type[x]` | Item type: `0` = general item, `1` = sword/weapon (can be drawn and sheathed, has both drawn and sheathed 3D files), `2` = hand object (may override a weapon, e.g. torch). |
| `flags[x]` | Bit-field of misc flags. `1` = remove on drop regardless of player total; `2` = drop item after use; `4` = remove item after use. Add values together to combine. |
| `hide[x]` | When `1`, the item is hidden from screen and inventory but still exists. Scripts may still call `UseItem()` on it; the player cannot select or use it directly. |
| `total[x]` | Total number of this item in the game world. |
| `player_max[x]` | Maximum number of this item the player can carry. |
| `player_total[x]` | Number of this item the player starts the game with. |
| `name[x]` | Short name RTX dialogue label (e.g. `?xsw`). |
| `description[x]` | Longer item description RTX dialogue label (e.g. `cisw`). |

`name[x]` and `description[x]` values are keys into the RTX dialogue/text system, not literal strings. Labels beginning with `?` are short display names; labels beginning with `ci` or `ti` are longer descriptions.

#### AI script fields

| Field | Description |
|---|---|
| `use_script[x]` | AI script file to execute when the item is used through the shell system. |
| `script_instances[x]` | Maximum concurrent instances of the use script. `1` prevents a new instance from starting if one is already running. `0` means unlimited instances. |

#### Objects and bitmaps

| Field | Description |
|---|---|
| `bitmap[x]` | Bitmap index into the `bitmap_file` GXA atlas for in-game display. |
| `inventory_object_file[x]` | 3D object file used in the inventory screen. |
| `game_object_file[x]` | 3D object file used in the game world. |

#### Drop and add lists

| Field | Description |
|---|---|
| `add_item_list[x]` | List of item IDs to add to the player when this item is picked up. |
| `drop_add_item_list[x]` | List of item IDs to drop into the world when this item is picked up. |
| `remove_drop_item_list[x]` | List of item IDs to remove from the world when this item is dropped. |
| `required_item_list[x]` | List of item IDs the player must already have before picking up this item. |
| `drop_use_item_list[x]` | List of item IDs to drop into the world when this item is used. |

#### Weapon fields

| Field | Description |
|---|---|
| `hand_object_file[x]` | 3D object for the item when held in hand. For weapon-type items this must be a weapon object. Ignored for general-type items. |
| `hilt_object_file[x]` | 3D object for the item when sheathed. |

Object file paths reference `3DART\` assets, which are the software-renderer models from the original CD release. The GOG release uses `fxart/` equivalents at runtime; the `3DART\` paths in this file reflect the original CD asset layout.

## Item Catalog

87 items are defined (IDs 0 through 86). Item 0 is always the compass.

| ID | Comment | Type |
|---|---|---|
| 0 | compass | 0 (general) |
| 1 | sabre | 1 (weapon) |
| 2 | gold | 0 (general) |
| 3 | Potion of ironskin | 0 (general) |
| 4 | health potion | 0 (general) |
| 5 | ring of invisibility | 0 (general) |
| 6 | Voa's ring | 0 (general) |
| 7 | guard sword | 1 (weapon) |
| 8 | rusty key | 0 (general) |
| 9 | gold Key | 0 (general) |
| 10 | silver key | 0 (general) |
| 11 | amulet | 0 (general) |
| 12 | soul gem | 0 (general) |
| 13 | soul sword | 1 (weapon) |
| 14 | crow bar | 0 (general) |
| 15 | rune 1 | 0 (general) |
| 16 | rune 2 | 0 (general) |
| 17 | rune 3 | 0 (general) |
| 18 | letter | 0 (general) |
| 19 | orc's blood | 0 (general) |
| 20 | orc's blood w/stuff | 0 (general) |
| 21 | spider's milk | 0 (general) |
| 22 | spider's milk w/stuff | 0 (general) |
| 23 | ectoplasm | 0 (general) |
| 24 | ectoplasm w/stuff | 0 (general) |
| 25 | hist sap | 0 (general) |
| 26 | hist sap w/ stuff | 0 (general) |
| 27 | book of dw lore | 0 (general) |
| 28 | dwarven gear | 0 (general) |
| 29 | vial | 0 (general) |
| 30 | vial w/elixir | 0 (general) |
| 31 | iron weight | 0 (general) |
| 32 | bucket | 0 (general) |
| 33 | bucket w/water | 0 (general) |
| 34 | fist rune | 0 (general) |
| 35 | elven book (copy) DO NOT USE | 0 (general) |
| 36 | elven book | 0 (general) |
| 37 | redguard book | 0 (general) |
| 38 | flora of hammerfell book | 0 (general) |
| 39 | reference map | 0 (general) |
| 40 | pouch | 0 (general) |
| 41 | trithick's map piece | 0 (general) |
| 42 | silver ship | 0 (general) |
| 43 | shovel | 0 (general) |
| 44 | aloe | 0 (general) |
| 45 | torch | 3 (hand object) |
| 46 | monocle | 0 (general) |
| 47 | red flag | 0 (general) |
| 48 | silver locket that talks of lakene | 0 (general) |
| 49 | redguard insignia | 0 (general) |
| 50 | joto's piece | 0 (general) |
| 51 | flask of lillandril | 0 (general) |
| 52 | talisman of hunding | 0 (general) |
| 53 | izara's journal open | 0 (general) |
| 54 | canah feather | 0 (general) |
| 55 | kithral's journal | 0 (general) |
| 56 | starsign's book | 0 (general) |
| 57 | izara's journal closed | 0 (general) |
| 58 | starstone | 0 (general) |
| 59 | krisandra's key | 0 (general) |
| 60 | key to iszara's lodge | 0 (general) |
| 61 | necro book - view only | 0 (general) |
| 62 | bar mug ..for thugs hands, etc | 0 (general) |
| 63 | mariah's watering can | 0 (general) |
| 64 | glass bottle | 0 (general) |
| 65 | glass bottle with water | 0 (general) |
| 66 | glass bottle w/aloe and water | 0 (general) |
| 67 | Strength Potion | 0 (general) |
| 68 | bandage | 0 (general) |
| 69 | bloody bandage | 0 (general) |
| 70 | skeleton sword | 1 (weapon) |
| 71 | keep out poster...mage's guild | 0 (general) |
| 72 | keep out poster...dwarven ruins | 0 (general) |
| 73 | tobias mar mug | 0 (general) |
| 74 | Bone Key | 0 (general) |
| 75 | flaming sabre | 1 (weapon) |
| 76 | goblin sword | 1 (weapon) |
| 77 | ogre's axe | 1 (weapon) |
| 78 | dram's sword | 1 (weapon) |
| 79 | palace key | 0 (general) |
| 80 | dram's bow | 0 (general) |
| 81 | dram's arrow | 0 (general) |
| 82 | silver locket that just says silver locker | 0 (general) |
| 83 | island map | 0 (general) |
| 84 | wanted poster | 0 (general) |
| 85 | palace diagram | 0 (general) |
| 86 | last | 0 (general) |

Notes on specific entries:

- Items 19/20, 21/22, 23/24, 25/26 are paired: the base ingredient and the same ingredient already combined with another substance. Both variants share the same `name[x]` label.
- Items 29/30 are the empty vial and the vial filled with elixir.
- Items 32/33 are the empty bucket and the bucket filled with water.
- Items 53/57 are the open and closed versions of Izara's journal, sharing the same `name[x]` label.
- Item 35 is marked "DO NOT USE" in the file comment; item 36 is the live elven book.
- Item 45 (torch) uses type `3` in the file, which the comment block does not define. The comment block describes type `2` as the hand-object type; type `3` appears to be an extension of that for the torch specifically.
- Items 62 and 63 (bar mug, watering can) reuse the journal inventory model (`Ijourn.3D`) as a placeholder, with distinct `game_object_file` paths for the actual in-world mesh.
- Items 80 and 81 (dram's bow and arrow) reuse the bone key inventory model (`ibone.3d`) as a placeholder.
- Item 86 is labeled "last" with no meaningful content, serving as a sentinel or end-of-list marker.

## External References

- [UESP: Redguard:Console](https://en.uesp.net/wiki/Redguard:Console) — documents the `item add` command, which uses item IDs matching the index numbers in this file.
