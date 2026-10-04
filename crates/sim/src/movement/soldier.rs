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
use crate::units::{Soldier, Unit};
use crate::world::{PropId, WorldGeometry};
use contract::random::Rng;

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

    /// The corridor point `ahead` metres on from share `t` of leg `leg`. It
    /// stops at the corridor's end.
    fn ahead(&self, leg: usize, t: f64, ahead: f64) -> V2 {
        let lane = self.lane(leg, t, ahead, 0.0);
        lane[lane.len() - 1]
    }

    /// First standing point ahead within a soldier's local rejoin reach.
    fn rejoin(
        &self,
        leg: usize,
        t: f64,
        step: f64,
        here: V2,
        reach: f64,
        mut stands: impl FnMut(V2) -> bool,
    ) -> Option<V2> {
        // Once the remaining arc is exhausted, ahead keeps returning the
        // endpoint. One extra sample covers rounding at an exact step boundary.
        let samples = (self.remaining(leg, t) / step).ceil() as usize + 1;
        (1..=samples)
            .map(|k| self.ahead(leg, t, step * k as f64))
            .take_while(|p| (*p - here).within_radius(reach))
            .find(|p| stands(*p))
    }

    /// His lane point `ahead` metres on from share `t` of leg `leg`, never
    /// past that leg's end: the leg shifted `offset` to its left. Held to his
    /// own leg, the point never swings round a waypoint onto the next leg's
    /// side, so he walks each leg's lane to its end before turning.
    fn lane_point(&self, leg: usize, t: f64, ahead: f64, offset: f64) -> V2 {
        let (a, b) = (self.start(leg), self.route[leg]);
        let length = (b - a).length();
        let dir = self.direction(leg);
        a + dir * (length * t + ahead).min(length) + left(dir) * offset
    }

    /// His lane from share `t` of leg `leg` to `ahead` metres on, as he
    /// walks it: at each waypoint it passes, the end of that leg's lane and
    /// the start of the next's, then the lane point `ahead` metres on.
    fn lane(&self, leg: usize, t: f64, ahead: f64, offset: f64) -> Vec<V2> {
        let (mut leg, mut left_m) = (leg, ahead);
        let mut at = self.start(leg) + (self.route[leg] - self.start(leg)) * t;
        let mut points = Vec::new();
        loop {
            let to = self.route[leg];
            let span = (to - at).length();
            let dir = self.direction(leg);
            if left_m <= span || leg == self.last() {
                points.push(at + dir * left_m.min(span) + left(dir) * offset);
                return points;
            }
            left_m -= span;
            at = to;
            leg += 1;
            points.push(to + left(dir) * offset);
            points.push(to + left(self.direction(leg)) * offset);
        }
    }

    /// Whether `p` has passed leg `leg`: its projection lies within
    /// [`PATH_REACHED_M`] of the leg's end, or beyond.
    fn passed(&self, leg: usize, p: V2) -> bool {
        let length = (self.route[leg] - self.start(leg)).length();
        (1.0 - self.along(leg, p)) * length < PATH_REACHED_M
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
        let speed = unit.drive_speed_mps;
        let mut left = speed.abs() * ctx.infantry.yield_horizon_s + hull.half.x;
        if speed < 0.0 {
            let backwards = v2(-1.0, 0.0).rotated(unit.yaw);
            return Some(Threat {
                half_width: hull.half.y,
                path: vec![here, here + backwards * left],
            });
        }
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
    /// Traffic visits only living squads; footprint radii are refreshed whenever
    /// movement or a hull shove changes their soldiers or settled centre.
    standing: Vec<usize>,
    radii: Vec<f64>,
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
    pub fn gather(units: &[Unit], radius: f64) -> Self {
        let mut base = Vec::with_capacity(units.len());
        let mut at = Vec::new();
        let mut standing_units = Vec::new();
        let mut radii = vec![0.0; units.len()];
        let mut buckets: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, u) in units.iter().enumerate() {
            base.push(at.len());
            let standing = !u.is_vehicle() && !u.garrisoned();
            if standing && u.alive() {
                standing_units.push(i);
                radii[i] = u.footprint_radius(radius);
            }
            for s in &u.members {
                let p = (standing && s.alive()).then(|| s.position.xy());
                if let Some(p) = p {
                    buckets.entry(bucket(p)).or_default().push(at.len() as u32);
                }
                at.push(p);
            }
        }
        Crowd {
            base,
            standing: standing_units,
            radii,
            at,
            buckets,
        }
    }

    pub(super) fn refresh_radius(&mut self, unit: &Unit, radius: f64) {
        self.radii[unit.id.0 as usize] = unit.footprint_radius(radius);
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
                        if (q - p).inside_radius(radius) {
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
        hulls: &[(Obb2, f64)],
    ) -> Around {
        let r = &ctx.infantry;
        let centre = unit.position.xy();
        let reach = unit.footprint_radius(ctx.soldier_radius_m)
            + (r.lane_lookahead_m + r.wander_m + r.spread_m)
                .max(r.window_m * std::f64::consts::FRAC_1_SQRT_2)
            + ENCOUNTER_RANGE_M;
        let mut bodies: Vec<Body> = hulls
            .iter()
            .filter(|(h, radius)| (h.center - centre).within_radius(*radius + reach))
            .map(|(h, radius)| Body {
                id: None,
                rect: *h,
                reach: *radius,
                known: true,
            })
            .collect();
        for p in ctx.world.props_near(centre, reach) {
            if p.blocks(MoverClass::Infantry) {
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
            .filter(|x| x.known && (x.rect.center - c).within_radius(x.reach + reach))
            .map(|x| x.rect)
            .collect()
    }

    /// Whether a soldier's disc of `radius` stands clear at `p` of every body
    /// his side plans with.
    fn stands(&self, p: V2, radius: f64) -> bool {
        self.bodies.iter().filter(|x| x.known).all(|x| {
            (x.rect.center - p).outside_radius(x.reach + radius) || !x.rect.contains(p, radius)
        })
    }

    /// Whether a soldier's disc may pass straight from `a` to `b` among the
    /// bodies his side plans with (one he stands in aside).
    fn clear(&self, a: V2, b: V2, radius: f64) -> bool {
        let r = radius - GRAZE_M;
        let mid = (a + b) * 0.5;
        let half = (b - a).length() / 2.0;
        self.bodies.iter().filter(|x| x.known).all(|x| {
            (x.rect.center - mid).outside_radius(x.reach + half + r)
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
    ctx.infantry.wander_m * libm::sin(std::f64::consts::TAU * (phase + t / period))
}

/// A soldier's share of the squad's speed this tick: it swings between
/// `1 - pace_variation` and full speed over about two thirds of his wander
/// period, from his seeded phase for this move, so each soldier surges and
/// drops back while nobody falls steadily behind.
fn stride(ctx: &MovementContext, s: &Soldier) -> f64 {
    let rules = ctx.infantry;
    let t = ctx.tick as f64 / ctx.tick_hz as f64;
    let swing =
        libm::sin(std::f64::consts::TAU * (s.pace + t / (rules.wander_period_s * 2.0 / 3.0)));
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
    let clear = |a: V2, b: V2| {
        let steps = ((b - a).length() / 0.5).ceil().max(1.0) as usize;
        around.clear(a, b, ctx.soldier_radius_m)
            && (1..=steps).all(|k| {
                let p = a + (b - a) * (k as f64 / steps as f64);
                ctx.world.traversable_at(p.x, p.y)
            })
    };
    let here = s.position.xy();
    let clear_by = ctx.soldier_radius_m + rules.yield_margin_m;
    if let Some(target) = threats.iter().find_map(|t| t.dodge(here, clear_by)) {
        return Some(Steer { target, pace: 1.0 });
    }
    let Some(corridor) = corridor else {
        // Holding: to his post (cover, or a step out to fire), on his own route.
        let post = holding_post(s)?;
        if s.path.last() != Some(&post) || stale(s, side, &clear) {
            plan_own(ctx, side, around, s, post, &[]);
        }
        return Some(follow(s, here, stride(ctx, s)));
    };
    let spot = s.spot?;
    if ctx.tick < s.start || (spot - here).inside_radius(1e-9) {
        return None;
    }
    let dt = 1.0 / ctx.tick_hz as f64;
    let pace = stride(ctx, s);
    let last = corridor.last();
    s.leg = s.leg.min(last);
    while s.leg < last && corridor.passed(s.leg, here) {
        s.leg += 1;
    }
    let t = corridor.along(s.leg, here).clamp(0.0, 1.0);
    let own_route = |s: &mut Soldier, side: &mut SideGeometry, to: V2| {
        plan_own(ctx, side, around, s, to, &[]);
    };
    if stale(s, side, &clear) {
        s.path.clear();
    }
    // His own route reaches only as far as its window.
    let reach = rules.window_m / 2.0 - 2.0;
    let near = (spot - here).within_radius(reach);
    if near
        && (corridor.remaining(s.leg, t) <= rules.final_leg_m
            || (spot - here).within_radius(rules.final_leg_m / 2.0))
    {
        // The final stretch: his own route to his spot.
        s.leg = last;
        if s.path.last() != Some(&spot) {
            own_route(s, side, spot);
        }
        return Some(follow(s, here, pace));
    }
    if s.path.len() == 1 && (s.path[0] - here).inside_radius(PATH_REACHED_M) {
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
    let lane = corridor.lane_point(s.leg, t, rules.steer_ahead_m, s.lateral);
    if clear(here, lane) {
        return Some(Steer { target: lane, pace });
    }
    // Else the furthest corridor point ahead in plain sight: off to one side
    // of a gap, he heads for its mouth.
    for share in [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0] {
        let p = corridor.ahead(s.leg, t, rules.steer_ahead_m * share);
        if (p - here).outside_radius(ON_SPOT_M) && clear(here, p) {
            return Some(Steer { target: p, pace });
        }
    }
    // Cut off from his lane and the corridor: find his own way back, to the
    // first corridor point ahead that is standing room within his window
    // (the corridor may run through a parked vehicle), else to his spot.
    let every = (REJOIN_EVERY_S * ctx.tick_hz as f64) as u64;
    if ctx.tick >= s.planned_at + every || s.planned_at == 0 {
        let r = ctx.soldier_radius_m;
        let rejoin = corridor
            .rejoin(s.leg, t, rules.steer_ahead_m, here, reach, |p| {
                around.stands(p, r)
            })
            .or(near.then_some(spot));
        if let Some(to) = rejoin {
            own_route(s, side, to);
            return Some(follow(s, here, pace));
        }
    }
    let on = corridor.ahead(s.leg, t, rules.steer_ahead_m);
    Some(Steer { target: on, pace })
}

fn holding_post(s: &Soldier) -> Option<V2> {
    s.post
        .filter(|p| (*p - s.position.xy()).at_least_radius(ON_SPOT_M))
}

/// Plan a soldier's own route from where he stands to `to` on the exact
/// bodies his side knows ([`final_leg`]), and round the soldiers standing
/// at `standing`; straight at it if none is found.
fn plan_own(
    ctx: &MovementContext,
    side: &mut SideGeometry,
    around: &Around,
    s: &mut Soldier,
    to: V2,
    standing: &[V2],
) {
    let rules = ctx.infantry;
    let here = s.position.xy();
    s.planned_at = ctx.tick;
    s.path_revision = side.revision;
    let solids = around.known_near(
        (here + to) * 0.5,
        rules.window_m * std::f64::consts::FRAC_1_SQRT_2,
    );
    let planned = final_leg(
        here,
        to,
        &solids,
        standing,
        ctx.soldier_radius_m,
        rules.window_m,
        |p| ctx.world.traversable_at(p.x, p.y),
    );
    side.searches += u64::from(planned.searched);
    s.path = planned.path.unwrap_or_else(|| vec![to]);
}

/// Whether a route of his own is one his side has since learned is blocked
/// (Q13, Q26): judged once per knowledge change.
fn stale(s: &mut Soldier, side: &SideGeometry, clear: &impl Fn(V2, V2) -> bool) -> bool {
    if s.path.is_empty() || s.path_revision == side.revision {
        return false;
    }
    s.path_revision = side.revision;
    let mut from = s.position.xy();
    !s.path.iter().all(|&p| {
        let ok = clear(from, p);
        from = p;
        ok
    })
}

/// The offset a soldier's lane takes: the one wanted if the lane from where
/// he stands on to `lane_lookahead_m` ahead, as he walks it (round each
/// waypoint too), crosses no body his side knows;
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
    let open = |offset: f64| {
        let mut at = corridor.lane_point(leg, t, rules.steer_ahead_m, offset);
        if !clear(here, at) {
            return false;
        }
        corridor
            .lane(leg, t, rules.lane_lookahead_m, offset)
            .into_iter()
            .all(|lane| {
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
    while s.path.len() > 1 && (s.path[0] - here).inside_radius(PATH_REACHED_M) {
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
    hulls: &[(Obb2, f64)],
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
    // Posts are for holding: a squad on the move walks its corridor and
    // keeps them (an attack-move's halt can lapse for a tick) until it
    // arrives or plans anew.
    let route = if advancing { unit.route.take() } else { None };
    let posted = unit.members.iter().any(|s| s.post.is_some());
    if route.is_none() && threats.is_empty() && !posted {
        return;
    }
    let clear_by = ctx.soldier_radius_m + ctx.infantry.yield_margin_m;
    if route.is_none()
        && !threats.is_empty()
        && unit.members.iter().filter(|s| s.alive()).all(|s| {
            holding_post(s).is_none()
                && threats
                    .iter()
                    .all(|t| t.dodge(s.position.xy(), clear_by).is_none())
        })
    {
        // A far moving hull cannot make stationary soldiers need local geometry.
        // Keep the same centroid update as the ordinary no-corridor tail.
        unit.settle();
        return;
    }
    let corridor = route.as_deref().map(|r| Corridor {
        from: unit.route_from,
        route: r,
    });
    let around = Around::gather(ctx, side, unit, hulls);
    // Suppression's tier slows infantry; it never turns them around (P14).
    let suppressed = 1.0
        - ctx
            .rules
            .suppression
            .penalties(unit.suppression)
            .map_or(0.0, |t| t.move_penalty);
    let personal = ctx.infantry.personal_space_m;
    let every = (REJOIN_EVERY_S * ctx.tick_hz as f64) as u64;
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
            arrived &= s.spot.is_none_or(|p| (p - here.xy()).inside_radius(1e-9));
            continue;
        };
        let speed = ctx.world.surface_at(here.x, here.y).map_or(0.0, |g| {
            mobility.speed(g.road_factor, g.forest, g.slope_deg)
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
        if velocity.outside_radius(speed) {
            velocity = velocity * (speed / velocity.length());
        }
        let wanted = here.xy() + velocity * dt;
        let clear_by = ctx.soldier_radius_m + ctx.infantry.yield_margin_m;
        let enters_traffic = threats
            .iter()
            .any(|t| t.dodge(here.xy(), clear_by).is_none() && t.dodge(wanted, clear_by).is_some());
        // Once clear, wait rather than stepping back into the approaching hull's path.
        let next = if enters_traffic {
            here
        } else {
            walk(ctx, side, &around, here, wanted)
                .map(|p| keep_apart(ctx, crowd, id, &around, here, p))
                .unwrap_or(here)
        };
        let s = &mut unit.members[k];
        s.velocity = (next.xy() - here.xy()) * (1.0 / dt);
        s.position = next;
        crowd.set(id, next.xy());
        if corridor.is_none() {
            if let Some(post) = s.post {
                let to_post = post - next.xy();
                let stuck = (next.xy() - here.xy()).inside_radius(1e-3);
                if to_post.inside_radius(ON_SPOT_M) {
                    s.position = post.with_z(next.z);
                    crowd.set(id, post);
                }
                if to_post.inside_radius(ON_SPOT_M) || (to_post.inside_radius(SETTLE_M) && stuck) {
                    s.post = None;
                    s.path.clear();
                } else if stuck && ctx.tick >= s.planned_at + every {
                    // Jammed on his way: someone stands in it (a squadmate
                    // at his place by a corner, a gap one man wide). He
                    // plans again round the soldiers about him.
                    let mut standing = Vec::new();
                    crowd.near(id, next.xy(), CROWD_BUCKET_M, |q| standing.push(q));
                    plan_own(ctx, side, &around, s, post, &standing);
                }
            }
            continue;
        }
        let Some(spot) = s.spot else {
            continue;
        };
        let to_spot = spot - next.xy();
        let homing = s.leg == corridor.map_or(0, |c| c.last()) && s.path.last() == Some(&spot);
        let stuck = (next.xy() - here.xy()).inside_radius(1e-3);
        if homing && to_spot.inside_radius(ON_SPOT_M) {
            s.position = spot.with_z(next.z);
            crowd.set(id, spot);
        } else if homing && to_spot.inside_radius(SETTLE_M) && stuck {
            // He can get no closer (someone stands there): here will do.
            s.spot = Some(next.xy());
        } else if stuck && ctx.tick >= s.planned_at + every {
            // A man may be jammed while his squadmates still advance. Find
            // his own way round the nearby soldiers, on the final stretch
            // or back to the corridor, without waiting for the whole squad.
            // A failed local target search is an attempt too.
            s.planned_at = ctx.tick;
            let mut standing = Vec::new();
            crowd.near(id, next.xy(), CROWD_BUCKET_M, |q| standing.push(q));
            let to = if homing {
                Some(spot)
            } else {
                s.path.last().copied().or_else(|| {
                    let c = corridor?;
                    let t = c.along(s.leg, next.xy()).clamp(0.0, 1.0);
                    let reach = ctx.infantry.window_m / 2.0 - 2.0;
                    c.rejoin(
                        s.leg,
                        t,
                        ctx.infantry.steer_ahead_m,
                        next.xy(),
                        reach,
                        |p| {
                            around.stands(p, ctx.soldier_radius_m)
                                && standing
                                    .iter()
                                    .all(|q| (*q - p).at_least_radius(2.0 * ctx.soldier_radius_m))
                        },
                    )
                })
            };
            if let Some(to) = to {
                plan_own(ctx, side, &around, s, to, &standing);
            }
        }
        let on = s
            .spot
            .is_some_and(|p| (p - s.position.xy()).inside_radius(1e-9));
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
    if ahead.outside_radius(1.0) {
        unit.yaw = libm::atan2(ahead.y, ahead.x);
    }
    if arrived {
        let end = *route.last().expect("a route ends somewhere");
        super::arrive(unit);
        super::take_cover::arrive(unit, end);
        for s in &mut unit.members {
            s.spot = None;
            s.post = None;
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
    if (wanted - from).inside_radius(1e-9) {
        return None;
    }
    let r = ctx.soldier_radius_m;
    let reach = (wanted - from).length() + r + ENCOUNTER_RANGE_M;
    let mut solids: Vec<Obb2> = Vec::new();
    for b in &around.bodies {
        if (b.rect.center - from).outside_radius(b.reach + reach) {
            continue;
        }
        if let Some(prop) = b.id.and_then(|id| ctx.world.prop(id)) {
            if b.rect.contains(from, r + ENCOUNTER_RANGE_M) {
                let resting = ctx.world.resting(prop.id, ctx.tick);
                side.learn(prop, ctx.authored, ctx.rules.pushing.relearn_m, resting);
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
    if solids.iter().any(|b| b.contains(next, r)) || (next - from).inside_radius(1e-9) {
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
    if (p - next.xy()).inside_radius(1e-12) {
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
pub(super) fn shove(ctx: &MovementContext, units: &mut [Unit], vehicle: usize, crowd: &mut Crowd) {
    let Some(hull) = units[vehicle].hull_box() else {
        return;
    };
    let r = ctx.soldier_radius_m;
    let reach = hull.half.length() + r;
    for k in 0..crowd.standing.len() {
        let j = crowd.standing[k];
        let unit = &mut units[j];
        if (unit.position.xy() - hull.center).outside_radius(reach + crowd.radii[j]) {
            continue;
        }
        let mut moved = false;
        for (k, s) in unit.members.iter_mut().enumerate() {
            if !s.alive() || !hull.contains(s.position.xy(), r) {
                continue;
            }
            let p = out_of(ctx.world, &hull, s.position.xy(), r);
            let z = ctx.world.surface_at(p.x, p.y).map_or(s.position.z, |g| g.z);
            s.position = p.with_z(z);
            crowd.set(crowd.id(j, k), p);
            moved = true;
        }
        if moved {
            unit.settle();
            crowd.refresh_radius(unit, r);
        }
    }
}

/// A shoved body now covers some standing soldiers: each steps out through
/// its nearest side, and out of any solid that leaves him in, unhurt.
pub fn clear_of(world: &WorldGeometry, units: &mut [Unit], body: &Obb2, r: f64) {
    let reach = body.half.length() + r;
    for unit in units.iter_mut() {
        if unit.is_vehicle() || unit.garrisoned() {
            continue;
        }
        if (unit.position.xy() - body.center).outside_radius(reach + unit.footprint_radius(r)) {
            continue;
        }
        let mut moved = false;
        for s in unit.members.iter_mut() {
            if !s.alive() || !body.contains(s.position.xy(), r) {
                continue;
            }
            let p = out_of(world, body, s.position.xy(), r);
            let z = world.surface_at(p.x, p.y).map_or(s.position.z, |g| g.z);
            s.position = p.with_z(z);
            moved = true;
        }
        if moved {
            unit.settle();
        }
    }
}

/// `p` pushed out of `rect` through its nearest side, then slid out of any
/// prop that stops infantry it lands in.
fn out_of(world: &WorldGeometry, rect: &Obb2, p: V2, r: f64) -> V2 {
    let mut p = rect.push_out(p, r);
    for _ in 0..SLIDE_PASSES {
        let mut clear = true;
        for q in world.props_near(p, r + 1.0) {
            if q.blocks(MoverClass::Infantry) && q.footprint().contains(p, r) {
                p = q.footprint().push_out(p, r);
                clear = false;
            }
        }
        if clear {
            break;
        }
    }
    p
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocked_nearby_corridor_end_exhausts_the_rejoin_search() {
        let route = [v2(65.0, 50.0)];
        let corridor = Corridor {
            from: v2(50.0, 50.0),
            route: &route,
        };
        let mut attempts = 0;
        let point = corridor.rejoin(0, 0.0, 3.0, corridor.from, 18.0, |_| {
            attempts += 1;
            // Fail the old infinite endpoint loop without hanging the test runner.
            assert!(attempts <= 6, "rejoin keeps retrying the blocked endpoint");
            false
        });
        assert_eq!(point, None);
    }
    #[test]
    fn rejoin_still_accepts_the_first_open_point_including_the_end() {
        let end = v2(65.0, 50.0);
        let route = [end];
        let corridor = Corridor {
            from: v2(50.0, 50.0),
            route: &route,
        };
        for t in [0.0, 0.2, 0.8, 1.0] {
            assert_eq!(
                corridor.rejoin(0, t, 3.0, corridor.from, 18.0, |p| p == end),
                Some(end)
            );
        }
        assert_eq!(
            corridor.rejoin(0, 0.0, 3.0, corridor.from, 18.0, |p| p.x >= 56.0),
            Some(v2(56.0, 50.0))
        );
    }
}
