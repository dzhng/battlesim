//! Weapon rows and mounts. A mount is one targeting owner that may hold
//! several ammunition kinds (the tank cannon's AP and HE); a hull lists its
//! mounts and a soldier kind the ones he carries (`catalog`).
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ballistics::{Delivery, WeaponBallistics};

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
    /// What the player reads on the unit card: a concise label
    /// (`crate::labels`), never the row's id.
    pub name: String,
    pub description: String,
    /// Its generated icon, `assets/icons/weapons/<icon>.svg`.
    pub icon: String,
    #[serde(flatten)]
    pub ballistics: WeaponBallistics,
    /// Closest muzzle-to-aim-point distance at which the crew fires.
    /// Zero leaves close fire unrestricted; this is not an arming fuse.
    #[serde(default)]
    pub min_range_m: f64,
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
    /// How gently its damage fades as its penetration falls short of the
    /// armour: the share of the armour it must reach to do any damage.
    pub partial_from: f64,
    pub damage: f64,
    pub near_miss_suppression: f64,
    pub blast_radius_m: f64,
    /// A proximity fuse: the round bursts this close to the unit it was
    /// fired at (until a flare spoofs it), and a row that lists `missile`
    /// destroys an interceptable round within this reach. Absent, it bursts
    /// only on what it strikes; a row listing `missile` must author one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proximity_fuse_m: Option<f64>,
    #[serde(default)]
    pub structural_damage: f64,
    /// Anti-radiation seeker: it homes on a radar's emission, so it also
    /// fires at a radar contact or an identified unit whose radar emits,
    /// and never at an area that doesn't emit. This is distinct from a
    /// radar/IR seeker on an ordinary air-to-air missile.
    #[serde(default)]
    pub anti_radiation: bool,
    /// It fires on its own at what it may engage, whatever its unit's
    /// engagement policy: a fire-and-forget missile its crew looses at any
    /// target its seeker takes.
    #[serde(default)]
    pub fires_on_its_own: bool,
    /// A missile/rocket body vulnerable to active protection; shells and
    /// bullets do not inherit eligibility from penetration or guidance.
    #[serde(default)]
    pub interceptable: bool,
    /// Optional seeker mounted on a guided air weapon. A seeker is authored
    /// separately from launcher guidance so fire-and-forget rows are explicit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeker: Option<SeekerSupport>,
    /// Forward seeker cone in degrees. It is required when a seeker exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeker_fov_deg: Option<f64>,
    /// Whether a seeker can be diverted by an aircraft flare.
    #[serde(default)]
    pub decoyable: bool,
    /// The launcher holds fire while anything stands in its round's first
    /// flight from the muzzle (`launch.clearance_m`): the ground, a
    /// body that stops rounds, or tree crowns. A launcher that would not fire
    /// into branches or a wall at its own muzzle, as a gun firing through
    /// what it can break does.
    #[serde(default)]
    pub requires_clear_launch_path: bool,
    /// Armour-piercing: preferred against identified vehicles, never at contacts.
    #[serde(default)]
    pub armor_piercing: bool,
    /// The altitude layers it can engage (D1), each with how it fires there
    /// and the kinds of target it is brought to bear on. A rifle, a machine
    /// gun or an autocannon swings up at a helicopter, from much closer and
    /// with a much wider spread than along the ground; a tank gun, a grenade
    /// or a ground missile cannot. A layer it does not list, it never engages.
    pub targets: BTreeMap<crate::catalog::AltitudeLayer, LayerFire>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeekerSupport {
    Infrared,
    Radar,
    Optical,
}

/// How a weapon fires at one altitude layer. Reach and spread are explicit;
/// only what changes with the target's height lives here: the gun, its
/// magazine and its round are the same whatever it aims at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerFire {
    /// Its reach at this layer; the largest layer range bounds the flight.
    /// An unpowered bomb authors zero.
    pub range_m: f64,
    /// Its spread at this layer, milliradians.
    pub scatter_mrad: f64,
    /// The kinds of target it engages at this layer: `infantry` and
    /// `vehicle` on the ground, `air_rotor` and `air_fixed_wing` in the air,
    /// and `missile` in any layer, an interceptable round fired from it.
    /// A dedicated anti-tank row lists `vehicle` alone.
    pub classes: Vec<TargetClass>,
}

/// What a target is, for what may be fired at it: soldiers on foot, a hull
/// on the ground, an aircraft by how it flies (its `mobility.air.flight`),
/// or a round in flight. A unit's is read from its body and mobility, never
/// its name ([`crate::catalog::UnitType::target_class`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetClass {
    Infantry,
    Vehicle,
    AirRotor,
    AirFixedWing,
    /// An interceptable round, in the layer it was fired from (`Round.layer`
    /// in the simulation): a row listing it shoots such rounds down.
    Missile,
}

impl TargetClass {
    /// The classes found in `layer`: aircraft fly, soldiers and hulls stand
    /// on the ground, and a round may be fired from any layer.
    pub fn found_in(self, layer: crate::catalog::AltitudeLayer) -> bool {
        let ground = layer == crate::catalog::AltitudeLayer::Ground;
        match self {
            TargetClass::Infantry | TargetClass::Vehicle => ground,
            TargetClass::AirRotor | TargetClass::AirFixedWing => !ground,
            TargetClass::Missile => true,
        }
    }
}

/// A weapon's resolved fire at one altitude layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fire {
    pub range_m: f64,
    pub scatter_mrad: f64,
}

impl WeaponDefinition {
    /// How it fires at `layer`, or `None` when its crew can't bring it to
    /// bear there. The one place `targets` is read.
    pub fn against(&self, layer: crate::catalog::AltitudeLayer) -> Option<Fire> {
        self.targets.get(&layer).map(|fire| Fire {
            range_m: fire.range_m,
            scatter_mrad: fire.scatter_mrad,
        })
    }

    /// How it fires at a target of `class` in `layer`, or `None` when that
    /// layer does not list the class. `None` for `class` is a point or an
    /// area rather than a unit or a round: no class bars it. The one place a
    /// target's class is matched.
    pub fn engages(
        &self,
        layer: crate::catalog::AltitudeLayer,
        class: Option<TargetClass>,
    ) -> Option<Fire> {
        let listed = self.targets.get(&layer)?;
        class
            .is_none_or(|class| listed.classes.contains(&class))
            .then(|| self.against(layer))?
    }

    /// Whether it shoots down rounds fired from some layer (`missile`).
    pub fn intercepts(&self) -> bool {
        self.targets
            .keys()
            .any(|&layer| self.engages(layer, Some(TargetClass::Missile)).is_some())
    }
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

/// Whole-body equipment variants worn by the gun's carrier. Variant identity
/// stays the same when switching between active and carried equipment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorAppearance {
    pub active: Vec<String>,
    pub carried: Vec<String>,
    /// A fixed authored hold for a stationary active operator only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_pose: Option<OperatorPose>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorPose {
    pub clip: String,
    pub phase: f64,
}

/// A turret's arc: `width_deg` of bearings centred `centre_deg` off the
/// hull's heading, counter-clockwise (90: its left).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountArc {
    pub centre_deg: f64,
    pub width_deg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountDefinition {
    /// What other fields name it by (`sensors.on`, a mount's `on`, a model's
    /// rig declarations) and what variants merge on; never shown.
    pub id: String,
    /// What the player reads on the unit card: a concise label
    /// (`crate::labels`).
    pub name: String,
    /// Ammunition kinds available to the mount, by weapon row id.
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
    /// Equipment worn by the current carrier of a single infantry gun.
    /// Absent, the carrier keeps his soldier kind's ordinary appearance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_appearance: Option<OperatorAppearance>,
    /// Traverses at the turret rate; fires only within the bearing tolerance.
    #[serde(default)]
    pub turret: bool,
    /// The bearings a turret can swing through, off the hull's heading (a
    /// door gun's door); absent, it swings all round.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc: Option<MountArc>,
    /// The earlier mount of the same unit whose turret carries this one (a
    /// roof gun on the cannon's turret). Absent, the hull carries it.
    #[serde(default)]
    pub on: Option<String>,
    /// Where this mount turns, in the hull's frame at rest (forward, left, up
    /// from the hull or infantry operator origin). A mount `on` a turret
    /// swings round that turret's pivot with its bearing; a hull mount turns
    /// with the hull's yaw.
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
    // Name a directly-authored bad value before inheritance expands children.
    // This keeps diagnostics attached to the row the author edited (rather
    // than to the first descendant that happens to be visited).
    for (id, row) in &rows {
        for field in [
            "damage",
            "penetration",
            "aim_s",
            "reload_s",
            "blast_radius_m",
            "structural_damage",
            "near_miss_suppression",
            "suppression_radius_m",
            "min_range_m",
        ] {
            if let Some(value) = row.get(field).and_then(serde_json::Value::as_f64) {
                if !value.is_finite() || value < 0.0 {
                    return Err(Error::custom(format!(
                        "weapons.{id}: {field} must be finite and nonnegative"
                    )));
                }
            }
        }
        let targets = row.get("targets").and_then(serde_json::Value::as_object);
        for (layer, fire) in targets.into_iter().flatten() {
            let classes = fire.get("classes").and_then(serde_json::Value::as_array);
            for class in classes.into_iter().flatten() {
                if serde_json::from_value::<TargetClass>(class.clone()).is_err() {
                    return Err(Error::custom(format!(
                        "weapons.{id}: targets.{layer}.classes: unknown target class {class}"
                    )));
                }
            }
        }
        if let Some(value) = row.get("turn_deg_s").and_then(serde_json::Value::as_f64) {
            if !value.is_finite() || value <= 0.0 {
                return Err(Error::custom(format!(
                    "weapons.{id}: turn_deg_s must be finite and positive"
                )));
            }
        }
    }
    crate::catalog::inherit("weapons", &rows)
        .map_err(Error::custom)?
        .into_iter()
        .map(|(id, row)| {
            serde_json::from_value::<WeaponDefinition>(row.clone())
                .and_then(|mut def| {
                    let range = def.targets.values().map(|fire| fire.range_m).fold(0.0_f64, f64::max);
                    if row.get("range_m").is_some() && def.ballistics.range_m != range {
                        return Err(serde::de::Error::custom("range_m must equal the largest target layer range"));
                    }
                    def.ballistics.range_m = range;
                    // Flattened ballistics keep serde from refusing unknown
                    // fields itself. A key the loaded row holds is a field;
                    // one it does not (skipped at its default, or null) is a
                    // field only if a value no field holds fails to load
                    // under it, as serde passes over a key it does not know.
                    let known = serde_json::to_value(&def)?;
                    let fields = row.as_object().into_iter().flatten().map(|(k, _)| k);
                    if let Some(field) = fields
                        .filter(|k| known.get(k.as_str()).is_none())
                        .find(|k| {
                            let mut probe = row.clone();
                            probe[k.as_str()] = serde_json::json!([[["no field holds this"]]]);
                            serde_json::from_value::<WeaponDefinition>(probe).is_ok()
                        })
                    {
                        return Err(serde::de::Error::custom(format!("unknown field `{field}`")));
                    }
                    crate::labels::check_label(&def.name)
                        .and_then(|()| crate::labels::check_icon(&def.icon))
                        .map_err(serde::de::Error::custom)?;
                    for (field, value) in [
                        ("speed_mps", def.ballistics.speed_mps),
                        ("range_m", def.ballistics.range_m),
                        ("damage", def.damage),
                        ("penetration", def.penetration),
                        ("aim_s", def.aim_s),
                        ("reload_s", def.reload_s),
                        ("blast_radius_m", def.blast_radius_m),
                        ("structural_damage", def.structural_damage),
                        ("near_miss_suppression", def.near_miss_suppression),
                        ("suppression_radius_m", def.ballistics.suppression_radius_m),
                        ("min_range_m", def.min_range_m),
                    ] {
                        if !value.is_finite() || value < 0.0 {
                            return Err(serde::de::Error::custom(format!("{field} must be finite and nonnegative")));
                        }
                    }
                    let unpowered = def.ballistics.delivery == Delivery::Unpowered;
                    if unpowered {
                        if def.ballistics.speed_mps != 0.0 || def.ballistics.range_m != 0.0 {
                            return Err(serde::de::Error::custom(
                                "an unpowered row must not author range_m or speed_mps",
                            ));
                        }
                    } else if def.ballistics.speed_mps <= 0.0 || def.ballistics.range_m <= 0.0 {
                        return Err(serde::de::Error::custom(
                            "a powered row needs positive range_m and speed_mps",
                        ));
                    }
                    if def.proximity_fuse_m.is_some_and(|m| !(m.is_finite() && m > 0.0)) {
                        return Err(serde::de::Error::custom(
                            "proximity_fuse_m must be finite and positive",
                        ));
                    }
                    if def.intercepts() && def.proximity_fuse_m.is_none() {
                        return Err(serde::de::Error::custom(
                            "a row listing `missile` authors the proximity_fuse_m that destroys a round",
                        ));
                    }
                    if !(0.0..1.0).contains(&def.partial_from) {
                        return Err(serde::de::Error::custom("partial_from must lie in [0, 1)"));
                    }
                    if !unpowered && def.min_range_m > def.ballistics.range_m {
                        return Err(serde::de::Error::custom("min_range_m must not exceed range_m"));
                    }
                    for (&at, fire) in &def.targets {
                        // The layer as authored (`low_air`), not its Rust name.
                        let layer = serde_json::to_value(at)?;
                        let layer = layer.as_str().unwrap_or_default();
                        if fire.classes.is_empty() {
                            return Err(serde::de::Error::custom(format!(
                                "targets.{layer}.classes must name at least one target class"
                            )));
                        }
                        for (i, class) in fire.classes.iter().enumerate() {
                            if !class.found_in(at) || fire.classes[..i].contains(class) {
                                return Err(serde::de::Error::custom(format!(
                                    "targets.{layer}.classes: {} is repeated or never found in {layer}",
                                    serde_json::to_value(class)?
                                )));
                            }
                        }
                        if unpowered && fire.range_m != 0.0 {
                            return Err(serde::de::Error::custom(format!(
                                "targets.{layer}.range_m must be zero for an unpowered row"
                            )));
                        }
                        if !unpowered
                            && !(fire.range_m > 0.0 && fire.range_m <= def.ballistics.range_m)
                        {
                            return Err(serde::de::Error::custom(format!(
                                "targets.{layer}.range_m must be positive and not exceed range_m"
                            )));
                        }
                        if !fire.scatter_mrad.is_finite() || fire.scatter_mrad < 0.0 {
                            return Err(serde::de::Error::custom(format!(
                                "targets.{layer}.scatter_mrad must be finite and nonnegative"
                            )));
                        }
                    }
                    if def.ballistics.turn_deg_s.is_some_and(|rate| !rate.is_finite() || rate <= 0.0) {
                        return Err(serde::de::Error::custom("turn_deg_s must be finite and positive"));
                    }
                    if def.ballistics.turn_deg_s.is_some() != def.ballistics.guidance.is_some() {
                        return Err(serde::de::Error::custom("a guided row (turn_deg_s) states its guidance, and only a guided row"));
                    }
                    if def.seeker.is_some() != def.seeker_fov_deg.is_some() {
                        return Err(serde::de::Error::custom(
                            "a seeker row must author seeker_fov_deg, and only a seeker row",
                        ));
                    }
                    if def.anti_radiation
                        && !matches!(def.seeker, Some(SeekerSupport::Radar))
                    {
                        return Err(serde::de::Error::custom(
                            "an anti-radiation row needs a radar seeker",
                        ));
                    }
                    if def.ballistics.delivery == Delivery::Unpowered
                        && !def.targets.contains_key(&crate::catalog::AltitudeLayer::Ground)
                    {
                        return Err(serde::de::Error::custom(
                            "an unpowered row (a bomb) must target Ground",
                        ));
                    }
                    if def.seeker_fov_deg.is_some_and(|fov| {
                        !fov.is_finite() || !(fov > 0.0 && fov <= 360.0)
                    }) {
                        return Err(serde::de::Error::custom(
                            "seeker_fov_deg must be finite and in (0, 360]",
                        ));
                    }
                    if def.intercepts() && def.ballistics.guidance.is_none() && def.seeker.is_some() {
                        return Err(serde::de::Error::custom(
                            "a seeker interception row must be guided",
                        ));
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
