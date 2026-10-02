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
    /// Absent means continuous fire until the magazine is empty.
    #[serde(default)]
    pub burst: Option<Burst>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Burst {
    pub rounds: u32,
    /// Independently sampled aim delay, from zero to this limit, before each burst.
    pub aim_max_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountDefinition {
    pub name: String,
    /// Ammunition kinds available to the mount, by weapon row name.
    pub weapons: Vec<String>,
    /// Infantry's default guns share one targeting/readout row; every
    /// living carrier still has an independent weapon cycle.
    #[serde(default)]
    pub squad: bool,
    /// A soldier's weapon that passes to the next living soldier when its
    /// carrier falls. A soldier operates one such gun; additional recovered
    /// guns remain spares. Any other soldier's weapon is lost with him.
    #[serde(default)]
    pub special: bool,
    /// Appearance set worn by the current operator of a single infantry gun.
    /// Absent, the operator keeps his soldier kind's ordinary appearance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator_appearance: Vec<String>,
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
                    for (field, value) in [
                        ("damage", def.damage),
                        ("penetration", def.penetration),
                        ("aim_s", def.aim_s),
                        ("reload_s", def.reload_s),
                        ("blast_radius_m", def.blast_radius_m),
                        ("structural_damage", def.structural_damage),
                        ("near_miss_suppression", def.near_miss_suppression),
                        ("suppression_radius_m", def.ballistics.suppression_radius_m),
                    ] {
                        if !value.is_finite() || value < 0.0 {
                            return Err(serde::de::Error::custom(format!("{field} must be finite and nonnegative")));
                        }
                    }
                    if !(0.0..=1.0).contains(&def.armor_fraction) {
                        return Err(serde::de::Error::custom("armor_fraction must lie in [0, 1]"));
                    }
                    if def.ballistics.turn_deg_s.is_some_and(|rate| !rate.is_finite() || rate <= 0.0) {
                        return Err(serde::de::Error::custom("turn_deg_s must be finite and positive"));
                    }
                    if let Some(m) = def.magazine {
                        if m.rounds < 2 || !m.shot_interval_s.is_finite() || m.shot_interval_s <= 0.0
                            || !def.reload_s.is_finite() || (def.reload_s != 0.0 && def.reload_s <= m.shot_interval_s) {
                            return Err(serde::de::Error::custom("magazine needs at least two rounds, positive shot interval, and either zero reload or a longer finite reload"));
                        }
                        if let Some(b) = m.burst {
                            if b.rounds < 2 || b.rounds > m.rounds
                                || !b.aim_max_s.is_finite() || b.aim_max_s < 0.0 {
                                return Err(serde::de::Error::custom("burst needs two or more rounds within the magazine and a finite nonnegative aim limit"));
                            }
                        }
                    }
                    Ok((id.clone(), def))
                })
                .map_err(|e| Error::custom(format!("weapons.{id}: {e}")))
        })
        .collect()
}
