//! Mounts choose targets from their side's knowledge (W06, W10).
//! Physical guns own aim and cycles (W01–W02); rounds leave only through flight.
use contract::ballistics::Delivery;
use contract::catalog::{AltitudeLayer, TypeIndex};
use contract::command::{Engagement, TargetRef};
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ActionReason, ContactId, MountReadiness, WeaponPose};
use contract::scenario::{Armor, Rules};
use contract::weapons::{AmmoCapacity, Fire, TargetClass, WeaponDefinition};

mod bearing;
mod cycle;
mod muzzle;
pub mod tick;
use bearing::{bearing_from, hull_fixed, point_mount, turned_mount};
pub use bearing::{nose_on, nose_on_bearing, Arc};
use cycle::Cycle;
use muzzle::{infantry_offset, muzzle, placed_muzzle};

use crate::damage;
use crate::deployment;
use crate::digest::Digest;
use crate::flight::{
    predicted_path, timed_path, Aim, BodyId, FiringSolution, FlightConfig, Launch, LaunchProfile,
    NoSolution, ProjectileId, Shooter,
};
use crate::knowledge::SideKnowledge;
use crate::lean;
use crate::math::{body_distance, v2, v3, wrap_angle, V3};
use crate::units::Unit;
use crate::world::{Collider, PropId, WorldGeometry};
use contract::random::Rng;

/// Vehicle hull bodies take ids above every soldier id.
pub const VEHICLE_BODY_BASE: u32 = 1 << 24;

/// A weapon row with its validated flight profile.
pub struct Weapon {
    /// The row's id in the rules' `weapons`.
    pub id: String,
    pub def: WeaponDefinition,
    pub profile: LaunchProfile,
}

/// An authored mount: ammunition kinds (weapon indices) sharing targeting.
#[derive(Clone)]
pub struct MountSpec {
    pub kinds: Vec<usize>,
    pub squad: bool,
    pub turret: bool,
    /// A squad's slots whose soldiers carry it; empty on a hull.
    pub carriers: Vec<usize>,
    /// It passes to the next living soldier when its carrier falls.
    pub special: bool,
    /// The mount (index in the type's list) whose turret carries this one;
    /// `None` is the hull.
    pub on: Option<usize>,
    /// Where it turns, in the hull's frame at rest (forward, left, up).
    pub pivot: V3,
    /// The ring its carrier turns round, on the ground under the carrier's
    /// pivot: its pivot swings round this with the carrier. The hull's
    /// middle for a hull mount.
    pub ring: V3,
    /// Its muzzle from the pivot along its own bearing; `None` is a hand
    /// weapon, fired at the infantry muzzle height.
    pub muzzle: Option<V3>,
    /// The bearings a turret swings through off the hull's heading; `None`,
    /// all round.
    pub arc: Option<Arc>,
}

/// Every weapon and mount in the battle, fixed at setup.
pub struct Arsenal {
    pub weapons: Vec<Weapon>,
    /// Each unit type's mounts, by type index.
    mounts: Vec<Vec<MountSpec>>,
    /// Each unit type's optics mount (`sensors.on`), by type index.
    optics: Vec<Option<usize>>,
    pub config: FlightConfig,
}

/// Admit every row's flight and the weapon table.
pub fn check_rules(rules: &Rules) -> Result<FlightConfig, String> {
    let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz)
        .map_err(|e| format!("physics: {e}"))?;
    for (id, weapon) in &rules.weapons {
        config
            .profile(&weapon.ballistics)
            .map_err(|e| format!("weapons.{id}: {e}"))?;
        for (layer, fire) in &weapon.targets {
            config
                .check_layer(&weapon.ballistics, fire.range_m, fire.scatter_mrad)
                .map_err(|e| format!("weapons.{id}.{layer:?}: {e}"))?;
        }
    }
    if rules.weapons.len() > crate::publication::MAX_WEAPON_ROWS {
        return Err(format!(
            "weapons: more than {} rows, the publication limit",
            crate::publication::MAX_WEAPON_ROWS
        ));
    }
    for t in rules.catalog.indices() {
        for mount in rules.catalog.mounts(t) {
            if mount.def.weapons.len() > crate::publication::MAX_AMMO_KINDS {
                return Err(format!(
                    "units.{}: mount {:?} has more than {} ammunition kinds, the publication limit",
                    rules.catalog.id(t),
                    mount.def.id,
                    crate::publication::MAX_AMMO_KINDS
                ));
            }
        }
    }
    Ok(config)
}

impl Arsenal {
    pub fn new(rules: &Rules) -> Self {
        let config = check_rules(rules).expect("fixture weapon rules are valid");
        let weapons: Vec<Weapon> = rules
            .weapons
            .iter()
            .map(|(id, def)| Weapon {
                id: id.clone(),
                profile: config
                    .profile(&def.ballistics)
                    .expect("fixture weapon rows are valid"),
                def: def.clone(),
            })
            .collect();
        // The catalog's checks at load hold every mount to an existing row
        // and an earlier turret (`Catalog::check_weapons`, `catalog::check`).
        let index = |id: &str| {
            weapons
                .iter()
                .position(|w| w.id == id)
                .expect("mounts name weapon rows (checked at load)")
        };
        let catalog = &rules.catalog;
        let mounts: Vec<Vec<MountSpec>> = catalog
            .indices()
            .map(|t| {
                let list = catalog.mounts(t);
                list.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let m = &c.def;
                        let on = m.on.as_ref().map(|carrier| {
                            list[..i]
                                .iter()
                                .position(|c| c.def.id == *carrier && c.def.turret)
                                .expect("a mount rides an earlier turret (checked at load)")
                        });
                        let v = |[x, y, z]: [f64; 3]| v3(x, y, z);
                        let ring = on.map_or(V3::default(), |c| {
                            let [x, y, _] = list[c].def.pivot_m;
                            v3(x, y, 0.0)
                        });
                        MountSpec {
                            kinds: m.weapons.iter().map(|w| index(w)).collect(),
                            squad: m.squad,
                            turret: m.turret,
                            carriers: c.carriers.clone(),
                            special: m.special,
                            on,
                            pivot: v(m.pivot_m),
                            ring,
                            muzzle: m.muzzle_m.map(v),
                            arc: m.arc.map(|a| Arc {
                                centre: a.centre_deg.to_radians(),
                                half: (a.width_deg / 2.0).to_radians(),
                            }),
                        }
                    })
                    .collect()
            })
            .collect();
        let optics = catalog
            .indices()
            .map(|t| {
                let on = catalog.get(t).sensors.on.as_ref()?;
                catalog.mounts(t).iter().position(|m| &m.def.id == on)
            })
            .collect();
        Arsenal {
            weapons,
            mounts,
            optics,
            config,
        }
    }

    /// A mount's report as a firing report hears it: a bit for every row it
    /// fires (the layout's `roundKinds`), since its sound doesn't say which.
    pub fn heard(&self, kind: TypeIndex, mount: usize) -> u64 {
        self.mounts[kind.0 as usize][mount]
            .kinds
            .iter()
            .fold(0, |m, &k| m | 1u64 << k)
    }

    pub fn specs(&self, kind: TypeIndex) -> &[MountSpec] {
        &self.mounts[kind.0 as usize]
    }

    /// The mount a type's optics turn with; `None`, its hull.
    pub fn optics(&self, kind: TypeIndex) -> Option<usize> {
        self.optics[kind.0 as usize]
    }

    /// Fresh mounts for a unit of type `kind` facing `yaw`, loaded and aimed nowhere.
    pub fn mounts_for(
        &self,
        kind: TypeIndex,
        yaw: f64,
        members: &[crate::units::Soldier],
    ) -> Vec<Mount> {
        self.specs(kind)
            .iter()
            .enumerate()
            .map(|(i, spec)| Mount {
                spec: i,
                ammo: spec
                    .kinds
                    .iter()
                    .map(|&k| match self.weapons[k].def.ammo {
                        AmmoCapacity::Unlimited(_) => None,
                        AmmoCapacity::Rounds(n) => Some(n),
                    })
                    .collect(),
                cycles: if spec.squad {
                    members
                        .iter()
                        .filter(|s| spec.carriers.contains(&s.slot))
                        .map(|s| Cycle::new(Some(s.id), spec, &self.weapons))
                        .collect()
                } else {
                    vec![Cycle::new(None, spec, &self.weapons)]
                },
                operator: (!spec.squad)
                    .then(|| {
                        members
                            .iter()
                            .find(|s| s.alive() && spec.carriers.contains(&s.slot))
                            .map(|s| s.id)
                    })
                    .flatten(),
                lock: None,
                support: None,
                bearing: yaw,
                last_enemy_lock_tick: None,
                elevation: 0.0,
                shots: 0,
                reason: ActionReason::NoCompatibleTarget,
            })
            .collect()
    }
}

/// A guided missile in flight and what its launcher steers it at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Support {
    pub projectile: ProjectileId,
    pub target: Target,
}

/// What a lock points at, in the sim's own terms (never exported as such).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Unit(UnitId),
    Contact(ContactId),
    Ground(V3),
    Round(RoundTarget),
}

/// A round in flight, by its id and the layer it was fired from; where it
/// is and how it flies are read live ([`IncomingRound`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundTarget {
    pub id: ProjectileId,
    pub layer: AltitudeLayer,
}

/// An interceptable round in flight, as every mount may engage it: who fired
/// it, where it is, how it flies, and the layer it was fired from.
#[derive(Clone, Copy, Debug)]
pub struct IncomingRound {
    pub id: ProjectileId,
    pub side: Side,
    pub position: V3,
    pub velocity: V3,
    pub layer: AltitudeLayer,
    /// Seconds until its flight ends of itself (its lifetime).
    pub ends_in_s: f64,
    /// By side: whether one of its living units sees the round now. A mount
    /// fires only at a round its side sees, as at any other target.
    pub seen: [bool; Side::ALL.len()],
}

impl Target {
    pub fn digest(self, d: &mut Digest) {
        match self {
            Target::Unit(u) => d.u64(0).u64(u.0 as u64),
            Target::Contact(c) => d.u64(1).u64(c.0 as u64),
            Target::Ground(p) => d.u64(2).f64(p.x).f64(p.y).f64(p.z),
            Target::Round(round) => d.u64(3).u64(round.id.0).u64(round.layer as u64),
        };
    }
}

#[derive(Clone, Debug)]
pub struct Lock {
    pub target: Target,
    /// This tick's assessment cleared the target and the mount is working the
    /// shot: aiming, loading, traversing, guiding or firing (not held off by
    /// range, sight, a facing slot, a move or a building's doorway).
    pub engaging: bool,
}

#[derive(Clone, Debug)]
pub struct Mount {
    pub spec: usize,
    /// Total rounds left per kind, including loaded magazines; `None` is unlimited.
    pub ammo: Vec<Option<u32>>,
    /// One physical gun per carrier; transferable/hull weapons use owner None.
    pub cycles: Vec<Cycle>,
    /// Current operator of a single infantry weapon; None on hulls, grouped
    /// rifles and unoperated spares. Identity and cycle stay with the mount.
    pub operator: Option<u32>,
    pub lock: Option<Lock>,
    /// The missile this mount is guiding, if any (one at a time).
    pub support: Option<Support>,
    pub bearing: f64,
    pub last_enemy_lock_tick: Option<Tick>,
    /// Elevation of the last round this mount launched (radians); 0 before.
    pub elevation: f64,
    /// Rounds launched since the battle began, wrapping at 2³².
    pub shots: u32,
    pub reason: ActionReason,
}

impl Mount {
    pub(crate) fn has_rounds(&self, k: usize) -> bool {
        self.ammo[k].is_none_or(|n| n > 0)
    }

    /// Every kind spent, the loaded round included.
    fn out_of_ammo(&self) -> bool {
        self.ammo.iter().all(|a| *a == Some(0))
    }

    /// Fold the mount's complete carried state into `d`.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.spec as u64).u64(self.ammo.len() as u64);
        for a in &self.ammo {
            d.u64(a.map_or(u64::MAX, |n| n as u64));
        }
        d.u64(self.operator.map_or(u64::MAX, u64::from));
        d.f64(self.bearing)
            .u64(self.last_enemy_lock_tick.unwrap_or(u64::MAX))
            .f64(self.elevation)
            .u64(self.shots as u64)
            .u64(self.reason as u64)
            .u64(self.cycles.len() as u64);
        for c in &self.cycles {
            c.digest(d);
        }
        d.u64(self.support.is_some() as u64);
        if let Some(s) = &self.support {
            d.u64(s.projectile.0);
            s.target.digest(d);
        }
        d.u64(self.lock.is_some() as u64);
        if let Some(l) = &self.lock {
            l.target.digest(d);
            d.u64(l.engaging as u64);
        }
    }

    /// Stop (W15): clear aim once, drop an unfinished reload, keep what is
    /// loaded, and release any guided missile (P06).
    pub fn stop(&mut self) {
        self.lock = None;
        for c in &mut self.cycles {
            c.started = false;
            c.aim = 0.0;
            c.reload = None;
        }
        self.support = None;
    }
}

/// A resolved aim: where to shoot now and how it moves as observed.
#[derive(Clone, Copy, Debug)]
struct Resolved {
    point: V3,
    velocity: V3,
    /// Currently observed (not only remembered within the grace).
    current: bool,
    /// The target's armour if the side knows its type (identified units
    /// only): `Some(None)` for a soldier.
    armor: Option<Option<Armor>>,
    /// The height band it is in; an area or a ground point is on the ground.
    layer: AltitudeLayer,
    /// What an identified unit or a round in flight is to a weapon; `None`
    /// for an area or a ground point, which no class bars.
    class: Option<TargetClass>,
    /// It radiates: a radar contact, or an identified unit whose radar
    /// emits. An anti-radiation seeker homes on it; no other weapon reads it.
    emits: bool,
}

impl Resolved {
    /// How `def` fires here; `None` when its layer or classes leave it out.
    fn fire(&self, def: &WeaponDefinition) -> Option<Fire> {
        def.engages(self.layer, self.class)
    }

    /// Whether `def` is worth firing at it: at a round, a row listing
    /// `missile` in the round's layer (a round is no area, W10); otherwise
    /// what [`effective`] says.
    fn worth(&self, def: &WeaponDefinition, rules: &Rules) -> bool {
        if self.class == Some(TargetClass::Missile) {
            self.fire(def).is_some()
        } else {
            effective(def, self.armor, rules)
        }
    }
}

/// One weapon's shot this tick: its rounds and who it was aimed at.
pub struct Shot {
    pub unit: UnitId,
    /// Index of the firing mount on the unit.
    pub mount: usize,
    pub weapon: usize,
    pub launches: Vec<Launch>,
    pub target: Target,
}

pub struct FireContext<'a> {
    pub world: &'a WorldGeometry,
    pub structures: &'a crate::structures::Structures,
    pub ground: &'a crate::ground::GroundLayer,
    pub arsenal: &'a Arsenal,
    pub rules: &'a Rules,
    pub tick: Tick,
    pub knowledge: &'a [SideKnowledge; 3],
    /// The interceptable rounds in flight.
    pub rounds: &'a [IncomingRound],
}

/// How far a squad's own weapons reach: its squad mount's longest range
/// (the rifles every soldier carries). The cover search judges where a
/// soldier can engage from by it.
pub fn squad_range(arsenal: &Arsenal, kind: TypeIndex) -> f64 {
    reach(
        arsenal,
        AltitudeLayer::Ground,
        arsenal
            .specs(kind)
            .iter()
            .filter(|s| s.squad)
            .flat_map(|s| s.kinds.iter().copied()),
    )
}

/// The longest range of these weapon rows at `layer` (0 for none).
fn reach(arsenal: &Arsenal, layer: AltitudeLayer, rows: impl Iterator<Item = usize>) -> f64 {
    rows.filter_map(|k| arsenal.weapons[k].def.against(layer))
        .map(|fire| fire.range_m)
        .fold(0.0, f64::max)
}

/// Whether a weapon can hurt a target with this hull armour (`None`: a
/// soldier) at all (P10: some of its damage gets through the weakest face).
pub fn can_damage(def: &WeaponDefinition, armor: Option<&Armor>, rules: &Rules) -> bool {
    damage_share(def, armor.map(Armor::weakest), rules) > 0.0
}

/// The share of a weapon's damage a fresh hit delivers through a hull face
/// of `armor`, or into a soldier (`None`): all of it. Whether it is fired at
/// soldiers at all is its target classes' call.
fn damage_share(def: &WeaponDefinition, armor: Option<f64>, rules: &Rules) -> f64 {
    armor.map_or(1.0, |armor| damage::pierce_share(def, 0, rules, armor))
}

/// Whether the mount's kind `k` can hurt `target`; an area or a ground point
/// is always worth its rounds.
fn hurts(ctx: &FireContext, spec: &MountSpec, k: usize, target: Target, units: &[Unit]) -> bool {
    match target {
        Target::Unit(u) => can_damage(
            &ctx.arsenal.weapons[spec.kinds[k]].def,
            units[u.0 as usize].armor(ctx.rules),
            ctx.rules,
        ),
        _ => true,
    }
}

/// The unit's always-available gun with rounds to spare (a squad's rifles, a
/// roof HMG): it fires at an identified enemy it cannot hurt, to keep heads
/// down (W09). A weapon with a finite supply keeps it for what it can hurt.
fn fires_regardless(def: &WeaponDefinition) -> bool {
    def.default && matches!(def.ammo, AmmoCapacity::Unlimited(_))
}

/// General-purpose ammunition, made for men on the ground: may fire at an
/// area or a ground point (never AP, nor a row whose classes leave infantry
/// out, W10).
pub(crate) fn area_capable(def: &WeaponDefinition) -> bool {
    !def.armor_piercing
        && def
            .engages(AltitudeLayer::Ground, Some(TargetClass::Infantry))
            .is_some()
}

/// Whether `def` is effective against a target whose type is known, or an area.
fn effective(def: &WeaponDefinition, armor: Option<Option<Armor>>, rules: &Rules) -> bool {
    match armor {
        Some(armor) => can_damage(def, armor.as_ref(), rules),
        None => area_capable(def),
    }
}

fn resolve(ctx: &FireContext, side: Side, target: Target, units: &[Unit]) -> Option<Resolved> {
    let knowledge = &ctx.knowledge[side.index()];
    match target {
        Target::Unit(u) => {
            let track = knowledge.track(u)?;
            let unit = &units[u.0 as usize];
            Some(Resolved {
                point: track.position + v3(0.0, 0.0, unit.aim_height(ctx.rules)),
                velocity: track
                    .velocity
                    .with_z(unit.air.as_ref().map_or(0.0, |air| air.velocity.z)),
                current: track.last_seen == ctx.tick,
                armor: Some(unit.armor(ctx.rules).copied()),
                layer: unit.layer(),
                class: Some(unit.unit_type(ctx.rules).target_class()),
                emits: unit.unit_type(ctx.rules).capabilities.radar_emitter,
            })
        }
        Target::Contact(c) => {
            let contact = knowledge.ground_contact(c)?;
            let z = ctx.world.height_at(contact.center.x, contact.center.y)?;
            Some(Resolved {
                point: contact
                    .center
                    .xy()
                    .with_z(z + ctx.rules.physics.infantry_aim_m),
                velocity: v3(0.0, 0.0, 0.0),
                current: true,
                armor: None,
                layer: AltitudeLayer::Ground,
                class: None,
                emits: contact.source == contract::observation::ContactSource::Radar,
            })
        }
        Target::Ground(p) => Some(Resolved {
            point: p,
            velocity: v3(0.0, 0.0, 0.0),
            current: true,
            armor: None,
            // Ground orders always remain Ground; only a round target carries
            // the layer it was fired from.
            layer: AltitudeLayer::Ground,
            class: None,
            emits: false,
        }),
        Target::Round(round) => {
            let flying = ctx
                .rounds
                .iter()
                .find(|r| r.id == round.id && r.seen[side.index()])?;
            Some(Resolved {
                point: flying.position,
                velocity: flying.velocity,
                current: true,
                armor: None,
                layer: round.layer,
                class: Some(TargetClass::Missile),
                emits: false,
            })
        }
    }
}

/// The mount's preferred kind for a target: armour-piercing for identified
/// vehicles, otherwise non-AP; the unlimited default gun also against what it
/// cannot hurt (W09); AP never at an area (W10). A radar contact is an area
/// like any other; an anti-radiation row also takes what radiates.
fn preferred_kind(
    ctx: &FireContext,
    mount: &Mount,
    spec: &MountSpec,
    resolved: &Resolved,
) -> Option<usize> {
    let weapons = &ctx.arsenal.weapons;
    let usable = |k: usize, want_ap: Option<bool>| {
        let def = &weapons[spec.kinds[k]].def;
        mount.has_rounds(k)
            && resolved.fire(def).is_some()
            && want_ap.is_none_or(|ap| def.armor_piercing == ap)
            && (resolved.worth(def, ctx.rules)
                || (resolved.emits && def.anti_radiation)
                || (resolved.armor.is_some() && fires_regardless(def)))
    };
    let n = spec.kinds.len();
    let first = |want_ap: Option<bool>| (0..n).find(|&k| usable(k, want_ap));
    match resolved.armor {
        Some(Some(_)) => first(Some(true)).or_else(|| first(None)),
        _ => first(Some(false)).or_else(|| first(None)),
    }
}

/// A point inside a body fire can destroy (a building) is aimed at the face
/// toward the shooter: striking that face is the attack (structural
/// damage), not an obstruction. Inside a body it cannot destroy (a ruin),
/// the point stands.
fn facade(ctx: &FireContext, unit: &Unit, mount: &Mount, spec: &MountSpec, point: V3) -> V3 {
    let origin = muzzle(unit, mount, spec, ctx.rules, bearing_from(unit, point));
    let to = point - origin;
    let len = to.length();
    if len < 1e-6 {
        return point;
    }
    let dir = to * (1.0 / len);
    let Some(hit) = ctx.world.raycast(origin, dir, len) else {
        return point;
    };
    let crate::world::Collider::Prop(id) = hit.collider else {
        return point;
    };
    match ctx.world.prop(id) {
        Some(prop) if prop.body.hp.is_some() && prop.footprint().contains(point.xy(), 0.0) => {
            hit.point - dir * 0.05
        }
        _ => point,
    }
}

/// Where this mount aims at `target`: a ground point inside a building moves
/// to the facade it fires at (`facade`).
fn resolve_for(
    ctx: &FireContext,
    unit: &Unit,
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    units: &[Unit],
) -> Option<Resolved> {
    let r = resolve(ctx, unit.side, target, units)?;
    Some(match target {
        Target::Ground(p) => Resolved {
            point: facade(ctx, unit, mount, spec, p),
            ..r
        },
        _ => r,
    })
}

/// P11: withhold when a friendly vehicle will be on the predicted path as the
/// round passes, or in the blast at `burst` when it bursts. Each hull is
/// judged where its own velocity carries it by then, so a wingman in trail
/// holds for the lead he would hit, not for where the lead was when he fired.
/// Friendly infantry never withholds a shot (it can still be hit).
/// `leaning_round` is the hull a soldier leans round: his lean point
/// clears it, so it never withholds his shot.
#[allow(clippy::too_many_arguments)]
fn friendly_in_line(
    ctx: &FireContext,
    shooter: &Unit,
    units: &[Unit],
    origin: V3,
    (s, burst): (&FiringSolution, V3),
    weapon: &Weapon,
    leaning_round: Option<UnitId>,
    carrier: V3,
) -> bool {
    let margin = ctx.rules.physics.friendly_prefire_margin_m;
    let (def, profile) = (&weapon.def, &weapon.profile);
    let path = timed_path(
        &ctx.arsenal.config,
        profile,
        origin,
        s.velocity + carrier,
        s.time_of_flight_s,
    );
    units
        .iter()
        .filter(|u| {
            u.side == shooter.side && u.id != shooter.id && u.hull.is_some() && u.on_field()
        })
        .filter(|u| Some(u.id) != leaning_round)
        .any(|u| {
            // One frame per hull: its rotation is not recomputed per sample.
            // The path is read in the hull's own moving frame instead.
            let hull = u.hull_frame();
            let drift = u.flight_velocity();
            let relative = |(p, t): (V3, f64)| p - drift * t;
            path.windows(2).any(|w| {
                let (a, b) = (relative(w[0]), relative(w[1]));
                // A chord that never comes near the hull has no sample near it.
                if !hull.may_come_within(a, b, margin) {
                    return false;
                }
                // Sample each chord densely enough for a hull-sized margin.
                let n = ((b - a).length() / 1.0).ceil().max(1.0) as usize;
                (0..=n).any(|k| hull.distance(a + (b - a) * (k as f64 / n as f64)) < margin)
            }) || (def.blast_radius_m > 0.0
                && hull.distance(burst - drift * s.time_of_flight_s) < def.blast_radius_m)
        })
}

/// Whether a launcher that needs a clear launch path has one: the first
/// `launch.clearance_m` of its round's arc from the muzzle meets
/// neither the ground, nor a body that stops rounds (whether fire could
/// break it or not), nor tree crowns. Only the launcher's end is judged: a
/// high arc clears a low wall beside it, and a target in a wood or past a
/// hill is still fair. This is not the target line ([`guidance_clear`]).
fn launch_clear(
    ctx: &FireContext,
    origin: V3,
    s: &FiringSolution,
    weapon: &Weapon,
    carrier: V3,
) -> bool {
    let path = predicted_path(
        &ctx.arsenal.config,
        &weapon.profile,
        origin,
        s.velocity + carrier,
        s.time_of_flight_s,
    );
    let mut left = ctx.rules.launch.clearance_m;
    for w in path.windows(2) {
        let chord = w[1] - w[0];
        let len = chord.length();
        if left <= 0.0 {
            break;
        }
        if len == 0.0 {
            continue;
        }
        let end = w[0] + chord * (left.min(len) / len);
        if !ctx.world.segment_clear(w[0], end) || ctx.world.foliage_depth(w[0], end) > 0.0 {
            return false;
        }
        left -= len;
    }
    true
}

/// A gun holds fire only for what its rounds cannot break. It
/// fires into the first body on its arc when that body is what it was
/// ordered to hit (the ground point lies in it), or when the body can be
/// destroyed, the gun can see past it, and the rounds it has left of this
/// kind can destroy it by direct hits (structural damage times the body's
/// armour; `None` rounds is unlimited). Anything else in the way (terrain,
/// a body without integrity, an occluder, a body too tough for what is left)
/// holds fire. The solver reads the true world, as it does for every
/// obstruction.
fn fires_into(
    ctx: &FireContext,
    weapon: &Weapon,
    rounds: Option<u32>,
    target: Target,
    id: PropId,
) -> bool {
    let Some(prop) = ctx.world.prop(id) else {
        return false;
    };
    if matches!(target, Target::Ground(p) if prop.footprint().contains(p.xy(), 0.0)) {
        return true;
    }
    let Some(hp) = ctx.structures.hp(ctx.world, id) else {
        return false;
    };
    let per_round = weapon.def.structural_damage * prop.body.armor;
    !prop.body.occludes && per_round > 0.0 && rounds.is_none_or(|n| per_round * n as f64 >= hp)
}

/// Can this mount fire the given kind at the resolved point from here? The
/// solution, or why not.
#[allow(clippy::too_many_arguments)]
fn engage(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    k: usize,
    target: Target,
    r: &Resolved,
) -> Result<FiringSolution, ActionReason> {
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]];
    let carrier = unit.flight_velocity();
    let bearing = if spec.turret {
        bearing_from(unit, r.point)
    } else {
        mount.bearing
    };
    let from = |origin: V3, past: Option<PropId>, hull: Option<UnitId>| {
        let distance = (r.point - origin).length();
        if distance < weapon.def.min_range_m {
            // OutOfRange asks an ordered unit to advance; too close holds.
            return Err(ActionReason::HoldingFire);
        }
        if weapon.def.ballistics.delivery == Delivery::Unpowered {
            // A bomb released now falls from here with the carrier's
            // velocity; its pass's release cue is its reach. It holds for a
            // friendly on its fall or in its blast where it lands.
            let Some(fall) = crate::flight::release(
                origin,
                carrier,
                weapon.profile.gravity(&ctx.arsenal.config),
                r.point,
            ) else {
                // Dropped from below its target, it falls away from it.
                return Err(ActionReason::OutOfRange);
            };
            let fall_s = fall.fall_s;
            let lands = (origin.xy() + carrier.xy() * fall_s).with_z(r.point.z);
            let s = FiringSolution {
                velocity: V3::default(),
                time_of_flight_s: fall_s,
                intercept: lands,
                arc: crate::flight::ArcKind::Low,
            };
            if friendly_in_line(ctx, unit, units, origin, (&s, lands), weapon, hull, carrier) {
                return Err(ActionReason::FriendlyInLine);
            }
            return Ok(s);
        }
        let Some(reach) = r.fire(&weapon.def).map(|fire| fire.range_m) else {
            return Err(ActionReason::OutOfRange);
        };
        let aim = Aim {
            origin,
            target: r.point,
            target_velocity: r.velocity,
            carrier,
        };
        match crate::flight::solve_fire(
            ctx.world,
            &ctx.arsenal.config,
            &weapon.profile,
            &aim,
            past,
            |id| fires_into(ctx, weapon, mount.ammo[k], target, id),
        ) {
            Err(NoSolution::OutOfReach) => Err(ActionReason::OutOfRange),
            Err(NoSolution::Blocked { .. }) => Err(ActionReason::BlockedTrajectory),
            // The shot must meet its target within its layer's reach, not
            // merely start inside it: the lead's meeting point is the one
            // point the aim uses and the one the reach is judged at.
            Ok((s, _)) if (s.intercept - origin).length() > reach => Err(ActionReason::OutOfRange),
            Ok((s, _))
                if weapon.def.requires_clear_launch_path
                    && !launch_clear(ctx, origin, &s, weapon, carrier) =>
            {
                Err(ActionReason::BlockedTrajectory)
            }
            Ok((s, burst))
                if friendly_in_line(
                    ctx,
                    unit,
                    units,
                    origin,
                    (&s, burst),
                    weapon,
                    hull,
                    carrier,
                ) =>
            {
                Err(ActionReason::FriendlyInLine)
            }
            Ok((s, _)) => Ok(s),
        }
    };
    if let Some(garrison) = unit
        .hold
        .as_ref()
        .filter(|_| unit.garrisoned())
        .and_then(|hold| hold.building())
    {
        let facing = ctx.rules.garrison.slot_facing_min_deg.to_radians();
        let mut result = Err(ActionReason::NoFacingSlot);
        for k in candidates(unit, mount, spec) {
            let Some(seat) = garrison
                .seat(k)
                .filter(|s| s.slot.faces(r.point.xy(), facing))
            else {
                continue;
            };
            // Assessment and launch use the same occupied window's muzzle.
            result = from(
                seat.position + infantry_offset(spec, ctx.rules, bearing),
                None,
                None,
            );
            if result.is_ok() {
                return result;
            }
        }
        return result;
    }
    let origin = muzzle(unit, mount, spec, ctx.rules, bearing);
    let at_unit = from(origin, None, None);
    // A soldier's weapon fires from his own muzzle: with the squad's
    // middle (or the operator where he stands) blocked, or a friendly hull
    // in the way, it can fire if any of its soldiers can, from where he
    // stands or out on his lean.
    if !matches!(
        at_unit,
        Err(ActionReason::BlockedTrajectory | ActionReason::FriendlyInLine)
    ) || unit.hull.is_some()
    {
        return at_unit;
    }
    let blockers = lean::hulls(units, ctx.rules);
    candidates(unit, mount, spec)
        .map(|k| {
            let f = fire_from(
                ctx,
                &blockers,
                &unit.members[k],
                r.point,
                infantry_offset(spec, ctx.rules, bearing),
            )?;
            from(f.origin, f.past, f.hull).ok()
        })
        .find_map(|s| s.map(Ok))
        .unwrap_or(at_unit)
}

/// Whether the unit's engagement policy allows firing at this target (W12–W13).
fn permitted(ctx: &FireContext, unit: &Unit, target: Target) -> bool {
    match unit.engagement {
        // Which weapons fire at a radar contact is theirs to say
        // (`anti_radiation`, [`preferred_kind`]).
        Engagement::FireAtWill => true,
        Engagement::ReturnFireOnly => match target {
            Target::Unit(u) => unit.attackers.contains(&u),
            Target::Contact(c) => ctx.knowledge[unit.side.index()]
                .contact(c)
                .is_some_and(|contact| unit.attackers.contains(&contact.emitter)),
            Target::Ground(_) => false,
            Target::Round(_) => true,
        },
    }
}

/// One mount's assessments this tick, by target: choosing its lock and then
/// firing ask of the same target with the same world, so each target is
/// assessed once (its line tests and firing solutions are the costly part).
#[derive(Default)]
struct Assessed {
    values: Vec<(Target, Result<Ready, ActionReason>)>,
    prospective_windows: bool,
}

/// A shot an assessment found: the kind it would load, how long its round
/// flies to meet the target and where it meets it.
#[derive(Clone, Copy)]
struct Ready {
    kind: usize,
    time_of_flight_s: f64,
    intercept: V3,
}

impl Assessed {
    /// The kind this mount would load at `target`, or why it can't fire.
    #[allow(clippy::too_many_arguments)]
    fn target(
        &mut self,
        ctx: &FireContext,
        unit: &Unit,
        units: &[Unit],
        mount: &Mount,
        spec: &MountSpec,
        target: Target,
        resolved: &Resolved,
    ) -> Result<usize, ActionReason> {
        self.shot(ctx, unit, units, mount, spec, target, resolved)
            .map(|ready| ready.kind)
    }

    #[allow(clippy::too_many_arguments)]
    fn shot(
        &mut self,
        ctx: &FireContext,
        unit: &Unit,
        units: &[Unit],
        mount: &Mount,
        spec: &MountSpec,
        target: Target,
        resolved: &Resolved,
    ) -> Result<Ready, ActionReason> {
        if let Some(&(_, a)) = self.values.iter().find(|(t, _)| *t == target) {
            return a;
        }
        let a = if self.prospective_windows {
            assess_windows(ctx, unit, units, mount, spec, target, resolved)
        } else {
            assess(ctx, unit, units, mount, spec, target, resolved)
        };
        self.values.push((target, a));
        a
    }
}

/// Could this mount shoot `target` now: permitted, with a usable kind and a
/// clear solution. The kind it would load, or why not.
fn assess(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    r: &Resolved,
) -> Result<Ready, ActionReason> {
    let k = preferred_kind(ctx, mount, spec, r);
    let on_its_own = k.is_some_and(|k| ctx.arsenal.weapons[spec.kinds[k]].def.fires_on_its_own);
    if !on_its_own && !permitted(ctx, unit, target) {
        return Err(ActionReason::HoldingFire);
    }
    let k = k.ok_or(if mount.out_of_ammo() {
        ActionReason::OutOfAmmo
    } else {
        ActionReason::NoCompatibleTarget
    })?;
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]].def;
    if r.layer == AltitudeLayer::Ground {
        let pass = unit.air.as_ref().and_then(|air| air.pass);
        let bomb_pass = pass.is_some_and(|p| p.weapon == crate::air::pass::PassWeapon::Bomb);
        // Bombs leave only on a bombing pass's release cue, never between
        // passes (only a winged aircraft carries them: catalog admission).
        let release_cue = bomb_pass && unit.air.as_ref().is_some_and(|air| air.pass_release);
        match crate::air::pass::pass_weapon(weapon) {
            Some(crate::air::pass::PassWeapon::Bomb) if !release_cue => {
                return Err(ActionReason::HoldingFire);
            }
            Some(crate::air::pass::PassWeapon::Gun) if bomb_pass => {
                return Err(ActionReason::NoCompatibleTarget);
            }
            _ => {}
        }
    }
    // A seeker only acquires inside its authored forward cone.  Once the
    // round is launched, its existing guidance contract takes over.
    if let Some(fov) = weapon.seeker_fov_deg {
        let bearing = bearing_from(unit, r.point);
        let forward = if spec.turret { mount.bearing } else { unit.yaw };
        if !seeker_in_fov(forward, bearing, fov) {
            return Err(ActionReason::NoFacingSlot);
        }
    }
    // A winged aircraft's flight, never a weapon, points the airframe, so a
    // hull-fixed row it aims by the nose (unguided and powered: a fixed gun,
    // a rocket pod) fires only nose-on. A seeker row aims by its cone, a
    // guided one by its guidance and a bomb by its pass's release cue.
    if unit.fixed_wing()
        && hull_fixed(unit, spec)
        && weapon.ballistics.guidance.is_none()
        && weapon.ballistics.delivery == contract::ballistics::Delivery::Powered
        && !r.fire(weapon).is_some_and(|fire| {
            let cone = ctx.rules.air.fixed_wing.nose_on_deg.to_radians();
            nose_on(unit, r.point, fire.range_m, cone)
        })
    {
        return Err(ActionReason::NoFacingSlot);
    }
    // Shared identification supplies the target (or the interceptor lock a
    // round); the launcher still needs a physical sight line to acquire and
    // support it (P05).
    let guided = ctx.arsenal.weapons[spec.kinds[k]]
        .profile
        .turn_rad_s
        .is_some();
    let anti_radiation_target = weapon.anti_radiation && r.emits;
    if guided
        && (!matches!(target, Target::Unit(_) | Target::Round(_)) && !anti_radiation_target
            || !r.current
            || (!anti_radiation_target
                && !guidance_clear(ctx.world, ctx.rules, unit, mount, r.point)))
    {
        return Err(ActionReason::NoOwnSight);
    }
    engage(ctx, unit, units, mount, spec, k, target, r).map(|s| Ready {
        kind: k,
        time_of_flight_s: s.time_of_flight_s,
        intercept: s.intercept,
    })
}

/// Whether a seeker can acquire a bearing inside its forward cone. This is
/// kept as a small pure rule so native and WASM tests can pin the wraparound
/// at north without constructing a battle.
pub fn seeker_in_fov(forward: f64, bearing: f64, fov_deg: f64) -> bool {
    fov_deg.is_finite()
        && fov_deg > 0.0
        && fov_deg <= 360.0
        && wrap_angle(bearing - forward).abs() <= fov_deg.to_radians() * 0.5
}

/// Optical support is physical LOS from the operator, independent of spotting
/// range or concealment. Opaque terrain/props and fully blocking foliage stop it.
pub fn guidance_clear(
    world: &WorldGeometry,
    rules: &Rules,
    unit: &Unit,
    mount: &Mount,
    target: V3,
) -> bool {
    let eye = if unit.hull.is_some() {
        crate::sensing::eye(unit, rules)
    } else {
        let Some(k) = operator(unit, mount) else {
            return false;
        };
        // Garrison occupants stand on their assigned facade/floor seat,
        // the same physical anchor their launch muzzle uses.
        unit.members[k].position + v3(0.0, 0.0, rules.physics.infantry_eye_m)
    };
    world.sight_clear(eye, target)
        && world.foliage_depth(eye, target) < rules.sensors.foliage_full_block
}

/// Whether an identified enemy the unit may fire at stands within reach of
/// one of this mount's kinds with rounds left. While one does, the mount
/// never fires at an area on its own (W10): a crew shoots at the enemy it can
/// see, not at an unknown in a treeline. The hold ends when that enemy dies,
/// leaves reach or drops out of sight.
fn enemy_in_reach(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
) -> bool {
    // Only kinds that can be brought to bear on the enemy's height band hold
    // the mount: a helicopter overhead does not stop a tank shelling a treeline.
    let reach = |layer| {
        reach(
            ctx.arsenal,
            layer,
            (0..spec.kinds.len())
                .filter(|&k| mount.has_rounds(k))
                .map(|k| spec.kinds[k]),
        )
    };
    ctx.knowledge[unit.side.index()]
        .identified_now(ctx.tick)
        .any(|(u, t)| {
            body_distance(t.position, unit.position) <= reach(units[u.0 as usize].layer())
                && permitted(ctx, unit, Target::Unit(u))
        })
}

/// A known platform can threaten us from its observed pose, independent of
/// its private ammunition, cycle, orders or current weapon choice. This asks
/// physical shot geometry, not whether the opposing side would choose to fire.
fn return_fire_threat(
    ctx: &FireContext,
    shooter: &Unit,
    kind: TypeIndex,
    track: &crate::knowledge::Track,
) -> bool {
    let target = shooter.position + v3(0.0, 0.0, shooter.aim_height(ctx.rules));
    let delta = target.xy() - track.position.xy();
    let bearing = libm::atan2(delta.y, delta.x);
    let eye = track.position
        + v3(
            0.0,
            0.0,
            ctx.rules
                .catalog
                .get(kind)
                .hull()
                .map_or(ctx.rules.physics.infantry_eye_m, |h| h.eye_m),
        );
    let carrier = if matches!(
        ctx.rules.catalog.get(kind).mobility,
        contract::catalog::Mobility::Air { .. }
    ) {
        v3(track.velocity.x, track.velocity.y, 0.0)
    } else {
        V3::default()
    };
    ctx.arsenal.specs(kind).iter().any(|spec| {
        let origin = match spec.muzzle {
            Some(muzzle) => placed_muzzle(
                track.position,
                track.yaw,
                spec,
                muzzle,
                if spec.on.is_some() || ctx.rules.catalog.get(kind).hull().is_none() {
                    bearing
                } else {
                    track.yaw
                },
                bearing,
            ),
            None => track.position + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m),
        };
        spec.kinds.iter().any(|&k| {
            let weapon = &ctx.arsenal.weapons[k];
            let distance = (target - origin).length();
            can_damage(&weapon.def, shooter.armor(ctx.rules), ctx.rules)
                && distance >= weapon.def.min_range_m
                && weapon
                    .def
                    .engages(
                        shooter.layer(),
                        Some(shooter.unit_type(ctx.rules).target_class()),
                    )
                    .is_some_and(|fire| distance <= fire.range_m)
                && (weapon.profile.turn_rad_s.is_none()
                    || (ctx.world.sight_clear(eye, target)
                        && ctx.world.foliage_depth(eye, target)
                            < ctx.rules.sensors.foliage_full_block))
                && crate::flight::solve_launch_past(
                    ctx.world,
                    &ctx.arsenal.config,
                    &weapon.profile,
                    &Aim {
                        origin,
                        target,
                        target_velocity: v3(0.0, 0.0, 0.0),
                        carrier,
                    },
                    None,
                )
                .is_ok()
        })
    })
}

/// Automatic choice (W06, W09, W10), in order: an identified target some kind can
/// damage, ranked first by whether a loaded seeker row engages it, then by
/// known return-fire threat, then the share of the mount's
/// damage its facing armour lets through, then value, distance and observed
/// id; for the unlimited default gun, the nearest
/// identified target it cannot hurt; and only with no identified enemy in
/// reach (`enemy_in_reach`), the nearest area for general-purpose kinds. A
/// weapon with a finite supply never fires at what it cannot hurt. Only a
/// target the mount could shoot now qualifies; otherwise the first obstacle
/// met is the reason.
fn select(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> (Option<Target>, ActionReason) {
    let knowledge = &ctx.knowledge[unit.side.index()];
    let here = unit.position;
    let weapons = &ctx.arsenal.weapons;
    // What a loaded seeker row engages comes first: a seeker is carried
    // for those targets.
    let seeker_target = |target: &Unit| {
        let class = target.unit_type(ctx.rules).target_class();
        spec.kinds.iter().enumerate().any(|(slot, &kind)| {
            let def = &weapons[kind].def;
            mount.has_rounds(slot)
                && def.seeker.is_some()
                && def.engages(target.layer(), Some(class)).is_some()
        })
    };
    let mut ranked: Vec<(bool, bool, f64, u32, f64, u32, UnitId)> = knowledge
        .identified_now(ctx.tick)
        .map(|(u, t)| {
            let target = &units[u.0 as usize];
            (
                seeker_target(target),
                return_fire_threat(ctx, unit, target.kind, t),
                share_through(ctx, unit, mount, spec, target),
                target.unit_type(ctx.rules).cost,
                body_distance(t.position, here),
                t.id.0,
                u,
            )
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(b.2.total_cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(a.4.total_cmp(&b.4))
            .then(a.5.cmp(&b.5))
    });
    let mut by_distance: Vec<(f64, u32, UnitId)> = ranked
        .iter()
        .map(|&(_, _, _, _, d, id, u)| (d, id, u))
        .collect();
    by_distance.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut areas: Vec<(f64, ContactId)> = if enemy_in_reach(ctx, unit, units, mount, spec) {
        Vec::new()
    } else {
        knowledge
            .all_contacts()
            .map(|c| (body_distance(c.center, here), c.id))
            .collect()
    };
    areas.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

    let stages = ranked
        .iter()
        .map(|&(_, _, _, _, _, _, u)| (Target::Unit(u), true))
        .chain(by_distance.iter().map(|&(.., u)| (Target::Unit(u), false)))
        .chain(areas.iter().map(|&(_, c)| (Target::Contact(c), true)));
    let mut reason = None;
    for (target, must_damage) in stages {
        // An area centred off the map, or an air contact, has no ground to aim at.
        let Some(r) = resolve(ctx, unit.side, target, units) else {
            continue;
        };
        match assessed.target(ctx, unit, units, mount, spec, target, &r) {
            Ok(k) => {
                // The middle stage is only the unlimited default gun against
                // what it cannot hurt.
                let harms = hurts(ctx, spec, k, target, units);
                if harms == must_damage && (harms || fires_regardless(&weapons[spec.kinds[k]].def))
                {
                    return (Some(target), ActionReason::Aiming);
                }
            }
            Err(e) => {
                reason.get_or_insert(e);
            }
        }
    }
    (None, reason.unwrap_or(ActionReason::NoCompatibleTarget))
}

/// The largest share of its damage any of the mount's loaded kinds gets
/// through `target`: the armour it shows the shooter, or all of it into
/// soldiers.
fn share_through(
    ctx: &FireContext,
    unit: &Unit,
    mount: &Mount,
    spec: &MountSpec,
    target: &Unit,
) -> f64 {
    let armor = target
        .armor(ctx.rules)
        .map(|a| a.face(target.hull_face(unit.position)));
    (0..spec.kinds.len())
        .filter(|&k| mount.has_rounds(k))
        .map(|k| &ctx.arsenal.weapons[spec.kinds[k]].def)
        .filter(|def| {
            def.engages(
                target.layer(),
                Some(target.unit_type(ctx.rules).target_class()),
            )
            .is_some()
        })
        .map(|def| damage_share(def, armor, ctx.rules))
        .fold(0.0, f64::max)
}

/// Whether the mount has a kind that can damage an explicit target (W08).
fn compatible(
    ctx: &FireContext,
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    units: &[Unit],
) -> bool {
    let (armor, layer, class) = match target {
        Target::Unit(u) => {
            let unit = &units[u.0 as usize];
            let class = unit.unit_type(ctx.rules).target_class();
            (
                Some(unit.armor(ctx.rules).copied()),
                unit.layer(),
                Some(class),
            )
        }
        _ => (None, AltitudeLayer::Ground, None),
    };
    (0..spec.kinds.len()).any(|k| {
        let def = &ctx.arsenal.weapons[spec.kinds[k]].def;
        mount.has_rounds(k)
            && def.engages(layer, class).is_some()
            && effective(def, armor, ctx.rules)
    })
}

/// What `choose_lock` concluded: the reason to show if the mount ends with
/// no lock, and whether it could shoot the ordered target now (None with no
/// order, or while the order's target is not seen this tick).
struct LockChoice {
    idle_reason: ActionReason,
    ordered: Option<Result<usize, ActionReason>>,
}

/// Keep, replace or choose the mount's lock (W07, W08, V12). An order names
/// a priority, never a silent weapon: whenever the mount can shoot the
/// ordered target it does, at once; while it cannot, it fights whatever it
/// can, and only with nothing to shoot does it turn to the ordered target and
/// wait for its shot.
fn choose_lock(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    spec: &MountSpec,
    explicit: Option<Target>,
    assessed: &mut Assessed,
) -> LockChoice {
    // A ground point never outlives the order that named it.
    if mount
        .lock
        .as_ref()
        .is_some_and(|l| Some(l.target) != explicit && matches!(l.target, Target::Ground(_)))
    {
        mount.lock = None;
    }
    let Some(t) = explicit else {
        return LockChoice {
            idle_reason: automatic_lock(ctx, unit, units, mount, spec, assessed),
            ordered: None,
        };
    };
    let take = |mount: &mut Mount| {
        if mount.lock.as_ref().is_none_or(|lock| lock.target != t) {
            mount.lock = Some(Lock {
                target: t,
                engaging: false,
            })
        }
    };
    let resolved = resolve_for(ctx, unit, mount, spec, t, units);
    let ordered = resolved
        .filter(|r| r.current)
        .map(|r| assessed.target(ctx, unit, units, mount, spec, t, &r));
    // Shootable now; or just out of sight, within the grace, where the order
    // keeps its aim on the last sighting rather than turning away a moment.
    let holding =
        resolved.is_some_and(|r| !r.current) && mount.lock.as_ref().is_some_and(|l| l.target == t);
    if matches!(ordered, Some(Ok(_))) || holding {
        take(mount);
        return LockChoice {
            idle_reason: ActionReason::NoCompatibleTarget,
            ordered,
        };
    }
    let idle_reason = automatic_lock(ctx, unit, units, mount, spec, assessed);
    let shootable = mount.lock.as_ref().is_some_and(|l| {
        resolve_for(ctx, unit, mount, spec, l.target, units)
            .filter(|r| r.current)
            .is_some_and(|r| {
                assessed
                    .target(ctx, unit, units, mount, spec, l.target, &r)
                    .is_ok()
            })
    });
    if !shootable && resolved.is_some() {
        take(mount);
    }
    LockChoice {
        idle_reason,
        ordered,
    }
}

/// Automatic choice for a mount without an order it can follow now: keep a
/// target it can still shoot, else choose afresh (`select`). Returns the
/// reason to show if it ends with no lock.
fn automatic_lock(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> ActionReason {
    // An incoming round a shot can catch comes before anything else: the crew
    // shoots it down, then turns back to what it was on (`select`, once the
    // round is gone).
    if !matches!(
        mount.lock.as_ref().map(|l| l.target),
        Some(Target::Round(_))
    ) {
        if let Some(round) = catchable_round(ctx, unit, units, mount, spec, assessed) {
            mount.lock = Some(Lock {
                target: round,
                engaging: false,
            });
            return ActionReason::Aiming;
        }
    }
    // An area is never kept on its own once an identified enemy is in reach.
    if mount
        .lock
        .as_ref()
        .is_some_and(|l| matches!(l.target, Target::Contact(_)))
        && enemy_in_reach(ctx, unit, units, mount, spec)
    {
        mount.lock = None;
    }
    let current = mount
        .lock
        .as_ref()
        .and_then(|l| resolve_for(ctx, unit, mount, spec, l.target, units).map(|r| (l.target, r)));
    let reconsider = match &current {
        None => true,
        // Identification lapsing within the grace never makes it replaceable.
        Some((_, r)) if !r.current => false,
        // A target the mount can still shoot is kept until it no longer can
        // (out of range, obstructed, dead): a crew does not swap targets as
        // the ranking shifts. The one exception is the default gun working on
        // what it cannot hurt, which takes anything it can.
        Some((t, r)) => match assessed.target(ctx, unit, units, mount, spec, *t, r) {
            Err(_) => true,
            Ok(k) => !hurts(ctx, spec, k, *t, units),
        },
    };
    if !reconsider {
        return ActionReason::NoCompatibleTarget;
    }
    let (choice, reason) = select(ctx, unit, units, mount, spec, assessed);
    match choice {
        Some(t) if current.is_none_or(|(c, _)| c != t) => {
            mount.lock = Some(Lock {
                target: t,
                engaging: false,
            });
        }
        // Nothing better: a lock that still resolves stays and reports its own obstacle.
        Some(_) => {}
        None if current.is_some() => {}
        None => mount.lock = None,
    }
    reason
}

/// The nearest enemy round in this mount's `missile` reach (by the layer it
/// was fired from) that a shot can catch now: the same assessment as any
/// target, its lead solution included (a round speeding away has none),
/// and the shot meets it in time and within reach.
fn catchable_round(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> Option<Target> {
    let reach = |layer| {
        (0..spec.kinds.len())
            .filter(|&k| mount.has_rounds(k))
            .filter_map(|k| {
                ctx.arsenal.weapons[spec.kinds[k]]
                    .def
                    .engages(layer, Some(TargetClass::Missile))
            })
            .map(|fire| fire.range_m)
            .fold(0.0, f64::max)
    };
    let mut near: Vec<(f64, &IncomingRound)> = ctx
        .rounds
        .iter()
        .filter(|r| r.side != unit.side)
        .map(|r| (body_distance(r.position, unit.position), r))
        .filter(|&(d, r)| {
            let reach = reach(r.layer);
            reach > 0.0 && d <= reach
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
    near.into_iter().find_map(|(_, round)| {
        let target = Target::Round(RoundTarget {
            id: round.id,
            layer: round.layer,
        });
        let r = resolve(ctx, unit.side, target, units)?;
        // The shot's lead must meet the round before the round's own flight
        // ends; that it meets it within reach, every shot's assessment asks
        // ([`engage`]).
        assessed
            .shot(ctx, unit, units, mount, spec, target, &r)
            .is_ok_and(|ready| ready.time_of_flight_s <= round.ends_in_s)
            .then_some(target)
    })
}

/// What a unit's weapons concluded this tick, read by next tick's movement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reach {
    /// Some mount could shoot a target it can hurt from here (attack-move halts, W16).
    pub can_engage: bool,
    /// The attack order's target is out of reach of every mount that can hurt
    /// it (the attack pursues, W17).
    pub needs_closer: bool,
    /// The heading a gun fixed in the hull needs the body to turn to (D8).
    pub face: Option<f64>,
}

/// Candidate targeting, assessed before any soldier works a gun this tick.
struct MountPlan {
    stationary: bool,
    idle_reason: ActionReason,
    resolved: Option<Resolved>,
    assessment: Option<Result<Ready, ActionReason>>,
    kind: Option<usize>,
}

fn preferred_mount(
    unit: &Unit,
    member: usize,
    specs: &[MountSpec],
    weapons: &[Weapon],
    rules: &Rules,
    plans: &[Option<MountPlan>],
    moving: bool,
) -> Option<usize> {
    let eligible = |m: usize| {
        candidates(unit, &unit.mounts[m], &specs[unit.mounts[m].spec]).any(|k| k == member)
    };
    let available =
        |m: usize| plans[m].as_ref().is_some_and(|p| !(moving && p.stationary)) && eligible(m);
    let useful = |m: usize| {
        plans[m].as_ref().is_some_and(|p| {
            p.assessment.is_some_and(|a| {
                a.is_ok_and(|ready| {
                    p.resolved.is_some_and(|r| {
                        r.worth(
                            &weapons[specs[unit.mounts[m].spec].kinds[ready.kind]].def,
                            rules,
                        )
                    })
                })
            })
        })
    };
    let assigned = |m: usize| !specs[unit.mounts[m].spec].squad && available(m);
    let default = |m: usize| {
        available(m)
            && specs[unit.mounts[m].spec]
                .kinds
                .iter()
                .any(|&k| weapons[k].def.default)
    };
    let mounts = || 0..unit.mounts.len();
    mounts()
        .find(|&m| assigned(m) && unit.mounts[m].support.is_some())
        .or_else(|| mounts().find(|&m| assigned(m) && useful(m)))
        .or_else(|| {
            mounts().find(|&m| {
                default(m)
                    && plans[m]
                        .as_ref()
                        .is_some_and(|p| p.assessment.is_some_and(|a| a.is_ok()))
            })
        })
        .or_else(|| {
            mounts().find(|&m| {
                assigned(m)
                    && unit.mounts[m].cycles.iter().any(|c| {
                        c.needs_reload(weapons, &unit.mounts[m].ammo, &specs[unit.mounts[m].spec])
                    })
            })
        })
        .or_else(|| mounts().find(|&m| default(m)))
}

/// Launch ready physical weapons. Movement, suppression and target cover
/// each widen dispersion once.
#[allow(clippy::too_many_arguments)]
fn fire(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    m: usize,
    spec: &MountSpec,
    ready: Ready,
    target: Target,
    r: &Resolved,
    moving: bool,
    rng: &mut Rng,
    leaned: &mut Vec<u32>,
) -> Option<Shot> {
    let k = ready.kind;
    let weapon_index = spec.kinds[k];
    let weapon = &ctx.arsenal.weapons[weapon_index];
    // Its spread at the target's height: wide for a ground gun swung up at
    // an aircraft.
    let mut scatter = r.fire(&weapon.def)?.scatter_mrad
        * ctx
            .rules
            .suppression
            .penalties(unit.suppression)
            .map_or(1.0, |t| t.scatter_multiplier);
    if moving {
        scatter *= ctx.rules.physics.moving_scatter_multiplier;
    }
    // A fast pass is harder to stabilise for a ground attack.  Keep the
    // penalty on the weapon's authored dispersion so guns, rockets and bombs
    // all read the same physical rule.
    if r.layer == AltitudeLayer::Ground && unit.air.as_ref().is_some_and(|air| air.afterburner) {
        scatter *= ctx.rules.air.fixed_wing.afterburner_scatter_multiplier;
    }
    let knowledge = &ctx.knowledge[unit.side.index()];
    // Rounds and their muzzles: the hull's one weapon, or each soldier
    // taking part (every living carrier of a squad weapon, a single
    // weapon's operator); in a building, each from his slot. A soldier out
    // in the open picks his own muzzle per round (`fire_from`): where he
    // stands, or out on his lean.
    let shooters: Vec<(V3, BodyId, Option<usize>)> = match unit.hull {
        Some(_) => vec![(
            muzzle(unit, mount, spec, ctx.rules, mount.bearing),
            BodyId(VEHICLE_BODY_BASE + unit.id.0),
            None,
        )],
        None => participants(unit, mount, spec)
            .map(|k| {
                (
                    unit.members[k].position + infantry_offset(spec, ctx.rules, mount.bearing),
                    BodyId(unit.members[k].id),
                    (!unit.garrisoned()).then_some(k),
                )
            })
            .collect(),
    };
    let blockers = if shooters.iter().any(|s| s.2.is_some()) {
        lean::hulls(units, ctx.rules)
    } else {
        Vec::new()
    };
    // Aim points: seen soldiers in turn, a sampled point in an area, or the point.
    let seen: Vec<V3> = match target {
        Target::Unit(u) if !units[u.0 as usize].is_vehicle() => knowledge
            .track(u)
            .map(|t| {
                t.members
                    .iter()
                    .map(|&m| {
                        units[u.0 as usize].members[m].exposed(ctx.tick)
                            + v3(0.0, 0.0, ctx.rules.physics.infantry_aim_m)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    // Cover comes from how the target is observed: a garrison at its
    // building's slots (Q22); a soldier aimed at in the open by his cover
    // body's tier (Q20), vehicles near him among them. Vehicles get none.
    let shelter = match target {
        Target::Unit(u) => crate::garrison::shelter(&units[u.0 as usize], ctx.rules),
        _ => 0.0,
    };
    let hulls = if shelter == 0.0 && !seen.is_empty() {
        crate::cover::hull_bodies(&lean::hulls(units, ctx.rules))
    } else {
        Vec::new()
    };
    let mut launches = Vec::new();
    for (n, (origin, body, member)) in shooters.into_iter().enumerate() {
        let owner = spec.squad.then_some(body.0);
        let cycle = mount
            .cycles
            .iter_mut()
            .find(|c| c.owner == owner)
            .expect("physical weapon cycle");
        if cycle.aim < weapon.def.aim_s
            || cycle.ready() != Some(k)
            || cycle.cooldown > 0.0
            || mount.ammo[k] == Some(0)
        {
            continue;
        }
        let point = match target {
            Target::Contact(c) => {
                let contact = knowledge.ground_contact(c)?;
                let radius = contact.radius * rng.unit().sqrt();
                let angle = std::f64::consts::TAU * rng.unit();
                let p = contact.center.xy() + v2(libm::cos(angle), libm::sin(angle)) * radius;
                p.with_z(
                    ctx.world.height_at(p.x, p.y).unwrap_or(0.0) + ctx.rules.physics.infantry_aim_m,
                )
            }
            _ if !seen.is_empty() => seen[n % seen.len()],
            _ => r.point,
        };
        // A soldier whose slot does not face this round's point holds it.
        if unit.garrisoned()
            && !crate::garrison::faces(unit, participant_of(unit, body), point, ctx.rules, ctx.tick)
        {
            cycle.started = false;
            continue;
        }
        // A soldier in the open fires at the first of the seen soldiers, from
        // his turn on, that his round reaches; with none, at his own.
        // Blocked every way by the body he hides behind, he holds his round
        // (he would not fire into his own cover) until his squad re-resolves.
        let standing = FirePoint {
            origin,
            past: None,
            hull: None,
            leaning: false,
        };
        let soldier = member.map(|k| &unit.members[k]);
        let turn = (0..seen.len()).map(|j| seen[(n + j) % seen.len()]);
        let found = turn.chain([point]).find_map(|p| {
            // A farther candidate can face a different side of the building.
            if unit.garrisoned()
                && !crate::garrison::faces(unit, participant_of(unit, body), p, ctx.rules, ctx.tick)
            {
                return None;
            }
            let from = match soldier {
                Some(s) => fire_from(
                    ctx,
                    &blockers,
                    s,
                    p,
                    infantry_offset(spec, ctx.rules, mount.bearing),
                )?,
                None => standing,
            };
            ((p - from.origin).length() >= weapon.def.min_range_m).then_some((p, from))
        });
        let (point, from) = match found {
            Some(f) => f,
            None if soldier.is_some_and(|s| {
                hides_behind(ctx, s, origin, point)
                    || blockers.iter().any(|h| h.meets(origin, point))
            }) =>
            {
                cycle.started = false;
                continue;
            }
            None => (point, standing),
        };
        let origin = from.origin;
        // Sampling a contact or choosing a soldier/lean can shorten the
        // shot after the mount's target-level assessment.
        if (point - origin).length() < weapon.def.min_range_m {
            cycle.started = false;
            continue;
        }
        let aim = Aim {
            origin,
            target: point,
            target_velocity: r.velocity,
            carrier: unit.flight_velocity(),
        };
        // Cover widens the spread; it never softens a hit (V03).
        let cover = if shelter > 0.0 {
            crate::damage::shelter_spread(ctx.rules, shelter)
        } else if !seen.is_empty() {
            let tier = crate::cover::at(
                ctx.world,
                ctx.ground,
                &hulls,
                ctx.rules,
                point.xy(),
                origin.xy(),
            );
            crate::cover::spread(tier, &ctx.rules.cover)
        } else {
            1.0
        };
        let scatter = scatter * cover;
        let shooter = Some(Shooter {
            unit: unit.id,
            body,
            cover: from.past.map(crate::flight::Struck::Prop).or_else(|| {
                from.hull
                    .map(|u| crate::flight::Struck::Body(BodyId(VEHICLE_BODY_BASE + u.0)))
            }),
        });
        let rounds = mount.ammo[k];
        // A cycle's first round waits out its aiming pause (a burst) or its
        // place in the squad's stagger once the arc is known.
        let mut hold = |rng: &mut Rng| {
            if cycle.started {
                return false;
            }
            cycle.started = true;
            if let Some(burst) = weapon.def.magazine.and_then(|m| m.burst) {
                cycle.cooldown = rng.unit() * burst.aim_max_s;
                cycle.cooldown > 0.0
            } else if spec.squad {
                cycle.cooldown = weapon
                    .def
                    .magazine
                    .map_or(weapon.def.reload_s, |m| m.shot_interval_s)
                    * ((body.0.wrapping_mul(2654435761) >> 16) as f64 / 65536.0);
                true
            } else {
                false
            }
        };
        let launch = if weapon.def.ballistics.delivery == Delivery::Unpowered {
            (!hold(rng)).then(|| {
                // A bomb leaves the jet with the jet's velocity; its
                // scatter is a small push off the rack that spreads where it
                // lands around where the drop carries it (its assessment's
                // landing point and fall).
                let lands = ready.intercept;
                let scattered = crate::flight::scatter_aim(aim.origin, lands, scatter, rng);
                let push = (scattered - lands).xy()
                    * (1.0 / ready.time_of_flight_s.max(f64::MIN_POSITIVE));
                Launch {
                    origin: aim.origin,
                    carrier: aim.carrier,
                    velocity: push.with_z(0.0),
                    gravity_scale: weapon.profile.gravity_scale,
                    lifetime_s: weapon.profile.lifetime_s,
                    suppression_radius_m: weapon.profile.suppression_radius_m,
                    shooter,
                    guidance: None,
                    motor: None,
                    fall: None,
                }
            })
        } else {
            crate::flight::fire_round(
                ctx.world,
                &ctx.arsenal.config,
                &weapon.profile,
                &aim,
                scatter,
                rng,
                shooter,
                |id| fires_into(ctx, weapon, rounds, target, id),
                hold,
            )
            .ok()
            .flatten()
            .map(|(launch, _)| launch)
        };
        if let Some(launch) = launch {
            launches.push(launch);
            if let Some(n) = mount.ammo[k].as_mut() {
                *n -= 1;
            }
            cycle.rounds -= 1;
            if cycle.rounds == 0 && weapon.def.reload_s == 0.0 {
                if let Some(magazine) = weapon.def.magazine {
                    cycle.rounds = magazine.rounds;
                }
            }
            if cycle.rounds == 0 {
                cycle.loaded = None;
                if weapon.def.magazine.is_some_and(|m| m.burst.is_some()) {
                    cycle.started = false;
                }
            } else if let Some(magazine) = weapon.def.magazine {
                cycle.cooldown = magazine
                    .burst
                    .filter(|b| (magazine.rounds - cycle.rounds).is_multiple_of(b.rounds))
                    .map_or(magazine.shot_interval_s, |b| {
                        (rng.unit() * b.aim_max_s).max(magazine.shot_interval_s)
                    });
            }
            if from.leaning {
                leaned.push(body.0);
            }
        }
    }
    for cycle in &mut mount.cycles {
        if cycle.loaded.is_some_and(|k| mount.ammo[k] == Some(0)) {
            cycle.loaded = None;
            cycle.rounds = 0;
        }
    }
    let last = launches.last()?.velocity;
    mount.elevation = libm::atan2(last.z, libm::hypot(last.x, last.y));
    mount.shots = mount.shots.wrapping_add(launches.len() as u32);
    Some(Shot {
        unit: unit.id,
        mount: m,
        weapon: weapon_index,
        launches,
        target,
    })
}

/// Where a soldier fires a round from.
#[derive(Clone, Copy)]
struct FirePoint {
    origin: V3,
    /// The body he leans round, which his rounds pass (the own-cover
    /// rule): a round grazing its edge does not strike it.
    past: Option<PropId>,
    /// The friendly hull he leans round, which never withholds his shot.
    hull: Option<UnitId>,
    /// Fired from his lean point.
    leaning: bool,
}

/// Where a soldier fires a round at `point` from, when it reaches:
/// from his muzzle where he stands when the round reaches straight from
/// there; else from his claimed lean point (while he is at the place it was
/// claimed with) when it reaches from there. Either way his round passes
/// the body he leans round. `None`: it reaches from neither. The line test
/// is [`lean::reaches`], the one the cover search judges places by.
fn fire_from(
    ctx: &FireContext,
    blockers: &[lean::Hull],
    soldier: &crate::units::Soldier,
    point: V3,
    offset: V3,
) -> Option<FirePoint> {
    let muzzle = |p: crate::math::V2| {
        p.with_z(ctx.world.height_at(p.x, p.y).unwrap_or(soldier.position.z)) + offset
    };
    let lean = soldier
        .lean
        .as_ref()
        .filter(|l| (l.from - soldier.position.xy()).length() <= lean::AT_PLACE_M);
    let past = lean.and_then(|l| l.past());
    let hull = lean.and_then(|l| match l.body {
        lean::Round::Hull(u) => Some(u),
        lean::Round::Prop(_) => None,
    });
    let standing = soldier.position + offset;
    if lean::reaches(ctx.world, blockers, standing, point, None) {
        return Some(FirePoint {
            origin: standing,
            past,
            hull,
            leaning: false,
        });
    }
    // Out on a burst, or tucked in long enough to lean out again.
    let l = lean.filter(|l| {
        (ctx.tick < soldier.leaning_until || ctx.tick >= soldier.tucked_until)
            && lean::reaches(ctx.world, blockers, muzzle(l.at), point, past)
    })?;
    Some(FirePoint {
        origin: muzzle(l.at),
        past,
        hull,
        leaning: true,
    })
}

/// Whether the first body on a soldier's straight line from `origin` to
/// `point` is one he takes cover behind from it (Q20's test, the one that
/// gives him a tier).
fn hides_behind(ctx: &FireContext, soldier: &crate::units::Soldier, origin: V3, point: V3) -> bool {
    let to = point - origin;
    let len = to.length();
    if len < 1e-6 {
        return false;
    }
    let Some(hit) = ctx.world.raycast(origin, to * (1.0 / len), len) else {
        return false;
    };
    let Collider::Prop(id) = hit.collider else {
        return false;
    };
    let Some(prop) = ctx.world.prop(id) else {
        return false;
    };
    let (at, reach) = (soldier.position.xy(), ctx.rules.cover.reach_m);
    let radius = ctx.rules.physics.soldier_radius_m;
    crate::cover::covers(&prop.footprint(), at, point.xy(), reach, radius)
}

/// Eligible carriers, independent of active choice: every living soldier
/// carrying a squad weapon; otherwise its [`operator`].
fn candidates<'a>(
    unit: &'a Unit,
    mount: &Mount,
    spec: &'a MountSpec,
) -> impl Iterator<Item = usize> + 'a {
    let operator = (!spec.squad).then(|| operator(unit, mount)).flatten();
    (0..unit.members.len()).filter(move |&k| match operator {
        Some(o) => k == o,
        None => spec.squad && carries(unit, spec, k),
    })
}

/// Soldiers actually working this mount; candidate discovery never reads activity.
fn participants<'a>(
    unit: &'a Unit,
    mount: &'a Mount,
    spec: &'a MountSpec,
) -> impl Iterator<Item = usize> + 'a {
    candidates(unit, mount, spec).filter(move |&k| unit.members[k].active_mount == Some(mount.spec))
}

fn cycle_active(unit: &Unit, mount: &Mount, cycle: &Cycle) -> bool {
    unit.hull.is_some()
        || unit.members.iter().any(|s| {
            s.alive()
                && s.active_mount == Some(mount.spec)
                && Some(s.id) == cycle.owner.or(mount.operator)
        })
}

/// The living soldier currently operating this physical gun.
fn operator(unit: &Unit, mount: &Mount) -> Option<usize> {
    unit.members
        .iter()
        .position(|s| Some(s.id) == mount.operator && s.alive())
}

/// Assign single infantry guns exclusively. Living original carriers get
/// their own weapon, then survivors keep handoffs, then free soldiers take
/// stocked spares in mount order. An empty launcher stays operated only while
/// it guides its last missile; after that its soldier can pick up a spare.
pub fn assign_operators(arsenal: &Arsenal, unit: &mut Unit) {
    if unit.hull.is_some() {
        return;
    }
    let specs = arsenal.specs(unit.kind);
    let mut assigned = vec![None; unit.mounts.len()];
    let mut used = Vec::new();
    let needs = |m: &Mount| !specs[m.spec].squad && (!m.out_of_ammo() || m.support.is_some());
    for (i, mount) in unit.mounts.iter().enumerate().filter(|(_, m)| needs(m)) {
        if let Some(s) = unit.members.iter().find(|s| {
            s.alive() && specs[mount.spec].carriers.contains(&s.slot) && !used.contains(&s.id)
        }) {
            assigned[i] = Some(s.id);
            used.push(s.id);
        }
    }
    for (i, mount) in unit
        .mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| needs(m) && specs[m.spec].special)
    {
        if assigned[i].is_none() {
            if let Some(s) = unit
                .members
                .iter()
                .find(|s| s.alive() && Some(s.id) == mount.operator && !used.contains(&s.id))
            {
                assigned[i] = Some(s.id);
                used.push(s.id);
            }
        }
    }
    for (i, _) in unit
        .mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| needs(m) && specs[m.spec].special)
    {
        if assigned[i].is_none() {
            if let Some(s) = unit
                .members
                .iter()
                .find(|s| s.alive() && !used.contains(&s.id))
            {
                assigned[i] = Some(s.id);
                used.push(s.id);
            }
        }
    }
    for (mount, operator) in unit.mounts.iter_mut().zip(assigned) {
        mount.operator = operator;
    }
}

fn carries(unit: &Unit, spec: &MountSpec, k: usize) -> bool {
    let s = &unit.members[k];
    s.alive() && spec.carriers.contains(&s.slot)
}

fn participant_of(unit: &Unit, body: BodyId) -> usize {
    unit.members
        .iter()
        .position(|s| BodyId(s.id) == body)
        .expect("a shooter is a member")
}

/// A garrisoned squad's mounts that have something to face: each one's
/// participants and the point it faces.
pub struct MountAim {
    pub members: Vec<usize>,
    pub point: V3,
    /// Authored physical ownership, independent of how many riflemen survive.
    pub single_operator: bool,
}
pub type MountAims = Vec<MountAim>;

/// Whether a carried gun could engage after taking a suitable window. The
/// normal assessment owns range, trajectory and guidance LOS here too; only
/// position and idle watching permission are projected. Known attackers still
/// constrain return fire; a missing current window cannot prevent requesting one.
fn assess_windows(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    resolved: &Resolved,
) -> Result<Ready, ActionReason> {
    let mut prospective = unit.clone();
    if unit.attackers.is_empty() {
        prospective.engagement = Engagement::FireAtWill;
    }
    let current = assess(ctx, &prospective, units, mount, spec, target, resolved);
    if current.is_ok() || !permitted(ctx, &prospective, target) {
        return current;
    }
    let g = unit
        .hold
        .as_ref()
        .and_then(|hold| hold.building())
        .expect("a garrison requests windows");
    let facing = ctx.rules.garrison.slot_facing_min_deg.to_radians();
    for member in candidates(unit, mount, spec) {
        for (seat, window) in g
            .slots
            .iter()
            .enumerate()
            .filter(|(_, w)| w.slot.faces(resolved.point.xy(), facing))
        {
            prospective.members[member].position = window.position;
            prospective
                .hold
                .as_mut()
                .unwrap()
                .building_mut()
                .unwrap()
                .seats[member] = Some(seat);
            let assessment = assess(ctx, &prospective, units, mount, spec, target, resolved);
            if assessment.is_ok() {
                return assessment;
            }
        }
        prospective.members[member].position = unit.members[member].position;
        prospective
            .hold
            .as_mut()
            .unwrap()
            .building_mut()
            .unwrap()
            .seats[member] = g.seats[member];
    }
    current
}

/// Each soldier requests a window for one weapon, using the same priority as
/// active selection but assessing prospective windows. This breaks the circular
/// dependency between a useful weapon and the window it needs, without letting
/// an inactive rifle pull a launcher operator away from his target.
pub fn garrison_aims(ctx: &FireContext, units: &[Unit]) -> Vec<(usize, MountAims)> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.alive() && u.garrisoned())
        .map(|(i, u)| {
            let specs = ctx.arsenal.specs(u.kind);
            let plans: Vec<_> = u
                .mounts
                .iter()
                .map(|m| {
                    let spec = &specs[m.spec];
                    let mut candidate = m.clone();
                    let mut assessed = Assessed {
                        prospective_windows: true,
                        ..Default::default()
                    };
                    let explicit = u
                        .attack_target()
                        .filter(|&t| compatible(ctx, m, spec, t, units));
                    let idle_reason =
                        choose_lock(ctx, u, units, &mut candidate, spec, explicit, &mut assessed)
                            .idle_reason;
                    let target = m
                        .support
                        .map(|s| s.target)
                        .or_else(|| candidate.lock.as_ref().map(|l| l.target))?;
                    let r = resolve_for(ctx, u, m, spec, target, units)?;
                    let assessment = assessed.shot(ctx, u, units, m, spec, target, &r);
                    Some(MountPlan {
                        stationary: false,
                        idle_reason,
                        resolved: Some(r),
                        assessment: Some(assessment),
                        kind: preferred_kind(ctx, m, spec, &r),
                    })
                })
                .collect();
            let choices: Vec<_> = (0..u.members.len())
                .map(|member| {
                    preferred_mount(
                        u,
                        member,
                        specs,
                        &ctx.arsenal.weapons,
                        ctx.rules,
                        &plans,
                        false,
                    )
                })
                .collect();
            let aims = plans
                .iter()
                .enumerate()
                .filter_map(|(m, plan)| {
                    let r = plan.as_ref()?.resolved?;
                    let members: Vec<_> = candidates(u, &u.mounts[m], &specs[u.mounts[m].spec])
                        .filter(|&member| choices[member] == Some(m))
                        .collect();
                    (!members.is_empty()).then_some(MountAim {
                        members,
                        point: r.point,
                        single_operator: !specs[u.mounts[m].spec].squad,
                    })
                })
                .collect();
            (i, aims)
        })
        .collect()
}

/// Whether a unit is fighting: a mount aiming at, reloading on, traversing to
/// or firing at a target, or guiding a missile. A fighting unit is "firing"
/// for service (L04), not only on the tick a round leaves.
pub fn engaged(unit: &Unit) -> bool {
    unit.mounts
        .iter()
        .any(|m| m.support.is_some() || m.lock.as_ref().is_some_and(|l| l.engaging))
}

/// A mount's pose for animation: published for own units and identified enemies.
pub fn pose(mount: &Mount) -> WeaponPose {
    WeaponPose {
        mount: mount.spec as u8,
        operator: mount.operator,
        bearing: mount.bearing,
        elevation: mount.elevation,
        shots: mount.shots,
    }
}

/// Exported readiness of a mount for its owner's panel and rings.
pub fn readiness(
    arsenal: &Arsenal,
    unit: &Unit,
    mount: &Mount,
    target_ref: Option<TargetRef>,
) -> MountReadiness {
    let spec = &arsenal.specs(unit.kind)[mount.spec];
    let loaded = mount.cycles.iter().find_map(Cycle::ready);
    let reloading = if loaded.is_some() {
        None
    } else {
        mount.cycles.iter().filter_map(|c| c.reload).min_by(|a, b| {
            let remaining = |(k, progress): (usize, f64)| {
                arsenal.weapons[spec.kinds[k]].def.reload_s - progress
            };
            remaining(*a).total_cmp(&remaining(*b))
        })
    };
    let aim = mount
        .cycles
        .iter()
        .filter(|c| cycle_active(unit, mount, c))
        .map(|c| {
            let k = c.loaded.or(c.reload.map(|(k, _)| k)).unwrap_or(0);
            (c.aim / arsenal.weapons[spec.kinds[k]].def.aim_s).min(1.0)
        })
        .fold(0.0, f64::max);
    let reload = reloading.map_or(0.0, |(k, p)| {
        (p / arsenal.weapons[spec.kinds[k]].def.reload_s).min(1.0)
    });
    MountReadiness {
        mount: mount.spec as u8,
        loaded: loaded.map(|k| k as u8),
        reloading: reloading.map(|(k, _)| k as u8),
        ammo: mount.ammo.clone(),
        aim,
        reload,
        target: target_ref,
        reason: mount.reason,
        guiding: mount.support.is_some(),
    }
}

#[cfg(test)]
mod target_priority_tests;
