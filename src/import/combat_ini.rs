//! Typed combat moves and taunts from COMBAT.INI.
use super::ini::{ini_enum, ini_section, required, sections};
use crate::{Result, error::Error};
use std::collections::BTreeMap;

/// Combat definitions indexed by the numeric suffix of their section names.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CombatIni {
    pub misc: CombatSettings,
    pub attacks: BTreeMap<u32, Attack>,
    pub defenses: BTreeMap<u32, Defense>,
    pub taunts: BTreeMap<u32, Taunt>,
}

impl CombatIni {
    /// Requires `[misc]`. Sparse IDs and omitted fields are preserved.
    /// Section/key matching ignores case and the first duplicate key wins.
    pub fn parse(text: &str) -> Result<Self> {
        let sections = sections(text, "COMBAT.INI")?;
        let mut result = Self {
            misc: CombatSettings::parse(Some(required(&sections, "COMBAT.INI", "misc")?))?,
            attacks: BTreeMap::new(),
            defenses: BTreeMap::new(),
            taunts: BTreeMap::new(),
        };
        for (name, section) in &sections {
            for prefix in ["attack", "defend", "taunt"] {
                let Some(suffix) = name.strip_prefix(prefix) else {
                    continue;
                };
                let id = suffix.parse::<u32>().map_err(|_| {
                    Error::Parse(format!("COMBAT.INI [{name}]: invalid section index"))
                })?;
                let duplicate = match prefix {
                    "attack" => result
                        .attacks
                        .insert(id, Attack::parse(Some(section))?)
                        .is_some(),
                    "defend" => result
                        .defenses
                        .insert(id, Defense::parse(Some(section))?)
                        .is_some(),
                    _ => result
                        .taunts
                        .insert(id, Taunt::parse(Some(section))?)
                        .is_some(),
                };
                if duplicate {
                    return Err(Error::Parse(format!(
                        "COMBAT.INI [{name}]: duplicate numeric section index {id}"
                    )));
                }
            }
        }
        Ok(result)
    }
}

ini_section! {
    /// Global combat parameters in original engine units.
    CombatSettings {
        sword_clank_1: i32 => "sword_clank_1",
        sword_clank_2: i32 => "sword_clank_2",
        non_engaged_dist: i32 => "non_engaged_dist",
        in_face_dist: i32 => "in_face_dist",
        non_engaged_spacing: i32 => "non_engaged_spacing",
        in_combat_threshold: i32 => "in_combat_threshold",
        player_can_die: bool => "player_can_die",
        cyrus_defend_interval: i32 => "cyrus_defend_interval",
        node_timer: i32 => "node_timer",
        max_taunts: u32 => "max_taunts",
    }
}

ini_enum! {
    /// Attack category; undocumented codes retain their numeric value.
    AttackType { Melee = 0, Missile = 1, Combo = 2, Stab = 3, Finishing = 4 }
}

ini_enum! {
    /// Vertical attack targeting zone.
    Elevation { Regular = 0, Above = 1, Below = 2 }
}

ini_section! {
    /// One attack, including the optional sub-attack references of a combo.
    Attack {
        attack_type: AttackType => "type",
        arc: i32 => "arc",
        arc_center: i32 => "arc_center",
        elevation: Elevation => "elevation",
        min_range: i32 => "min_range",
        max_range: i32 => "max_range",
        first_collide_frame: i32 => "first_collide_frame",
        collide_duration: i32 => "collide_duration",
        damage: i32 => "damage",
        force: i32 => "force",
        defense: i32 => "defense",
        first_defend_frame: i32 => "first_defend_frame",
        defend_duration: i32 => "defend_duration",
        animation: i32 => "animation",
        col_vertex: i32 => "col_vertex",
        col_sphere_size: i32 => "col_sphere_size",
        num_attacks: u32 => "num_attacks",
        attack00: u32 => "attack00",
        attack01: u32 => "attack01",
        attack02: u32 => "attack02",
    }
}

ini_section! {
    /// One defense move. Omitted `arc`, including in defend00, remains `None`.
    Defense {
        arc: i32 => "arc",
        min_range: i32 => "min_range",
        max_range: i32 => "max_range",
        first_collide_frame: i32 => "first_collide_frame",
        collide_duration: i32 => "collide_duration",
        animation: i32 => "animation",
    }
}

ini_enum! {
    /// Event that triggers a combat voice line.
    TauntTrigger {
        BeginCombat = 0,
        Attack = 1,
        HurtByOpponent = 2,
        HitOpponent = 3,
        Defend = 4,
        Move = 5,
        Death = 6,
        KilledOpponent = 7,
        WantSwitchOut = 8,
        WantSwitchIn = 9,
        UnarmedMaleOpponent = 10,
        UnarmedFemaleOpponent = 11,
    }
}

ini_section! {
    /// Combat voice label, preserving its case and any `&NNN` reference.
    Taunt {
        trigger: TauntTrigger => "type",
        rtx_label: String => "rtx_label",
        animation: i32 => "animation",
    }
}
