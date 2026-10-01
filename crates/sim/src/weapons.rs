//! Mounts choose targets from their side's knowledge (W06, W10) and share aim.
//! Physical guns own their cycles (W01–W02); rounds leave only through flight.
use contract::catalog::TypeIndex;
use contract::command::{Engagement, TargetRef};
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ActionReason, ContactId, MountReadiness, WeaponPose};
use contract::scenario::{Armor, Rules};
use contract::weapons::{AmmoCapacity, WeaponDefinition};

mod cycle;
use cycle::Cycle;

use crate::digest::Digest;
use crate::flight::{
    launch_along, predicted_path, solve_launch_past, Aim, BodyId, FiringSolution, FlightConfig,
    Launch, LaunchProfile, NoSolution, ProjectileId, Shooter,
};
use crate::knowledge::SideKnowledge;
use crate::lean;
use crate::math::{v2, v3, wrap_angle, V3};
use crate::rng::Rng;
use crate::units::Unit;
use crate::world::{Collider, PropId, WorldGeometry};

/// Vehicle hull bodies take ids above every soldier id.
pub const VEHICLE_BODY_BASE: u32 = 1 << 24;

/// A weapon row with its validated flight profile.
pub struct Weapon {
    /// The row's id in the rules' `weapons`.
    pub id: String,
    pub def: WeaponDefinition,
    pub profile: LaunchProfile,
}

/// An authored mount: ammunition kinds (weapon indices) sharing aim/reload.
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
    /// Where it turns, in its carrier's frame (forward, left, up).
    pub pivot: V3,
    /// Its muzzle from the pivot along its own bearing; `None` is a hand
    /// weapon, fired at the infantry muzzle height.
    pub muzzle: Option<V3>,
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

impl Arsenal {
    pub fn new(rules: &Rules) -> Self {
        let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz)
            .expect("fixture flight rules are valid");
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
        assert!(
            weapons.len() <= crate::publication::MAX_WEAPON_ROWS,
            "more weapon rows than a firing report's heard mask carries"
        );
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
                        assert!(
                            m.weapons.len() <= crate::publication::MAX_AMMO_KINDS,
                            "mount {} has more ammunition kinds than the publication carries",
                            m.name
                        );
                        let on = m.on.as_ref().map(|carrier| {
                            list[..i]
                                .iter()
                                .position(|c| c.def.name == *carrier && c.def.turret)
                                .expect("a mount rides an earlier turret (checked at load)")
                        });
                        let v = |[x, y, z]: [f64; 3]| v3(x, y, z);
                        MountSpec {
                            kinds: m.weapons.iter().map(|w| index(w)).collect(),
                            squad: m.squad,
                            turret: m.turret,
                            carriers: c.carriers.clone(),
                            special: m.special,
                            on,
                            pivot: v(m.pivot_m),
                            muzzle: m.muzzle_m.map(v),
                        }
                    })
                    .collect()
            })
            .collect();
        let optics = catalog
            .indices()
            .map(|t| {
                let on = catalog.get(t).sensors.on.as_ref()?;
                catalog.mounts(t).iter().position(|m| &m.def.name == on)
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
    pub fn heard(&self, kind: TypeIndex, mount: usize) -> u32 {
        self.mounts[kind.0 as usize][mount]
            .kinds
            .iter()
            .fold(0, |m, &k| m | 1 << k)
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
}

impl Target {
    pub fn digest(self, d: &mut Digest) {
        match self {
            Target::Unit(u) => d.u64(0).u64(u.0 as u64),
            Target::Contact(c) => d.u64(1).u64(c.0 as u64),
            Target::Ground(p) => d.u64(2).f64(p.x).f64(p.y).f64(p.z),
        };
    }
}

#[derive(Clone, Debug)]
pub struct Lock {
    pub target: Target,
    /// Seconds of aim accumulated on this target.
    pub aim: f64,
    /// From an explicit attack order: kept against automatic reconsideration.
    pub explicit: bool,
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
    /// World heading of a turret (or the last aim of a hand weapon).
    pub bearing: f64,
    /// Elevation of the last round this mount launched (radians); 0 before.
    pub elevation: f64,
    /// Rounds launched since the battle began, wrapping at 2³².
    pub shots: u32,
    pub reason: ActionReason,
}

impl Mount {
    fn has_rounds(&self, k: usize) -> bool {
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
            d.f64(l.aim).u64(l.explicit as u64).u64(l.engaging as u64);
        }
    }

    /// Stop (W15): clear aim once, drop an unfinished reload, keep what is
    /// loaded, and release any guided missile (P06).
    pub fn stop(&mut self) {
        self.lock = None;
        for c in &mut self.cycles {
            c.started = false;
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
    pub knowledge: &'a [SideKnowledge; 2],
}

/// How far a squad's own weapons reach: its squad mount's longest range
/// (the rifles every soldier carries). The cover search judges where a
/// soldier can engage from by it.
pub fn squad_range(arsenal: &Arsenal, kind: TypeIndex) -> f64 {
    reach(
        arsenal,
        arsenal
            .specs(kind)
            .iter()
            .filter(|s| s.squad)
            .flat_map(|s| s.kinds.iter().copied()),
    )
}

/// The longest range of these weapon rows (0 for none).
fn reach(arsenal: &Arsenal, rows: impl Iterator<Item = usize>) -> f64 {
    rows.map(|k| arsenal.weapons[k].def.ballistics.range_m)
        .fold(0.0, f64::max)
}

/// Whether a weapon can hurt a target with this hull armour (`None`: a
/// soldier) at all (P10: penetration beats the weakest face; dedicated
/// anti-armour never engages infantry).
pub fn can_damage(def: &WeaponDefinition, armor: Option<&Armor>) -> bool {
    match armor {
        Some(armor) => def.penetration > armor.weakest() || def.armor_fraction > 0.0,
        None => !def.anti_armor,
    }
}

/// The unit's always-available gun with rounds to spare (a squad's rifles, a
/// roof HMG): it fires at an identified enemy it cannot hurt, to keep heads
/// down (W09). A weapon with a finite supply keeps it for what it can hurt.
fn fires_regardless(def: &WeaponDefinition) -> bool {
    def.default && matches!(def.ammo, AmmoCapacity::Unlimited(_))
}

/// General-purpose ammunition: may fire at an area (never AP or dedicated anti-armour, W10).
fn area_capable(def: &WeaponDefinition) -> bool {
    !def.anti_armor && !def.armor_piercing
}

/// Whether `def` is effective against a target whose type is known, or an area.
fn effective(def: &WeaponDefinition, armor: Option<Option<Armor>>) -> bool {
    match armor {
        Some(armor) => can_damage(def, armor.as_ref()),
        None => area_capable(def),
    }
}

fn resolve(ctx: &FireContext, side: Side, target: Target, units: &[Unit]) -> Option<Resolved> {
    let knowledge = &ctx.knowledge[side.index()];
    match target {
        Target::Unit(u) => {
            let track = knowledge.track(u)?;
            let unit = &units[u.0 as usize];
            let height = unit.hull.map_or(ctx.rules.physics.infantry_aim_m, |h| {
                2.0 * h.z * ctx.rules.physics.vehicle_aim_height_fraction
            });
            Some(Resolved {
                point: track.position + v3(0.0, 0.0, height),
                velocity: track.velocity.with_z(0.0),
                current: track.last_seen == ctx.tick,
                armor: Some(unit.armor(ctx.rules).copied()),
            })
        }
        Target::Contact(c) => {
            let contact = knowledge.contact(c)?;
            let z = ctx.world.height_at(contact.center.x, contact.center.y)?;
            Some(Resolved {
                point: contact.center.with_z(z + ctx.rules.physics.infantry_aim_m),
                velocity: v3(0.0, 0.0, 0.0),
                current: true,
                armor: None,
            })
        }
        Target::Ground(p) => Some(Resolved {
            point: p,
            velocity: v3(0.0, 0.0, 0.0),
            current: true,
            armor: None,
        }),
    }
}

/// The mount's preferred kind for a target: armour-piercing for identified
/// vehicles, otherwise non-AP; the unlimited default gun also against what it
/// cannot hurt (W09); AP never at an area (W10).
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
            && want_ap.is_none_or(|ap| def.armor_piercing == ap)
            && (effective(def, resolved.armor)
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

fn bearing_from(unit: &Unit, point: V3) -> f64 {
    let to = point.xy() - unit.position.xy();
    to.y.atan2(to.x)
}

/// Where a mount's rounds leave when it points along `bearing`: each mount
/// fires from its own muzzle. Its pivot turns with its carrier (the turret
/// it sits on, at that mount's bearing, or the hull), and its muzzle turns
/// with its own bearing about the pivot. A hand weapon fires at the
/// infantry muzzle height: a single one from its operator where he stands
/// (his lean is [`fire_from`]'s), a squad weapon's volley judged first from
/// the squad's middle (each soldier then fires from his own).
fn muzzle(unit: &Unit, mount: &Mount, spec: &MountSpec, rules: &Rules, bearing: f64) -> V3 {
    let Some(muzzle) = spec.muzzle else {
        let at = match operator(unit, mount).filter(|_| !spec.squad) {
            Some(k) => unit.members[k].position,
            None => unit.position,
        };
        return at + v3(0.0, 0.0, rules.physics.infantry_muzzle_m);
    };
    // A unit's mounts are its type's list, in order (`Arsenal::mounts_for`).
    let carried = spec.on.map_or(unit.yaw, |c| unit.mounts[c].bearing);
    let turn = |p: V3, by: f64| v2(p.x, p.y).rotated(by).with_z(p.z);
    // Offset first, then placed: a mount on the hull's axis lands on exactly
    // the point a single hull-frame offset did.
    unit.position + (turn(spec.pivot, carried) + turn(muzzle, bearing))
}

/// P11: withhold when a friendly vehicle sits on the predicted path or in the
/// blast at `burst`. Friendly infantry never withholds a shot (it can still be hit).
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
) -> bool {
    let margin = ctx.rules.physics.friendly_prefire_margin_m;
    let (def, profile) = (&weapon.def, &weapon.profile);
    let path = predicted_path(
        &ctx.arsenal.config,
        profile,
        origin,
        s.velocity,
        s.time_of_flight_s,
    );
    units
        .iter()
        .filter(|u| u.side == shooter.side && u.id != shooter.id && u.hull.is_some() && u.alive())
        .filter(|u| Some(u.id) != leaning_round)
        .any(|u| {
            // One frame per hull: its rotation is not recomputed per sample.
            let hull = u.hull_frame();
            path.windows(2).any(|w| {
                // A chord that never comes near the hull has no sample near it.
                if !hull.may_come_within(w[0], w[1], margin) {
                    return false;
                }
                // Sample each chord densely enough for a hull-sized margin.
                let n = ((w[1] - w[0]).length() / 1.0).ceil().max(1.0) as usize;
                (0..=n)
                    .any(|k| hull.distance(w[0] + (w[1] - w[0]) * (k as f64 / n as f64)) < margin)
            }) || (def.blast_radius_m > 0.0 && hull.distance(burst) < def.blast_radius_m)
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

/// The arc a round would fly to `aim` and where it bursts: its intercept, or
/// the body it is fired into.
fn solve(
    ctx: &FireContext,
    weapon: &Weapon,
    rounds: Option<u32>,
    aim: &Aim,
    target: Target,
    past: Option<PropId>,
) -> Result<(FiringSolution, V3), NoSolution> {
    match solve_launch_past(ctx.world, &ctx.arsenal.config, &weapon.profile, aim, past) {
        Ok(s) => Ok((s, s.intercept)),
        Err(NoSolution::Blocked {
            arc,
            point,
            by: Collider::Prop(id),
        }) if fires_into(ctx, weapon, rounds, target, id) => Ok((arc, point)),
        Err(e) => Err(e),
    }
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
    let bearing = if spec.turret {
        bearing_from(unit, r.point)
    } else {
        mount.bearing
    };
    let from = |origin: V3, past: Option<PropId>, hull: Option<UnitId>| {
        if (r.point - origin).length() > weapon.def.ballistics.range_m {
            return Err(ActionReason::OutOfRange);
        }
        let aim = Aim {
            origin,
            target: r.point,
            target_velocity: r.velocity,
        };
        match solve(ctx, weapon, mount.ammo[k], &aim, target, past) {
            Err(NoSolution::OutOfReach) => Err(ActionReason::OutOfRange),
            Err(NoSolution::Blocked { .. }) => Err(ActionReason::BlockedTrajectory),
            Ok((s, burst))
                if friendly_in_line(ctx, unit, units, origin, (&s, burst), weapon, hull) =>
            {
                Err(ActionReason::FriendlyInLine)
            }
            Ok((s, _)) => Ok(s),
        }
    };
    if let Some(garrison) = unit.garrison.as_ref().filter(|_| unit.garrisoned()) {
        let facing = ctx.rules.garrison.slot_facing_min_deg.to_radians();
        let mut result = Err(ActionReason::NoFacingSlot);
        for k in participants(unit, mount, spec) {
            let Some(seat) = garrison
                .seat(k)
                .filter(|s| s.slot.faces(r.point.xy(), facing))
            else {
                continue;
            };
            // Assessment and launch use the same occupied window's muzzle.
            result = from(
                seat.position + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m),
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
    participants(unit, mount, spec)
        .map(|k| {
            let f = fire_from(ctx, &blockers, &unit.members[k], r.point)?;
            from(f.origin, f.past, f.hull).ok()
        })
        .find_map(|s| s.map(Ok))
        .unwrap_or(at_unit)
}

/// Whether the unit's engagement policy allows firing at this target (W12–W13).
fn permitted(ctx: &FireContext, unit: &Unit, target: Target) -> bool {
    match unit.engagement {
        Engagement::FireAtWill => true,
        Engagement::ReturnFireOnly => match target {
            Target::Unit(u) => unit.attackers.contains(&u),
            Target::Contact(c) => ctx.knowledge[unit.side.index()]
                .contact(c)
                .is_some_and(|contact| unit.attackers.contains(&contact.emitter)),
            Target::Ground(_) => false,
        },
    }
}

/// One mount's assessments this tick, by target: choosing its lock and then
/// firing ask of the same target with the same world, so each target is
/// assessed once (its line tests and firing solutions are the costly part).
#[derive(Default)]
struct Assessed(Vec<(Target, Result<usize, ActionReason>)>);

impl Assessed {
    fn of(
        &mut self,
        target: Target,
        assess: impl FnOnce() -> Result<usize, ActionReason>,
    ) -> Result<usize, ActionReason> {
        if let Some(&(_, a)) = self.0.iter().find(|(t, _)| *t == target) {
            return a;
        }
        let a = assess();
        self.0.push((target, a));
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
) -> Result<usize, ActionReason> {
    if !permitted(ctx, unit, target) {
        return Err(ActionReason::HoldingFire);
    }
    let k = preferred_kind(ctx, mount, spec, r).ok_or(if mount.out_of_ammo() {
        ActionReason::OutOfAmmo
    } else {
        ActionReason::NoCompatibleTarget
    })?;
    // A guided launcher needs its own identification, not the team's (P05).
    if ctx.arsenal.weapons[spec.kinds[k]]
        .profile
        .turn_rad_s
        .is_some()
    {
        let own = match target {
            Target::Unit(u) => ctx.knowledge[unit.side.index()].own_sees(unit.id, u),
            _ => false,
        };
        if !own {
            return Err(ActionReason::NoOwnSight);
        }
    }
    engage(ctx, unit, units, mount, spec, k, target, r).map(|_| k)
}

/// Whether an identified enemy the unit may fire at stands within reach of
/// one of this mount's kinds with rounds left. While one does, the mount
/// never fires at an area on its own (W10): a crew shoots at the enemy it can
/// see, not at an unknown in a treeline. The hold ends when that enemy dies,
/// leaves reach or drops out of sight.
fn enemy_in_reach(ctx: &FireContext, unit: &Unit, mount: &Mount, spec: &MountSpec) -> bool {
    let reach = reach(
        ctx.arsenal,
        (0..spec.kinds.len())
            .filter(|&k| mount.has_rounds(k))
            .map(|k| spec.kinds[k]),
    );
    let here = unit.position.xy();
    ctx.knowledge[unit.side.index()]
        .identified_now(ctx.tick)
        .any(|(u, t)| {
            (t.position.xy() - here).length() <= reach && permitted(ctx, unit, Target::Unit(u))
        })
}

/// Automatic choice (W06, W09, W10), in order: the highest-cost identified
/// target some kind can damage; for the unlimited default gun, the nearest
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
    let here = unit.position.xy();
    let weapons = &ctx.arsenal.weapons;
    let mut by_cost: Vec<(u32, f64, u32, UnitId)> = knowledge
        .identified_now(ctx.tick)
        .map(|(u, t)| {
            let cost = units[u.0 as usize].unit_type(ctx.rules).cost;
            (cost, (t.position.xy() - here).length(), t.id.0, u)
        })
        .collect();
    // Highest cost, then nearest, then stable observed id.
    by_cost.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut by_distance: Vec<(f64, u32, UnitId)> =
        by_cost.iter().map(|&(_, d, id, u)| (d, id, u)).collect();
    by_distance.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut areas: Vec<(f64, ContactId)> = if enemy_in_reach(ctx, unit, mount, spec) {
        Vec::new()
    } else {
        knowledge
            .all_contacts()
            .iter()
            .map(|c| ((c.center - here).length(), c.id))
            .collect()
    };
    areas.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

    let damages = |k: usize, t: Target| match t {
        Target::Unit(u) => can_damage(
            &weapons[spec.kinds[k]].def,
            units[u.0 as usize].armor(ctx.rules),
        ),
        _ => true,
    };
    let stages = by_cost
        .iter()
        .map(|&(.., u)| (Target::Unit(u), true))
        .chain(by_distance.iter().map(|&(.., u)| (Target::Unit(u), false)))
        .chain(areas.iter().map(|&(_, c)| (Target::Contact(c), true)));
    let mut reason = None;
    for (target, must_damage) in stages {
        // An area centred off the map has no ground to aim at.
        let Some(r) = resolve(ctx, unit.side, target, units) else {
            continue;
        };
        match assessed.of(target, || assess(ctx, unit, units, mount, spec, target, &r)) {
            Ok(k) => {
                // The middle stage is only the unlimited default gun against
                // what it cannot hurt.
                let hurts = damages(k, target);
                if hurts == must_damage && (hurts || fires_regardless(&weapons[spec.kinds[k]].def))
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

/// Whether the mount has a kind that can damage an explicit target (W08).
fn compatible(
    ctx: &FireContext,
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    units: &[Unit],
) -> bool {
    let armor = match target {
        Target::Unit(u) => Some(units[u.0 as usize].armor(ctx.rules).copied()),
        _ => None,
    };
    (0..spec.kinds.len())
        .any(|k| mount.has_rounds(k) && effective(&ctx.arsenal.weapons[spec.kinds[k]].def, armor))
}

/// Keep, replace or choose the mount's lock (W07, W08, V12). Returns the
/// reason to show if it ends with no lock.
fn choose_lock(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    spec: &MountSpec,
    explicit: Option<Target>,
    assessed: &mut Assessed,
) -> ActionReason {
    if let Some(t) = explicit {
        match mount.lock.as_mut() {
            Some(lock) if lock.target == t => lock.explicit = true,
            // A player order replaces immediately.
            _ => {
                mount.lock = Some(Lock {
                    target: t,
                    aim: 0.0,
                    explicit: true,
                    engaging: false,
                })
            }
        }
        return ActionReason::NoCompatibleTarget;
    }
    // The order ended: its lock becomes an ordinary one; a ground point never
    // outlives the order that named it.
    if let Some(lock) = mount.lock.as_mut() {
        lock.explicit = false;
        if matches!(lock.target, Target::Ground(_)) {
            mount.lock = None;
        }
    }
    // An area is never kept on its own once an identified enemy is in reach.
    if mount
        .lock
        .as_ref()
        .is_some_and(|l| matches!(l.target, Target::Contact(_)))
        && enemy_in_reach(ctx, unit, mount, spec)
    {
        mount.lock = None;
    }
    let current = mount
        .lock
        .as_ref()
        .and_then(|l| resolve(ctx, unit.side, l.target, units).map(|r| (l.target, r)));
    let reconsider = match &current {
        None => true,
        // Identification lapsing within the grace never makes it replaceable.
        Some((_, r)) if !r.current => false,
        Some((t, r)) => match assessed.of(*t, || assess(ctx, unit, units, mount, spec, *t, r)) {
            // An invalid target can be replaced early.
            Err(_) => true,
            // Independent rifles need not all be empty to reconsider a target.
            Ok(_) => mount
                .cycles
                .iter()
                .any(|c| c.cooldown > 0.0 || c.reload.is_some()),
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
                aim: 0.0,
                explicit: false,
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

/// What a unit's weapons concluded this tick, read by next tick's movement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reach {
    /// Some mount could shoot a target it can hurt from here (attack-move halts, W16).
    pub can_engage: bool,
    /// The attack order's target is out of reach of every mount that can hurt
    /// it (the attack pursues, W17).
    pub needs_closer: bool,
}

/// Advance every mount one tick; returns the shots fired.
pub fn advance(ctx: &FireContext, units: &mut [Unit], moved: &[bool], rng: &mut Rng) -> Vec<Shot> {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let turret_rate = ctx.rules.movement.turret_turn_deg_s.to_radians() * dt;
    let tolerance = ctx.rules.movement.bearing_tolerance_deg.to_radians();
    let mut shots = Vec::new();
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let specs = ctx.arsenal.specs(units[i].kind);
        // Entering or leaving a building suspends every weapon (contracts).
        if units[i]
            .garrison
            .as_ref()
            .is_some_and(|g| g.phase != crate::garrison::Phase::Inside)
        {
            for mount in &mut units[i].mounts {
                mount.reason = ActionReason::ChangingPosition;
                if let Some(lock) = mount.lock.as_mut() {
                    lock.engaging = false;
                }
            }
            units[i].reach = Reach::default();
            continue;
        }
        let ordered = units[i].attack_target();
        let mut can_engage = false;
        let mut leaned: Vec<u32> = Vec::new();
        // Ordered target: some compatible mount can shoot it / none can reach it.
        let (mut ordered_ok, mut ordered_far) = (false, false);
        for m in 0..units[i].mounts.len() {
            let unit = &units[i];
            let spec = &specs[unit.mounts[m].spec];
            let mut mount = unit.mounts[m].clone();
            let stationary = spec
                .kinds
                .iter()
                .any(|&k| ctx.arsenal.weapons[k].def.stationary);
            if spec.squad {
                mount.cycles.retain(|c| {
                    unit.members
                        .iter()
                        .enumerate()
                        .any(|(k, s)| carries(unit, spec, k) && c.owner == Some(s.id))
                });
                for k in participants(unit, &mount, spec) {
                    let owner = Some(unit.members[k].id);
                    if !mount.cycles.iter().any(|c| c.owner == owner) {
                        mount
                            .cycles
                            .push(Cycle::new(owner, spec, &ctx.arsenal.weapons));
                    }
                }
            }
            // Without an operator a transferable gun remains a spare.
            if unit.hull.is_none() && participants(unit, &mount, spec).next().is_none() {
                // A recoverable spare keeps its magazine and reload progress.
                // Moving a stationary gun still interrupts its reload.
                if spec.special && !(stationary && moved[i]) {
                    mount.lock = None;
                    mount.support = None;
                    for c in &mut mount.cycles {
                        c.started = false;
                    }
                } else {
                    mount.stop();
                }
                mount.reason = if mount.out_of_ammo() {
                    ActionReason::OutOfAmmo
                } else {
                    ActionReason::NoCompatibleTarget
                };
                units[i].mounts[m] = mount;
                continue;
            }
            let explicit = ordered.filter(|&t| compatible(ctx, &mount, spec, t, units));
            let mut assessed = Assessed::default();
            let idle_reason =
                choose_lock(ctx, unit, units, &mut mount, spec, explicit, &mut assessed);
            let resolved = mount
                .lock
                .as_ref()
                .and_then(|l| resolve(ctx, unit.side, l.target, units))
                .map(|r| match mount.lock.as_ref().map(|l| l.target) {
                    Some(Target::Ground(p)) => Resolved {
                        point: facade(ctx, unit, &mount, spec, p),
                        ..r
                    },
                    _ => r,
                });
            if resolved.is_none() {
                mount.lock = None;
            }
            // Could shoot now, from here, and to effect?
            let assessment = mount.lock.as_ref().zip(resolved.as_ref()).map(|(l, r)| {
                // A ground point's resolved point was moved to its facade
                // since the lock was chosen: assessed afresh.
                let a = if r.current && !matches!(l.target, Target::Ground(_)) {
                    assessed.of(l.target, || {
                        assess(ctx, unit, units, &mount, spec, l.target, r)
                    })
                } else if r.current {
                    assess(ctx, unit, units, &mount, spec, l.target, r)
                } else {
                    Err(ActionReason::TrackingLastSighting)
                };
                if let Ok(k) = a {
                    can_engage |= effective(&ctx.arsenal.weapons[spec.kinds[k]].def, r.armor);
                }
                if explicit.is_some() {
                    ordered_ok |= a.is_ok();
                    ordered_far |= matches!(
                        a,
                        Err(ActionReason::OutOfRange | ActionReason::BlockedTrajectory)
                    );
                }
                a
            });

            // Movement clears a stationary weapon's aim and unfinished reload (W03).
            if stationary && moved[i] {
                if let Some(lock) = mount.lock.as_mut() {
                    lock.aim = 0.0;
                    lock.engaging = false;
                }
                for c in &mut mount.cycles {
                    c.reload = None;
                }
                mount.reason = ActionReason::MovingStationaryWeapon;
                units[i].mounts[m] = mount;
                continue;
            }

            // Aim advances on the lock, even toward a last sighting in the grace.
            let kind_for_target = resolved
                .as_ref()
                .and_then(|r| preferred_kind(ctx, &mount, spec, r));
            if let (Some(lock), Some(k)) = (mount.lock.as_mut(), kind_for_target) {
                let aim_s = ctx.arsenal.weapons[spec.kinds[k]].def.aim_s;
                lock.aim = (lock.aim + dt).min(aim_s);
            }
            // Suppression's tier slows the cycle without resetting its progress (P14).
            let rate = 1.0
                - ctx
                    .rules
                    .suppression
                    .penalties(unit.suppression)
                    .map_or(0.0, |t| t.reload_cycle_penalty);
            for cycle in &mut mount.cycles {
                cycle.advance(
                    &ctx.arsenal.weapons,
                    &mount.ammo,
                    spec,
                    kind_for_target,
                    dt * rate,
                );
            }

            // Turrets traverse; hand weapons point at once.
            if let Some(r) = &resolved {
                let desired = bearing_from(unit, r.point);
                mount.bearing = if spec.turret {
                    let err = wrap_angle(desired - mount.bearing);
                    mount.bearing + err.clamp(-turret_rate, turret_rate)
                } else {
                    desired
                };
            }

            // Fire when everything lines up.
            let mut engaging = false;
            mount.reason = match (&mount.lock, &resolved, assessment) {
                (Some(lock), Some(r), Some(Ok(k))) => {
                    let target = lock.target;
                    engaging = true;
                    if lock.aim < ctx.arsenal.weapons[spec.kinds[k]].def.aim_s {
                        ActionReason::Aiming
                    } else if !mount.cycles.iter().any(|c| c.loaded == Some(k)) {
                        ActionReason::Reloading
                    } else if spec.turret
                        && wrap_angle(bearing_from(unit, r.point) - mount.bearing).abs() > tolerance
                    {
                        ActionReason::TurretTraversing
                    } else if mount.support.is_some() {
                        // One missile guided at a time: the next waits, loaded and aimed.
                        ActionReason::Guiding
                    } else if unit.garrisoned()
                        && !participants(unit, &mount, spec)
                            .any(|k| crate::garrison::faces(unit, k, r.point, ctx.rules, ctx.tick))
                    {
                        // No shooter at a window facing it yet: wait, never
                        // fire through the squad's own shell.
                        engaging = false;
                        if participants(unit, &mount, spec)
                            .any(|k| crate::garrison::changing_window(unit, k, ctx.rules, ctx.tick))
                        {
                            ActionReason::ChangingPosition
                        } else {
                            ActionReason::NoFacingSlot
                        }
                    } else {
                        let moving = moved[i];
                        shots.extend(fire(
                            ctx,
                            unit,
                            units,
                            &mut mount,
                            m,
                            spec,
                            k,
                            target,
                            r,
                            moving,
                            rng,
                            &mut leaned,
                        ));
                        ActionReason::Firing
                    }
                }
                (_, _, Some(Err(e))) => e,
                _ if mount.out_of_ammo() => ActionReason::OutOfAmmo,
                _ => idle_reason,
            };
            if !engaging {
                for c in &mut mount.cycles {
                    c.started = false;
                }
            }
            if let Some(lock) = mount.lock.as_mut() {
                lock.engaging = engaging;
            }
            units[i].mounts[m] = mount;
        }
        // Who fired from his lean point stays out on it a while; after a
        // burst out he tucks back in for a spell.
        let ticks = |s: f64| (s * ctx.rules.tick_hz as f64).round() as u64;
        let c = &ctx.rules.cover;
        for s in units[i].members.iter_mut() {
            if !leaned.contains(&s.id) {
                continue;
            }
            if ctx.tick >= s.leaning_until {
                // A new stretch out: a burst at most, then the spell in.
                s.lean_since = ctx.tick;
                s.tucked_until = ctx.tick + ticks(c.lean_burst_s) + ticks(c.lean_tuck_s);
            }
            s.leaning_until =
                (ctx.tick + ticks(c.lean_hold_s)).min(s.lean_since + ticks(c.lean_burst_s));
        }
        units[i].reach = Reach {
            can_engage,
            needs_closer: ordered_far && !ordered_ok,
        };
    }
    shots
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
    k: usize,
    target: Target,
    r: &Resolved,
    moving: bool,
    rng: &mut Rng,
    leaned: &mut Vec<u32>,
) -> Option<Shot> {
    let weapon_index = spec.kinds[k];
    let weapon = &ctx.arsenal.weapons[weapon_index];
    let mut scatter = weapon.profile.scatter_mrad
        * ctx
            .rules
            .suppression
            .penalties(unit.suppression)
            .map_or(1.0, |t| t.scatter_multiplier);
    if moving {
        scatter *= ctx.rules.physics.moving_scatter_multiplier;
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
                    unit.members[k].position + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m),
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
        if cycle.loaded != Some(k) || cycle.cooldown > 0.0 || mount.ammo[k] == Some(0) {
            continue;
        }
        let point = match target {
            Target::Contact(c) => {
                let contact = knowledge.contact(c)?;
                let radius = contact.radius * rng.unit().sqrt();
                let angle = std::f64::consts::TAU * rng.unit();
                let p = contact.center + v2(angle.cos(), angle.sin()) * radius;
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
        let (point, from) = match member {
            Some(k) => {
                let soldier = &unit.members[k];
                let turn = (0..seen.len()).map(|j| seen[(n + j) % seen.len()]);
                let found = turn
                    .chain([point])
                    .find_map(|p| Some((p, fire_from(ctx, &blockers, soldier, p)?)));
                match found {
                    Some(f) => f,
                    None if hides_behind(ctx, soldier, origin, point)
                        || blockers.iter().any(|h| h.meets(origin, point)) =>
                    {
                        cycle.started = false;
                        continue;
                    }
                    None => (point, standing),
                }
            }
            None => (point, standing),
        };
        let origin = from.origin;
        let aim = Aim {
            origin,
            target: point,
            target_velocity: r.velocity,
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
        let cover = from.past;
        let shooter = Some(Shooter {
            unit: unit.id,
            body,
            cover: cover.map(crate::flight::Struck::Prop).or_else(|| {
                from.hull
                    .map(|u| crate::flight::Struck::Body(BodyId(VEHICLE_BODY_BASE + u.0)))
            }),
        });
        let Ok((intended, _)) = solve(ctx, weapon, mount.ammo[k], &aim, target, cover) else {
            continue;
        };
        if !cycle.started {
            cycle.started = true;
            if let Some(burst) = weapon.def.magazine.and_then(|m| m.burst) {
                cycle.cooldown = rng.unit() * burst.aim_max_s;
                if cycle.cooldown > 0.0 {
                    continue;
                }
            } else if spec.squad {
                cycle.cooldown = weapon
                    .def
                    .magazine
                    .map_or(weapon.def.reload_s, |m| m.shot_interval_s)
                    * ((body.0.wrapping_mul(2654435761) >> 16) as f64 / 65536.0);
                continue;
            }
        }
        if let Ok((launch, _)) = launch_along(
            ctx.world,
            &ctx.arsenal.config,
            &weapon.profile,
            &aim,
            &intended,
            scatter,
            rng,
            shooter,
        ) {
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
    mount.elevation = last.z.atan2(last.x.hypot(last.y));
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
) -> Option<FirePoint> {
    let muzzle = |p: crate::math::V2| {
        p.with_z(ctx.world.height_at(p.x, p.y).unwrap_or(soldier.position.z))
            + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m)
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
    let standing = soldier.position + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m);
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

/// Members taking part in a mount's shot: every living soldier carrying a
/// squad weapon; otherwise its [`operator`].
fn participants<'a>(
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
pub type MountAims = Vec<(Vec<usize>, V3)>;

/// For each garrisoned squad (by index), its [`MountAims`]: what garrison
/// slot allocation turns toward. A mount faces its lock; without one, the
/// threat it would take on: the costliest enemy the side identifies that one
/// of its loaded kinds can damage (then nearest, then observed id), in range
/// or not, fire held or not. So the ATGM gunner watches the tank and the
/// riflemen the infantry before either is in reach.
pub fn garrison_aims(ctx: &FireContext, units: &[Unit]) -> Vec<(usize, MountAims)> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.alive() && u.garrisoned())
        .map(|(i, u)| {
            let specs = ctx.arsenal.specs(u.kind);
            let here = u.position.xy();
            let knowledge = &ctx.knowledge[u.side.index()];
            let aims = u
                .mounts
                .iter()
                .filter_map(|m| {
                    let spec = &specs[m.spec];
                    let target = match m.lock.as_ref() {
                        Some(lock) => lock.target,
                        None => knowledge
                            .identified_now(ctx.tick)
                            .filter(|(e, _)| compatible(ctx, m, spec, Target::Unit(*e), units))
                            .map(|(e, t)| {
                                let cost = units[e.0 as usize].unit_type(ctx.rules).cost;
                                (cost, (t.position.xy() - here).length(), t.id.0, e)
                            })
                            .min_by(|a, b| {
                                b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2))
                            })
                            .map(|(.., e)| Target::Unit(e))?,
                    };
                    let r = resolve(ctx, u.side, target, units)?;
                    Some((participants(u, m, spec).collect(), r.point))
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
    let loaded = mount.cycles.iter().find_map(|c| c.loaded);
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
    let aim = mount.lock.as_ref().map_or(0.0, |l| {
        let k = loaded.or(reloading.map(|(k, _)| k)).unwrap_or(0);
        (l.aim / arsenal.weapons[spec.kinds[k]].def.aim_s).min(1.0)
    });
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
