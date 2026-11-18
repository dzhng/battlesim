//! Swept per-tick flight, impacts, and expiry.

use super::config::subsegments_for;
use super::*;

/// Positions and velocities at the ends of `n` equal chords over `span`
/// seconds: `p + v·h + ½a·h²`, then `v + a·h`, exact under constant gravity.
pub(crate) fn chords(p: V3, v: V3, a: V3, span: f64, n: u32, out: &mut Vec<(V3, V3)>) {
    let h = span / n as f64;
    out.clear();
    out.push((p, v));
    let (mut p, mut v) = (p, v);
    for _ in 0..n {
        p = p + v * h + a * (0.5 * h * h);
        v = v + a * h;
        out.push((p, v));
    }
}

/// Fly every round one tick against the world and `bodies` (posed over this
/// same tick), each hit judged by `resolver` as it happens. Appends this
/// tick's events in time order; consumed and expired rounds leave the store.
pub fn advance_projectiles(
    store: &mut Projectiles,
    world: &WorldGeometry,
    bodies: &[Body],
    events: &mut Vec<FlightEvent>,
    resolver: &mut impl ImpactResolver,
) {
    let first = events.len();
    let Projectiles {
        config,
        active,
        flares,
        grid,
        ..
    } = store;
    grid.rebuild(world.width(), world.depth(), bodies);
    let mut scratch = Scratch::default();
    let flight = Flight {
        config,
        world,
        bodies,
        grid,
        flares,
    };
    // Projectile interception is checked before body standoffs. A short,
    // deterministic proximity window is enough for two physical missiles to
    // meet; later seeker work can replace this with continuous swept paths
    // without changing the resolver seam.
    let mut projectile_intercepts = std::collections::BTreeSet::new();
    for interceptor in active.iter() {
        if projectile_intercepts.contains(&interceptor.id) {
            continue;
        }
        let reach = resolver.projectile_interceptor_reach(interceptor.id);
        if !(reach.is_finite() && reach > 0.0) {
            continue;
        }
        for target in active.iter() {
            if target.id == interceptor.id || projectile_intercepts.contains(&target.id) {
                continue;
            }
            let relative_position = interceptor.position - target.position;
            let relative_velocity = interceptor.velocity - target.velocity;
            let speed2 = relative_velocity.dot(relative_velocity);
            let closest_s = if speed2 > 0.0 {
                (-relative_position.dot(relative_velocity) / speed2).clamp(0.0, config.tick_s)
            } else {
                0.0
            };
            let closest = relative_position + relative_velocity * closest_s;
            if closest.length() <= reach && resolver.intercept_projectile(interceptor.id, target.id)
            {
                projectile_intercepts.insert(interceptor.id);
                projectile_intercepts.insert(target.id);
                break;
            }
        }
    }
    // Protection decisions consume shared unit resources, so evaluate imminent
    // swept hits first and resolve by physical event time, then projectile id.
    let mut candidates: Vec<_> = active
        .iter()
        .filter_map(|p| flight.interception(p, &mut scratch, resolver))
        .collect();
    candidates.sort_by(|a, b| {
        a.time
            .total_cmp(&b.time)
            .then(a.projectile.cmp(&b.projectile))
    });
    let intercepted: std::collections::BTreeMap<_, _> = candidates
        .into_iter()
        .filter(|event| resolver.intercept(event))
        .map(|event| (event.projectile, event))
        .collect();
    scratch.passes.clear();
    scratch.misses.clear();
    active.retain_mut(|p| {
        if projectile_intercepts.contains(&p.id) {
            return false;
        }
        if let Some(event) = intercepted.get(&p.id) {
            // Trace only to the premature detonation, preserving ordinary
            // pass-through and near-miss evidence before that point.
            let mut traced = p.clone();
            let span = event.time * config.tick_s;
            let tick_span = config.tick_s.min(traced.lifetime_s - traced.age_s);
            let gravity = flight.acceleration(&mut traced, tick_span);
            scratch.misses.clear();
            let _ = flight.fly_leg(&traced, &mut scratch, gravity, span, 0.0, None, None);
            events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
            events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
            events.push(FlightEvent::Impact(Impact {
                projectile: p.id,
                struck: Struck::Terrain,
                point: event.point,
                normal: V3::default(),
                velocity: event.velocity,
                time: event.time,
                bounces: p.bounces,
                pose: None,
                detonated: true,
            }));
            false
        } else {
            flight.fly_tick(p, &mut scratch, events, resolver)
        }
    });
    flares.retain(|_, flare| {
        let decoy = &mut flare.decoy;
        flare.from = decoy.position;
        *decoy = decoy.after(config.tick_s);
        if decoy.burn_s <= 0.0 {
            return false;
        }
        world
            .height_at(decoy.position.x, decoy.position.y)
            .is_some_and(|ground| decoy.position.z > ground)
    });
    events[first..].sort_by(|a, b| {
        let (ka, kb) = (a.order_key(), b.order_key());
        ka.0.total_cmp(&kb.0)
            .then(ka.1.cmp(&kb.1))
            .then(ka.2.cmp(&kb.2))
            .then(ka.3.cmp(&kb.3))
    });
}

#[derive(Default)]
struct Scratch {
    path: Vec<(V3, V3)>,
    path_times: Vec<f64>,
    accelerations: Vec<V3>,
    leg: Vec<(V3, V3)>,
    candidates: Vec<u32>,
    motions: Vec<(usize, sweep::Motion)>,
    misses: Vec<NearMiss>,
    passes: Vec<Pass>,
    crossed: Vec<(f64, PropId)>,
}

/// What every round flies against this tick.
struct Flight<'a> {
    config: &'a FlightConfig,
    world: &'a WorldGeometry,
    bodies: &'a [Body],
    grid: &'a BodyGrid,
    flares: &'a std::collections::BTreeMap<ProjectileId, Flare>,
}

/// The earliest hit along one leg of a round's flight.
struct Hit {
    struck: Struck,
    /// Index of the struck body in the tick's bodies.
    body: Option<usize>,
    point: V3,
    normal: V3,
    velocity: V3,
    time: f64,
}

impl Flight<'_> {
    fn acceleration(&self, p: &mut Projectile, span: f64) -> V3 {
        if p.guidance.is_some_and(|g| !p.tracks(g.point, None)) {
            // A missed aim behind the flight direction ends this attempt;
            // the live round continues under its ordinary physical forces.
            p.guidance = None;
        }
        let gravity = match p.guidance {
            Some(g) => {
                p.velocity = p
                    .velocity
                    .turned_toward(g.aim(p.position) - p.position, g.turn_rad_s * span);
                V3::default()
            }
            None => self.config.gravity * p.gravity_scale,
        };
        gravity
            + p.motor
                .map_or(V3::default(), |motor| motor.thrust(p.velocity, span))
    }

    fn interception(
        &self,
        original: &Projectile,
        scratch: &mut Scratch,
        resolver: &impl ImpactResolver,
    ) -> Option<Interception> {
        let max_reach = resolver.max_standoff(original.id);
        if max_reach <= 0.0 {
            return None;
        }
        let mut p = original.clone();
        let tick_span = self.config.tick_s.min(p.lifetime_s - p.age_s);
        let gravity = self.acceleration(&mut p, tick_span);
        // Look only one standoff travel ahead of this tick. Guidance and body
        // motion are re-evaluated on every ordinary advancement.
        let span =
            (tick_span + max_reach / p.velocity.length().max(1.0)).min(p.lifetime_s - p.age_s);
        scratch.passes.clear();
        scratch.misses.clear();
        let hit = self.fly_leg(&p, scratch, gravity, span, 0.0, None, None)?;
        let body = &self.bodies[hit.body?];
        let reach = resolver.standoff(p.id, body)?;
        for k in 0..scratch.accelerations.len() {
            let s0 = scratch.path_times[k] / self.config.tick_s;
            let s1 = scratch.path_times[k + 1] / self.config.tick_s;
            if s0 > hit.time || s0 > 1.0 {
                break;
            }
            let end = ((hit.time - s0) / (s1 - s0)).min(1.0);
            let motion = sweep::Motion::new(body, s0, s1);
            let ((a0, v0), (a1, _)) = (scratch.path[k], scratch.path[k + 1]);
            if let Some(u) = sweep::standoff_entry(&body.shape, &motion, a0, a1, end, reach) {
                let time = s0 + (s1 - s0) * u;
                if time > 1.0 {
                    return None;
                }
                return Some(Interception {
                    projectile: p.id,
                    body: body.id,
                    unit: body.unit,
                    point: a0 + (a1 - a0) * u,
                    velocity: v0 + scratch.accelerations[k] * (u * (s1 - s0) * self.config.tick_s),
                    time,
                    pose: body.pose_at(time),
                });
            }
        }
        None
    }
    /// Returns whether the round is still in flight.
    fn fly_tick(
        &self,
        p: &mut Projectile,
        scratch: &mut Scratch,
        events: &mut Vec<FlightEvent>,
        resolver: &mut impl ImpactResolver,
    ) -> bool {
        let config = self.config;
        let span = config.tick_s.min(p.lifetime_s - p.age_s);
        if let (Some(flare), Some(g)) = (
            p.decoy.and_then(|id| self.flares.get(&id)),
            p.guidance.as_mut(),
        ) {
            let decoy = flare.decoy;
            g.point = decoy.after(span).position;
        }
        let gravity = self.acceleration(p, span);
        let fuse = resolver.target_fuse(p.id);
        scratch.misses.clear();
        // Each leg flies from the round's state `flown` seconds into the tick;
        // a ricochet starts the next leg at its hit, clear of the body it
        // glanced off.
        let (mut flown, mut glanced) = (0.0, None);
        while let Some(hit) = self.fly_leg(p, scratch, gravity, span, flown, glanced, fuse) {
            events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
            let pose = hit.body.map(|i| self.bodies[i].pose_at(hit.time));
            let decision = resolver.resolve(&ImpactContext {
                projectile: p.id,
                bounces: p.bounces,
                struck: hit.struck,
                point: hit.point,
                normal: hit.normal,
                velocity: hit.velocity,
                pose,
            });
            if let (ImpactDecision::Bounce { velocity }, Struck::Body(body), Some(pose)) =
                (decision, hit.struck, pose)
            {
                events.push(FlightEvent::Ricochet(Ricochet {
                    projectile: p.id,
                    body,
                    point: hit.point,
                    normal: hit.normal,
                    velocity: hit.velocity,
                    deflected: velocity,
                    time: hit.time,
                    bounces: p.bounces,
                    pose,
                }));
                if p.bounces == 0 && config.ricochet_lifetime_s > 0.0 {
                    p.lifetime_s = p
                        .lifetime_s
                        .min(p.age_s + hit.time * config.tick_s + config.ricochet_lifetime_s);
                }
                p.fall = None;
                p.bounces += 1;
                p.position = hit.point;
                p.velocity = velocity;
                flown = hit.time * config.tick_s;
                glanced = Some(body);
                continue;
            }
            events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
            events.push(FlightEvent::Impact(Impact {
                projectile: p.id,
                struck: hit.struck,
                point: hit.point,
                normal: hit.normal,
                velocity: hit.velocity,
                time: hit.time,
                bounces: p.bounces,
                pose,
                detonated: decision == ImpactDecision::Detonate,
            }));
            return false;
        }
        events.extend(scratch.passes.drain(..).map(FlightEvent::Pass));
        events.extend(scratch.misses.drain(..).map(FlightEvent::NearMiss));
        let (end, v_end) = *scratch.path.last().expect("path has its start point");
        p.position = end;
        p.velocity = v_end;
        p.age_s += span;
        let expire = |cause| {
            FlightEvent::Expired(Expired {
                projectile: p.id,
                cause,
                point: end,
                time: span / config.tick_s,
            })
        };
        if p.age_s >= p.lifetime_s - TIME_EPSILON_S {
            events.push(expire(Expiry::Lifetime));
            return false;
        }
        let leaving = |x: f64, vx: f64, hi: f64| (x < 0.0 && vx <= 0.0) || (x > hi && vx >= 0.0);
        let world = self.world;
        if leaving(end.x, v_end.x, world.width()) || leaving(end.y, v_end.y, world.depth()) {
            events.push(expire(Expiry::LeftMap));
            return false;
        }
        true
    }

    /// Fly the round from its state `flown` seconds into the tick to the
    /// tick's `span`, in chords, against the world and every body but
    /// `glanced`. Returns the earliest hit, leaving the flown path in
    /// `scratch.path` and folding each unit's closest pass into
    /// `scratch.misses`, so a round passes a unit at most once a tick however
    /// often it glances.
    #[allow(clippy::too_many_arguments)]
    fn fly_leg(
        &self,
        p: &Projectile,
        scratch: &mut Scratch,
        gravity: V3,
        span: f64,
        flown: f64,
        glanced: Option<BodyId>,
        fuse: Option<(UnitId, f64)>,
    ) -> Option<Hit> {
        let Flight {
            config,
            world,
            bodies,
            grid,
            ..
        } = *self;
        let rest = span - flown;
        let split = p
            .fall
            .map_or(rest, |l| (l.after_s - p.age_s - flown).clamp(0.0, rest));
        scratch.path.clear();
        scratch.path_times.clear();
        scratch.accelerations.clear();
        scratch.path.push((p.position, p.velocity));
        scratch.path_times.push(flown);
        for (start, duration, acceleration) in [
            (flown, split, gravity),
            (
                flown + split,
                rest - split,
                p.fall.map_or(gravity, |f| config.gravity * f.gravity_scale),
            ),
        ] {
            if duration <= TIME_EPSILON_S {
                continue;
            }
            let (position, velocity) = *scratch.path.last().unwrap();
            let n = subsegments_for(acceleration.length(), duration, config.chord_error_m);
            chords(
                position,
                velocity,
                acceleration,
                duration,
                n,
                &mut scratch.leg,
            );
            for (k, endpoint) in scratch.leg.iter().enumerate().skip(1) {
                scratch.path.push(*endpoint);
                scratch
                    .path_times
                    .push(start + duration * k as f64 / n as f64);
                scratch.accelerations.push(acceleration);
            }
        }
        // Bodies near this leg's path, out to the near-miss reach.
        let reach = p.suppression_radius_m;
        let query_reach = fuse.map_or(reach, |(_, radius)| reach.max(radius));
        let (mut lo, mut hi) = (p.position.xy(), p.position.xy());
        for (q, _) in &scratch.path {
            lo = v2(lo.x.min(q.x), lo.y.min(q.y));
            hi = v2(hi.x.max(q.x), hi.y.max(q.y));
        }
        let pad = v2(query_reach, query_reach);
        grid.query(lo - pad, hi + pad, &mut scratch.candidates);
        // A unit's rounds never strike its own bodies (a squad keeps its own
        // fire lanes), and a ricochet never meets the body it glanced off.
        let shooter = p.shooter;
        let past = shooter.and_then(|s| match s.cover {
            Some(Struck::Prop(id)) => Some(id),
            _ => None,
        });
        scratch.candidates.retain(|&i| {
            let body = &bodies[i as usize];
            shooter.is_none_or(|s| s.unit != body.unit && s.cover != Some(Struck::Body(body.id)))
                && glanced != Some(body.id)
        });
        for k in 0..scratch.accelerations.len() {
            let chord_s = scratch.path_times[k + 1] - scratch.path_times[k];
            let acceleration = scratch.accelerations[k];
            let ((a0, v0), (a1, _)) = (scratch.path[k], scratch.path[k + 1]);
            let (s0, s1) = (
                scratch.path_times[k] / config.tick_s,
                scratch.path_times[k + 1] / config.tick_s,
            );
            let chord = a1 - a0;
            let len = chord.length();
            let mut best: Option<(f64, Struck, V3, Option<usize>)> = None;
            if len > 0.0 {
                if let Some(hit) = world.raycast_past(a0, chord * (1.0 / len), len, past) {
                    let struck = match hit.collider {
                        Collider::Terrain => Struck::Terrain,
                        Collider::Prop(id) => Struck::Prop(id),
                    };
                    best = Some((hit.t / len, struck, hit.normal, None));
                }
            }
            // A proximity fuse of the decoy's meeting distance follows the
            // flare's swept motion.
            // Earlier terrain or body collisions still win, so a flare never
            // grants immunity to an aircraft already in the missile's path.
            if let Some(flare) = p.decoy.and_then(|id| self.flares.get(&id)) {
                let decoy = flare.decoy;
                let t0 = scratch.path_times[k];
                let t1 = scratch.path_times[k + 1];
                let d0 = decoy.after(t0).position;
                let d1 = decoy.after(t1).position;
                let relative = a0 - d0;
                let travel = chord - (d1 - d0);
                let aa = travel.dot(travel);
                let bb = relative.dot(travel);
                let cc = relative.dot(relative) - decoy.meet_m * decoy.meet_m;
                let determinant = bb * bb - aa * cc;
                let entry = if cc <= 0.0 {
                    Some(0.0)
                } else if aa > 0.0 && determinant >= 0.0 {
                    let u = (-bb - determinant.sqrt()) / aa;
                    (0.0..=1.0).contains(&u).then_some(u)
                } else {
                    None
                };
                if let Some(u) = entry {
                    if best.is_none_or(|(earlier, ..)| u < earlier) {
                        best = Some((u, Struck::Terrain, v3(0.0, 0.0, 1.0), None));
                    }
                }
            }
            scratch.motions.clear();
            scratch.motions.extend(scratch.candidates.iter().map(|&i| {
                let body = &bodies[i as usize];
                (i as usize, sweep::Motion::new(body, s0, s1))
            }));
            if let Some((target, radius)) = fuse {
                for (i, motion) in &scratch.motions {
                    let body = &bodies[*i];
                    if body.unit != target {
                        continue;
                    }
                    if let Some(u) = sweep::standoff_entry(&body.shape, motion, a0, a1, 1.0, radius)
                    {
                        let point = a0 + chord * u;
                        let pose = body.pose_at(s0 + (s1 - s0) * u);
                        let centre = match body.shape {
                            Shape::Box { half } => pose.base + v3(0.0, 0.0, half.z),
                            Shape::Capsule { height, .. } => pose.base + v3(0.0, 0.0, height * 0.5),
                        };
                        let normal = (point - centre).normalized();
                        let struck = Struck::Body(body.id);
                        if best.is_none_or(|(bu, bs, ..)| u < bu || (u == bu && struck < bs)) {
                            best = Some((u, struck, normal, Some(*i)));
                        }
                    }
                }
            }
            for (i, motion) in &scratch.motions {
                let body = &bodies[*i];
                if let Some((u, normal)) = sweep::entry(&body.shape, motion, a0, a1) {
                    let struck = Struck::Body(body.id);
                    if best.is_none_or(|(bu, bs, ..)| u < bu || (u == bu && struck < bs)) {
                        best = Some((u, struck, normal, Some(*i)));
                    }
                }
            }
            let u_end = best.map_or(1.0, |b| b.0);
            // Bodies it flies through on the way (not past its hit).
            if len > 0.0 {
                scratch.crossed.clear();
                world.passes(
                    a0,
                    chord * (1.0 / len),
                    len * u_end,
                    past,
                    &mut scratch.crossed,
                );
                scratch
                    .passes
                    .extend(scratch.crossed.iter().map(|&(t, prop)| Pass {
                        projectile: p.id,
                        prop,
                        point: a0 + chord * (t / len),
                        along: chord * (1.0 / len),
                        time: s0 + (s1 - s0) * (t / len),
                    }));
            }
            for (i, motion) in &scratch.motions {
                let body = &bodies[*i];
                if best.is_some_and(|b| b.1 == Struck::Body(body.id)) {
                    continue;
                }
                let Some((distance, u)) = sweep::closest_approach(
                    &body.shape,
                    motion,
                    body.footprint_radius(),
                    a0,
                    a1,
                    u_end,
                    reach,
                ) else {
                    continue;
                };
                let miss = NearMiss {
                    projectile: p.id,
                    unit: body.unit,
                    body: body.id,
                    distance,
                    point: a0 + chord * u,
                    time: s0 + (s1 - s0) * u,
                };
                match scratch.misses.iter_mut().find(|m| m.unit == body.unit) {
                    Some(m)
                        if distance < m.distance
                            || (distance == m.distance && body.id < m.body) =>
                    {
                        *m = miss
                    }
                    Some(_) => {}
                    None => scratch.misses.push(miss),
                }
            }
            if let Some((u, struck, normal, body)) = best {
                return Some(Hit {
                    struck,
                    body,
                    point: a0 + chord * u,
                    normal,
                    velocity: v0 + acceleration * (u * chord_s),
                    time: s0 + (s1 - s0) * u,
                });
            }
        }
        None
    }
}
