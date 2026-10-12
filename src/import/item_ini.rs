//! Typed inventory settings and item definitions from ITEM.INI.
use super::ini::{Section, ini_enum, ini_section, required, sections};
use crate::{Result, error::Error};
use std::collections::BTreeMap;

/// Inventory configuration with sparse item IDs preserved.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ItemIni {
    pub settings: InventorySettings,
    pub items: BTreeMap<u32, Item>,
}

impl ItemIni {
    /// Requires `[items]`. Empty values are absent; the first duplicate key wins.
    /// Unknown fields are ignored; malformed known values and item IDs fail.
    pub fn parse(text: &str) -> Result<Self> {
        let sections = sections(text, "ITEM.INI")?;
        let section = required(&sections, "ITEM.INI", "items")?;
        let mut fields = BTreeMap::new();
        for (key, &(value, line)) in &section.values {
            let Some((field, suffix)) = key.split_once('[') else {
                if Item::is_field(key) {
                    return Err(Error::Parse(format!(
                        "ITEM.INI [items] {key}, line {line}: missing item index"
                    )));
                }
                continue;
            };
            if !Item::is_field(field) {
                continue;
            }
            let id = suffix
                .strip_suffix(']')
                .and_then(|id| id.parse::<u32>().ok())
                .ok_or_else(|| {
                    Error::Parse(format!(
                        "ITEM.INI [items] {key}, line {line}: invalid item index"
                    ))
                })?;
            if value.is_empty() {
                continue;
            }
            let item = fields.entry(id).or_insert_with(BTreeMap::new);
            if item.insert(field.to_owned(), (value, line)).is_some() {
                return Err(Error::Parse(format!(
                    "ITEM.INI [items] {key}, line {line}: duplicate numeric item index"
                )));
            }
        }
        let mut items = BTreeMap::new();
        for (id, values) in fields {
            let item = Section {
                file: "ITEM.INI",
                name: format!("items, item {id}"),
                values,
            };
            items.insert(id, Item::parse(Some(&item))?);
        }
        Ok(Self {
            settings: InventorySettings::parse(Some(section))?,
            items,
        })
    }
}

ini_section! {
    /// Global inventory configuration, including the ordered starting item list.
    InventorySettings {
        bitmap_file: String => "bitmap_file",
        bitmap_selected_file: String => "bitmap_selected_file",
        start_item_list: Vec<u32> => "start_item_list",
        start_item_select: u32 => "start_item_select",
        additional_length: i32 => "additional_length",
        weapon_sphere_size: i32 => "weapon_sphere_size",
        default_weapon_item: u32 => "default_weapon_item",
        torch_time: i32 => "torch_time",
    }
}

ini_enum! {
    /// Item type codes. The shipped torch's undocumented code 3 is `Unknown(3)`.
    ItemType { General = 0, Weapon = 1, HandObject = 2 }
}

ini_section! {
    /// One item's optional fields; labels and DOS paths retain their spelling.
    indexed Item {
        item_type: ItemType => "type",
        flags: u32 => "flags",
        hide: bool => "hide",
        total: i32 => "total",
        player_max: i32 => "player_max",
        player_total: i32 => "player_total",
        name: String => "name",
        description: String => "description",
        use_script: String => "use_script",
        script_instances: i32 => "script_instances",
        bitmap: u32 => "bitmap",
        inventory_object_file: String => "inventory_object_file",
        game_object_file: String => "game_object_file",
        add_item_list: Vec<u32> => "add_item_list",
        drop_add_item_list: Vec<u32> => "drop_add_item_list",
        remove_drop_item_list: Vec<u32> => "remove_drop_item_list",
        required_item_list: Vec<u32> => "required_item_list",
        drop_use_item_list: Vec<u32> => "drop_use_item_list",
        hand_object_file: String => "hand_object_file",
        hilt_object_file: String => "hilt_object_file",
    }
}
