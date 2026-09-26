//! Consequences of physical fire: what this tick's impacts and
//! near misses do to bodies. Rounds hit whatever they meet, of any side (P09);
//! a hit consumes the round; armour is judged on the struck face with fixed
//! penetration (P10); blast is sampled per soldier, where cover lowers the
//! chance of a damaging fragment but never its damage (P13); near misses,
//! impacts and blasts suppress infantry without damage (P14). Building cover
//! is one more source: garrisoned soldiers are harder to hit and to
//! reach with fragments, never softer when hit (P12); rounds striking a
//! building report structural damage for the garrison owner to apply.
use std::collections::BTreeMap;

use contract::ids::{Tick, UnitId};
use contract::scenario::Rules;
use contract::weapons::WeaponDefinition;

use crate::battle::Round;
use crate::flight::{BodyId, FlightEvent, ProjectileId, Struck};
use crate::math::{v3, V3};
use crate::rng::Rng;
use crate::units::Unit;
use crate::weapons::{Arsenal, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};

/// Height above a soldier's feet where blast is sampled (the body's middle).
const SOLDIER_CENTER_M: f64 = 0.9;
/// Blast samples start this far off the struck surface.
const BLAST_LIFT_M: f64 = 0.05;

pub struct DamageContext<'a> {
    pub world: &'a WorldGeometry,
    pub arsenal: &'a Arsenal,
    pub rules: &'a Rules,
    pub tick: Tick,
}

/// Cover strength of the ground at a point, in [0, 1]: the forest edge counts
/// from its first metre and deepens to full cover (contracts: ground cover).
pub fn ground_cover(world: &WorldGeometry, p: V3) -> f64 {
    world
        .forest_depth(p.x, p.y)
        .map_or(0.0, |d| (0.4 + d / 50.0).clamp(0.0, 1.0))
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Incoming spread multiplier for a round aimed at `p` (V03, P12), where the
/// target is observed with building cover strength `shelter`. Overlapping
/// sources give the strongest protection, not a product: cover applies once.
pub fn cover_spread(world: &WorldGeometry, rules: &Rules, p: V3, shelter: f64) -> f64 {
    let c = &rules.cover;
    lerp(1.0, c.forest_spread_multiplier, ground_cover(world, p)).max(lerp(
        1.0,
        c.building_spread_multiplier,
        shelter,
    ))
}

/// Fragment probability multiplier for a soldier at `p` with building cover
/// strength `shelter` (P13): the strongest source, once.
pub fn fragment_exposure(world: &WorldGeometry, rules: &Rules, p: V3, shelter: f64) -> f64 {
    let c = &rules.cover;
    lerp(
        1.0,
        c.forest_fragment_probability_multiplier,
        ground_cover(world, p),
    )
    .min(lerp(
        1.0,
        c.building_fragment_probability_multiplier,
        shelter,
    ))
}

/// What happened to units this tick, for the battle to act on.
#[derive(Default)]
pub struct Outcome {
    /// Units that died this tick.
    pub destroyed: Vec<UnitId>,
    /// (victim, shooter): a hostile round damaged or suppressed the victim.
    pub attacked: Vec<(UnitId, UnitId)>,
    /// Structural damage rounds did to props they struck, in event order.
    pub structural: Vec<(PropId, f64)>,
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

/// Apply this tick's flight events in order.
pub fn resolve(
    ctx: &DamageContext,
    events: &[FlightEvent],
    rounds: &BTreeMap<ProjectileId, Round>,
    units: &mut [Unit],
    rng: &mut Rng,
) -> Outcome {
    let was_alive: Vec<bool> = units.iter().map(|u| u.alive()).collect();
    // One suppression contribution per projectile per squad per tick: the
    // strongest of its near miss and its impact.
    let mut suppression: BTreeMap<(ProjectileId, usize), f64> = BTreeMap::new();
    let mut hurt: Vec<(usize, UnitId)> = Vec::new();
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
                // Only weapons with structural damage wear buildings down (L10).
                if let Struck::Prop(id) = hit.struck {
                    if def.structural_damage > 0.0 {
                        structural.push((id, def.structural_damage));
                    }
                }
                if let Some((_, (i, soldier))) = direct {
                    let unit = &mut units[i];
                    let damage = match soldier {
                        Some(_) => def.damage,
                        // Fixed penetration against the struck face (P10); a
                        // failed penetration stops the round and deals only the
                        // weapon's armour fraction (HE's partial effect).
                        None => {
                            let armor = unit.armor(&ctx.rules.health).expect("vehicle armour");
                            let face = unit.hull_face(hit.point + hit.normal * BLAST_LIFT_M);
                            if def.penetration > armor.face(face) {
                                def.damage
                            } else {
                                def.damage * def.armor_fraction
                            }
                        }
                    };
                    if damage > 0.0 {
                        take(unit, soldier, damage);
                        hurt.push((i, round.unit));
                    }
                }
                let at = hit.point + hit.normal * BLAST_LIFT_M;
                if def.blast_radius_m > 0.0 {
                    let skip = direct.map(|(b, _)| b);
                    blast(ctx, def, at, skip, units, rng, |i| {
                        hurt.push((i, round.unit))
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
                            (unit.member_position(k) + v3(0.0, 0.0, SOLDIER_CENTER_M) - at).length()
                        })
                        .fold(f64::INFINITY, f64::min);
                    let v = near_miss(def, nearest);
                    if v > 0.0 {
                        let e = suppression.entry((hit.projectile, i)).or_insert(0.0);
                        *e = e.max(v);
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
        let unit = &mut units[i];
        unit.suppression = (unit.suppression + v).min(1.0);
        unit.suppressed_at = ctx.tick;
        hurt.push((i, rounds[&projectile].unit));
    }
    for (i, shooter) in hurt {
        if units[i].side != units[shooter.0 as usize].side {
            outcome.attacked.push((units[i].id, shooter));
        }
    }
    // The fallen stay where they fell (a soldier is down once its hp is gone,
    // so later events this tick already passed it by).
    for unit in units.iter_mut() {
        for k in 0..unit.members.len() {
            let s = &unit.members[k];
            if !s.alive() && s.corpse.is_none() {
                let at = unit.member_position(k);
                unit.members[k].fall(at);
            }
        }
    }
    for (i, unit) in units.iter_mut().enumerate() {
        if was_alive[i] && !unit.alive() {
            unit.orders.clear();
            unit.route = None;
            unit.garrison = None;
            outcome.destroyed.push(unit.id);
        }
    }
    outcome
}

/// Weapon-specific suppression at `distance` from a round's path or impact.
fn near_miss(def: &WeaponDefinition, distance: f64) -> f64 {
    let r = def.ballistics.suppression_radius_m;
    if r <= 0.0 {
        return 0.0;
    }
    def.near_miss_suppression * (1.0 - distance / r).max(0.0)
}

fn take(unit: &mut Unit, soldier: Option<usize>, damage: f64) {
    match soldier {
        Some(k) => unit.members[k].hp -= damage,
        None => unit.hp -= damage,
    }
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
    mut hurt: impl FnMut(usize),
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
                let armor = unit.armor(&ctx.rules.health).expect("vehicle armour");
                if def.penetration > armor.face(unit.hull_face(at)) {
                    unit.hp -= def.damage * (1.0 - r / radius);
                    hurt(i);
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
                    let body = unit.member_position(k) + v3(0.0, 0.0, SOLDIER_CENTER_M);
                    let r = (body - at).length();
                    if r >= radius || !ctx.world.segment_clear_except(at, body, shell) {
                        continue;
                    }
                    let exposure = fragment_exposure(ctx.world, ctx.rules, body, shelter);
                    if let Some(damage) = fragment(r, radius, exposure, def.damage, rng.unit()) {
                        unit.members[k].hp -= damage;
                        hurt(i);
                    }
                }
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
