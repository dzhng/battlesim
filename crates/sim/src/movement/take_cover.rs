//! Squads taking cover (D3–D5, Q7, Q9, Q11). At the order, each soldier's
//! spot in the fresh arrangement gives way to the best cover spot near it,
//! if any ([`at_order`]). A squad that holds (at rest, or an attack-move
//! halted on contact) re-resolves around where its soldiers stand when it
//! arrives or halts, when its side learns of a body or a crater, when
//! the threat swings past `swing_deg`, or when a vehicle a soldier hides
//! behind drives off; at most once per `reresolve_s` ([`hold`]). A soldier
//! who already has the best cover he could claim stays put, so a re-resolve
//! never shuffles a squad that is well placed. Each soldier then walks to
//! his post on his own route.
//!
//! Against an enemy his side has seen, a place counts only if the soldier
//! there has a straight line to it: cover he could not fight from is no
//! place to take. Whoever is left without a line steps to the nearest
//! clear place within `step_out_m`, or sits out (D3).
use contract::ids::UnitId;
use contract::map::MoverClass;

use super::{MovementContext, SideGeometry};
use crate::arrangement;
use crate::cover::{self, Body, Known, Spot, Tier, Watch};
use crate::math::{v2, wrap_angle, V2};
use crate::units::Unit;
use crate::world::Prop;

/// A soldier this close to his claimed place is already there.
const IN_PLACE_M: f64 = 0.3;
/// Room around a squad's soldiers searched for cover beyond `search_m`.
const SEARCH_SLACK_M: f64 = 2.0;

/// What cover is sought among this tick: each side's live vehicles (Q24)
/// and where every squad's soldiers stand, by unit and member.
pub(super) struct Field {
    hulls: [Vec<Body>; 2],
    soldiers: Vec<Vec<V2>>,
}

impl Field {
    pub(super) fn gather(ctx: &MovementContext, units: &[Unit]) -> Field {
        let hulls = contract::ids::Side::ALL
            .map(|side| cover::hulls(units.iter().filter(|u| u.side == side), ctx.rules));
        let soldiers = units
            .iter()
            .map(|u| u.members.iter().map(|s| s.position.xy()).collect())
            .collect();
        Field { hulls, soldiers }
    }

    fn own(&self, unit: &Unit) -> &[Body] {
        &self.hulls[unit.side.index()]
    }
}

/// What a squad takes cover from: a point; whether it is an enemy its side
/// has seen (else only far along the way the squad was sent); and the
/// places a soldier fires at, to judge his line (the enemy's soldiers his
/// side sees, else the point).
#[derive(Clone)]
struct Threat {
    at: V2,
    hostile: bool,
    aims: Vec<V2>,
}

impl Threat {
    fn enemy(ctx: &MovementContext, unit: &Unit, field: &Field, enemy: cover::Enemy) -> Threat {
        let knowledge = &ctx.knowledge[unit.side.index()];
        let seen = enemy.unit.and_then(|u| {
            let t = knowledge.track(u)?;
            let soldiers = field.soldiers.get(u.0 as usize)?;
            Some(
                t.members
                    .iter()
                    .filter_map(|&m| soldiers.get(m).copied())
                    .collect::<Vec<_>>(),
            )
        });
        Threat {
            at: enemy.at,
            hostile: true,
            aims: seen
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| vec![enemy.at]),
        }
    }
}
/// The tick of each side's latest sensing: movement runs before this tick's.
fn sensed(ctx: &MovementContext) -> u64 {
    ctx.tick.saturating_sub(1)
}

/// Whether a soldier standing at `p` has a straight line from his muzzle
/// to any of `aims` (the soldiers he would fire at).
fn line(ctx: &MovementContext, p: V2, aims: &[V2]) -> bool {
    let ground = |q: V2| ctx.world.height_at(q.x, q.y).unwrap_or(0.0);
    let muzzle = p.with_z(ground(p) + ctx.rules.bodies.infantry_muzzle_m);
    aims.iter().any(|&a| {
        let aim = a.with_z(ground(a) + crate::weapons::SOLDIER_AIM_M);
        ctx.world.segment_clear(muzzle, aim)
    })
}

/// Where the squad's threat is at an order: the enemy it engages or sees,
/// else far along the way it is sent (Q7).
fn order_threat(ctx: &MovementContext, unit: &Unit, field: &Field, from: V2, end: V2) -> Threat {
    let knowledge = &ctx.knowledge[unit.side.index()];
    match cover::threat(unit, knowledge, sensed(ctx)) {
        Some(enemy) => Threat::enemy(ctx, unit, field, enemy),
        None => {
            let way = end - from;
            let dir = if way.length() > 1e-6 {
                way.normalized()
            } else {
                v2(1.0, 0.0).rotated(unit.yaw)
            };
            let at = end + dir * cover::FAR_M;
            Threat {
                at,
                hostile: false,
                aims: vec![at],
            }
        }
    }
}

/// The cover a side may seek within `radius` of `centre`, and the test of
/// where one of its soldiers can stand.
fn known<'a>(
    ctx: &'a MovementContext,
    unit: &Unit,
    side: &'a SideGeometry,
    field: &Field,
    centre: V2,
    radius: f64,
) -> (Known, impl Fn(V2) -> bool + 'a) {
    let own = field.own(unit);
    let knows = move |p: &Prop| side.knows(p, ctx.authored);
    let ground = ctx.knowledge[unit.side.index()].ground();
    let known = Known::gather(ctx.world, ground, ctx.rules, own, &knows, centre, radius);
    let hulls: Vec<_> = known
        .bodies
        .iter()
        .filter(|b| b.vehicle.is_some())
        .map(|b| b.rect)
        .collect();
    let r = ctx.soldier_radius_m;
    let stands = move |p: V2| {
        let solid = |q: &Prop| q.kind.blocks(MoverClass::Infantry) && knows(q);
        arrangement::standing_room(ctx.world, p, r, &solid)
            && hulls.iter().all(|h| !h.contains(p, r))
    };
    (known, stands)
}

/// The cover spots against `threat`: against an enemy, only those with a
/// line to it.
fn offers(
    ctx: &MovementContext,
    known: &Known,
    stands: &impl Fn(V2) -> bool,
    threat: &Threat,
) -> Vec<Spot> {
    let (c, r) = (&ctx.rules.cover, ctx.soldier_radius_m);
    let mut spots = cover::spots(known, threat.at, c, r, ctx.infantry.spacing_m, stands);
    if threat.hostile {
        spots.retain(|s| line(ctx, s.at, &threat.aims));
    }
    spots
}

/// The vehicles whose hulls cover any of `places`, and where each stands.
fn covering_vehicles(
    ctx: &MovementContext,
    known: &Known,
    places: &[V2],
    threat: V2,
) -> Vec<(UnitId, V2)> {
    let mut out: Vec<(UnitId, V2)> = Vec::new();
    for &p in places {
        let id = known.vehicle(p, threat, &ctx.rules.cover, ctx.soldier_radius_m);
        if let Some(id) = id.filter(|id| out.iter().all(|(o, _)| o != id)) {
            if let Some(b) = known.bodies.iter().find(|b| b.vehicle == Some(id)) {
                out.push((id, b.rect.center));
            }
        }
    }
    out
}

/// What this resolution was made against, for the next one's triggers.
fn watch(
    ctx: &MovementContext,
    side: &SideGeometry,
    known: &Known,
    places: &[V2],
    threat: &Threat,
) -> Watch {
    Watch {
        threat: Some(threat.at),
        hostile: threat.hostile,
        resolved_at: ctx.tick,
        revision: side.revision,
        craters: known.craters.len() as u32,
        vehicles: covering_vehicles(ctx, known, places, threat.at),
        due: false,
    }
}

/// Resolve cover at the order (D4): each living soldier's spot `spots[k]`,
/// drawn at random around `end`, gives way to the best cover spot within
/// `search_m` of it that nobody else has claimed; whoever finds none keeps
/// his random spot, moved off any claimed one. Returns the tier each spot
/// gives, and writes the squad's watch.
pub(super) fn at_order(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &Field,
    from: V2,
    end: V2,
    spots: &mut [V2],
) -> Vec<Option<Tier>> {
    let c = &ctx.rules.cover;
    let r = ctx.soldier_radius_m;
    let spacing = ctx.infantry.spacing_m;
    let threat = order_threat(ctx, unit, field, from, end);
    let reach = arrangement::spread(ctx.infantry, spots.len()) / 2.0 + c.search_m + SEARCH_SLACK_M;
    let (known, stands) = known(ctx, unit, side, field, end, reach);
    let offered = offers(ctx, &known, &stands, &threat);
    let claims = cover::claim(
        spots,
        &vec![None; spots.len()],
        &offered,
        c.search_m,
        spacing,
        0.0,
    );
    let solid = |q: &Prop| q.kind.blocks(MoverClass::Infantry) && side.knows(q, ctx.authored);
    let mut placed: Vec<V2> = claims.iter().flatten().map(|&i| offered[i].at).collect();
    let mut tiers = Vec::with_capacity(spots.len());
    for (k, claim) in claims.iter().enumerate() {
        if let Some(i) = claim {
            spots[k] = offered[*i].at;
            tiers.push(Some(offered[*i].tier));
            continue;
        }
        let apart = |p: V2, placed: &[V2]| placed.iter().all(|q| (*q - p).length() >= spacing);
        if !apart(spots[k], &placed) {
            let reach = 2.0 * arrangement::spread(ctx.infantry, spots.len());
            spots[k] = arrangement::nearest_free(spots[k], reach, |p| {
                apart(p, &placed)
                    && stands(p)
                    && arrangement::reachable(ctx.world, end, p, r, &solid)
            })
            .unwrap_or(spots[k]);
        }
        placed.push(spots[k]);
        tiers.push(known.tier(spots[k], threat.at, c, r));
    }
    unit.cover = watch(ctx, side, &known, spots, &threat);
    tiers
}

/// A holding squad keeps its cover current (D5, Q11): see the module.
pub(super) fn hold(ctx: &MovementContext, unit: &mut Unit, side: &SideGeometry, field: &Field) {
    let c = &ctx.rules.cover;
    let every = (c.reresolve_s * ctx.tick_hz as f64).round().max(1.0) as u64;
    let w = &unit.cover;
    if ctx.tick.saturating_sub(w.resolved_at) < every {
        return;
    }
    // Each squad looks once a second, on its own tick of the second.
    if !w.due && !(ctx.tick + unit.id.0 as u64).is_multiple_of(every) {
        return;
    }
    let knowledge = &ctx.knowledge[unit.side.index()];
    let enemy = cover::threat(unit, knowledge, sensed(ctx));
    // An enemy out of sight is still where it was last seen.
    let threat = match (enemy, w.threat) {
        (Some(enemy), _) => Threat::enemy(ctx, unit, field, enemy),
        (None, Some(at)) => Threat {
            at,
            hostile: w.hostile,
            aims: vec![at],
        },
        (None, None) => return,
    };
    let centre = unit.position.xy();
    let bearing = |p: V2| (p - centre).y.atan2((p - centre).x);
    let swung = w.threat.is_none_or(|t| {
        wrap_angle(bearing(threat.at) - bearing(t)).abs() > c.swing_deg.to_radians()
    });
    let drove_off = w.vehicles.iter().any(|(id, at)| {
        field
            .own(unit)
            .iter()
            .find(|b| b.vehicle == Some(*id))
            .is_some_and(|b| (b.rect.center - *at).length() > c.vehicle_moved_m)
    });
    let reach = unit.footprint_radius() + c.search_m + SEARCH_SLACK_M;
    let craters = cover::craters(knowledge.ground(), ctx.rules, centre, reach).len() as u32;
    let changed = side.revision != w.revision || craters != w.craters;
    if !(w.due || swung || drove_off || changed) {
        return;
    }
    resolve(ctx, unit, side, field, &threat);
    // Facing (Q9): an enemy seen, engaged or last seen turns the squad to it.
    if threat.hostile && (threat.at - centre).length() > 1e-6 {
        unit.yaw = bearing(threat.at);
    }
}

/// Re-resolve a holding squad's cover against `threat` around where its
/// soldiers stand, then step out whoever is left without a line (D3).
fn resolve(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &Field,
    threat: &Threat,
) {
    let c = &ctx.rules.cover;
    let r = ctx.soldier_radius_m;
    let spacing = ctx.infantry.spacing_m;
    let living: Vec<usize> = (0..unit.members.len())
        .filter(|&k| unit.members[k].alive())
        .collect();
    let from: Vec<V2> = living
        .iter()
        .map(|&k| unit.members[k].position.xy())
        .collect();
    let reach = unit.footprint_radius() + c.search_m + SEARCH_SLACK_M;
    let (known, stands) = known(ctx, unit, side, field, unit.position.xy(), reach);
    let offered = offers(ctx, &known, &stands, threat);
    // Where he stands holds its cover; against an enemy, only with a line.
    let stay: Vec<Option<Option<Tier>>> = from
        .iter()
        .map(|&p| {
            let fights = !threat.hostile || line(ctx, p, &threat.aims);
            Some(known.tier(p, threat.at, c, r).filter(|_| fights))
        })
        .collect();
    let claims = cover::claim(&from, &stay, &offered, c.search_m, spacing, spacing);
    let mut places: Vec<(V2, Option<Tier>)> = claims
        .iter()
        .zip(&from)
        .map(|(claim, &p)| match claim {
            Some(i) => (offered[*i].at, Some(offered[*i].tier)),
            None => (p, known.tier(p, threat.at, c, r)),
        })
        .collect();
    if threat.hostile {
        step_out(ctx, side, &known, &stands, threat, &mut places);
    }
    for (&k, &(place, tier)) in living.iter().zip(&places) {
        let s = &mut unit.members[k];
        s.cover = tier;
        let post = ((place - s.position.xy()).length() > IN_PLACE_M).then_some(place);
        if post != s.post {
            s.path.clear();
        }
        s.post = post;
    }
    let spots: Vec<V2> = places.iter().map(|(p, _)| *p).collect();
    unit.cover = watch(ctx, side, &known, &spots, threat);
}

/// A soldier whose line to the enemy is blocked steps to the nearest clear
/// place within `step_out_m` of a straight walk, keeping the best cover he
/// can (D3, Q8); with none, he sits out where he is.
fn step_out(
    ctx: &MovementContext,
    side: &SideGeometry,
    known: &Known,
    stands: &impl Fn(V2) -> bool,
    threat: &Threat,
    places: &mut [(V2, Option<Tier>)],
) {
    let solid = |q: &Prop| q.kind.blocks(MoverClass::Infantry) && side.knows(q, ctx.authored);
    let (c, r) = (&ctx.rules.cover, ctx.soldier_radius_m);
    let clear = |p: V2| line(ctx, p, &threat.aims);
    let apart = ctx.infantry.spacing_m;
    for k in 0..places.len() {
        let (place, _) = places[k];
        if clear(place) {
            continue;
        }
        let others: Vec<V2> = places
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .map(|(_, (p, _))| *p)
            .collect();
        let free = |p: V2| {
            stands(p)
                && others.iter().all(|q| (*q - p).length() >= apart)
                && arrangement::reachable(ctx.world, place, p, r, &solid)
        };
        if let Some(p) = cover::step_out(place, threat.at, known, c, r, &free, &clear) {
            places[k] = (p, known.tier(p, threat.at, c, r));
        }
    }
}
