//! Weapon rows and mounts from the fixture. A mount is one aim/reload owner
//! that may hold several ammunition kinds (the tank cannon's AP and HE).
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ballistics::WeaponBallistics;

/// `"unlimited"` or a round count. Unlimited is its own state, never a big number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmmoCapacity {
    Unlimited(UnlimitedTag),
    Rounds(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnlimitedTag {
    Unlimited,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponDefinition {
    #[serde(flatten)]
    pub ballistics: WeaponBallistics,
    /// The unit's always-available gun (W09).
    #[serde(default)]
    pub default: bool,
    /// Aims, reloads and fires only while the unit is stationary (W03).
    pub stationary: bool,
    pub range_m: f64,
    pub aim_s: f64,
    pub reload_s: f64,
    pub ammo: AmmoCapacity,
    pub penetration: f64,
    pub damage: f64,
    pub near_miss_suppression: f64,
    pub blast_radius_m: f64,
    #[serde(default)]
    pub structural_damage: f64,
    /// Dedicated anti-armour: fires only at identified vehicles (W10).
    #[serde(default)]
    pub anti_armor: bool,
    /// Share of `damage` a vehicle still takes when this round fails to
    /// penetrate the struck face: HE's partial effect on armour. Absent
    /// means none (the round does nothing to armour it cannot pierce).
    #[serde(default)]
    pub armor_fraction: f64,
    /// Armour-piercing: preferred against identified vehicles, never at contacts.
    #[serde(default)]
    pub armor_piercing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountDefinition {
    pub name: String,
    /// Ammunition kinds sharing this mount's aim and reload, by weapon row name.
    pub weapons: Vec<String>,
    /// Every living squad member fires one round per shot.
    #[serde(default)]
    pub squad: bool,
    /// Traverses at the turret rate; fires only within the bearing tolerance.
    #[serde(default)]
    pub turret: bool,
}

pub type WeaponRules = BTreeMap<String, WeaponDefinition>;
pub type MountRules = BTreeMap<String, Vec<MountDefinition>>;
