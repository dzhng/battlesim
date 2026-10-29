//! The effects running, the spent ones kept for reuse, and how each kind
//! is born from its cause: seeded, so both clients start it alike.
use super::*;

/// The effects running, the spent ones, and how each is born.
#[derive(Default)]
pub(super) struct Pool {
    pub(super) live: Vec<Effect>,
    pub(super) spent: Vec<Effect>,
    /// Seconds a tick.
    pub(super) dt: f64,
}

impl Pool {
    pub(super) fn push(&mut self, look: Look, start: f64, end: f64) -> &mut Effect {
        let mut e = self.spent.pop().unwrap_or_default();
        e.path.clear();
        e.cum.clear();
        e.sparks.clear();
        e.length = 0.0;
        e.size = 0.0;
        e.rotation = 0.0;
        e.scale = 1.0;
        e.alpha = 1.0;
        e.frame = 0.0;
        e.look = look;
        e.start = start;
        e.end = end;
        e.span = end - start;
        self.live.push(e);
        self.live.last_mut().expect("just pushed")
    }

    /// A tracer over the tick from `t0` along `s`; returns its index.
    pub(super) fn tracer(
        &mut self,
        p: &EffectPresentation,
        t0: f64,
        s: &Segment,
        rng: &mut Mulberry32,
        visible: bool,
    ) -> usize {
        let style = &p.looks.get(&s.kind).tracer;
        let dt = self.dt;
        let e = self.push(
            Look::Tracer {
                style: style.clone(),
                visible,
            },
            t0,
            t0,
        );
        e.cause = format!("round:{}", s.kind).into();
        let mut total = 0.0;
        for (i, q) in s.path.iter().enumerate() {
            if i > 0 {
                let r = s.path[i - 1];
                total += crate::world::length([q[0] - r[0], q[1] - r[1], q[2] - r[2]]);
            }
            e.path.extend_from_slice(q);
            e.cum.push(total);
        }
        e.length = total;
        e.span = dt;
        // The tail: where the round was `tail_s` ago, at this stretch's speed.
        let [least, most] = style.tail_m;
        e.size = most.min(least.max(total / dt * style.tail_s));
        // Until the streak's tail has left the stretch's end.
        e.end = if total > 1e-6 {
            t0 + (dt * (1.0 + e.size / total)).min(dt + TRACER_TAIL_MAX_S)
        } else {
            t0
        };
        let index = self.live.len() - 1;
        if let (true, Some(trail)) = (visible && total > 1e-6, &style.smoke) {
            self.trail(t0, index, trail, rng);
        }
        index
    }

    /// A smoke trail along tracer `index`'s stretch: its ribbon, and a puff
    /// every `spacing_m` on a lattice along the stretch's heading, so one
    /// tick's puffs run on evenly from the last one's.
    pub(super) fn trail(
        &mut self,
        t0: f64,
        index: usize,
        style: &TrailStyle,
        rng: &mut Mulberry32,
    ) {
        let dt = self.dt;
        self.push(Look::Trail(*style), t0, t0 + dt + style.puff.life_s);
        let (done, ribbon) = self.live.split_at_mut(index + 1);
        let (e, ribbon) = (&done[index], ribbon.last_mut().expect("just pushed"));
        ribbon.path.extend_from_slice(&e.path);
        ribbon.cum.extend_from_slice(&e.cum);
        ribbon.length = e.length;
        let (path, spacing, length) = (&e.path, style.spacing_m, e.length);
        let first = e.cum[1].max(1e-6);
        let ahead = (path[0] * (path[3] - path[0])
            + path[1] * (path[4] - path[1])
            + path[2] * (path[5] - path[2]))
            / first;
        let past = ((ahead % spacing) + spacing) % spacing;
        let mut at = (spacing - past) % spacing;
        while at < length {
            let p = point_along(&self.live[index], at);
            self.puff(t0 + dt * at / length, p, &style.puff, 1.0, 1.0, rng);
            at += spacing;
        }
    }

    /// A flash at launch `l`'s muzzle; its look.
    pub(super) fn flash(
        &mut self,
        p: &EffectPresentation,
        t0: f64,
        l: &Launch,
        rng: &mut Mulberry32,
    ) -> Arc<FlashStyle> {
        let style = &p.looks.get(&l.kind).flash;
        let fireball = if style.fireball_m > 0.0 {
            style.fireball_s
        } else {
            0.0
        };
        let last = style
            .duration_s
            .max(fireball)
            .max(style.cast.map_or(0.0, |c| c.duration_s));
        let look = Look::Flash {
            style: style.clone(),
            shooter: l.shooter,
            mount: l.mount,
            soldier: l.soldier,
        };
        let e = self.push(look, t0, t0 + last);
        e.cause = format!("flash:{}", l.kind).into();
        e.p = l.at;
        e.n = l.dir;
        e.rotation = rng.sample() * PI * 2.0;
        style.clone()
    }

    /// The dust hull `u`'s shot `l` raises at `strength` of `style`'s, from
    /// `at`: shaken off the whole hull, from its flanks to its roof, end to
    /// end, and blown off the ground by the muzzle blast, from just behind
    /// the muzzle out to `reach_m` ahead of it and as wide as half that.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn shot_dust(
        &mut self,
        style: &ShotDustStyle,
        at: f64,
        l: &Launch,
        u: &Shooter,
        [hl, hw, hh]: Vec3,
        strength: f64,
        rng: &mut Mulberry32,
    ) {
        let (hull, ground) = (&style.hull, &style.ground);
        let [x, y, z] = u.position;
        let (s, c) = (libm::sin(u.yaw), libm::cos(u.yaw));
        let shaken = (hull.count * strength).round();
        let mut k = 0.0;
        while k < shaken {
            let along = (rng.sample() * 2.0 - 1.0) * hl * 0.9;
            let side = rng.sample() * 2.0 - 1.0;
            let up = 2.0 * hh * (0.2 + 0.8 * rng.sample());
            // Low down it comes off a flank; high up, off the roof.
            let across = if up < 2.0 * hh * 0.6 {
                side.signum() * hw * 0.95
            } else {
                side * hw * 0.9
            };
            let when = at + hull.over_s * (k / (shaken - 1.0).max(1.0));
            let p = [
                x + along * c - across * s,
                y + along * s + across * c,
                z + up,
            ];
            self.puff(when, p, &hull.puff, 1.0, 1.0, rng);
            k += 1.0;
        }
        // Level along the shot; straight up or down, along the hull.
        let [dx, dy, _] = l.dir;
        let level = libm::hypot(dx, dy);
        let (fx, fy) = if level > 1e-3 {
            (dx / level, dy / level)
        } else {
            (c, s)
        };
        let blown = (ground.burst.count * strength).round();
        let mut k = 0.0;
        while k < blown {
            let along = ground.reach_m * (rng.sample().sqrt() * 1.25 - 0.25);
            let across = (rng.sample() * 2.0 - 1.0) * ground.reach_m * 0.5;
            let when = at + ground.burst.over_s * (k / (blown - 1.0).max(1.0));
            let p = [
                l.at[0] + fx * along - fy * across,
                l.at[1] + fy * along + fx * across,
                z + 0.3,
            ];
            self.puff(when, p, &ground.burst.puff, 1.0, 1.0, rng);
            k += 1.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn impact(
        &mut self,
        p: &EffectPresentation,
        at: f64,
        point: Vec3,
        normal: Option<Vec3>,
        hit: &str,
        kind: &str,
        tracer: bool,
        rng: &mut Mulberry32,
    ) {
        let style = pick(&p.impacts, hit);
        let impact = &p.looks.get(kind).impact;
        let scale = impact.surfaces.get(hit);
        let end = at + style.duration_s * scale.map_or(1.0, |s| s.duration);
        let size = style.size_m * scale.map_or_else(|| impact.scale, |s| s.size);
        let e = self.push(
            Look::Impact {
                style: style.clone(),
                visible: tracer,
            },
            at,
            end,
        );
        e.cause = format!("impact:{hit}").into();
        e.p = point;
        e.n = normal.unwrap_or(UP);
        e.size = size;
        e.rotation = rng.sample() * PI * 2.0;
        e.frame = rng.sample(); // the puff's first frame
        let n = e.n;
        if tracer && style.sparks > 0.0 {
            self.sparks(&p.sparks, at, point, n, n, style.sparks, rng, 1.0);
        }
    }

    /// Sparks from `point`, thrown along `dir` and off the face `normal`; a
    /// blast's (`scale`) are faster, longer, thicker and longer-lived.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sparks(
        &mut self,
        style: &SparkStyle,
        at: f64,
        point: Vec3,
        normal: Vec3,
        dir: Vec3,
        count: f64,
        rng: &mut Mulberry32,
        scale: f64,
    ) {
        let e = self.push(Look::Sparks, at, at + style.duration_s * scale.sqrt());
        e.size = scale;
        // A hand's breadth off the face, so the face does not swallow them.
        e.p = along(point, normal, SURFACE_LIFT_M);
        let mut k = 0.0;
        while k < count {
            // A random direction, pulled toward the glance and off the face.
            let r: Vec3 = std::array::from_fn(|_| rng.sample() * 2.0 - 1.0);
            let v: Vec3 = std::array::from_fn(|i| dir[i] * 0.8 + normal[i] * 0.6 + r[i] * 0.6);
            let len = crate::world::length(v);
            let len = if len == 0.0 || len.is_nan() { 1.0 } else { len };
            let speed = style.speed_mps * scale * (0.4 + 0.6 * rng.sample());
            e.sparks.extend(v.map(|c| c / len * speed));
            k += 1.0;
        }
    }

    /// A blast's fireball, a share `sparks` of its style's sparks, and dirt and smoke.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn blast(
        &mut self,
        p: &EffectPresentation,
        at: f64,
        point: Vec3,
        radius: f64,
        scale: f64,
        sparks: f64,
        rng: &mut Mulberry32,
    ) {
        let style = &p.blast;
        let e = self.push(Look::Blast, at, at + style.duration_s);
        e.cause = "blast".into();
        e.p = point;
        e.size = radius * style.size_per_radius * scale;
        let size = e.size;
        let share = size / style.reference_size_m;
        e.rotation = rng.sample() * PI * 2.0;
        // A burst below the style's throws fewer sparks and puffs, and smaller ones.
        let fewer = share.min(1.0);
        let count = (style.sparks * sparks * fewer).round();
        if count != 0.0 {
            self.sparks(&p.sparks, at, point, UP, UP, count, rng, style.spark_scale);
        }
        let scale = share.sqrt();
        for burst in [&style.plume, &style.smoke] {
            let n = (burst.count * fewer).round();
            let mut k = 0.0;
            while k < n {
                let r = size * 0.3 * rng.sample().sqrt();
                let a = rng.sample() * PI * 2.0;
                let born = at + burst.over_s * (k / (burst.count - 1.0).max(1.0));
                let p = [
                    point[0] + libm::cos(a) * r,
                    point[1] + libm::sin(a) * r,
                    point[2] + size * 0.2,
                ];
                self.puff(born, p, &burst.puff, scale, 1.0, rng);
                k += 1.0;
            }
        }
    }

    /// A hull cooking off from `at`: each fireball out of its ring, then the
    /// sparks and dust its turret throws up where it lands.
    pub(super) fn cook_off(
        &mut self,
        p: &EffectPresentation,
        at: f64,
        c: &CookOff,
        rng: &mut Mulberry32,
    ) {
        let feel = &p.cook_off;
        let top = c.center[2] + c.height;
        for f in &feel.fireballs {
            let point = [c.center[0], c.center[1], top + f.up_m];
            let born = at + feel.delay_s + f.after_s;
            self.blast(p, born, point, f.radius_m, 1.0, f.sparks, rng);
        }
        let Some(landing) = c.landing else { return };
        let plume = &p.blast.plume;
        let landed = at + feel.impact_after();
        // Steel slamming onto steel: sparks off the deck, then the dust it shakes up.
        if feel.landing_sparks != 0.0 {
            self.sparks(
                &p.sparks,
                landed,
                landing,
                UP,
                UP,
                feel.landing_sparks,
                rng,
                1.0,
            );
        }
        let mut k = 0.0;
        while feel.landing_dust > 0.0 && k < plume.count {
            let r = feel.landing_dust * rng.sample().sqrt();
            let a = rng.sample() * PI * 2.0;
            let born = landed + plume.over_s * 0.5 * (k / (plume.count - 1.0).max(1.0));
            let at = [
                landing[0] + libm::cos(a) * r,
                landing[1] + libm::sin(a) * r,
                landing[2],
            ];
            self.puff(born, at, &plume.puff, feel.landing_dust, 1.0, rng);
            k += 1.0;
        }
    }

    /// A puff born at `at` at p: its own drift, spin and first frame.
    pub(super) fn puff(
        &mut self,
        at: f64,
        p: Vec3,
        style: &PuffStyle,
        scale: f64,
        alpha: f64,
        rng: &mut Mulberry32,
    ) {
        let e = self.push(Look::Puff(*style), at, at + style.life_s);
        e.scale = scale;
        e.alpha = alpha;
        e.p = p;
        let a = rng.sample() * PI * 2.0;
        let spread = style.spread_mps * scale * rng.sample();
        let rise = style.rise_mps * scale * (0.7 + 0.6 * rng.sample());
        e.n = [libm::cos(a) * spread, libm::sin(a) * spread, rise];
        e.rotation = rng.sample() * PI * 2.0;
        e.size = rng.sample() - 0.5; // its spin
        e.frame = rng.sample() * (DUST_FRAMES - PUFF_TURN_FRAMES - 1.0);
    }

    /// A flame born at `at`, at `strength` of its style (a fire dying down).
    pub(super) fn flame(
        &mut self,
        at: f64,
        p: Vec3,
        style: &FlameStyle,
        strength: f64,
        rng: &mut Mulberry32,
    ) {
        let e = self.push(Look::Flame(*style), at, at + style.life_s);
        e.scale = strength * (0.7 + 0.6 * rng.sample());
        e.p = p;
        e.rotation = rng.sample() * PI * 2.0;
        e.frame = 3.0 + rng.sample() * 3.0;
    }
}
