//! Weapon mounts: each mount owns one target lock, one aim and one reload
//! (W01–W02), chooses targets only from its side's knowledge (W06, W10), and
//! fires only through the flight module's launch path.
use contract::command::{Engagement, TargetRef};
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ActionReason, ContactId, MountReadiness};
use contract::scenario::{HealthRules, Rules, UnitKind};
use contract::weapons::{AmmoCapacity, WeaponDefinition};

use crate::digest::Digest;
use crate::flight::{
    predicted_path, prepare_launch, solve_launch, Aim, BodyId, FiringSolution, FlightConfig,
    Launch, LaunchProfile, NoSolution, ProjectileId, Shooter,
};
use crate::knowledge::SideKnowledge;
use crate::math::{v2, v3, wrap_angle, V3};
use crate::rng::Rng;
use crate::units::Unit;
use crate::world::WorldGeometry;

/// Height above a soldier's feet that rounds aim at.
pub const SOLDIER_AIM_M: f64 = 1.0;
/// Vehicle hull bodies take ids above every soldier id.
pub const VEHICLE_BODY_BASE: u32 = 1 << 24;

/// A weapon row with its validated flight profile.
pub struct Weapon {
    pub name: String,
    pub def: WeaponDefinition,
    pub profile: LaunchProfile,
}

/// An authored mount: ammunition kinds (weapon indices) sharing aim/reload.
#[derive(Clone)]
pub struct MountSpec {
    pub kinds: Vec<usize>,
    pub squad: bool,
    pub turret: bool,
}

/// Every weapon and mount in the battle, fixed at setup.
pub struct Arsenal {
    pub weapons: Vec<Weapon>,
    mounts: Vec<(UnitKind, Vec<MountSpec>)>,
    pub config: FlightConfig,
}

impl Arsenal {
    pub fn new(rules: &Rules) -> Self {
        let config = FlightConfig::new(&rules.bodies.flight, rules.tick_hz)
            .expect("fixture flight rules are valid");
        let weapons: Vec<Weapon> = rules
            .weapons
            .iter()
            .map(|(name, def)| Weapon {
                name: name.clone(),
                profile: config
                    .profile(&def.ballistics)
                    .expect("fixture weapon rows are valid"),
                def: def.clone(),
            })
            .collect();
        let index = |name: &str| {
            weapons
                .iter()
                .position(|w| w.name == name)
                .expect("mount names a weapon row")
        };
        let mounts = crate::publication::UNIT_KINDS
            .into_iter()
            .map(|kind| {
                let key = serde_json::to_value(kind).expect("unit kinds serialize");
                let specs = rules
                    .mounts
                    .get(key.as_str().expect("unit kinds are strings"))
                    .map(|list| {
                        list.iter()
                            .map(|m| {
                                assert!(
                                m.weapons.len() <= crate::publication::MAX_AMMO_KINDS,
                                "mount {} has more ammunition kinds than the publication carries",
                                m.name
                            );
                                MountSpec {
                                    kinds: m.weapons.iter().map(|w| index(w)).collect(),
                                    squad: m.squad,
                                    turret: m.turret,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                (kind, specs)
            })
            .collect();
        Arsenal {
            weapons,
            mounts,
            config,
        }
    }

    pub fn specs(&self, kind: UnitKind) -> &[MountSpec] {
        &self.mounts.iter().find(|(k, _)| *k == kind).unwrap().1
    }

    /// Fresh mounts for a unit of `kind` facing `yaw`, loaded and aimed nowhere.
    pub fn mounts_for(&self, kind: UnitKind, yaw: f64) -> Vec<Mount> {
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
                // The first kind with rounds starts loaded.
                loaded: spec
                    .kinds
                    .iter()
                    .position(|&k| !matches!(self.weapons[k].def.ammo, AmmoCapacity::Rounds(0))),
                reload: None,
                lock: None,
                support: None,
                bearing: yaw,
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
    /// Rounds left per kind, including a loaded one; `None` is unlimited.
    pub ammo: Vec<Option<u32>>,
    /// Index into the spec's kinds.
    pub loaded: Option<usize>,
    /// Kind being loaded and seconds of progress.
    pub reload: Option<(usize, f64)>,
    pub lock: Option<Lock>,
    /// The missile this mount is guiding, if any (one at a time).
    pub support: Option<Support>,
    /// World heading of a turret (or the last aim of a hand weapon).
    pub bearing: f64,
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
        d.u64(self.loaded.map_or(u64::MAX, |k| k as u64))
            .f64(self.bearing)
            .u64(self.reason as u64);
        d.u64(self.reload.is_some() as u64);
        if let Some((k, p)) = self.reload {
            d.u64(k as u64).f64(p);
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
        self.reload = None;
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
    /// Target kind if the side knows it (identified units only).
    kind: Option<UnitKind>,
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
    pub arsenal: &'a Arsenal,
    pub rules: &'a Rules,
    pub tick: Tick,
    pub knowledge: &'a [SideKnowledge; 2],
}

/// Whether a weapon can hurt a unit of `kind` at all (P10: penetration beats
/// the weakest face; dedicated anti-armour never engages infantry).
pub fn can_damage(def: &WeaponDefinition, kind: UnitKind, health: &HealthRules) -> bool {
    match kind {
        UnitKind::Tank => def.penetration > health.tank_armor.weakest() || def.armor_fraction > 0.0,
        UnitKind::Supply => {
            def.penetration > health.supply_armor.weakest() || def.armor_fraction > 0.0
        }
        _ => !def.anti_armor,
    }
}

/// General-purpose ammunition: may fire at an area (never AP or dedicated anti-armour, W10).
fn area_capable(def: &WeaponDefinition) -> bool {
    !def.anti_armor && !def.armor_piercing
}

/// Whether `def` is effective against a target of this known kind, or an area.
fn effective(def: &WeaponDefinition, kind: Option<UnitKind>, health: &HealthRules) -> bool {
    match kind {
        Some(kind) => can_damage(def, kind, health),
        None => area_capable(def),
    }
}

fn resolve(ctx: &FireContext, side: Side, target: Target, units: &[Unit]) -> Option<Resolved> {
    let knowledge = &ctx.knowledge[side.index()];
    match target {
        Target::Unit(u) => {
            let track = knowledge.track(u)?;
            let unit = &units[u.0 as usize];
            let height = unit.hull.map_or(SOLDIER_AIM_M, |h| h.z);
            Some(Resolved {
                point: track.position + v3(0.0, 0.0, height),
                velocity: track.velocity.with_z(0.0),
                current: track.last_seen == ctx.tick,
                kind: Some(unit.kind),
            })
        }
        Target::Contact(c) => {
            let contact = knowledge.contact(c)?;
            let z = ctx.world.height_at(contact.center.x, contact.center.y)?;
            Some(Resolved {
                point: contact.center.with_z(z + SOLDIER_AIM_M),
                velocity: v3(0.0, 0.0, 0.0),
                current: true,
                kind: None,
            })
        }
        Target::Ground(p) => Some(Resolved {
            point: p,
            velocity: v3(0.0, 0.0, 0.0),
            current: true,
            kind: None,
        }),
    }
}

/// The mount's preferred kind for a target: armour-piercing for identified
/// vehicles, otherwise non-AP; the default gun also against what it cannot
/// hurt (W09); AP never at an area (W10).
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
            && (effective(def, resolved.kind, &ctx.rules.health)
                || (resolved.kind.is_some() && def.default))
    };
    let n = spec.kinds.len();
    let first = |want_ap: Option<bool>| (0..n).find(|&k| usable(k, want_ap));
    match resolved.kind {
        Some(kind) if !kind.is_infantry() => first(Some(true)).or_else(|| first(None)),
        _ => first(Some(false)).or_else(|| first(None)),
    }
}

/// A ground point inside a building is aimed at the wall facing the shooter:
/// striking that wall is the attack (structural damage), not an obstruction.
fn facade(ctx: &FireContext, unit: &Unit, point: V3) -> V3 {
    let origin = muzzle(unit, ctx.rules, bearing_from(unit, point));
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
        Some(prop)
            if prop.kind == contract::map::PropKind::Building
                && prop.footprint().contains(point.xy(), 0.0) =>
        {
            hit.point - dir * 0.05
        }
        _ => point,
    }
}

fn bearing_from(unit: &Unit, point: V3) -> f64 {
    let to = point.xy() - unit.position.xy();
    to.y.atan2(to.x)
}

fn muzzle(unit: &Unit, rules: &Rules, bearing: f64) -> V3 {
    let b = &rules.bodies;
    match unit.hull {
        Some(_) => {
            let m = b.tank_muzzle_local_m;
            unit.position + v2(m[0], m[1]).rotated(bearing).with_z(m[2])
        }
        None => unit.position + v3(0.0, 0.0, b.infantry_muzzle_m),
    }
}

/// P11: withhold when a friendly vehicle sits on the predicted path or in the
/// blast. Friendly infantry never withholds a shot (it can still be hit).
fn friendly_in_line(
    ctx: &FireContext,
    shooter: &Unit,
    units: &[Unit],
    origin: V3,
    s: &FiringSolution,
    weapon: &Weapon,
) -> bool {
    let margin = ctx.rules.bodies.friendly_prefire_margin_m;
    let (def, profile) = (&weapon.def, &weapon.profile);
    let path = predicted_path(
        &ctx.arsenal.config,
        profile.gravity(&ctx.arsenal.config),
        origin,
        s.velocity,
        s.time_of_flight_s,
    );
    units
        .iter()
        .filter(|u| u.side == shooter.side && u.id != shooter.id && u.hull.is_some() && u.alive())
        .any(|u| {
            path.windows(2).any(|w| {
                // Sample each chord densely enough for a hull-sized margin.
                let n = ((w[1] - w[0]).length() / 1.0).ceil().max(1.0) as usize;
                (0..=n)
                    .any(|k| u.hull_distance(w[0] + (w[1] - w[0]) * (k as f64 / n as f64)) < margin)
            }) || (def.blast_radius_m > 0.0 && u.hull_distance(s.intercept) < def.blast_radius_m)
        })
}

/// Can this mount fire the given kind at the resolved point from here? The
/// solution, or why not.
fn engage(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    k: usize,
    r: &Resolved,
) -> Result<FiringSolution, ActionReason> {
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]];
    let bearing = if spec.turret {
        bearing_from(unit, r.point)
    } else {
        mount.bearing
    };
    // A garrisoned squad fires from the building's slot facing the target.
    let origin = if unit.garrisoned() {
        crate::garrison::facing_origin(unit, r.point, ctx.rules)
            .ok_or(ActionReason::NoFacingSlot)?
            + v3(0.0, 0.0, ctx.rules.bodies.infantry_muzzle_m)
    } else {
        muzzle(unit, ctx.rules, bearing)
    };
    if (r.point - origin).length() > weapon.def.range_m {
        return Err(ActionReason::OutOfRange);
    }
    let aim = Aim {
        origin,
        target: r.point,
        target_velocity: r.velocity,
    };
    match solve_launch(ctx.world, &ctx.arsenal.config, &weapon.profile, &aim) {
        Err(NoSolution::OutOfReach) => Err(ActionReason::OutOfRange),
        Err(NoSolution::Blocked { .. }) => Err(ActionReason::BlockedTrajectory),
        Ok(s) if friendly_in_line(ctx, unit, units, origin, &s, weapon) => {
            Err(ActionReason::FriendlyInLine)
        }
        Ok(s) => Ok(s),
    }
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
    engage(ctx, unit, units, mount, spec, k, r).map(|_| k)
}

/// Automatic choice (W06, W09, W10), in order: the highest-cost identified
/// target some kind can damage; the nearest area for general-purpose kinds;
/// for the default gun, the nearest identified target it cannot hurt. Only a
/// target the mount could shoot now qualifies; otherwise the first obstacle met
/// is the reason.
fn select(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
) -> (Option<Target>, ActionReason) {
    let knowledge = &ctx.knowledge[unit.side.index()];
    let here = unit.position.xy();
    let weapons = &ctx.arsenal.weapons;
    let health = &ctx.rules.health;
    let mut by_cost: Vec<(u32, f64, u32, UnitId)> = knowledge
        .identified_now(ctx.tick)
        .map(|(u, t)| {
            let cost = crate::units::cost(units[u.0 as usize].kind, ctx.rules);
            (cost, (t.position.xy() - here).length(), t.id.0, u)
        })
        .collect();
    // Highest cost, then nearest, then stable observed id.
    by_cost.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut by_distance: Vec<(f64, u32, UnitId)> =
        by_cost.iter().map(|&(_, d, id, u)| (d, id, u)).collect();
    by_distance.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut areas: Vec<(f64, ContactId)> = knowledge
        .all_contacts()
        .iter()
        .map(|c| ((c.center - here).length(), c.id))
        .collect();
    areas.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

    let damages = |k: usize, t: Target| match t {
        Target::Unit(u) => can_damage(
            &weapons[spec.kinds[k]].def,
            units[u.0 as usize].kind,
            health,
        ),
        _ => true,
    };
    let stages = by_cost
        .iter()
        .map(|&(.., u)| (Target::Unit(u), true))
        .chain(areas.iter().map(|&(_, c)| (Target::Contact(c), true)))
        .chain(by_distance.iter().map(|&(.., u)| (Target::Unit(u), false)));
    let mut reason = None;
    for (target, must_damage) in stages {
        // An area centred off the map has no ground to aim at.
        let Some(r) = resolve(ctx, unit.side, target, units) else {
            continue;
        };
        match assess(ctx, unit, units, mount, spec, target, &r) {
            Ok(k) => {
                // The last stage is only the default gun against what it cannot hurt.
                let hurts = damages(k, target);
                if hurts == must_damage && (hurts || weapons[spec.kinds[k]].def.default) {
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
    let kind = match target {
        Target::Unit(u) => Some(units[u.0 as usize].kind),
        _ => None,
    };
    (0..spec.kinds.len()).any(|k| {
        mount.has_rounds(k)
            && effective(
                &ctx.arsenal.weapons[spec.kinds[k]].def,
                kind,
                &ctx.rules.health,
            )
    })
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
    let current = mount
        .lock
        .as_ref()
        .and_then(|l| resolve(ctx, unit.side, l.target, units).map(|r| (l.target, r)));
    let reconsider = match &current {
        None => true,
        // Identification lapsing within the grace never makes it replaceable.
        Some((_, r)) if !r.current => false,
        Some((t, r)) => match assess(ctx, unit, units, mount, spec, *t, r) {
            // An invalid target can be replaced early.
            Err(_) => true,
            // A valid one is held through its pending shot, then reconsidered
            // while reloading.
            Ok(_) => mount.loaded.is_none() && mount.reload.is_some(),
        },
    };
    if !reconsider {
        return ActionReason::NoCompatibleTarget;
    }
    let (choice, reason) = select(ctx, unit, units, mount, spec);
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
    let health = &ctx.rules.health;
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
        // Ordered target: some compatible mount can shoot it / none can reach it.
        let (mut ordered_ok, mut ordered_far) = (false, false);
        for m in 0..units[i].mounts.len() {
            let unit = &units[i];
            let spec = &specs[unit.mounts[m].spec];
            let mut mount = unit.mounts[m].clone();
            let explicit = ordered.filter(|&t| compatible(ctx, &mount, spec, t, units));
            let idle_reason = choose_lock(ctx, unit, units, &mut mount, spec, explicit);
            let resolved = mount
                .lock
                .as_ref()
                .and_then(|l| resolve(ctx, unit.side, l.target, units))
                .map(|r| match mount.lock.as_ref().map(|l| l.target) {
                    Some(Target::Ground(p)) => Resolved {
                        point: facade(ctx, unit, p),
                        ..r
                    },
                    _ => r,
                });
            if resolved.is_none() {
                mount.lock = None;
            }
            // Could shoot now, from here, and to effect?
            let assessment = mount.lock.as_ref().zip(resolved.as_ref()).map(|(l, r)| {
                let a = if r.current {
                    assess(ctx, unit, units, &mount, spec, l.target, r)
                } else {
                    Err(ActionReason::TrackingLastSighting)
                };
                if let Ok(k) = a {
                    can_engage |=
                        effective(&ctx.arsenal.weapons[spec.kinds[k]].def, r.kind, health);
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
            let stationary = spec
                .kinds
                .iter()
                .any(|&k| ctx.arsenal.weapons[k].def.stationary);
            if stationary && moved[i] {
                if let Some(lock) = mount.lock.as_mut() {
                    lock.aim = 0.0;
                    lock.engaging = false;
                }
                mount.reload = None;
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
            // Suppression slows the cycle without resetting its progress (P14).
            let rate =
                (1.0 - ctx.rules.suppression.max_reload_cycle_penalty * unit.suppression).max(0.0);
            reload(ctx, &mut mount, spec, kind_for_target, dt * rate);

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
                    } else if mount.loaded != Some(k) {
                        ActionReason::Reloading
                    } else if spec.turret
                        && wrap_angle(bearing_from(unit, r.point) - mount.bearing).abs() > tolerance
                    {
                        ActionReason::TurretTraversing
                    } else if mount.support.is_some() {
                        // One missile guided at a time: the next waits, loaded and aimed.
                        ActionReason::Guiding
                    } else if unit.garrisoned()
                        && !participants(unit, spec.squad)
                            .any(|k| crate::garrison::faces(unit, k, r.point, ctx.rules))
                    {
                        // Target-facing slots are all taken: wait, never
                        // fire through the squad's own shell.
                        engaging = false;
                        ActionReason::NoFacingSlot
                    } else {
                        let moving = moved[i];
                        shots.extend(fire(
                            ctx, unit, units, &mut mount, m, spec, k, target, r, moving, rng,
                        ));
                        ActionReason::Firing
                    }
                }
                (_, _, Some(Err(e))) => e,
                _ if mount.out_of_ammo() => ActionReason::OutOfAmmo,
                _ => idle_reason,
            };
            if let Some(lock) = mount.lock.as_mut() {
                lock.engaging = engaging;
            }
            units[i].mounts[m] = mount;
        }
        units[i].reach = Reach {
            can_engage,
            needs_closer: ordered_far && !ordered_ok,
        };
    }
    shots
}

/// Keep a loaded round that suits the target, or set it aside (the total
/// still counts it) and load the right kind; a different kind restarts the
/// reload from zero (W05).
fn reload(ctx: &FireContext, mount: &mut Mount, spec: &MountSpec, want: Option<usize>, dt: f64) {
    if let (Some(l), Some(p)) = (mount.loaded, want) {
        if l != p {
            mount.loaded = None;
            mount.reload = None;
        }
    }
    if mount.loaded.is_some() {
        return;
    }
    let want = want
        .or(mount.reload.map(|(k, _)| k))
        .or_else(|| (0..spec.kinds.len()).find(|&k| mount.has_rounds(k)));
    mount.reload = match want {
        Some(k) if mount.has_rounds(k) => {
            let progress = match mount.reload {
                Some((rk, p)) if rk == k => p,
                _ => 0.0,
            } + dt;
            if progress >= ctx.arsenal.weapons[spec.kinds[k]].def.reload_s {
                mount.loaded = Some(k);
                None
            } else {
                Some((k, progress))
            }
        }
        _ => None,
    };
}

/// Launch the mount's rounds: one per living soldier for a squad weapon,
/// otherwise one. Spread is the row's, widened while moving (W04).
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
) -> Option<Shot> {
    let weapon_index = spec.kinds[k];
    let weapon = &ctx.arsenal.weapons[weapon_index];
    let mut scatter = weapon.def.ballistics.scatter_mrad;
    if moving {
        scatter *= ctx.rules.bodies.moving_scatter_multiplier;
    }
    let knowledge = &ctx.knowledge[unit.side.index()];
    // Rounds and their muzzles: every living soldier, or the one weapon; in a
    // building, each from its slot (the operator's for a single weapon).
    let shooters: Vec<(V3, BodyId)> = if spec.squad || unit.garrisoned() {
        participants(unit, spec.squad)
            .map(|k| {
                (
                    unit.member_position(k) + v3(0.0, 0.0, ctx.rules.bodies.infantry_muzzle_m),
                    BodyId(unit.members[k].id),
                )
            })
            .collect()
    } else {
        let body = match unit.hull {
            Some(_) => BodyId(VEHICLE_BODY_BASE + unit.id.0),
            None => BodyId(unit.members.iter().find(|s| s.alive()).map_or(0, |s| s.id)),
        };
        vec![(muzzle(unit, ctx.rules, mount.bearing), body)]
    };
    // Aim points: seen soldiers in turn, a sampled point in an area, or the point.
    let seen: Vec<V3> = match target {
        Target::Unit(u) if !units[u.0 as usize].is_vehicle() => knowledge
            .track(u)
            .map(|t| {
                t.members
                    .iter()
                    .map(|&m| units[u.0 as usize].member_position(m) + v3(0.0, 0.0, SOLDIER_AIM_M))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    // Cover comes from how the target is observed: at its building's slots.
    let shelter = match target {
        Target::Unit(u) => crate::garrison::shelter(&units[u.0 as usize], ctx.rules),
        _ => 0.0,
    };
    let mut launches = Vec::new();
    for (n, (origin, body)) in shooters.into_iter().enumerate() {
        let point = match target {
            Target::Contact(c) => {
                let contact = knowledge.contact(c)?;
                let radius = ctx.rules.sensors.contact_radius_m * rng.unit().sqrt();
                let angle = std::f64::consts::TAU * rng.unit();
                let p = contact.center + v2(angle.cos(), angle.sin()) * radius;
                p.with_z(ctx.world.height_at(p.x, p.y).unwrap_or(0.0) + SOLDIER_AIM_M)
            }
            _ if !seen.is_empty() => seen[n % seen.len()],
            _ => r.point,
        };
        // A soldier whose slot does not face this round's point holds it.
        if unit.garrisoned()
            && !crate::garrison::faces(unit, participant_of(unit, body), point, ctx.rules)
        {
            continue;
        }
        let aim = Aim {
            origin,
            target: point,
            target_velocity: r.velocity,
        };
        // Cover at the aimed point widens the spread; it never softens a hit (V03).
        let scatter = scatter * crate::damage::cover_spread(ctx.world, ctx.rules, point, shelter);
        let shooter = Some(Shooter {
            unit: unit.id,
            body,
        });
        if let Ok((launch, _)) = prepare_launch(
            ctx.world,
            &ctx.arsenal.config,
            &weapon.profile,
            &aim,
            scatter,
            rng,
            shooter,
        ) {
            launches.push(launch);
        }
    }
    if launches.is_empty() {
        return None;
    }
    if let Some(n) = mount.ammo[k].as_mut() {
        *n -= 1;
    }
    mount.loaded = None;
    Some(Shot {
        unit: unit.id,
        mount: m,
        weapon: weapon_index,
        launches,
        target,
    })
}

/// Members taking part in a mount's shot: every living soldier of a squad
/// weapon, otherwise the operator (the first living soldier).
fn participants(unit: &Unit, squad: bool) -> impl Iterator<Item = usize> + '_ {
    (0..unit.members.len())
        .filter(|&k| unit.members[k].alive())
        .take(if squad { usize::MAX } else { 1 })
}

fn participant_of(unit: &Unit, body: BodyId) -> usize {
    unit.members
        .iter()
        .position(|s| BodyId(s.id) == body)
        .expect("a shooter is a member")
}

/// For each garrisoned squad (by index), each locked mount's participation
/// (whole squad or operator) and the point its side observes it aiming at:
/// what garrison slot allocation turns toward.
pub fn garrison_aims(ctx: &FireContext, units: &[Unit]) -> Vec<(usize, Vec<(bool, V3)>)> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.alive() && u.garrisoned())
        .map(|(i, u)| {
            let specs = ctx.arsenal.specs(u.kind);
            let aims = u
                .mounts
                .iter()
                .filter_map(|m| {
                    let lock = m.lock.as_ref()?;
                    let r = resolve(ctx, u.side, lock.target, units)?;
                    Some((specs[m.spec].squad, r.point))
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

/// Exported readiness of a mount for its owner's panel and rings.
pub fn readiness(
    arsenal: &Arsenal,
    unit: &Unit,
    mount: &Mount,
    target_ref: Option<TargetRef>,
) -> MountReadiness {
    let spec = &arsenal.specs(unit.kind)[mount.spec];
    let aim = mount.lock.as_ref().map_or(0.0, |l| {
        let k = mount.loaded.or(mount.reload.map(|(k, _)| k)).unwrap_or(0);
        (l.aim / arsenal.weapons[spec.kinds[k]].def.aim_s).min(1.0)
    });
    let reload = mount.reload.map_or(0.0, |(k, p)| {
        (p / arsenal.weapons[spec.kinds[k]].def.reload_s).min(1.0)
    });
    MountReadiness {
        mount: mount.spec as u8,
        loaded: mount.loaded.map(|k| k as u8),
        reloading: mount.reload.map(|(k, _)| k as u8),
        ammo: mount.ammo.clone(),
        aim,
        reload,
        target: target_ref,
        reason: mount.reason,
        guiding: mount.support.is_some(),
    }
}
