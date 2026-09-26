//! Soldiers walking as themselves, Company of Heroes style (Q6, Q10, Q23).
//! A squad plans one corridor on the coarse grid; each soldier then walks
//! his own lane beside it: the corridor shifted by his spot's side offset,
//! drifting with a seeded wander, at his own seeded pace after his own
//! seeded start. His lane narrows toward the corridor wherever it would
//! cross a body his side knows (the mock's lesson). For the final stretch,
//! or to find the corridor again, he plans his own route on the exact bodies
//! ([`super::final_leg`]). He keeps a soft personal space from everyone, never
//! overlaps another soldier, and steps aside from a vehicle about to run him
//! down; one that reaches him shoves him aside unhurt.
use std::collections::HashMap;

use contract::map::MoverClass;

use super::final_leg::final_leg;
use super::{MovementContext, SideGeometry, ENCOUNTER_RANGE_M};
use crate::math::{v2, Obb2, V2, V3};
use crate::rng::Rng;
use crate::units::{Soldier, Unit};
use crate::world::PropId;

/// Times a soldier's step is pushed out of the solids it meets.
const SLIDE_PASSES: usize = 3;
/// A soldier's own route is followed point by point, each passed this close.
const PATH_REACHED_M: f64 = 0.3;
/// A soldier closer than this to his spot steps onto it.
const ON_SPOT_M: f64 = 0.05;
/// A soldier this close to his spot who cannot get closer stands there.
const SETTLE_M: f64 = 1.0;
/// Sight lines graze a face they slide along: tests of a clear way use a
/// disc this much smaller than the body.
const GRAZE_M: f64 = 0.05;
/// A soldier plans his own way back to the corridor at most this often.
const REJOIN_EVERY_S: f64 = 1.0;
/// Strength of personal space and how much of it turns to the right, so
/// two soldiers meeting head-on pass rather than stop.
const PERSONAL_PUSH: f64 = 1.0;
const PASS_RIGHT: f64 = 0.5;

/// A squad's corridor: its remaining waypoints and where it runs from.
#[derive(Clone, Copy)]
pub struct Corridor<'a> {
    pub from: V2,
    pub route: &'a [V2],
}

impl Corridor<'_> {
    fn start(&self, leg: usize) -> V2 {
        if leg == 0 {
            self.from
        } else {
            self.route[leg - 1]
        }
    }

    fn last(&self) -> usize {
        self.route.len() - 1
    }

    fn end(&self) -> V2 {
        self.route[self.last()]
    }

    /// Where `p` projects along leg `leg`, as a share of it (unclamped).
    fn along(&self, leg: usize, p: V2) -> f64 {
        let (a, b) = (self.start(leg), self.route[leg]);
        let d = b - a;
        let l2 = d.dot(d);
        if l2 < 1e-12 {
            1.0
        } else {
            (p - a).dot(d) / l2
        }
    }

    fn direction(&self, leg: usize) -> V2 {
        let d = self.route[leg] - self.start(leg);
        if d.length() < 1e-9 {
            if leg > 0 {
                return self.direction(leg - 1);
            }
            return v2(1.0, 0.0);
        }
        d.normalized()
    }

    /// The corridor point `ahead` metres on from share `t` of leg `leg`, and
    /// the corridor's direction there. It stops at the corridor's end.
    fn ahead(&self, leg: usize, t: f64, ahead: f64) -> (V2, V2) {
        let (mut leg, mut left) = (leg, ahead);
        let mut at = self.start(leg) + (self.route[leg] - self.start(leg)) * t;
        loop {
            let to = self.route[leg];
            let span = (to - at).length();
            if left <= span {
                let dir = self.direction(leg);
                return (at + dir * left, dir);
            }
            if leg == self.last() {
                return (to, self.direction(leg));
            }
            left -= span;
            at = to;
            leg += 1;
        }
    }

    /// Metres of corridor left from share `t` of leg `leg`.
    fn remaining(&self, leg: usize, t: f64) -> f64 {
        let at = self.start(leg) + (self.route[leg] - self.start(leg)) * t;
        let mut total = (self.route[leg] - at).length();
        for k in leg + 1..self.route.len() {
            total += (self.route[k] - self.route[k - 1]).length();
        }
        total
    }
}

/// Left of `d`.
fn left(d: V2) -> V2 {
    v2(-d.y, d.x)
}

/// A live vehicle on the move, as soldiers see it coming (Q23): its hull's
/// half width and the path its centre will run over the next
/// `yield_horizon_s`, reaching on to its nose.
pub struct Threat {
    pub half_width: f64,
    pub path: Vec<V2>,
}

impl Threat {
    /// The threat a moving vehicle poses, or `None` for one standing still.
    pub fn of(ctx: &MovementContext, unit: &Unit) -> Option<Threat> {
        let hull = unit.hull_box()?;
        let route = unit.route.as_ref().filter(|r| !r.is_empty())?;
        if unit.halted() || !unit.may_translate() {
            return None;
        }
        let here = unit.position.xy();
        let speed = ctx.world.surface_at(here.x, here.y).map_or(0.0, |s| {
            let road = matches!(
                s.kind,
                crate::world::SurfaceKind::Road | crate::world::SurfaceKind::Bridge
            );
            unit.mobility.speed(road, s.forest, s.slope_deg)
        });
        let mut left = speed * ctx.infantry.yield_horizon_s + hull.half.x;
        let mut path = vec![here];
        let mut at = here;
        for &w in route {
            let span = (w - at).length();
            if span >= left {
                if span > 1e-9 {
                    path.push(at + (w - at) * (left / span));
                }
                left = 0.0;
                break;
            }
            path.push(w);
            left -= span;
            at = w;
        }
        if left > 0.0 {
            // The route ends: the hull still reaches its nose past the end.
            let dir = v2(1.0, 0.0).rotated(unit.yaw);
            path.push(at + dir * hull.half.x.min(left));
        }
        Some(Threat {
            half_width: hull.half.y,
            path,
        })
    }

    /// Where a soldier at `p` must step to be clear, if its path would cross
    /// him: straight out from its path, on the side he already stands.
    fn dodge(&self, p: V2, clear: f64) -> Option<V2> {
        let mut best: Option<(f64, V2, V2)> = None;
        for w in self.path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let l2 = d.dot(d);
            let t = if l2 < 1e-12 {
                0.0
            } else {
                ((p - a).dot(d) / l2).clamp(0.0, 1.0)
            };
            let q = a + d * t;
            let gap = (p - q).length();
            if best.is_none_or(|(g, _, _)| gap < g) {
                best = Some((gap, q, d));
            }
        }
        let (gap, q, d) = best?;
        let reach = self.half_width + clear;
        if gap >= reach {
            return None;
        }
        let out = if gap > 1e-6 {
            (p - q) * (1.0 / gap)
        } else {
            left(d.normalized())
        };
        Some(q + out * (reach + 0.3))
    }
}

/// Every standing soldier on the field, for personal space: positions kept
/// current as each soldier steps, found through 2 m buckets of where each
/// stood at the start of the tick (no one moves further in a tick).
pub struct Crowd {
    /// First entry of each unit's soldiers.
    base: Vec<usize>,
    at: Vec<Option<V2>>,
    buckets: HashMap<(i32, i32), Vec<u32>>,
}

const CROWD_BUCKET_M: f64 = 2.0;

fn bucket(p: V2) -> (i32, i32) {
    (
        (p.x / CROWD_BUCKET_M).floor() as i32,
        (p.y / CROWD_BUCKET_M).floor() as i32,
    )
}

impl Crowd {
    pub fn gather(units: &[Unit]) -> Self {
        let mut base = Vec::with_capacity(units.len());
        let mut at = Vec::new();
        let mut buckets: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for u in units {
            base.push(at.len());
            let standing = !u.is_vehicle() && !u.garrisoned();
            for s in &u.members {
                let p = (standing && s.alive()).then(|| s.position.xy());
                if let Some(p) = p {
                    buckets.entry(bucket(p)).or_default().push(at.len() as u32);
                }
                at.push(p);
            }
        }
        Crowd { base, at, buckets }
    }

    fn id(&self, unit: usize, member: usize) -> usize {
        self.base[unit] + member
    }

    fn set(&mut self, id: usize, p: V2) {
        if let Some(slot) = self.at.get_mut(id) {
            if slot.is_some() {
                *slot = Some(p);
            }
        }
    }

    /// Other standing soldiers within `radius` of `p`.
    fn near(&self, id: usize, p: V2, radius: f64, mut each: impl FnMut(V2)) {
        let (bi, bj) = bucket(p);
        for j in bj - 1..=bj + 1 {
            for i in bi - 1..=bi + 1 {
                for &k in self.buckets.get(&(i, j)).into_iter().flatten() {
                    let k = k as usize;
                    if k == id {
                        continue;
                    }
                    if let Some(q) = self.at[k] {
                        if (q - p).length() < radius {
                            each(q);
                        }
                    }
                }
            }
        }
    }
}

/// A body near a squad: a prop that stops infantry or a live hull.
struct Body {
    id: Option<PropId>,
    rect: Obb2,
    reach: f64,
    /// The side plans with it (a hull always).
    known: bool,
}

/// The bodies around one squad this tick, gathered once for all its
/// soldiers: every prop that stops infantry and every live hull within
/// reach of its lanes and its soldiers' own routes.
pub struct Around {
    bodies: Vec<Body>,
}

impl Around {
    fn gather(
        ctx: &MovementContext,
        side: &SideGeometry,
        unit: &Unit,
        hulls: &[Obb2],
    ) -> Around {
        let r = &ctx.infantry;
        let centre = unit.position.xy();
        let reach = unit.footprint_radius()
            + (r.lane_lookahead_m + r.wander_m + r.spread_m)
                .max(r.window_m * std::f64::consts::FRAC_1_SQRT_2)
            + ENCOUNTER_RANGE_M;
        let mut bodies: Vec<Body> = hulls
            .iter()
            .filter(|h| (h.center - centre).length() <= h.half.length() + reach)
            .map(|h| Body {
                id: None,
                rect: *h,
                reach: h.half.length(),
                known: true,
            })
            .collect();
        for p in ctx.world.props_near(centre, reach) {
            if p.kind.blocks(MoverClass::Infantry) {
                bodies.push(Body {
                    id: Some(p.id),
                    rect: p.footprint(),
                    reach: p.footprint().half.length(),
                    known: side.knows(p, ctx.authored),
                });
            }
        }
        Around { bodies }
    }

    /// The boxes his side plans with near `c`, for a soldier's own route.
    fn known_near(&self, c: V2, reach: f64) -> Vec<Obb2> {
        self.bodies
            .iter()
            .filter(|x| x.known && (x.rect.center - c).length() <= x.reach + reach)
            .map(|x| x.rect)
            .collect()
    }

    /// Whether a soldier's disc of `radius` stands clear at `p` of every body
    /// his side plans with.
    fn stands(&self, p: V2, radius: f64) -> bool {
        self.bodies
            .iter()
            .filter(|x| x.known)
            .all(|x| (x.rect.center - p).length() > x.reach + radius || !x.rect.contains(p, radius))
    }

    /// Whether a soldier's disc may pass straight from `a` to `b` among the
    /// bodies his side plans with (one he stands in aside).
    fn clear(&self, a: V2, b: V2, radius: f64) -> bool {
        let r = radius - GRAZE_M;
        let mid = (a + b) * 0.5;
        let half = (b - a).length() / 2.0;
        self.bodies.iter().filter(|x| x.known).all(|x| {
            (x.rect.center - mid).length() > x.reach + half + r
                || !x.rect.meets_segment(a, b, r)
                || x.rect.contains(a, r)
        })
    }
}

/// What a soldier does this tick: head for `target` at `pace` of his speed.
pub struct Steer {
    pub target: V2,
    pub pace: f64,
}

/// A soldier's seeded wander: its phase and period, fixed for the battle.
fn wander(ctx: &MovementContext, unit: u32, soldier: u32) -> f64 {
    let mut rng = Rng::new(ctx.seed ^ (u64::from(unit) << 32) ^ u64::from(soldier) ^ 0x5eed_1a9e);
    let (phase, stretch) = (rng.unit(), rng.unit());
    let period = ctx.infantry.wander_period_s * (0.75 + 0.5 * stretch);
    let t = ctx.tick as f64 / ctx.tick_hz as f64;
    ctx.infantry.wander_m * (std::f64::consts::TAU * (phase + t / period)).sin()
}

/// A soldier's share of the squad's speed this tick: it swings between
/// `1 - pace_variation` and full speed over about two thirds of his wander
/// period, from his seeded phase for this move, so each soldier surges and
/// drops back while nobody falls steadily behind.
fn stride(ctx: &MovementContext, s: &Soldier) -> f64 {
    let rules = ctx.infantry;
    let t = ctx.tick as f64 / ctx.tick_hz as f64;
    let swing = (std::f64::consts::TAU * (s.pace + t / (rules.wander_period_s * 2.0 / 3.0))).sin();
    1.0 - rules.pace_variation * 0.5 * (1.0 + swing)
}

/// Where one soldier heads this tick (Q6, Q23): clear of a vehicle about to
/// cross him; else, on the move, along his own route (the final stretch to
/// his spot, or back to the corridor) or his lane beside the squad's
/// corridor. `None`: he stands where he is.
pub fn soldier_steer(
    ctx: &MovementContext,
    side: &mut SideGeometry,
    around: &Around,
    s: &mut Soldier,
    unit: u32,
    corridor: Option<Corridor>,
    threats: &[Threat],
) -> Option<Steer> {
    let rules = ctx.infantry;
    let clear = |a: V2, b: V2| around.clear(a, b, ctx.soldier_radius_m);
    let here = s.position.xy();
    let clear_by = ctx.soldier_radius_m + rules.yield_margin_m;
    if let Some(target) = threats.iter().find_map(|t| t.dodge(here, clear_by)) {
        return Some(Steer { target, pace: 1.0 });
    }
    let corridor = corridor?;
    let spot = s.spot?;
    if ctx.tick < s.start || (spot - here).length() < 1e-9 {
        return None;
    }
    let dt = 1.0 / ctx.tick_hz as f64;
    let pace = stride(ctx, s);
    let last = corridor.last();
    let mut t = corridor.along(s.leg.min(last), here);
    s.leg = s.leg.min(last);
    while t >= 1.0 && s.leg < last {
        s.leg += 1;
        t = corridor.along(s.leg, here);
    }
    let t = t.clamp(0.0, 1.0);
    let remaining = corridor.remaining(s.leg, t);
    let own_route = |s: &mut Soldier, side: &mut SideGeometry, to: V2| {
        side.searches += 1;
        s.planned_at = ctx.tick;
        s.path_revision = side.revision;
        let solids = around.known_near(
            (here + to) * 0.5,
            rules.window_m * std::f64::consts::FRAC_1_SQRT_2,
        );
        s.path = final_leg(
            here,
            to,
            &solids,
            ctx.soldier_radius_m,
            rules.window_m,
            |p| {
                ctx.world
                    .surface_at(p.x, p.y)
                    .is_some_and(|g| g.traversable)
            },
        )
        .unwrap_or_else(|| vec![to]);
    };
    // A route of his own that his side has since learned is blocked is
    // planned again (Q13, Q26).
    if !s.path.is_empty() && s.path_revision != side.revision {
        s.path_revision = side.revision;
        let mut from = here;
        let open = s.path.iter().all(|&p| {
            let ok = clear(from, p);
            from = p;
            ok
        });
        if !open {
            s.path.clear();
        }
    }
    // His own route reaches only as far as its window.
    let reach = rules.window_m / 2.0 - 2.0;
    let near = (spot - here).length() <= reach;
    if near && (remaining <= rules.final_leg_m || (spot - here).length() <= rules.final_leg_m / 2.0)
    {
        // The final stretch: his own route to his spot.
        s.leg = last;
        if s.path.last() != Some(&spot) {
            own_route(s, side, spot);
        }
        return Some(follow(s, here, pace));
    }
    if s.path.len() == 1 && (s.path[0] - here).length() < PATH_REACHED_M {
        s.path.clear(); // back on the corridor
    }
    if !s.path.is_empty() {
        // On his way back to the corridor.
        return Some(follow(s, here, pace));
    }
    // His lane: the corridor shifted by his spot's side offset and his
    // wander, narrowed toward the corridor wherever it would cross a body.
    let base = (spot - corridor.end()).dot(left(corridor.direction(last)));
    let wanted = base + wander(ctx, unit, s.id);
    let offset = lane_offset(&corridor, s.leg, t, here, wanted, rules, &clear);
    let step = rules.lane_shift_mps * dt;
    s.lateral += (offset - s.lateral).clamp(-step, step);
    let (on, dir) = corridor.ahead(s.leg, t, rules.steer_ahead_m);
    let lane = on + left(dir) * s.lateral;
    if clear(here, lane) {
        return Some(Steer {
            target: lane,
            pace,
        });
    }
    // Else the furthest corridor point ahead in plain sight: off to one side
    // of a gap, he heads for its mouth.
    for share in [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0] {
        let (p, _) = corridor.ahead(s.leg, t, rules.steer_ahead_m * share);
        if (p - here).length() > ON_SPOT_M && clear(here, p) {
            return Some(Steer {
                target: p,
                pace,
            });
        }
    }
    // Cut off from his lane and the corridor: find his own way back, to the
    // first corridor point ahead that is standing room within his window
    // (the corridor may run through a parked vehicle), else to his spot.
    let every = (REJOIN_EVERY_S * ctx.tick_hz as f64) as u64;
    if ctx.tick >= s.planned_at + every || s.planned_at == 0 {
        let r = ctx.soldier_radius_m;
        let rejoin = (1..)
            .map(|k| corridor.ahead(s.leg, t, rules.steer_ahead_m * k as f64).0)
            .take_while(|p| (*p - here).length() <= reach)
            .find(|p| around.stands(*p, r))
            .or(Some(spot).filter(|_| near));
        if let Some(to) = rejoin {
            own_route(s, side, to);
            return Some(follow(s, here, pace));
        }
    }
    Some(Steer {
        target: on,
        pace,
    })
}

/// The offset a soldier's lane takes: the one wanted if the lane from where
/// he stands on to `lane_lookahead_m` ahead crosses no body his side knows;
/// else the nearest to it that does, shifted up to 3 m either way (so
/// each man picks his own gap in a line of teeth), or halved; else the
/// corridor itself.
fn lane_offset(
    corridor: &Corridor,
    leg: usize,
    t: f64,
    here: V2,
    wanted: f64,
    rules: &contract::scenario::InfantryMovementRules,
    clear: &dyn Fn(V2, V2) -> bool,
) -> f64 {
    let marks = [
        rules.steer_ahead_m,
        rules.lane_lookahead_m / 2.0,
        rules.lane_lookahead_m,
    ];
    let points: Vec<(V2, V2)> = marks
        .iter()
        .map(|&d| corridor.ahead(leg, t, d))
        .collect();
    let open = |offset: f64| {
        let mut at = here;
        points.iter().all(|&(c, d)| {
            let lane = c + left(d) * offset;
            let ok = clear(at, lane);
            at = lane;
            ok
        })
    };
    [0.0, 0.5, -0.5, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0]
        .iter()
        .map(|shift| wanted + shift)
        .chain([wanted * 0.5])
        .find(|&offset| open(offset))
        .unwrap_or(0.0)
}

/// Head for the next point of his own route, dropping those he has reached.
fn follow(s: &mut Soldier, here: V2, pace: f64) -> Steer {
    while s.path.len() > 1 && (s.path[0] - here).length() < PATH_REACHED_M {
        s.path.remove(0);
    }
    Steer {
        target: s.path[0],
        pace,
    }
}

/// Walk every soldier of a squad one tick: each steers for himself
/// ([`soldier_steer`]), eases away from anyone in his personal space, and
/// steps as a body. On the move, the squad passes a corridor waypoint once
/// every soldier has, and arrives once every soldier stands on his spot.
#[allow(clippy::too_many_arguments)]
pub(super) fn step_squad(
    ctx: &MovementContext,
    unit: &mut Unit,
    index: usize,
    side: &mut SideGeometry,
    hulls: &[Obb2],
    threats: &[Threat],
    crowd: &mut Crowd,
    advancing: bool,
) {
    let dt = 1.0 / ctx.tick_hz as f64;
    for s in &mut unit.members {
        s.velocity = V2::default();
    }
    if unit.garrison.is_some() {
        return;
    }
    let route = if advancing { unit.route.take() } else { None };
    if route.is_none() && threats.is_empty() {
        return;
    }
    let corridor = route.as_deref().map(|r| Corridor {
        from: unit.route_from,
        route: r,
    });
    let around = Around::gather(ctx, side, unit, hulls);
    // Suppression slows infantry; it never turns them around (P14).
    let suppressed = (1.0 - ctx.suppression_move_penalty * unit.suppression).max(0.0);
    let personal = ctx.infantry.personal_space_m;
    let (unit_id, mobility) = (unit.id.0, unit.mobility);
    let mut arrived = true;
    for k in 0..unit.members.len() {
        if !unit.members[k].alive() {
            continue;
        }
        let id = crowd.id(index, k);
        let s = &mut unit.members[k];
        let here = s.position;
        let steer = soldier_steer(ctx, side, &around, s, unit_id, corridor, threats);
        let Some(Steer { target, pace }) = steer else {
            arrived &= s.spot.is_none_or(|p| (p - here.xy()).length() < 1e-9);
            continue;
        };
        let speed = ctx.world.surface_at(here.x, here.y).map_or(0.0, |g| {
            let road = matches!(
                g.kind,
                crate::world::SurfaceKind::Road | crate::world::SurfaceKind::Bridge
            );
            mobility.speed(road, g.forest, g.slope_deg)
        }) * suppressed
            * pace;
        let to = target - here.xy();
        let distance = to.length();
        let mut velocity = if distance > 1e-9 {
            to * ((speed.min(distance / dt)) / distance)
        } else {
            V2::default()
        };
        // Personal space (Q10): ease away from anyone closer, passing on
        // the right of whoever stands ahead.
        let heading = if distance > 1e-9 {
            to * (1.0 / distance)
        } else {
            V2::default()
        };
        let mut push = V2::default();
        crowd.near(id, here.xy(), personal, |q| {
            let d = here.xy() - q;
            let gap = d.length().max(1e-6);
            let weight = (personal - gap) / personal;
            push = push + d * (weight / gap);
            if heading.dot(q - here.xy()) > 0.0 {
                push = push - left(heading) * (weight * PASS_RIGHT);
            }
        });
        velocity = velocity + push * (PERSONAL_PUSH * speed);
        if velocity.length() > speed {
            velocity = velocity * (speed / velocity.length());
        }
        let wanted = here.xy() + velocity * dt;
        let next = walk(ctx, side, &around, here, wanted)
            .map(|p| keep_apart(ctx, crowd, id, &around, here, p))
            .unwrap_or(here);
        let s = &mut unit.members[k];
        s.velocity = (next.xy() - here.xy()) * (1.0 / dt);
        s.position = next;
        crowd.set(id, next.xy());
        let Some(spot) = s.spot.filter(|_| corridor.is_some()) else {
            continue;
        };
        let to_spot = (spot - next.xy()).length();
        let homing = s.leg == corridor.map_or(0, |c| c.last()) && s.path.last() == Some(&spot);
        if homing && to_spot < ON_SPOT_M {
            s.position = spot.with_z(next.z);
            crowd.set(id, spot);
        } else if homing && to_spot < SETTLE_M && (next.xy() - here.xy()).length() < 1e-3 {
            // He can get no closer (someone stands there): here will do.
            s.spot = Some(next.xy());
        }
        let on = s.spot.is_some_and(|p| (p - s.position.xy()).length() < 1e-9);
        arrived &= on;
    }
    let Some(mut route) = route else {
        unit.settle();
        return;
    };
    // The stall watch: every living soldier's distance to his spot, so a
    // man still waiting to set off counts as much as one on his way.
    let remaining: f64 = unit
        .members
        .iter()
        .filter(|s| s.alive())
        .filter_map(|s| s.spot.map(|p| (p - s.position.xy()).length()))
        .sum();
    let passed = unit
        .members
        .iter()
        .filter(|s| s.alive())
        .map(|s| s.leg)
        .min()
        .unwrap_or(0);
    if passed > 0 {
        unit.route_from = route[passed - 1];
        route.drain(..passed);
        for s in &mut unit.members {
            s.leg = s.leg.saturating_sub(passed);
        }
        unit.progress = (f64::INFINITY, ctx.tick);
    } else if remaining < unit.progress.0 - super::PROGRESS_EPSILON_M {
        unit.progress = (remaining, ctx.tick);
    }
    unit.state = contract::observation::MoveState::Moving;
    unit.blocker = None;
    unit.settle();
    // The squad faces its next waypoint while it is still some way off.
    let ahead = route[0] - unit.position.xy();
    if ahead.length() > 1.0 {
        unit.yaw = ahead.y.atan2(ahead.x);
    }
    if arrived {
        super::arrive(unit);
        for s in &mut unit.members {
            s.spot = None;
            s.leg = 0;
            s.path.clear();
        }
    } else {
        unit.route = Some(route);
    }
}

/// One soldier's step from `here` to `wanted`, as a body (L4, L5): his disc
/// slides along every prop that stops infantry and every live hull, onto
/// traversable ground, at the ground's height under him. A prop met here
/// becomes known to his side. `None` when he cannot move at all.
fn walk(
    ctx: &MovementContext,
    side: &mut SideGeometry,
    around: &Around,
    here: V3,
    wanted: V2,
) -> Option<V3> {
    let from = here.xy();
    if (wanted - from).length() < 1e-9 {
        return None;
    }
    let r = ctx.soldier_radius_m;
    let reach = (wanted - from).length() + r + ENCOUNTER_RANGE_M;
    let mut solids: Vec<Obb2> = Vec::new();
    for b in &around.bodies {
        if (b.rect.center - from).length() > b.reach + reach {
            continue;
        }
        if let Some(id) = b.id {
            if id >= ctx.authored && b.rect.contains(from, r + ENCOUNTER_RANGE_M) {
                side.learn(id);
            }
        }
        // A soldier may always step out of a solid he already stands in.
        if !b.rect.contains(from, r) {
            solids.push(b.rect);
        }
    }
    let mut next = wanted;
    for _ in 0..SLIDE_PASSES {
        let mut clear = true;
        for b in &solids {
            if b.contains(next, r) {
                next = b.push_out(next, r);
                clear = false;
            }
        }
        if clear {
            break;
        }
    }
    if solids.iter().any(|b| b.contains(next, r)) || (next - from).length() < 1e-9 {
        return None;
    }
    ctx.world
        .surface_at(next.x, next.y)
        .filter(|g| g.traversable)
        .map(|g| next.with_z(g.z))
}

/// A step never ends inside another soldier's disc, unless it takes him
/// further out of one he already overlaps; pushed out of their discs, it
/// must still be clear of every body, else he stays.
fn keep_apart(
    ctx: &MovementContext,
    crowd: &Crowd,
    id: usize,
    around: &Around,
    here: V3,
    next: V3,
) -> V3 {
    let r = ctx.soldier_radius_m;
    let apart = 2.0 * r;
    let mut p = next.xy();
    for _ in 0..2 {
        let mut moved = false;
        crowd.near(id, p, apart, |q| {
            let now = (p - q).length();
            if now < (here.xy() - q).length() {
                let out = if now > 1e-9 {
                    (p - q) * (1.0 / now)
                } else {
                    (here.xy() - q).normalized()
                };
                p = q + out * (apart + 1e-6);
                moved = true;
            }
        });
        if !moved {
            break;
        }
    }
    if (p - next.xy()).length() < 1e-12 {
        return next;
    }
    let inside = around
        .bodies
        .iter()
        .any(|b| b.rect.contains(p, r) && !b.rect.contains(here.xy(), r));
    let mut blocked = false;
    crowd.near(id, p, apart, |q| {
        blocked |= (p - q).length() < (here.xy() - q).length();
    });
    match ctx.world.surface_at(p.x, p.y).filter(|g| g.traversable) {
        Some(g) if !inside && !blocked => p.with_z(g.z),
        _ => here,
    }
}

/// A vehicle that has just moved shoves every standing soldier its hull now
/// covers out through its nearest side, unhurt, and out of any solid that
/// leaves him in (Q23): vehicles never stop for soldiers.
pub(super) fn shove(
    ctx: &MovementContext,
    units: &mut [Unit],
    vehicle: usize,
    crowd: &mut Crowd,
) {
    let Some(hull) = units[vehicle].hull_box() else {
        return;
    };
    let r = ctx.soldier_radius_m;
    let reach = hull.half.length() + r;
    for (j, unit) in units.iter_mut().enumerate() {
        if j == vehicle || unit.is_vehicle() || unit.garrisoned() {
            continue;
        }
        if (unit.position.xy() - hull.center).length() > reach + unit.footprint_radius() {
            continue;
        }
        let mut moved = false;
        for (k, s) in unit.members.iter_mut().enumerate() {
            if !s.alive() || !hull.contains(s.position.xy(), r) {
                continue;
            }
            let mut p = hull.push_out(s.position.xy(), r);
            for _ in 0..SLIDE_PASSES {
                let mut clear = true;
                for q in ctx.world.props_near(p, r + 1.0) {
                    if q.kind.blocks(MoverClass::Infantry) && q.footprint().contains(p, r) {
                        p = q.footprint().push_out(p, r);
                        clear = false;
                    }
                }
                if clear {
                    break;
                }
            }
            let z = ctx
                .world
                .surface_at(p.x, p.y)
                .map_or(s.position.z, |g| g.z);
            s.position = p.with_z(z);
            crowd.set(crowd.id(j, k), p);
            moved = true;
        }
        if moved {
            unit.settle();
        }
    }
}

/// Give each living soldier of a squad that has just planned `route` from
/// `from` his lane state: the corridor's first leg, and the side offset he
/// stands at from it, so nobody jumps onto the corridor.
pub(super) fn join(unit: &mut Unit, from: V2, route: &[V2]) {
    let corridor = Corridor { from, route };
    let dir = corridor.direction(0);
    for s in unit.members.iter_mut().filter(|s| s.alive()) {
        s.leg = 0;
        s.lateral = (s.position.xy() - from).dot(left(dir));
        if s.spot.is_none_or(|spot| s.path.last() != Some(&spot)) {
            s.path.clear();
        }
    }
}
