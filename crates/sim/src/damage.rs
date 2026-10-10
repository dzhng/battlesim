//! Consequences of physical fire: what this tick's impacts and
//! near misses do to bodies. Rounds hit whatever they meet, of any side (P09);
//! armour is judged on the struck face, at the hull's pose when struck, with
//! fixed penetration (P10). During flight, [`HullResolver`] decides each hit:
//! a burst, a stop, or a kinetic round that failed to penetrate glancing off
//! by its face's chance (Q9), flying on slower and weaker. Blast is sampled per soldier, where cover lowers the
//! chance of a damaging fragment but never its damage (P13); near misses,
//! impacts and blasts suppress infantry without damage (P14). Building cover
//! is one more source: garrisoned soldiers are harder to hit and to
//! reach with fragments, never softer when hit (P12); rounds striking a
//! building report structural damage for the garrison owner to apply.
//! Every other soldier's cover is a spread rule of its own (`cover`).
use std::collections::BTreeMap;

use contract::ids::{Tick, UnitId};
use contract::scenario::{Armor, Face, RicochetRules, Rules};
use contract::weapons::WeaponDefinition;

use crate::battle::Round;
use crate::flight::{
    BodyId, FlightEvent, ImpactContext, ImpactDecision, ImpactResolver, Pose, ProjectileId, Struck,
};
use crate::ground::GroundLayer;
use crate::math::{v3, V2, V3};
use crate::units::{hull_face_at, Unit};
use crate::weapons::{Arsenal, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};
use contract::random::Rng;

/// Blast samples start this far off the struck surface.
const BLAST_LIFT_M: f64 = 0.05;

pub struct DamageContext<'a> {
    pub world: &'a WorldGeometry,
    pub ground: &'a GroundLayer,
    pub arsenal: &'a Arsenal,
    pub rules: &'a Rules,
    pub tick: Tick,
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Incoming spread multiplier for a garrison observed with building cover
/// strength `shelter` (V03, P12). Every other soldier's cover is his cover
/// body's tier (`cover`).
pub fn shelter_spread(rules: &Rules, shelter: f64) -> f64 {
    lerp(1.0, rules.cover.building_spread_multiplier, shelter)
}

/// Fragment probability multiplier for a soldier with building cover
/// strength `shelter` (P13). Cover bodies work through spread only (Q5);
/// a garrison's building is the named exception (Q22).
pub fn fragment_exposure(rules: &Rules, shelter: f64) -> f64 {
    lerp(
        1.0,
        rules.cover.building_fragment_probability_multiplier,
        shelter,
    )
}

/// Ricochet rules the simulation can honour: probabilities in [0, 1], a
/// deflected round keeps some speed and no more penetration than it had.
pub fn validate(rules: &Rules) {
    let r = &rules.ricochet;
    for hull in rules
        .catalog
        .indices()
        .filter_map(|t| rules.catalog.get(t).hull())
    {
        let c = hull.armor.ricochet;
        for p in [c.front, c.side, c.rear, c.roof] {
            assert!(
                (0.0..=1.0).contains(&p),
                "ricochet chance {p} outside [0, 1]"
            );
        }
    }
    assert!(
        r.speed_kept > 0.0 && r.speed_kept <= 1.0,
        "ricochet speed_kept {}",
        r.speed_kept
    );
    assert!(
        (0.0..=1.0).contains(&r.penetration_kept),
        "ricochet penetration_kept {}",
        r.penetration_kept
    );
    assert!(
        (0.0..90.0).contains(&r.scatter_deg),
        "ricochet scatter_deg {}",
        r.scatter_deg
    );
}

/// The face a hit meets, judged just off the struck surface at the hull's
/// pose when struck.
pub fn struck_face(half: V3, pose: Pose, point: V3, normal: V3) -> Face {
    hull_face_at(pose.base, pose.yaw, half, point + normal * BLAST_LIFT_M)
}

/// The one penetration rule: a round of `penetration` pierces `face` once
/// each ricochet so far has cut it by `penetration_kept`.
pub fn pierces(
    penetration: f64,
    bounces: u8,
    rules: &RicochetRules,
    armor: &Armor,
    face: Face,
) -> bool {
    penetration * rules.penetration_kept.powi(bounces as i32) > armor.face(face)
}

/// What armour sees of a round: its penetration before any ricochet, and
/// whether it bursts on contact.
#[derive(Clone, Copy, Debug)]
pub struct RoundPower {
    pub penetration: f64,
    pub bursts: bool,
}

/// An armoured hull as struck: its armour, half extents and pose at the hit.
#[derive(Clone, Copy, Debug)]
pub struct StruckHull<'a> {
    pub armor: &'a Armor,
    pub half: V3,
    pub pose: Pose,
}

/// What a round does where it hits: meeting an armoured `hull`, what
/// [`meet_hull`] says; anywhere else a bursting round detonates and any
/// other stops. The battle, the flight lab and the flight tests all judge
/// impacts here.
pub fn decide(
    round: RoundPower,
    hull: Option<StruckHull>,
    hit: &ImpactContext,
    rules: &RicochetRules,
    rng: &mut Rng,
) -> ImpactDecision {
    match hull {
        Some(hull) => meet_hull(round, hull, hit, rules, rng),
        None if round.bursts => ImpactDecision::Detonate,
        None => ImpactDecision::Stop,
    }
}

/// What a round meeting an armoured hull does: a bursting round detonates; a
/// kinetic round that fails to pierce the face it met glances off with that
/// face's chance while it has ricochets left, and otherwise stops.
fn meet_hull(
    round: RoundPower,
    hull: StruckHull,
    hit: &ImpactContext,
    rules: &RicochetRules,
    rng: &mut Rng,
) -> ImpactDecision {
    if round.bursts {
        return ImpactDecision::Detonate;
    }
    let face = struck_face(hull.half, hull.pose, hit.point, hit.normal);
    if hit.bounces >= rules.max_bounces
        || pierces(round.penetration, hit.bounces, rules, hull.armor, face)
        || rng.unit() >= hull.armor.ricochet.face(face)
    {
        return ImpactDecision::Stop;
    }
    ImpactDecision::Bounce {
        velocity: deflect(hit.velocity, hit.normal, rules, rng),
    }
}

/// Least angle between a deflected path and the struck surface: scatter never
/// drives a round back into the plate it glanced off.
const MIN_DEFLECTION_RAD: f64 = 0.02;

/// The mirror reflection of `velocity` off a surface with outward `normal`,
/// turned by up to `scatter_deg` (uniform over that cone's solid angle) and
/// slowed to `speed_kept`.
fn deflect(velocity: V3, normal: V3, rules: &RicochetRules, rng: &mut Rng) -> V3 {
    let speed = velocity.length();
    let mirror = (velocity - normal * (2.0 * velocity.dot(normal))) * (1.0 / speed);
    // Two unit axes across the mirror path.
    let helper = if mirror.z.abs() < 0.9 {
        v3(0.0, 0.0, 1.0)
    } else {
        v3(1.0, 0.0, 0.0)
    };
    let a = mirror.cross(helper).normalized();
    let b = mirror.cross(a);
    // The scatter evaluator must preserve the same bits on Native and Wasm.
    let cone = rules.scatter_deg.to_radians();
    let cos_t = 1.0 - rng.unit() * (1.0 - libm::cos(cone));
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    let phi = std::f64::consts::TAU * rng.unit();
    let mut dir = mirror * cos_t + (a * libm::cos(phi) + b * libm::sin(phi)) * sin_t;
    let off = dir.dot(normal);
    let least = libm::sin(MIN_DEFLECTION_RAD);
    if off < least {
        dir = (dir + normal * (least - off)).normalized();
    }
    dir * (speed * rules.speed_kept)
}

/// The battle's [`ImpactResolver`]: a round meeting a vehicle hull is judged
/// by [`decide`] on the face it met at the moment of the hit; a bursting
/// round detonates wherever it hits; everything else stops.
pub struct HullResolver<'a> {
    pub rules: &'a Rules,
    pub arsenal: &'a Arsenal,
    pub rounds: &'a BTreeMap<ProjectileId, Round>,
    pub units: &'a mut [Unit],
    pub tick: Tick,
    /// The ricochet stream: rolls and scatter.
    pub rng: &'a mut Rng,
}

impl ImpactResolver for HullResolver<'_> {
    fn max_standoff(&self, projectile: ProjectileId) -> f64 {
        let Some(round) = self.rounds.get(&projectile) else {
            return 0.0;
        };
        if !self.arsenal.weapons[round.weapon].def.interceptable {
            return 0.0;
        }
        self.units
            .iter()
            .filter(|unit| unit.alive())
            .filter_map(|unit| {
                unit.unit_type(self.rules)
                    .capabilities
                    .active_protection
                    .as_ref()
            })
            .map(|cap| cap.standoff_m)
            .fold(0.0, f64::max)
    }
    fn standoff(&self, projectile: ProjectileId, body: &crate::flight::Body) -> Option<f64> {
        let round = self.rounds.get(&projectile)?;
        if !self.arsenal.weapons[round.weapon].def.interceptable {
            return None;
        }
        let unit = self.units.get(body.unit.0 as usize)?;
        (unit.alive() && unit.hull.is_some())
            .then(|| {
                unit.unit_type(self.rules)
                    .capabilities
                    .active_protection
                    .as_ref()
            })?
            .map(|cap| cap.standoff_m)
    }
    fn intercept(&mut self, event: &crate::flight::Interception) -> bool {
        let unit = &mut self.units[event.unit.0 as usize];
        let Some(cap) = unit
            .unit_type(self.rules)
            .capabilities
            .active_protection
            .as_ref()
        else {
            return false;
        };
        let time = self.tick.saturating_sub(1) as f64 + event.time;
        unit.protection
            .as_mut()
            .is_some_and(|state| state.intercept(cap, time, self.rules.tick_hz))
    }
    fn resolve(&mut self, hit: &ImpactContext) -> ImpactDecision {
        let Some(round) = self.rounds.get(&hit.projectile) else {
            return ImpactDecision::Stop;
        };
        let def = &self.arsenal.weapons[round.weapon].def;
        let power = RoundPower {
            penetration: def.penetration,
            bursts: def.blast_radius_m > 0.0,
        };
        let hull = match (hit.struck, hit.pose) {
            (Struck::Body(b), Some(pose)) => match locate(self.units, b) {
                Some((i, None)) => {
                    let unit = &self.units[i];
                    Some(StruckHull {
                        armor: unit.armor(self.rules).expect("vehicle armour"),
                        half: unit.hull.expect("vehicle hull"),
                        pose,
                    })
                }
                _ => None,
            },
            _ => None,
        };
        decide(power, hull, hit, &self.rules.ricochet, self.rng)
    }
}

/// What happened to units this tick, for the battle to act on.
#[derive(Default)]
pub struct Outcome {
    /// Units that died this tick.
    pub destroyed: Vec<UnitDeath>,
    /// (victim, shooter): a hostile round damaged or suppressed the victim.
    pub attacked: Vec<(UnitId, UnitId)>,
    /// Structural damage rounds did to props they struck, in event order.
    pub structural: Vec<StructuralHit>,
}

/// Causal identity survives the shooter and never depends on visible contacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LethalSource {
    pub unit: UnitId,
    pub side: contract::ids::Side,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitDeath {
    pub victim: UnitId,
    pub source: Option<LethalSource>,
}

/// Structural damage to one body, and the horizontal way it pushed: a
/// round's flight, or from a burst out to the body. A toppling body this
/// destroys falls that way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructuralHit {
    pub source: Option<LethalSource>,
    pub prop: PropId,
    pub amount: f64,
    pub toward: V2,
}

/// Where a body lives: (unit index, soldier index) or a vehicle.
fn locate(units: &[Unit], body: BodyId) -> Option<(usize, Option<usize>)> {
    if body.0 >= VEHICLE_BODY_BASE {
        let i = (body.0 - VEHICLE_BODY_BASE) as usize;
        return (i < units.len()).then_some((i, None));
    }
    units.iter().enumerate().find_map(|(i, u)| {
        u.members
            .iter()
            .position(|s| s.id == body.0)
            .map(|k| (i, Some(k)))
    })
}

/// Apply this tick's flight events in order. `suppressed` carries, per round
/// in flight and squad, the strongest suppression the round has dealt it.
pub fn resolve(
    ctx: &DamageContext,
    events: &[FlightEvent],
    rounds: &BTreeMap<ProjectileId, Round>,
    suppressed: &mut BTreeMap<(ProjectileId, usize), f64>,
    units: &mut [Unit],
    rng: &mut Rng,
) -> Outcome {
    let was_alive: Vec<bool> = units.iter().map(|u| u.alive()).collect();
    let mut lethal = vec![None; units.len()];
    // A round suppresses a squad once over its whole flight, by the strongest
    // of its near misses and its impact: this tick's strongest, less what it
    // has already dealt on earlier ticks (a slow round takes several ticks to
    // pass a squad, and the eye sees one pass).
    let mut suppression: BTreeMap<(ProjectileId, usize), f64> = BTreeMap::new();
    let mut hurt: Vec<(usize, LethalSource)> = Vec::new();
    let mut structural = Vec::new();
    for event in events {
        match event {
            FlightEvent::NearMiss(n) => {
                let Some(round) = rounds.get(&n.projectile) else {
                    continue;
                };
                let def = &ctx.arsenal.weapons[round.weapon].def;
                let i = n.unit.0 as usize;
                let v = near_miss(def, n.distance);
                if units[i].hull.is_none() && v > 0.0 {
                    let e = suppression.entry((n.projectile, i)).or_insert(0.0);
                    *e = e.max(v);
                }
            }
            FlightEvent::Impact(hit) => {
                let Some(round) = rounds.get(&hit.projectile) else {
                    continue;
                };
                let def = &ctx.arsenal.weapons[round.weapon].def;
                let direct = match hit.struck {
                    Struck::Body(b) => locate(units, b).map(|at| (b, at)),
                    _ => None,
                };
                // Only weapons with structural damage wear props down (L10,
                // Q17): the struck body by its kind's armour, and every other
                // destroyable body the burst reaches by its distance.
                let struck = match hit.struck {
                    Struck::Prop(id) => ctx.world.prop(id),
                    _ => None,
                };
                if let Some(prop) = struck.filter(|_| def.structural_damage > 0.0) {
                    structural.push(StructuralHit {
                        source: Some(LethalSource {
                            unit: round.unit,
                            side: round.side,
                        }),
                        prop: prop.id,
                        amount: def.structural_damage * prop.body.armor,
                        toward: hit.velocity.xy(),
                    });
                }
                if hit.detonated {
                    let at = hit.point + hit.normal * BLAST_LIFT_M;
                    blast_props(
                        ctx.world,
                        def,
                        at,
                        struck.map(|p| p.id),
                        Some(LethalSource {
                            unit: round.unit,
                            side: round.side,
                        }),
                        &mut structural,
                    );
                }
                if let Some((_, (i, soldier))) = direct {
                    let unit = &mut units[i];
                    let damage = match (soldier, hit.pose) {
                        (None, Some(pose)) => {
                            hull_damage(ctx, def, unit, pose, hit.point, hit.normal, hit.bounces)
                        }
                        _ => def.damage,
                    };
                    if damage > 0.0 {
                        if take(unit, soldier, damage) {
                            lethal[i] = Some(LethalSource {
                                unit: round.unit,
                                side: round.side,
                            });
                        }
                        hurt.push((
                            i,
                            LethalSource {
                                unit: round.unit,
                                side: round.side,
                            },
                        ));
                    }
                }
                let at = hit.point + hit.normal * BLAST_LIFT_M;
                if hit.detonated {
                    let skip = direct.map(|(b, _)| b);
                    blast(ctx, def, at, skip, units, rng, |i, died| {
                        let source = LethalSource {
                            unit: round.unit,
                            side: round.side,
                        };
                        hurt.push((i, source));
                        if died {
                            lethal[i] = Some(source);
                        }
                    });
                }
                // Impacts suppress squads near them, whoever was aimed at; like a
                // near miss, never the unit that fired.
                for (i, unit) in units.iter().enumerate() {
                    if unit.hull.is_some() || !unit.alive() || unit.id == round.unit {
                        continue;
                    }
                    let nearest = (0..unit.members.len())
                        .filter(|&k| unit.members[k].alive())
                        .map(|k| {
                            (unit.members[k].position
                                + v3(0.0, 0.0, ctx.rules.physics.infantry_center_m)
                                - at)
                                .length()
                        })
                        .fold(f64::INFINITY, f64::min);
                    let v = near_miss(def, nearest);
                    if v > 0.0 {
                        let e = suppression.entry((hit.projectile, i)).or_insert(0.0);
                        *e = e.max(v);
                    }
                }
            }
            // A glancing round failed to pierce: at most the armour fraction.
            FlightEvent::Ricochet(r) => {
                let Some(round) = rounds.get(&r.projectile) else {
                    continue;
                };
                let def = &ctx.arsenal.weapons[round.weapon].def;
                if let Some((i, None)) = locate(units, r.body) {
                    let unit = &mut units[i];
                    let damage = hull_damage(ctx, def, unit, r.pose, r.point, r.normal, r.bounces);
                    if damage > 0.0 {
                        if take(unit, None, damage) {
                            lethal[i] = Some(LethalSource {
                                unit: round.unit,
                                side: round.side,
                            });
                        }
                        hurt.push((
                            i,
                            LethalSource {
                                unit: round.unit,
                                side: round.side,
                            },
                        ));
                    }
                }
            }
            // A round through a body that doesn't stop it still hits it.
            FlightEvent::Pass(pass) => {
                let Some(round) = rounds.get(&pass.projectile) else {
                    continue;
                };
                let def = &ctx.arsenal.weapons[round.weapon].def;
                if let Some(prop) = ctx.world.prop(pass.prop) {
                    if def.structural_damage > 0.0 {
                        structural.push(StructuralHit {
                            source: Some(LethalSource {
                                unit: round.unit,
                                side: round.side,
                            }),
                            prop: prop.id,
                            amount: def.structural_damage * prop.body.armor,
                            toward: pass.along.xy(),
                        });
                    }
                }
            }
            FlightEvent::Expired(_) => {}
        }
    }
    let mut outcome = Outcome {
        structural,
        ..Default::default()
    };
    for ((projectile, i), v) in suppression {
        let dealt = suppressed.entry((projectile, i)).or_insert(0.0);
        let more = v - *dealt;
        if more <= 0.0 {
            continue;
        }
        *dealt = v;
        let unit = &mut units[i];
        unit.suppression = (unit.suppression + more).min(1.0);
        unit.suppressed_at = ctx.tick;
        let round = &rounds[&projectile];
        hurt.push((
            i,
            LethalSource {
                unit: round.unit,
                side: round.side,
            },
        ));
    }
    for (i, shooter) in hurt {
        if units[i].side != shooter.side {
            outcome.attacked.push((units[i].id, shooter.unit));
        }
    }
    outcome.destroyed = casualties(ctx, units, &was_alive, &lethal);
    outcome
}

/// A burst of `def` at `at` that no round carried (a downed airframe's crash):
/// its blast on units and on destroyable bodies, credited to `source`.
pub fn detonate(
    ctx: &DamageContext,
    def: &WeaponDefinition,
    at: V3,
    source: Option<LethalSource>,
    units: &mut [Unit],
    rng: &mut Rng,
) -> Outcome {
    let was_alive: Vec<bool> = units.iter().map(|u| u.alive()).collect();
    let mut structural = Vec::new();
    blast_props(ctx.world, def, at, None, source, &mut structural);
    let mut hit = Vec::new();
    blast(ctx, def, at, None, units, rng, |i, _| hit.push(i));
    let lethal: Vec<Option<LethalSource>> = (0..units.len())
        .map(|i| source.filter(|_| was_alive[i] && !units[i].alive()))
        .collect();
    let attacked = source.map_or(Vec::new(), |s| {
        hit.iter()
            .filter(|&&i| units[i].side != s.side)
            .map(|&i| (units[i].id, s.unit))
            .collect()
    });
    Outcome {
        destroyed: casualties(ctx, units, &was_alive, &lethal),
        attacked,
        structural,
    }
}

/// Lay down this tick's fallen soldiers, and clear and report every unit
/// that died, with what killed it.
fn casualties(
    ctx: &DamageContext,
    units: &mut [Unit],
    was_alive: &[bool],
    lethal: &[Option<LethalSource>],
) -> Vec<UnitDeath> {
    // The fallen stay where they fell (a soldier is down once its hp is gone,
    // so later events this tick already passed it by).
    for unit in units.iter_mut() {
        for k in 0..unit.members.len() {
            let s = &unit.members[k];
            if !s.alive() && s.corpse.is_none() {
                let at = unit.members[k].position;
                let support = unit
                    .garrison
                    .as_ref()
                    .filter(|_| {
                        unit.garrisoned()
                            && at.z > ctx.world.surface_at(at.x, at.y).map_or(0.0, |s| s.z)
                    })
                    .map(|g| g.building);
                let body = crate::garrison::body_position(unit, k, ctx.rules);
                unit.members[k].fall(body, unit.yaw, support);
            }
        }
    }
    let mut destroyed = Vec::new();
    for (i, unit) in units.iter_mut().enumerate() {
        if was_alive[i] && !unit.alive() {
            unit.orders.clear();
            unit.route = None;
            unit.garrison = None;
            destroyed.push(UnitDeath {
                victim: unit.id,
                source: lethal[i],
            });
        }
    }
    destroyed
}

/// A round's damage to the hull it struck, judged on the face it met at
/// `pose` (P10): full damage when it pierces, otherwise the weapon's armour
/// fraction (HE's partial effect).
fn hull_damage(
    ctx: &DamageContext,
    def: &WeaponDefinition,
    unit: &Unit,
    pose: Pose,
    point: V3,
    normal: V3,
    bounces: u8,
) -> f64 {
    let armor = unit.armor(ctx.rules).expect("vehicle armour");
    let face = struck_face(unit.hull.expect("vehicle hull"), pose, point, normal);
    if pierces(def.penetration, bounces, &ctx.rules.ricochet, armor, face) {
        def.damage
    } else {
        def.damage * def.armor_fraction
    }
}

/// Weapon-specific suppression at `distance` from a round's path or impact.
fn near_miss(def: &WeaponDefinition, distance: f64) -> f64 {
    let r = def.ballistics.suppression_radius_m;
    if r <= 0.0 {
        return 0.0;
    }
    def.near_miss_suppression * (1.0 - distance / r).max(0.0)
}

fn take(unit: &mut Unit, soldier: Option<usize>, damage: f64) -> bool {
    let alive = unit.alive();
    match soldier {
        Some(k) => unit.members[k].hp -= damage,
        None => unit.hp -= damage,
    }
    alive && !unit.alive()
}

/// Per-soldier fragment sampling and distance-scaled vehicle damage inside the
/// blast radius; solid obstacles between the burst and a body shield it.
fn blast(
    ctx: &DamageContext,
    def: &WeaponDefinition,
    at: V3,
    skip: Option<BodyId>,
    units: &mut [Unit],
    rng: &mut Rng,
    mut hurt: impl FnMut(usize, bool),
) {
    let radius = def.blast_radius_m;
    for (i, unit) in units.iter_mut().enumerate() {
        if !unit.alive() {
            continue;
        }
        match unit.hull {
            Some(half) => {
                if skip == Some(BodyId(VEHICLE_BODY_BASE + unit.id.0)) {
                    continue;
                }
                let r = unit.hull_distance(at);
                let center = unit.position + v3(0.0, 0.0, half.z);
                if r >= radius || !ctx.world.segment_clear(at, center) {
                    continue;
                }
                let armor = unit.armor(ctx.rules).expect("vehicle armour");
                let face = unit.hull_face(at);
                if pierces(def.penetration, 0, &ctx.rules.ricochet, armor, face) {
                    unit.hp -= def.damage * (1.0 - r / radius);
                    hurt(i, !unit.alive());
                }
            }
            None => {
                // An occupied building shelters its occupants through its
                // cover factor once, not as an extra wall (contracts).
                let shell = crate::garrison::shell(unit);
                let shelter = crate::garrison::shelter(unit, ctx.rules);
                for k in 0..unit.members.len() {
                    let s = &unit.members[k];
                    if !s.alive() || skip == Some(BodyId(s.id)) {
                        continue;
                    }
                    let body = unit.members[k].position
                        + v3(0.0, 0.0, ctx.rules.physics.infantry_center_m);
                    let r = (body - at).length();
                    if r >= radius || !ctx.world.segment_clear_except(at, body, shell) {
                        continue;
                    }
                    let exposure = fragment_exposure(ctx.rules, shelter);
                    if let Some(damage) = fragment(r, radius, exposure, def.damage, rng.unit()) {
                        unit.members[k].hp -= damage;
                        hurt(i, !unit.alive());
                    }
                }
            }
        }
    }
}

/// Blast overpressure on the destroyable props within a burst's radius
/// (Q17): `structural_damage · (1 − r/R)` by the distance `r` from the burst
/// to each body's footprint, unshielded and unscaled by armour. `skip` took
/// the direct hit instead.
pub fn blast_props(
    world: &WorldGeometry,
    def: &WeaponDefinition,
    at: V3,
    skip: Option<PropId>,
    source: Option<LethalSource>,
    out: &mut Vec<StructuralHit>,
) {
    let radius = def.blast_radius_m;
    if radius <= 0.0 || def.structural_damage <= 0.0 {
        return;
    }
    let skip = skip.and_then(|id| world.structure_owner(id));
    let mut owners = BTreeMap::<PropId, usize>::new();
    for prop in world.props_near(at.xy(), radius) {
        let owner = world.structure_owner(prop.id).unwrap();
        if prop.body.hp.is_none() || Some(owner) == skip {
            continue;
        }
        let r = prop.footprint().distance(at.xy());
        if r < radius {
            let amount = def.structural_damage * (1.0 - r / radius);
            if let Some(&index) = owners.get(&owner) {
                out[index].amount = out[index].amount.max(amount);
            } else {
                owners.insert(owner, out.len());
                out.push(StructuralHit {
                    source,
                    prop: owner,
                    amount,
                    toward: prop.center - at.xy(),
                });
            }
        }
    }
}

/// One soldier's blast sample at distance `r`: hit with probability
/// `(1 - (r/R)²) · exposure`, and a hit does `damage · (1 - r/R)` whatever the
/// cover (P13).
pub fn fragment(r: f64, radius: f64, exposure: f64, damage: f64, roll: f64) -> Option<f64> {
    if r >= radius {
        return None;
    }
    let p = (1.0 - (r / radius).powi(2)) * exposure;
    (roll < p).then(|| damage * (1.0 - r / radius))
}

/// Suppression fades at a steady rate once a lull outlasts the recovery delay.
pub fn recover(units: &mut [Unit], rules: &Rules, tick: Tick) {
    let s = &rules.suppression;
    let dt = 1.0 / rules.tick_hz as f64;
    let delay = (s.recovery_delay_s * rules.tick_hz as f64).round() as Tick;
    for unit in units.iter_mut() {
        if unit.suppression > 0.0 && tick >= unit.suppressed_at + delay {
            unit.suppression = (unit.suppression - s.decay_per_s * dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_lowers_the_chance_of_a_fragment_never_its_damage() {
        // Paired trials: the same rolls against an exposed and a covered soldier.
        let mut rng = Rng::new(11);
        let (radius, damage, covered) = (12.0, 80.0, 0.65);
        let (mut open_hits, mut cover_hits) = (0, 0);
        for n in 0..20_000 {
            let r = (n % 12) as f64;
            let roll = rng.unit();
            let a = fragment(r, radius, 1.0, damage, roll);
            let b = fragment(r, radius, covered, damage, roll);
            if let Some(d) = b {
                assert_eq!(Some(d), a, "a covered hit is an exposed hit");
                cover_hits += 1;
            }
            open_hits += a.is_some() as u32;
        }
        let ratio = cover_hits as f64 / open_hits as f64;
        assert!((ratio - covered).abs() < 0.03, "{ratio}");
        assert_eq!(
            fragment(12.0, radius, 1.0, damage, 0.0),
            None,
            "nothing at the radius"
        );
    }
}
