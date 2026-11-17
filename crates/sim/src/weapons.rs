//! Mounts choose targets from their side's knowledge (W06, W10).
//! Physical guns own aim and cycles (W01–W02); rounds leave only through flight.
use contract::ballistics::Delivery;
use contract::catalog::{AltitudeLayer, TypeIndex};
use contract::command::TargetRef;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ActionReason, ContactId};
use contract::scenario::{Armor, Rules};
use contract::weapons::{AmmoCapacity, Fire, TargetClass, WeaponDefinition};

mod arsenal;
pub use arsenal::{Arsenal, MountSpec, Weapon};
mod admission;
mod assessment;
pub use admission::check_rules;
use assessment::{assess, seen_soldiers, Assessed, Ready};
mod bearing;
mod bomb;
mod cycle;
mod engagement;
use engagement::engage;
mod fire_point;
use fire_point::{fire_from, hides_behind, FirePoint};
mod muzzle;
mod observation;
mod permission;
use permission::permitted;
pub mod operators;
mod window_requests;
pub use observation::{pose, readiness};

pub use window_requests::{garrison_aims, MountAim, MountAims};
pub mod tick;
use bearing::{bearing_from, hull_fixed, point_mount, turned_mount};
pub use bearing::{nose_on, nose_on_bearing, Arc};
use bomb::bomb_release;
use cycle::Cycle;
use muzzle::{infantry_offset, muzzle, placed_muzzle};

use crate::damage;
use crate::deployment;
use crate::digest::Digest;
use crate::flight::{Aim, BodyId, FlightConfig, Launch, LaunchProfile, ProjectileId, Shooter};
use crate::knowledge::SideKnowledge;
use crate::lean;
use crate::math::{body_distance, v2, v3, wrap_angle, V3};
use crate::units::Unit;
use crate::world::{PropId, WorldGeometry};
use contract::random::Rng;

/// Vehicle hull bodies take ids above every soldier id.
pub const VEHICLE_BODY_BASE: u32 = 1 << 24;

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

/// A round in flight, by its id and fixed engagement layer; where it
/// is and how it flies are read live ([`IncomingRound`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundTarget {
    pub id: ProjectileId,
    pub layer: AltitudeLayer,
}

/// An interceptable round in flight, as every mount may engage it: who fired
/// it, where it is, how it flies, and its fixed engagement layer.
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
    /// Rounds per kind, including magazines; `None` means unlimited.
    pub ammo: Vec<Option<u32>>,
    /// Pausable service seconds toward each kind's next round.
    pub resupply_s: Vec<f64>,
    /// One physical gun per carrier; transferable/hull weapons use owner None.
    pub cycles: Vec<Cycle>,
    /// Infantry operator; None for hulls, grouped rifles and unoperated spares.
    pub operator: Option<u32>,
    pub lock: Option<Lock>,
    /// The missile this mount is guiding, if any (one at a time).
    pub support: Option<Support>,
    pub bearing: f64,
    pub last_enemy_lock_tick: Option<Tick>,
    /// Elevation of the last round this mount launched (radians); 0 before.
    pub elevation: f64,
    /// Last launched weapon index, for publication only; never digested.
    pub round: u8,
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

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.spec as u64).u64(self.ammo.len() as u64);
        for a in &self.ammo {
            d.u64(a.map_or(u64::MAX, |n| n as u64));
        }
        for &seconds in &self.resupply_s {
            d.f64(seconds);
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
    /// A gun finished a firing cycle with this shot ([`Cycle::spend_round`]),
    /// or the mount ran dry.
    pub cycle_ended: bool,
}

pub struct FireContext<'a> {
    pub world: &'a WorldGeometry,
    pub windows: &'a crate::garrison::Windows,
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
            // its fixed engagement layer.
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
                || (resolved.armor.is_some() && def.fires_regardless()))
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
        && world.sight_depth(eye, target) < rules.sensors.foliage_full_block
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
            ctx.rules.physics.eye_m(ctx.rules.catalog.get(kind)),
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
                        && ctx.world.sight_depth(eye, target)
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
                if harms == must_damage && (harms || weapons[spec.kinds[k]].def.fires_regardless())
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

/// The nearest enemy round in this mount's `missile` reach, by its assigned
/// layer, that a shot can catch now: the same assessment as any
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
    /// Mounts (a bit each) working on the attack order's target this
    /// tick: firing, aiming, reloading, traversing or guiding.
    pub ordered_working: u64,
    /// Some mount can hurt the attack order's target ([`compatible`]).
    pub can_damage: bool,
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
    // Aim points: seen soldiers in turn, from the one the assessment found,
    // a sampled point in an area, or the point.
    let seen = seen_soldiers(ctx, unit.side, units, target);
    // Cover comes from how the target is observed: a garrison at its
    // building's slots (Q22); a soldier aimed at in the open by his cover
    // body's tier (Q20), vehicles near him among them. Vehicles get none.
    let shelter = match target {
        Target::Unit(u) => crate::garrison::shelter(&units[u.0 as usize]),
        _ => 0.0,
    };
    let hulls = if shelter == 0.0 && !seen.is_empty() {
        crate::cover::hull_bodies(&lean::hulls(units, ctx.rules))
    } else {
        Vec::new()
    };
    let mut launches = Vec::new();
    let mut fired = 0u32;
    let mut cycle_ended = false;
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
        let faces = |p| {
            ctx.windows
                .faces(unit, participant_of(unit, body), p, ctx.rules, ctx.tick)
        };
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
            _ if !seen.is_empty() => seen[(n + ready.soldier) % seen.len()],
            _ => r.point,
        };
        // A soldier whose slot does not face this round's point holds it.
        if unit.garrisoned() && !faces(point) {
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
        let turn = (0..seen.len()).map(|j| seen[(n + ready.soldier + j) % seen.len()]);
        let found = turn.chain([point]).find_map(|p| {
            // A farther candidate can face a different side of the building.
            if unit.garrisoned() && !faces(p) {
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
        let round: Vec<Launch> = if weapon.def.ballistics.delivery == Delivery::Unpowered {
            if hold(rng) {
                Vec::new()
            } else {
                bomb_release(weapon, &aim, ready, scatter, shooter, rng)
            }
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
            .into_iter()
            .collect()
        };
        if !round.is_empty() {
            launches.extend(round);
            fired += 1;
            if let Some(n) = mount.ammo[k].as_mut() {
                *n -= 1;
            }
            cycle_ended |= cycle.spend_round(&weapon.def, rng);
            if from.leaning {
                leaned.push(body.0);
            }
        }
    }
    for cycle in &mut mount.cycles {
        if cycle.loaded.is_some_and(|k| mount.ammo[k] == Some(0)) {
            cycle.loaded = None;
            cycle.rounds = 0;
            cycle_ended = true;
        }
    }
    let last = launches.last()?.velocity;
    mount.elevation = libm::atan2(last.z, libm::hypot(last.x, last.y));
    mount.round = k as u8;
    mount.shots = mount.shots.wrapping_add(fired);
    Some(Shot {
        unit: unit.id,
        mount: m,
        weapon: weapon_index,
        launches,
        target,
        cycle_ended,
    })
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

/// Whether a unit is fighting: a mount aiming at, reloading on, traversing to
/// or firing at a target, or guiding a missile.
pub fn engaged(unit: &Unit) -> bool {
    unit.mounts
        .iter()
        .any(|m| m.support.is_some() || m.lock.as_ref().is_some_and(|l| l.engaging))
}

#[cfg(test)]
mod target_priority_tests;
