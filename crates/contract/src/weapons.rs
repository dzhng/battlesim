//! Weapon rows and mounts. A mount is one targeting owner that may hold
//! several ammunition kinds (the tank cannon's AP and HE); a hull lists its
//! mounts and a soldier kind the ones he carries (`catalog`).
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

/// One weapon or ammunition row. A row may `extends` another and give only
/// what differs, as unit types do; the rows are resolved flat at load.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponDefinition {
    /// What the player reads on the unit card.
    pub name: String,
    pub description: String,
    /// Its generated icon, `assets/icons/weapons/<icon>.svg`.
    pub icon: String,
    #[serde(flatten)]
    pub ballistics: WeaponBallistics,
    /// The unit's always-available gun (W09).
    #[serde(default)]
    pub default: bool,
    /// Aims, reloads and fires only while the unit is stationary (W03).
    pub stationary: bool,
    pub aim_s: f64,
    pub reload_s: f64,
    /// Rounds in a magazine/belt and spacing within it; absent is single-shot.
    #[serde(default)]
    pub magazine: Option<Magazine>,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Magazine {
    pub rounds: u32,
    pub shot_interval_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountDefinition {
    pub name: String,
    /// Ammunition kinds available to the mount, by weapon row name.
    pub weapons: Vec<String>,
    /// Every living carrier has an independent weapon cycle.
    #[serde(default)]
    pub squad: bool,
    /// A soldier's weapon that passes to the next living soldier when its
    /// carrier falls, so the squad keeps it while anyone remains. Any other
    /// soldier's weapon is lost with him.
    #[serde(default)]
    pub special: bool,
    /// Traverses at the turret rate; fires only within the bearing tolerance.
    #[serde(default)]
    pub turret: bool,
    /// The earlier mount of the same unit whose turret carries this one (a
    /// roof gun on the cannon's turret). Absent, the hull carries it.
    #[serde(default)]
    pub on: Option<String>,
    /// Where this mount turns, in its carrier's frame (forward, left, up
    /// from the hull origin), turned by the carrier's bearing (or the
    /// hull's yaw).
    #[serde(default)]
    pub pivot_m: [f64; 3],
    /// Its muzzle from the pivot (forward, left, up), turned by this
    /// mount's own bearing. Absent: a hand weapon, fired from the soldier's
    /// muzzle height (`physics.infantry_muzzle_m`).
    #[serde(default)]
    pub muzzle_m: Option<[f64; 3]>,
}

pub type WeaponRules = BTreeMap<String, WeaponDefinition>;

/// The fixture's `weapons`, with `extends` resolved and abstract rows dropped.
pub fn resolve_weapons<'de, D: serde::Deserializer<'de>>(d: D) -> Result<WeaponRules, D::Error> {
    use serde::de::Error;
    let rows = serde_json::Map::deserialize(d)?;
    crate::catalog::inherit("weapons", &rows)
        .map_err(Error::custom)?
        .into_iter()
        .map(|(id, row)| {
            serde_json::from_value::<WeaponDefinition>(row)
                .and_then(|def| {
                    if let Some(m) = def.magazine {
                        if m.rounds < 2 || !m.shot_interval_s.is_finite() || m.shot_interval_s <= 0.0
                            || !def.reload_s.is_finite() || def.reload_s <= m.shot_interval_s {
                            return Err(serde::de::Error::custom("magazine needs at least two rounds, positive shot interval, and a longer finite reload"));
                        }
                    }
                    Ok((id.clone(), def))
                })
                .map_err(|e| Error::custom(format!("weapons.{id}: {e}")))
        })
        .collect()
}
