//! Squads taking cover (D3–D5, Q7, Q9, Q11). A squad owns an area: a
//! disc round its anchor ([`cover::Anchor`]), which only an order moves.
//! Inside it the squad assigns its soldiers' places together
//! ([`cover::claim`]): the most soldiers able to engage the enemy, then the
//! strongest cover, then the least walking. At the order the fresh
//! arrangement's spots give way to what the squad claims ([`at_order`]); a
//! squad that holds (at rest, or an attack-move halted on contact)
//! re-resolves round its anchor when it arrives or halts, when its side
//! learns of a body or a crater within its search reach (its area and
//! `cover.search_slack_m` round it), when the threat swings past `swing_deg`,
//! or when a vehicle a soldier hides behind drives off; at most once per
//! `reresolve_s` ([`hold`]). A soldier who already holds the best he could
//! claim stays put, so a re-resolve never shuffles a squad that is well
//! placed. Each soldier then walks to his post on his own route.
//!
//! Against an enemy his side has seen, a soldier can engage from a place
//! when a round of his reaches one of the enemy's soldiers within his
//! weapon's range: straight from the place, or from a free lean point round
//! the tall cover he hides behind ([`crate::lean`], the same line test the
//! fire code uses). Whoever is left unable to engage steps to the nearest
//! place inside the area he can engage from, or sits out (D3).
use contract::ids::UnitId;
use contract::map::MoverClass;

use super::{MovementContext, SideGeometry};
use crate::arrangement;
use crate::cover::{self, Anchor, Body, Claim, Known, Place, Tier, Watch};
use crate::lean::{self, Lean};
use crate::math::{v2, wrap_angle, V2, V3};
use crate::units::Unit;
use crate::world::{Prop, PropId};

/// A soldier this close to his claimed place is already there.
const IN_PLACE_M: f64 = 0.3;

/// What cover is sought among this tick: each side's live vehicles (Q24),
/// every live hull as rounds meet it, and where every squad's soldiers are
/// exposed (standing, or out on a lean), by unit and member.
pub(super) struct Field {
    hulls: [Vec<Body>; 2],
    blockers: Vec<lean::Hull>,
    soldiers: Vec<Vec<V2>>,
}

impl Field {
    pub(super) fn gather(ctx: &MovementContext, units: &[Unit]) -> Field {
        let blockers = lean::hulls(units, ctx.rules);
        let hulls = contract::ids::Side::ALL
            .map(|side| cover::hull_bodies(blockers.iter().filter(|h| h.side == side)));
        let soldiers = units
            .iter()
            .map(|u| u.members.iter().map(|s| s.exposed(ctx.tick).xy()).collect())
            .collect();
        Field {
            hulls,
            blockers,
            soldiers,
        }
    }

    fn own(&self, unit: &Unit) -> &[Body] {
        &self.hulls[unit.side.index()]
    }
}

/// What a squad takes cover from: a point; whether it is an enemy its side
/// has seen (else only far along the way the squad was sent); and the
/// places a soldier fires at, to judge where he can engage from (the
/// enemy's soldiers his side sees, else the point).
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

/// How a squad's soldiers can fight against a seen enemy: whether a
/// round from a point reaches one of its soldiers within the squad's range.
struct Fight<'a> {
    ctx: &'a MovementContext<'a>,
    blockers: &'a [lean::Hull],
    threat: V2,
    aims: Vec<V3>,
    range: f64,
}

impl<'a> Fight<'a> {
    /// `None` against a threat that is no enemy: nobody fights from anywhere.
    fn new(
        ctx: &'a MovementContext<'a>,
        unit: &Unit,
        field: &'a Field,
        threat: &Threat,
    ) -> Option<Fight<'a>> {
        threat.hostile.then(|| Fight {
            ctx,
            blockers: &field.blockers,
            threat: threat.at,
            aims: threat
                .aims
                .iter()
                .map(|&a| a.with_z(ground(ctx, a) + ctx.rules.physics.infantry_aim_m))
                .collect(),
            range: crate::weapons::squad_range(ctx.arsenal, unit.kind),
        })
    }

    /// Whether a soldier's round from `p` reaches an enemy soldier within
    /// range, passing only `past` (the body he leans round).
    fn reaches(&self, p: V2, past: Option<PropId>) -> bool {
        let muzzle = p.with_z(ground(self.ctx, p) + self.ctx.rules.physics.infantry_muzzle_m);
        self.aims.iter().any(|&a| {
            (a - muzzle).length() <= self.range
                && lean::reaches(self.ctx.world, self.blockers, muzzle, a, past)
        })
    }

    /// How a soldier at `p` behind `body` (with `tier`) fights: straight,
    /// and from each lean point round the body where he can stand, when
    /// the body is taller than his muzzle. He claims a lean behind tall
    /// cover even with a straight line to some of the enemy: the rest may
    /// be out of it, and his rounds pass the body he leans round.
    fn place(
        &self,
        p: V2,
        tier: Option<Tier>,
        body: Option<&Body>,
        stands: &impl Fn(V2) -> bool,
    ) -> Place {
        Place {
            at: p,
            tier,
            direct: self.reaches(p, None),
            leans: self.leans(p, body, stands),
        }
    }

    /// The lean points round `body`, when it is taller than his muzzle,
    /// from which a soldier at `p` fights.
    fn leans(&self, p: V2, body: Option<&Body>, stands: &impl Fn(V2) -> bool) -> Vec<Lean> {
        let r = self.ctx.soldier_radius_m;
        let muzzle = ground(self.ctx, p) + self.ctx.rules.physics.infantry_muzzle_m;
        let tall = body.filter(|b| b.top > muzzle);
        match tall.and_then(|b| Some((b, b.round()?))) {
            Some((b, round)) => lean::points(&b.rect, p, self.threat, r, &self.ctx.rules.cover)
                .into_iter()
                .filter(|&(at, _)| stands(at) && self.reaches(at, b.prop))
                .map(|(at, side)| Lean {
                    from: p,
                    at,
                    side,
                    body: round,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

fn ground(ctx: &MovementContext, p: V2) -> f64 {
    ctx.world.height_at(p.x, p.y).unwrap_or(0.0)
}

/// Where the squad's threat is at an order: the enemy it engages or sees,
/// else far off toward the right-drag's facing (Q9), else far along the way
/// it is sent (Q7).
fn order_threat(ctx: &MovementContext, unit: &Unit, field: &Field, from: V2, end: V2) -> Threat {
    let knowledge = &ctx.knowledge[unit.side.index()];
    match cover::threat(unit, knowledge, sensed(ctx)) {
        Some(enemy) => Threat::enemy(ctx, unit, field, enemy),
        None => {
            let way = end - from;
            let facing = unit
                .orders
                .front()
                .and_then(|o| o.movement())
                .and_then(|m| m.facing);
            let dir = if let Some(f) = facing {
                v2(1.0, 0.0).rotated(f)
            } else if way.length() > 1e-6 {
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
        let solid = |q: &Prop| q.blocks(MoverClass::Infantry) && knows(q);
        arrangement::standing_room(ctx.world, p, r, &solid)
            && hulls.iter().all(|h| !h.contains(p, r))
    };
    (known, stands)
}

/// A squad's area: its anchor and radius.
#[derive(Clone, Copy)]
struct Area {
    centre: V2,
    radius: f64,
}

impl Area {
    fn of(ctx: &MovementContext, unit: &Unit, centre: V2) -> Area {
        Area {
            centre,
            radius: cover::area_radius(ctx.rules, unit.members.len()),
        }
    }

    fn holds(&self, p: V2) -> bool {
        (p - self.centre).length() <= self.radius
    }
}

/// The cover spots inside the area against `threat`, each with how a
/// soldier there fights.
fn offers(
    ctx: &MovementContext,
    known: &Known,
    stands: &impl Fn(V2) -> bool,
    threat: &Threat,
    fight: Option<&Fight>,
    area: Area,
) -> Vec<Place> {
    let (c, r) = (&ctx.rules.cover, ctx.soldier_radius_m);
    cover::spots(known, threat.at, c, r, ctx.infantry.spacing_m, stands)
        .into_iter()
        .filter(|s| area.holds(s.at))
        .map(|s| match fight {
            Some(f) => f.place(s.at, Some(s.tier), s.body.map(|i| &known.bodies[i]), stands),
            None => Place::quiet(s.at, Some(s.tier)),
        })
        .collect()
}

/// How a soldier at `p` fights, from the cover he has there.
fn place_at(
    ctx: &MovementContext,
    known: &Known,
    stands: &impl Fn(V2) -> bool,
    threat: V2,
    fight: Option<&Fight>,
    p: V2,
) -> Place {
    let (c, r) = (&ctx.rules.cover, ctx.soldier_radius_m);
    let tier = known.tier(p, threat, c, r);
    match fight {
        Some(f) => f.place(p, tier, known.cover_body(p, threat, c, r), stands),
        None => Place::quiet(p, tier),
    }
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

/// Resolve cover at the order (D4): the squad's area is set round
/// `end`, the order's destination, and each living soldier's spot
/// `spots[k]`, drawn at random round `end`, gives way to what the squad
/// claims inside the area; whoever claims nothing keeps his random spot,
/// moved off any claimed one. Returns the tier each place gives and the
/// lean claimed with it, and writes the squad's watch and anchor.
pub(super) fn at_order(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &Field,
    from: V2,
    end: V2,
    spots: &mut [V2],
) -> Vec<(Option<Tier>, Option<Lean>)> {
    let c = &ctx.rules.cover;
    let r = ctx.soldier_radius_m;
    let spacing = ctx.infantry.spacing_m;
    let threat = order_threat(ctx, unit, field, from, end);
    let area = Area::of(ctx, unit, end);
    let (known, stands) = known(
        ctx,
        unit,
        side,
        field,
        end,
        area.radius + ctx.rules.cover.search_slack_m,
    );
    let fight = Fight::new(ctx, unit, field, &threat);
    // A squad sent into a building gathers at its door: no cover to seek.
    let entering = matches!(
        unit.orders.front(),
        Some(crate::units::UnitOrder::Garrison { .. })
    );
    let offered = if entering {
        Vec::new()
    } else {
        offers(ctx, &known, &stands, &threat, fight.as_ref(), area)
    };
    let claims = cover::claim(
        spots,
        &vec![None; spots.len()],
        &offered,
        spacing,
        0.0,
        ctx.rules.cover.lean_apart_m,
    );
    let solid = |q: &Prop| q.blocks(MoverClass::Infantry) && side.knows(q, ctx.authored);
    let mut placed: Vec<V2> = claims
        .iter()
        .flatten()
        .filter_map(|c| c.spot.map(|i| offered[i].at))
        .collect();
    let mut out = Vec::with_capacity(spots.len());
    for (k, claim) in claims.iter().enumerate() {
        if let Some(Claim {
            spot: Some(i),
            lean,
            ..
        }) = claim
        {
            spots[k] = offered[*i].at;
            out.push((offered[*i].tier, *lean));
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
        out.push((known.tier(spots[k], threat.at, c, r), None));
    }
    unit.cover = watch(ctx, side, &known, spots, &threat);
    unit.anchor = Some(Anchor {
        at: end,
        halt: false,
    });
    out
}

/// An attack-move halting on contact: the squad holds round where it
/// halted. A halt that lapses and resumes inside the area it set keeps it,
/// so a flickering halt never walks the squad along.
pub(super) fn halt(ctx: &MovementContext, unit: &mut Unit) {
    let here = unit.position.xy();
    let keep = unit
        .anchor
        .is_some_and(|a| a.halt && Area::of(ctx, unit, a.at).holds(here));
    if !keep {
        unit.anchor = Some(Anchor {
            at: here,
            halt: true,
        });
    }
}

/// A squad arriving at `end`, its route's end, holds round it: the anchor its
/// order set there, back from any halt on the way (a halt's area stays
/// behind where it halted), and it looks at its cover again (D5).
pub(super) fn arrive(unit: &mut Unit, end: V2) {
    unit.anchor = Some(Anchor {
        at: end,
        halt: false,
    });
    unit.cover.due = true;
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
    let centre = unit.anchor.map_or(unit.position.xy(), |a| a.at);
    let area = Area::of(ctx, unit, centre);
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
    let reach = area.radius + ctx.rules.cover.search_slack_m;
    let craters = cover::craters(knowledge.ground(), ctx.rules, centre, reach).len() as u32;
    let changed = side.changed_near(w.revision, centre, reach) || craters != w.craters;
    if !(w.due || swung || drove_off || changed) {
        return;
    }
    resolve(ctx, unit, side, field, &threat, area);
    // Facing (Q9): an enemy seen, engaged or last seen turns the squad to it.
    if threat.hostile && (threat.at - centre).length() > 1e-6 {
        unit.yaw = bearing(threat.at);
    }
}

/// Re-resolve a holding squad's places inside its area against `threat`,
/// then step out whoever is left unable to engage (D3).
fn resolve(
    ctx: &MovementContext,
    unit: &mut Unit,
    side: &SideGeometry,
    field: &Field,
    threat: &Threat,
    area: Area,
) {
    let spacing = ctx.infantry.spacing_m;
    let living: Vec<usize> = (0..unit.members.len())
        .filter(|&k| unit.members[k].alive())
        .collect();
    let from: Vec<V2> = living
        .iter()
        .map(|&k| unit.members[k].position.xy())
        .collect();
    let reach = area.radius + ctx.rules.cover.search_slack_m;
    let (known, stands) = known(ctx, unit, side, field, area.centre, reach);
    let fight = Fight::new(ctx, unit, field, threat);
    let offered = offers(ctx, &known, &stands, threat, fight.as_ref(), area);
    // Where he stands, if inside the area, is a place he may keep.
    let stay: Vec<Option<Place>> = from
        .iter()
        .map(|&p| {
            area.holds(p)
                .then(|| place_at(ctx, &known, &stands, threat.at, fight.as_ref(), p))
        })
        .collect();
    let claims = cover::claim(
        &from,
        &stay,
        &offered,
        spacing,
        spacing,
        ctx.rules.cover.lean_apart_m,
    );
    let mut places: Vec<(V2, Option<Tier>, Option<Lean>, bool)> = Vec::new();
    for (k, claim) in claims.iter().enumerate() {
        places.push(match claim {
            Some(c) => {
                let p = c
                    .spot
                    .map_or_else(|| stay[k].as_ref().unwrap(), |i| &offered[i]);
                (p.at, p.tier, c.lean, c.engages)
            }
            // Outside the area: back inside, to the nearest free standing room.
            None => {
                let p = into_area(ctx, area, from[k], &places, &stands);
                let tier = known.tier(p, threat.at, &ctx.rules.cover, ctx.soldier_radius_m);
                (p, tier, None, false)
            }
        });
    }
    if let Some(f) = &fight {
        step_out(ctx, side, &known, &stands, f, area, &mut places);
    }
    for (&k, &(place, tier, lean, _)) in living.iter().zip(&places) {
        let s = &mut unit.members[k];
        s.cover = tier;
        s.lean = lean;
        let post = ((place - s.position.xy()).length() > IN_PLACE_M).then_some(place);
        if post != s.post {
            s.path.clear();
        }
        s.post = post;
    }
    let spots: Vec<V2> = places.iter().map(|p| p.0).collect();
    unit.cover = watch(ctx, side, &known, &spots, threat);
}

/// The nearest standing room inside `area` to `p`, clear of `placed`.
fn into_area(
    ctx: &MovementContext,
    area: Area,
    p: V2,
    placed: &[(V2, Option<Tier>, Option<Lean>, bool)],
    stands: &impl Fn(V2) -> bool,
) -> V2 {
    let d = p - area.centre;
    let edge = if d.length() > area.radius - 1.0 {
        area.centre + d.normalized() * (area.radius - 1.0).max(0.0)
    } else {
        p
    };
    let spacing = ctx.infantry.spacing_m / 2.0;
    arrangement::nearest_free(edge, area.radius, |q| {
        area.holds(q) && stands(q) && placed.iter().all(|o| (o.0 - q).length() >= spacing)
    })
    .unwrap_or(edge)
}

/// A soldier who cannot engage from his place steps to the nearest place
/// inside the area he can engage from, keeping the best cover he can
/// within `step_out_m` (D3, Q8); with none, he sits out where he is.
fn step_out(
    ctx: &MovementContext,
    side: &SideGeometry,
    known: &Known,
    stands: &impl Fn(V2) -> bool,
    fight: &Fight,
    area: Area,
    places: &mut [(V2, Option<Tier>, Option<Lean>, bool)],
) {
    let solid = |q: &Prop| q.blocks(MoverClass::Infantry) && side.knows(q, ctx.authored);
    let (c, r) = (&ctx.rules.cover, ctx.soldier_radius_m);
    let apart = ctx.infantry.spacing_m;
    for k in 0..places.len() {
        let (place, _, _, engages) = places[k];
        if engages {
            continue;
        }
        let others: Vec<V2> = (0..places.len())
            .filter(|&j| j != k)
            .map(|j| places[j].0)
            .collect();
        let leans: Vec<V2> = (0..places.len())
            .filter(|&j| j != k)
            .filter_map(|j| places[j].2.map(|l| l.at))
            .collect();
        let clear_of = |p: V2, d: f64| {
            others.iter().all(|q| (*q - p).length() >= d)
                && leans
                    .iter()
                    .all(|q| (*q - p).length() >= ctx.rules.cover.lean_apart_m)
        };
        let fits = |p: V2| {
            // Cheapest tests first.
            if !(area.holds(p)
                && clear_of(p, apart)
                && stands(p)
                && arrangement::reachable(ctx.world, place, p, r, &solid))
            {
                return None;
            }
            // Straight from here needs no lean points worked out.
            if fight.reaches(p, None) {
                return Some(None);
            }
            fight
                .leans(p, known.cover_body(p, fight.threat, c, r), stands)
                .into_iter()
                .find(|l| clear_of(l.at, ctx.rules.cover.lean_apart_m))
                .map(Some)
        };
        // No ring further out than the area's far edge (and a metre) holds a
        // place inside it.
        let far = (2.0 * area.radius).min((place - area.centre).length() + area.radius + 1.0);
        if let Some((p, lean)) = cover::step_out(place, fight.threat, known, c, r, far, &fits) {
            places[k] = (p, known.tier(p, fight.threat, c, r), lean, true);
        }
    }
}
