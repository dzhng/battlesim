//! Combat effects: what a side's publications say happened
//! becomes short-lived effects, and each frame those become the effect
//! pass's instances at the presentation clock:
//!
//! - a tracer for every visible stretch of flight, styled by its round kind
//!   (a glow and a hot core along a tail fading behind the round, the round
//!   itself where it is seen as an object, and a smoke trail where its row
//!   has one), running along the stretch over the tick that flew it;
//! - a muzzle flash for every launch, on the drawn muzzle that fired (a
//!   [`MuzzleSource`]), else at the published launch point; a hull's heavy
//!   gun also shakes dust off the whole hull and blows it off the ground
//!   ahead of the muzzle (`FlashStyle::dust`);
//! - an impact puff where a stretch ends in a hit, and from a tracer, sparks,
//!   a hot flash and its light; sparks at every ricochet corner;
//! - a fireball for every blast, and the dirt and smoke it throws up;
//! - dust behind a hull the side sees move, by the distance it covers, and
//!   smoke behind a smoking one, by time;
//! - flames and a smoke column over every smoke source the side knows,
//!   burning, then smouldering, then out, all thinned alike past `smoke_budget`;
//! - a watched hull's cook-off: fireballs out of its ring, then sparks and
//!   dust where its thrown turret lands;
//! - every drawn rotor as a film camera's shutter sees it turn.
//!
//! Every effect has a published cause and a bounded life, and runs on the
//! presentation clock, so a pause holds them and a jump of the clock makes
//! only what would still be alive. A publication is taken once: the same
//! tick again is ignored, an earlier one starts over. Each effect is seeded
//! and summed as the browser does it, so both clients draw the same floats.
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use super::cook_off::GRAVITY;
use super::launches::{Launch, LaunchTracker};
use super::lights::CastLightList;
use super::publication::{CookOff, EffectPublication, Segment, Shooter};
use super::tables::{
    pick, CastStyle, EffectPresentation, FlameStyle, FlashStyle, ImpactStyle, PuffStyle,
    RotorBlurStyle, ShotDustStyle, SmokeSourceStyle, SparkStyle, TracerLight, TracerStyle,
    TrailStyle, TRACER_TAIL_MAX_S,
};
use super::{DrawnRotor, MuzzleSource};
use crate::seed::{Key, Mulberry32, GOLDEN};
use crate::world::Vec3;

// ---- The instances the effect pass draws (16 floats each). ----

/// Floats per instance: a, b, colour, misc (vec4 each).
pub const EFFECT_FLOATS: usize = 16;
/// Instance shapes (misc.x): a streak from a to b (a lit smoke ribbon when
/// its colour carries an opacity), a glow at a (a solid disc when its colour
/// carries an opacity), a flipbook sprite at a, a rotor's blur.
pub const SHAPE_NAMES: [&str; 4] = ["streak", "glow", "flipbook", "rotor"];
const STREAK: f64 = 0.0;
const GLOW: f64 = 1.0;
const FLIPBOOK: f64 = 2.0;
const ROTOR: f64 = 3.0;
/// Flipbook layers in the effect atlas, and each one's frames.
const FIRE: f64 = 0.0;
const DUST: f64 = 1.0;
const FIRE_FRAMES: f64 = 25.0;
const DUST_FRAMES: f64 = 64.0;

/// What the effect pass draws this frame, and the lights it casts.
pub struct EffectBatch {
    pub data: Vec<f32>,
    pub count: usize,
    /// Instances that did not fit this frame.
    pub dropped: usize,
    pub lights: CastLightList,
}

impl EffectBatch {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity * EFFECT_FLOATS],
            count: 0,
            dropped: 0,
            lights: CastLightList::default(),
        }
    }

    /// One instance in the shader's layout (a, b, colour, misc: a vec4
    /// each); dropped when the batch is full.
    fn put(&mut self, rows: [[f64; 4]; 4]) {
        let o = self.count * EFFECT_FLOATS;
        if o >= self.data.len() {
            self.dropped += 1;
            return;
        }
        for (d, x) in self.data[o..o + EFFECT_FLOATS]
            .iter_mut()
            .zip(rows.as_flattened())
        {
            *d = *x as f32;
        }
        self.count += 1;
    }

    /// An additive streak from a (`along[0]` of the way from tail to head)
    /// to b: a sharp line, or a soft glow. A negative width is screen pixels.
    #[allow(clippy::too_many_arguments)]
    fn streak(
        &mut self,
        a: Vec3,
        b: Vec3,
        width: f64,
        min_px: f64,
        c: Vec3,
        k: f64,
        along: [f64; 2],
        soft: bool,
        zoom_boost: f64,
    ) {
        let misc = match (zoom_boost != 0.0, soft) {
            (true, true) => -zoom_boost,
            (true, false) => zoom_boost,
            (false, soft) => f64::from(u8::from(soft)),
        };
        let shape = [STREAK, along[0], along[1], misc];
        self.put([v4(a, width), v4(b, min_px), v4(times(c, k), 0.0), shape]);
    }

    /// An additive glow of `radius` metres at p, with `rays` of star.
    #[allow(clippy::too_many_arguments)]
    fn glow(&mut self, p: Vec3, radius: f64, min_px: f64, c: Vec3, k: f64, turn: f64, rays: f64) {
        let shape = [GLOW, 0.0, 0.0, 0.0];
        self.put([
            v4(p, radius),
            [turn, min_px, rays, 0.0],
            v4(times(c, k), 0.0),
            shape,
        ]);
    }

    /// A solid disc at p in `c` at `opacity`, blended over: a round seen as an object.
    fn disc(&mut self, p: Vec3, radius: f64, min_px: f64, c: Vec3, opacity: f64) {
        let shape = [GLOW, 0.0, 0.0, 0.0];
        let colour = v4(times(c, opacity), opacity);
        self.put([v4(p, radius), [0.0, min_px, 0.0, 0.0], colour, shape]);
    }

    /// A lit smoke ribbon from a to b, cut square, blended over.
    fn ribbon(&mut self, a: Vec3, b: Vec3, width: f64, min_px: f64, albedo: Vec3, opacity: f64) {
        let shape = [STREAK, 1.0, 1.0, 0.0];
        self.put([v4(a, width), v4(b, min_px), v4(albedo, opacity), shape]);
    }

    /// A flipbook sprite at p: `layer`'s (fractional) `frame` times `tint`,
    /// lifted by `emissive`, fading into what it meets over `soft_m`; `lit`:
    /// `tint` is an albedo the sun lights (smoke, dust), else fire's own colour.
    #[allow(clippy::too_many_arguments)]
    fn flipbook(
        &mut self,
        p: Vec3,
        radius: f64,
        turn: f64,
        layer: f64,
        frame: f64,
        tint: Vec3,
        opacity: f64,
        emissive: f64,
        soft_m: f64,
        lit: bool,
    ) {
        let shape = [FLIPBOOK, emissive, f64::from(u8::from(lit)), 0.0];
        self.put([
            v4(p, radius),
            [turn, frame, layer, soft_m],
            v4(tint, opacity),
            shape,
        ]);
    }
}

/// `v` with `w` as its fourth.
fn v4(v: Vec3, w: f64) -> [f64; 4] {
    [v[0], v[1], v[2], w]
}

/// A colour times an intensity.
fn times(c: Vec3, k: f64) -> Vec3 {
    c.map(|x| x * k)
}

// ---- Effects in flight. ----

/// Each effect kind's name, in [`EffectCounts::running`] order.
pub const EFFECT_KINDS: [&str; 8] = [
    "tracer", "flash", "impact", "sparks", "blast", "puff", "flame", "trail",
];

/// What a frame drew: effects running by kind, instances by shape, lights.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectCounts {
    pub running: [usize; 8],
    pub instances: [usize; 4],
    pub lights: usize,
}

/// The effects still running, last drawn and dropped, and the smoke sources
/// still burning or smouldering, as of the last publication taken.
#[derive(Clone, Copy, Debug)]
pub struct EffectStats {
    pub live: usize,
    pub instances: usize,
    pub dropped: usize,
    pub sources: usize,
    pub tick: f64,
}

/// How far off a face sparks start, metres.
const SURFACE_LIFT_M: f64 = 0.08;
/// Frames a puff's flipbook turns through over its life.
const PUFF_TURN_FRAMES: f64 = 24.0;
/// A puff fades in over its first quarter second.
const PUFF_FADE_IN: f64 = 4.0;
/// A trail's ribbon's width over its puffs' diameter.
const RIBBON_WIDTH: f64 = 0.7;
const UP: Vec3 = [0.0, 0.0, 1.0];

/// What an effect is, with what it draws from.
#[derive(Clone, Debug, Default)]
enum Look {
    Tracer {
        style: Arc<TracerStyle>,
        visible: bool,
    },
    Flash {
        style: Arc<FlashStyle>,
        shooter: f64,
        mount: usize,
        soldier: Option<f64>,
    },
    /// An impact; `visible`: the round that ended was a tracer.
    Impact {
        style: Arc<ImpactStyle>,
        visible: bool,
    },
    #[default]
    Sparks,
    Blast,
    Puff(PuffStyle),
    Flame(FlameStyle),
    Trail(TrailStyle),
}

impl Look {
    fn kind(&self) -> usize {
        match self {
            Look::Tracer { .. } => 0,
            Look::Flash { .. } => 1,
            Look::Impact { .. } => 2,
            Look::Sparks => 3,
            Look::Blast => 4,
            Look::Puff(_) => 5,
            Look::Flame(_) => 6,
            Look::Trail(_) => 7,
        }
    }
}

/// One effect; spent ones are kept for reuse, so their lists keep their room.
#[derive(Clone, Debug, Default)]
struct Effect {
    look: Look,
    start: f64,
    end: f64,
    /// Seconds it runs its course over (a tracer: one tick).
    span: f64,
    /// A point, a direction and a size (meaning per kind).
    p: Vec3,
    n: Vec3,
    size: f64,
    rotation: f64,
    /// A puff's or flame's size and opacity over its style's, and its first frame.
    scale: f64,
    alpha: f64,
    frame: f64,
    /// A tracer's or trail's path, flat, its cumulative lengths and total.
    path: Vec<f64>,
    cum: Vec<f64>,
    length: f64,
    /// Spark velocities, flat.
    sparks: Vec<f64>,
    /// What it is, for a light it casts.
    cause: Arc<str>,
}

/// The point `at` metres along `e`'s path.
fn point_along(e: &Effect, at: f64) -> Vec3 {
    let (path, cum) = (&e.path, &e.cum);
    let mut i = 0;
    while i + 2 < cum.len() && cum[i + 1] < at {
        i += 1;
    }
    let u = (at - cum[i]) / (cum[i + 1] - cum[i]).max(1e-6);
    let o = i * 3;
    std::array::from_fn(|k| path[o + k] + (path[o + 3 + k] - path[o + k]) * u)
}

/// `a + b·s`, as `vec3.scaleAndAdd`.
fn along(a: Vec3, b: Vec3, s: f64) -> Vec3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// The effects running, the spent ones, and how each is born.
#[derive(Default)]
struct Pool {
    live: Vec<Effect>,
    spent: Vec<Effect>,
    /// Seconds a tick.
    dt: f64,
}

impl Pool {
    fn push(&mut self, look: Look, start: f64, end: f64) -> &mut Effect {
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
    fn tracer(
        &mut self,
        p: &EffectPresentation,
        t0: f64,
        s: &Segment,
        rng: &mut Mulberry32,
        visible: bool,
    ) -> usize {
        let style = pick(&p.tracers, &s.kind);
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
    fn trail(&mut self, t0: f64, index: usize, style: &TrailStyle, rng: &mut Mulberry32) {
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
    fn flash(
        &mut self,
        p: &EffectPresentation,
        t0: f64,
        l: &Launch,
        rng: &mut Mulberry32,
    ) -> Arc<FlashStyle> {
        let style = pick(&p.flashes, &l.kind);
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
    fn shot_dust(
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
    fn impact(
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
        let scale = style.round_scale.get(kind);
        let end = at + style.duration_s * scale.map_or(1.0, |s| s.duration);
        let size = style.size_m * scale.map_or_else(|| *pick(&p.impact_scale, kind), |s| s.size);
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
    fn sparks(
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
    fn blast(
        &mut self,
        p: &EffectPresentation,
        at: f64,
        point: Vec3,
        radius: f64,
        kind: &str,
        sparks: f64,
        rng: &mut Mulberry32,
    ) {
        let style = &p.blast;
        let e = self.push(Look::Blast, at, at + style.duration_s);
        e.cause = "blast".into();
        e.p = point;
        e.size = radius * style.size_per_radius * pick(&p.blast_scale, kind);
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
    fn cook_off(&mut self, p: &EffectPresentation, at: f64, c: &CookOff, rng: &mut Mulberry32) {
        let feel = &p.cook_off;
        let top = c.center[2] + c.height;
        for f in &feel.fireballs {
            let point = [c.center[0], c.center[1], top + f.up_m];
            let born = at + feel.delay_s + f.after_s;
            self.blast(p, born, point, f.radius_m, "cook_off", f.sparks, rng);
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
    fn puff(
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
    fn flame(&mut self, at: f64, p: Vec3, style: &FlameStyle, strength: f64, rng: &mut Mulberry32) {
        let e = self.push(Look::Flame(*style), at, at + style.life_s);
        e.scale = strength * (0.7 + 0.6 * rng.sample());
        e.p = p;
        e.rotation = rng.sample() * PI * 2.0;
        e.frame = 3.0 + rng.sample() * 3.0;
    }
}

/// A smoke source the side knows, and how far its emission has got.
struct Source {
    key: String,
    /// What its light is: `fire:<kind>`.
    cause: Arc<str>,
    style: Arc<SmokeSourceStyle>,
    seed: u32,
    /// When it was first known, on the presentation clock.
    start: f64,
    /// The top of its footprint's centre, its yaw and half extents.
    x: f64,
    y: f64,
    top: f64,
    yaw: f64,
    hx: f64,
    hy: f64,
    /// Its own size and thickness, so no two columns match.
    scale: f64,
    alpha: f64,
    /// Births so far per stream (smoke, smoulder, flame).
    emitted: [f64; 3],
    known: bool,
}

impl Source {
    /// The instances it holds `age` seconds after it is first known, once
    /// its streams run steadily.
    fn steady(&self, age: f64) -> f64 {
        let s = &self.style;
        if age < s.burn_s {
            s.smoke_hz * s.smoke.life_s + s.flame.rate_hz * s.flame.life_s + 1.0
        } else if age < s.burn_s + s.smoulder_s {
            s.smoulder_hz * s.smoulder.life_s
        } else {
            0.0
        }
    }

    /// Stream `stream`: a birth every 1/`hz` s from `from` to `to`, up to
    /// `t1`, skipping any already dead, each with its own seed and kept by
    /// its own draw under the budget (`keep`).
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        pool: &mut Pool,
        stream: usize,
        from: f64,
        to: f64,
        hz: f64,
        life: f64,
        t1: f64,
        keep: f64,
    ) {
        if hz <= 0.0 {
            return;
        }
        let last = ((to.min(t1) - from) * hz).ceil() - 1.0;
        let mut k = self.emitted[stream].max(((t1 - life - from) * hz).ceil());
        let (s, c) = libm::sincos(self.yaw);
        while k <= last {
            let at = from + k / hz;
            let seed =
                self.seed ^ ((k + 1.0) as u32).wrapping_mul(GOLDEN) ^ ((stream as u32) << 29);
            let mut rng = Mulberry32::new(seed);
            k += 1.0;
            if rng.sample() >= keep {
                continue; // over the budget
            }
            // A point on the footprint's top, toward its middle.
            let u = (rng.sample() * 2.0 - 1.0) * 0.6;
            let v = (rng.sample() * 2.0 - 1.0) * 0.6;
            let x = self.x + u * self.hx * c - v * self.hy * s;
            let y = self.y + u * self.hx * s + v * self.hy * c;
            let st = &self.style;
            if stream == 2 {
                // Flames die down over the burn's last third.
                let left = (to - at) / (to - from);
                pool.flame(
                    at,
                    [x, y, self.top - 0.3],
                    &st.flame,
                    (left * 3.0).min(1.0),
                    &mut rng,
                );
            } else {
                let style = if stream == 0 { &st.smoke } else { &st.smoulder };
                pool.puff(
                    at,
                    [x, y, self.top],
                    style,
                    self.scale,
                    self.alpha,
                    &mut rng,
                );
            }
        }
        self.emitted[stream] = self.emitted[stream].max(last + 1.0);
    }
}

/// Where a hull the side sees was last published, and how far it has gone
/// since its last dust.
struct Mover {
    at: Vec3,
    carry: f64,
    seen: bool,
}

pub struct EffectFrame {
    p: EffectPresentation,
    pool: Pool,
    sources: Vec<Source>,
    /// Each source's index in `sources`, by key.
    source_index: HashMap<String, usize>,
    /// The presentation time the last publication covers up to.
    noted: f64,
    /// The share of smoke sources' births kept, under `smoke_budget`.
    keep: f64,
    movers: HashMap<u64, Mover>,
    /// Where each smoking hull was last published.
    smokers: HashMap<u64, Mover>,
    last_tick: f64,
    launches: LaunchTracker,
    instances: usize,
    dropped: usize,
}

impl EffectFrame {
    pub fn new(tick_hz: f64, presentation: EffectPresentation) -> Self {
        Self {
            p: presentation,
            pool: Pool {
                dt: 1.0 / tick_hz,
                ..Pool::default()
            },
            sources: Vec::new(),
            source_index: HashMap::new(),
            noted: 0.0,
            keep: 1.0,
            movers: HashMap::new(),
            smokers: HashMap::new(),
            last_tick: -1.0,
            launches: LaunchTracker::default(),
            instances: 0,
            dropped: 0,
        }
    }

    pub fn presentation(&self) -> &EffectPresentation {
        &self.p
    }

    /// Forget everything (a new battle).
    pub fn reset(&mut self) {
        let pool = &mut self.pool;
        pool.spent.append(&mut pool.live);
        self.sources.clear();
        self.source_index.clear();
        self.movers.clear();
        self.smokers.clear();
        self.launches.reset();
        self.last_tick = -1.0;
        self.noted = 0.0;
        self.keep = 1.0;
    }

    /// Take one publication. The tick it covers runs from the previous
    /// tick's presentation time to its own: every effect it causes starts in there.
    /// Takes one publication; returns the launches it shows (`launches.rs`),
    /// which sound hears too. The same tick again is ignored.
    pub fn note(&mut self, publication: &EffectPublication) -> Vec<Launch> {
        let tick = publication.tick;
        if tick == self.last_tick {
            return Vec::new();
        }
        if tick < self.last_tick {
            self.reset();
        }
        let gap = self.last_tick >= 0.0 && tick != self.last_tick + 1.0;
        self.last_tick = tick;
        let dt = self.pool.dt;
        let (t0, t1) = ((tick - 1.0) * dt, tick * dt);
        let mut rng = Mulberry32::new((tick as u32).wrapping_mul(GOLDEN));
        let p = &self.p;
        let chance = |kind: &str| pick(&p.tracers, kind).chance.unwrap_or(1.0);
        let launches = self.launches.note(publication, gap, Some(&chance));
        // A flash at each, and a hull's heavy gun's dust.
        for l in &launches {
            let dust = self.pool.flash(p, t0, l, &mut rng).dust;
            let Some(style) = p.shot_dust.as_ref().filter(|_| l.hull && dust > 0.0) else {
                continue;
            };
            let hull = publication.shooters.iter().find(|u| u.key == l.shooter);
            if let Some(u) = hull.filter(|u| !u.airborne) {
                if let Some(half) = u.half {
                    self.pool.shot_dust(style, t0, l, u, half, dust, &mut rng);
                }
            }
        }
        for (index, s) in publication.segments.iter().enumerate() {
            if s.path.len() < 2 {
                continue;
            }
            let tracer = self.launches.tracers.contains(&index);
            let e = self.pool.tracer(p, t0, s, &mut rng, tracer);
            // A ricochet always sparks; an impact sparks only off a tracer round.
            for r in &s.ricochets {
                let (cum, length) = (self.pool.live[e].cum[r.point], self.pool.live[e].length);
                let at = t0 + dt * cum / length.max(1e-6);
                let (q, next) = (s.path[r.point], s.path[(r.point + 1).min(s.path.len() - 1)]);
                let d = [next[0] - q[0], next[1] - q[1], next[2] - q[2]];
                let mut len = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                if len > 0.0 {
                    len = 1.0 / len.sqrt();
                }
                let dir = d.map(|c| c * len);
                self.pool.sparks(
                    &p.sparks,
                    at,
                    q,
                    r.normal,
                    dir,
                    p.ricochet_sparks,
                    &mut rng,
                    1.0,
                );
            }
            if &*s.hit != "none" {
                let end = s.path[s.path.len() - 1];
                self.pool
                    .impact(p, t1, end, s.normal, &s.hit, &s.kind, tracer, &mut rng);
            }
        }
        for b in &publication.blasts {
            self.pool
                .blast(p, t1, b.point, b.radius, &b.kind, 1.0, &mut rng);
        }
        for c in &publication.cook_offs {
            self.pool.cook_off(p, t0, c, &mut rng);
        }
        self.note_movers(publication, gap, t0);
        self.note_smokers(publication, gap, t0, t1);
        self.note_sources(publication, t0, t1);
        launches
    }

    /// Dust behind every hull the side sees move on the ground: a puff off
    /// each track for every `spacing_m` it covers, when and where it
    /// passed. A hull first seen, or seen again after a gap, starts clean.
    fn note_movers(&mut self, publication: &EffectPublication, gap: bool, t0: f64) {
        let style = &self.p.dust;
        let dt = self.pool.dt;
        for m in self.movers.values_mut() {
            m.seen = false;
        }
        for u in &publication.shooters {
            let Some([hl, hw, _]) = u.half.filter(|_| !u.airborne) else {
                continue;
            };
            let [x, y, z] = u.position;
            let Some(m) = self.movers.get_mut(&u.key.to_bits()) else {
                let mover = Mover {
                    at: u.position,
                    carry: 0.0,
                    seen: true,
                };
                self.movers.insert(u.key.to_bits(), mover);
                continue;
            };
            m.seen = true;
            let (dx, dy) = (x - m.at[0], y - m.at[1]);
            let dist = libm::hypot(dx, dy);
            let speed = dist / dt;
            if !gap && speed >= style.min_speed_mps && dist > 1e-6 {
                let (fx, fy) = (dx / dist, dy / dist);
                let thick = (speed / style.full_speed_mps).min(1.0);
                let mut ahead = style.spacing_m - m.carry;
                while ahead <= dist {
                    let u01 = ahead / dist;
                    let px = m.at[0] + dx * u01;
                    let py = m.at[1] + dy * u01;
                    let pz = m.at[2] + (z - m.at[2]) * u01;
                    let at = t0 + dt * u01;
                    let mut rng = Key::new()
                        .num(u.key)
                        .num((px * 100.0).round())
                        .num((py * 100.0).round())
                        .rng();
                    for side in [-1.0, 1.0] {
                        let tx = px - fx * hl * 1.1 - fy * hw * 0.75 * side;
                        let ty = py - fy * hl * 1.1 + fx * hw * 0.75 * side;
                        self.pool
                            .puff(at, [tx, ty, pz + 0.3], &style.puff, 1.0, thick, &mut rng);
                    }
                    ahead += style.spacing_m;
                }
                m.carry = dist - (ahead - style.spacing_m);
            } else {
                m.carry = 0.0;
            }
            m.at = u.position;
        }
        self.movers.retain(|_, m| m.seen);
    }

    /// Smoke behind every smoking hull the side sees: a puff every
    /// 1/`rate_hz` s of the clock, born where the hull was at that moment
    /// along its tick's stretch, each seeded by its hull and index, so what
    /// is drawn does not hang on how publications arrive.
    fn note_smokers(&mut self, publication: &EffectPublication, gap: bool, t0: f64, t1: f64) {
        let style = &self.p.damage_smoke;
        for m in self.smokers.values_mut() {
            m.seen = false;
        }
        for u in publication.shooters.iter().filter(|u| u.smoking) {
            let top = u.position[2] + u.half.map_or(0.0, |h| h[2] * 2.0);
            let here = [u.position[0], u.position[1], top];
            let m = self.smokers.entry(u.key.to_bits()).or_insert(Mover {
                at: here,
                carry: 0.0,
                seen: true,
            });
            if gap {
                m.at = here;
            }
            m.seen = true;
            let hull = Key::new().text("smoke").num(u.key).seed();
            let mut k = (t0 * style.rate_hz).ceil();
            while k / style.rate_hz < t1 {
                let at = k / style.rate_hz;
                let f = (at - t0) / (t1 - t0);
                let mut rng = Mulberry32::new(hull ^ ((k + 1.0) as u32).wrapping_mul(GOLDEN));
                let p: Vec3 = std::array::from_fn(|i| m.at[i] + (here[i] - m.at[i]) * f);
                self.pool.puff(at, p, &style.puff, 1.0, 1.0, &mut rng);
                k += 1.0;
            }
            m.at = here;
        }
        self.smokers.retain(|_, m| m.seen);
    }

    /// Smoke sources: each known one burns from the tick it is first known,
    /// then smoulders, then is out. Every puff and flame has its own time
    /// and seed; after a gap only those still alive are made. A source no
    /// longer published stops; its puffs live out.
    fn note_sources(&mut self, publication: &EffectPublication, t0: f64, t1: f64) {
        for src in &mut self.sources {
            src.known = false;
        }
        for k in &publication.smokes {
            let Some(style) = self.p.smoke.get(&k.kind) else {
                continue;
            };
            if let Some(&i) = self.source_index.get(&k.key) {
                self.sources[i].known = true;
                continue;
            }
            let seed = Key::new()
                .text(&k.kind)
                .num(k.center[0])
                .num(k.center[1])
                .seed();
            self.source_index.insert(k.key.clone(), self.sources.len());
            self.sources.push(Source {
                key: k.key.clone(),
                cause: format!("fire:{}", k.kind).into(),
                style: style.clone(),
                seed,
                start: t0,
                x: k.center[0],
                y: k.center[1],
                top: k.center[2] + k.half[2] * 2.0,
                yaw: k.yaw,
                hx: k.half[0],
                hy: k.half[1],
                scale: 0.75 + 0.5 * (f64::from(seed & 0xffff) / 65535.0),
                alpha: 0.75 + 0.25 * (f64::from(seed >> 16) / 65535.0),
                emitted: [0.0; 3],
                known: true,
            });
        }
        self.noted = t1;
        let mut kept = 0;
        let mut demand = 0.0;
        for i in 0..self.sources.len() {
            if !self.sources[i].known {
                self.source_index.remove(&self.sources[i].key);
                continue;
            }
            self.sources.swap(kept, i);
            let src = &self.sources[kept];
            if let Some(at) = self.source_index.get_mut(&src.key) {
                *at = kept;
            }
            demand += src.steady(t1 - src.start);
            kept += 1;
        }
        self.sources.truncate(kept);
        self.keep = (self.p.smoke_budget / demand.max(1.0)).min(1.0);
        for src in &mut self.sources {
            let st = src.style.clone();
            let burn_end = src.start + st.burn_s;
            let (pool, keep) = (&mut self.pool, self.keep);
            src.emit(
                pool,
                0,
                src.start,
                burn_end,
                st.smoke_hz,
                st.smoke.life_s,
                t1,
                keep,
            );
            let smoulder_end = burn_end + st.smoulder_s;
            src.emit(
                pool,
                1,
                burn_end,
                smoulder_end,
                st.smoulder_hz,
                st.smoulder.life_s,
                t1,
                keep,
            );
            src.emit(
                pool,
                2,
                src.start,
                burn_end,
                st.flame.rate_hz,
                st.flame.life_s,
                t1,
                keep,
            );
        }
    }

    /// Every running effect at presentation time `clock` (seconds) into
    /// `batch`, the rotors' blurs under them; effects that have run their
    /// course are dropped. `muzzles` says where the models are drawn at
    /// `clock`, for the flashes to sit on.
    pub fn build(
        &mut self,
        clock: f64,
        batch: &mut EffectBatch,
        mut muzzles: Option<&mut dyn MuzzleSource>,
        rotors: &[DrawnRotor],
    ) {
        batch.count = 0;
        batch.dropped = 0;
        batch.lights.clear(self.p.cast_wrap);
        let p = &self.p;
        // Rotor blurs first, under every puff: smoke drifts across them.
        draw_rotors(&p.rotor_blur, rotors, batch);
        let dt = self.pool.dt;
        // Trails' ribbons next, under every puff: drawn in their turn, a
        // later stretch's ribbon would cover an earlier stretch's puffs.
        for e in &self.pool.live {
            if let (Look::Trail(s), true) = (&e.look, e.start <= clock && clock < e.end) {
                draw_trail(p, e, s, clock - e.start, dt, batch);
            }
        }
        let live = &mut self.pool.live;
        let mut kept = 0;
        for i in 0..live.len() {
            if live[i].end <= clock {
                continue;
            }
            live.swap(kept, i);
            let e = &live[kept];
            kept += 1;
            if clock < e.start {
                continue;
            }
            let age = clock - e.start;
            match &e.look {
                Look::Tracer { style, visible } => {
                    if *visible {
                        draw_tracer(p, e, style, age, batch);
                    }
                }
                Look::Flash {
                    style,
                    shooter,
                    mount,
                    soldier,
                } => {
                    let drawn = muzzles
                        .as_deref_mut()
                        .and_then(|m| m.muzzle(*shooter, *mount, *soldier));
                    draw_flash(p, e, style, drawn.unwrap_or(e.p), age, batch);
                }
                Look::Impact { style, visible } => draw_impact(p, e, style, *visible, age, batch),
                Look::Sparks => draw_sparks(p, e, age, batch),
                Look::Blast => draw_blast(p, e, age, batch),
                Look::Puff(s) => draw_puff(&p.wind_mps, e, s, age, batch),
                Look::Flame(s) => draw_flame(e, s, age, batch),
                Look::Trail(_) => {}
            }
        }
        self.pool.spent.extend(live.drain(kept..));
        for src in &self.sources {
            draw_fire_light(src, clock, self.noted, self.keep, batch);
        }
        self.instances = batch.count;
        self.dropped = batch.dropped;
    }

    /// What the last `build`, at `clock` into `batch`, drew.
    pub fn counts(&self, clock: f64, batch: &EffectBatch) -> EffectCounts {
        let mut out = EffectCounts::default();
        for e in self.pool.live.iter().filter(|e| e.start <= clock) {
            out.running[e.look.kind()] += 1;
        }
        for i in 0..batch.count {
            out.instances[batch.data[i * EFFECT_FLOATS + 12] as usize] += 1;
        }
        out.lights = batch.lights.count;
        out
    }

    pub fn stats(&self) -> EffectStats {
        let burning = |s: &&Source| self.noted < s.start + s.style.burn_s + s.style.smoulder_s;
        EffectStats {
            live: self.pool.live.len(),
            instances: self.instances,
            dropped: self.dropped,
            sources: self.sources.iter().filter(burning).count(),
            tick: self.last_tick,
        }
    }
}

// ---- Drawing (per frame). ----

fn tracer_amplitude(u: f64, retained: f64) -> f64 {
    if retained != 0.0 {
        (retained + (1.0 - retained) * u * u).sqrt()
    } else {
        u
    }
}

/// A round in flight: its glow along the tail, its hot core along the
/// tail's front, and its body and light while this stretch holds the round.
fn draw_tracer(
    p: &EffectPresentation,
    e: &Effect,
    style: &TracerStyle,
    age: f64,
    batch: &mut EffectBatch,
) {
    let length = e.length;
    if length <= 1e-6 {
        return;
    }
    let head = length * age / e.span;
    let glow_px = style.line_px.unwrap_or(style.glow.min_px);
    draw_tail(
        e,
        style,
        head,
        e.size,
        &style.glow,
        glow_px,
        style.line_px.is_none(),
        batch,
    );
    if let Some(core) = &style.core {
        draw_tail(
            e,
            style,
            head,
            e.size * core.share,
            core,
            core.min_px,
            false,
            batch,
        );
    }
    // The round is on this stretch from its tick's start to its end (a
    // clock at the tick's end lands a rounding off it).
    if (style.body.is_none() && style.cast.is_none()) || age <= 0.0 || head > length * (1.0 + 1e-6)
    {
        return;
    }
    let a = point_along(e, head.min(length));
    if let Some(body) = &style.body {
        batch.disc(a, body.size_m, p.min_px * 2.0, body.color, 1.0);
    }
    if let Some(c) = &style.cast {
        batch
            .lights
            .offer(a, c.radius_m, c.color, c.intensity, &e.cause);
    }
}

/// The part of `e`'s stretch within `length` behind `head`, in `light`, as
/// streaks fading out toward the tail's end.
#[allow(clippy::too_many_arguments)]
fn draw_tail(
    e: &Effect,
    style: &TracerStyle,
    head: f64,
    length: f64,
    light: &TracerLight,
    min_px: f64,
    soft: bool,
    batch: &mut EffectBatch,
) {
    let tail = head - length;
    let (path, cum) = (&e.path, &e.cum);
    let retained = style.tail_brightness.unwrap_or(0.0);
    let width = match style.line_px {
        Some(px) if px != 0.0 => -px,
        _ => light.width_m,
    };
    for i in 0..cum.len().saturating_sub(1) {
        let (c0, c1) = (cum[i], cum[i + 1]);
        let (lo, hi) = (c0.max(tail), c1.min(head));
        if hi <= lo || c1 <= c0 {
            continue;
        }
        let (u0, u1) = ((lo - c0) / (c1 - c0), (hi - c0) / (c1 - c0));
        let o = i * 3;
        let a = std::array::from_fn(|k| path[o + k] + (path[o + 3 + k] - path[o + k]) * u0);
        let b = std::array::from_fn(|k| path[o + k] + (path[o + 3 + k] - path[o + k]) * u1);
        let along = [
            tracer_amplitude((lo - tail) / length, retained),
            tracer_amplitude((hi - tail) / length, retained),
        ];
        let boost = style.zoom_boost.unwrap_or(0.0);
        batch.streak(
            a,
            b,
            width,
            min_px,
            light.color,
            light.intensity,
            along,
            soft,
            boost,
        );
    }
}

/// Each rotor as a shutter sees it: its blades smeared over the angle they
/// swept while it was open, a faint disc over its reach while it spins, its
/// hub, and its mast.
fn draw_rotors(blur: &RotorBlurStyle, rotors: &[DrawnRotor], batch: &mut EffectBatch) {
    let RotorBlurStyle {
        albedo,
        shutter,
        hub_share,
        min_chord_px,
        disc,
    } = *blur;
    for r in rotors {
        // Past a whole turn every blade covers the whole disc alike.
        let sweep = (2.0 * PI).min(r.turned.abs() * shutter);
        let mut rotor = |radius: f64, sweep: f64, chord: f64, blades: f64, misc: f64| {
            let shape = [ROTOR, chord, blades, misc];
            batch.put([
                v4(r.hub, radius),
                v4(r.axis, sweep),
                v4(r.blade, albedo),
                shape,
            ]);
        };
        rotor(r.radius, sweep, r.chord / r.radius, r.blades, min_chord_px);
        if sweep > 0.0 && disc > 0.0 {
            rotor(r.radius, 0.0, 0.0, 0.0, disc);
        }
        rotor(r.radius * hub_share, 0.0, 0.0, 0.0, 1.0);
        if let Some(mast) = &r.mast {
            let width = 2.0 * r.mast_radius;
            batch.ribbon(mast.top, mast.base, width, 1.0, [albedo; 3], 1.0);
        }
    }
}

/// A burning-out light `age` into its life: 1 − (age/duration)² of its
/// intensity, its reach scaled by `reach`.
fn offer_burning_out(
    batch: &mut EffectBatch,
    c: &CastStyle,
    age: f64,
    at: Vec3,
    reach: f64,
    cause: &Arc<str>,
    strength: f64,
) {
    let f = age / c.duration_s;
    let k = c.intensity * strength * (1.0 - f * f);
    batch
        .lights
        .offer(at, c.radius_m * reach, c.color, k, cause);
}

/// A muzzle flash at `at` (the drawn muzzle, else the launch point),
/// pointing where the round went: its glow, its tongue, its fireball and light.
fn draw_flash(
    p: &EffectPresentation,
    e: &Effect,
    s: &FlashStyle,
    at: Vec3,
    age: f64,
    batch: &mut EffectBatch,
) {
    if let Some(c) = s.cast.as_ref().filter(|c| age < c.duration_s) {
        offer_burning_out(
            batch,
            c,
            age,
            along(at, e.n, c.forward_m),
            1.0,
            &e.cause,
            1.0,
        );
    }
    let x = age / s.duration_s;
    if x < 1.0 {
        let k = (1.0 - x) * (1.0 - x);
        let size = s.size_m * (0.7 + 0.5 * x);
        batch.glow(
            at,
            size,
            p.min_px,
            s.color,
            s.intensity * k,
            e.rotation,
            0.0,
        );
        let b = along(at, e.n, s.tongue_m * (0.6 + 0.4 * x));
        let width = s.size_m * 0.9;
        batch.streak(
            at,
            b,
            width,
            p.min_px,
            s.color,
            s.intensity * k,
            [1.0, 0.15],
            false,
            0.0,
        );
    }
    if s.fireball_m > 0.0 && age < s.fireball_s {
        let f = age / s.fireball_s;
        let a = along(at, e.n, s.fireball_m * (0.2 + 0.6 * f));
        let radius = s.fireball_m * (0.6 + 0.6 * f.sqrt());
        let heat = s.intensity * 0.25 * (1.0 - f);
        let opacity = (1.0 - f) * (1.0 - f);
        batch.flipbook(
            a,
            radius,
            e.rotation,
            FIRE,
            f * 10.0,
            s.color,
            opacity,
            heat,
            0.5,
            false,
        );
    }
}

/// An impact's puff off the face, rising as it spreads; off a tracer, its
/// light and a hot flash too.
fn draw_impact(
    p: &EffectPresentation,
    e: &Effect,
    s: &ImpactStyle,
    tracer: bool,
    age: f64,
    batch: &mut EffectBatch,
) {
    let x = age / e.span;
    let size = e.size * (0.7 + 0.6 * x.sqrt());
    let mut a = along(e.p, e.n, size * 0.45);
    a[2] += e.size * 0.3 * x;
    let opacity = s.opacity * (x * 12.0).min(1.0) * (1.0 - x) * (1.0 - x);
    let frame = (e.frame + x * 0.4) * (DUST_FRAMES - 1.0);
    batch.flipbook(
        a,
        size,
        e.rotation,
        DUST,
        frame,
        s.color,
        opacity,
        0.0,
        size * 0.6,
        false,
    );
    if !tracer {
        return;
    }
    if let Some(c) = s.cast.as_ref().filter(|c| age < c.duration_s) {
        // Off the face, reaching farther for a bigger round.
        let reach = (e.size / s.size_m).sqrt();
        offer_burning_out(batch, c, age, along(e.p, e.n, 0.5), reach, &e.cause, 1.0);
    }
    if s.flash > 0.0 && x < 0.12 {
        let k = 1.0 - x / 0.12;
        // Off the face by its own radius: a sprite in the face would fade into it.
        let r = e.size * 0.35;
        let at = along(e.p, e.n, r);
        batch.glow(
            at,
            r,
            p.min_px,
            p.sparks.color,
            s.flash * k * k,
            e.rotation,
            1.0,
        );
    }
}

/// Sparks flying out under gravity, each a short streak along its path.
fn draw_sparks(p: &EffectPresentation, e: &Effect, age: f64, batch: &mut EffectBatch) {
    let s = &p.sparks;
    let fade = 1.0 - age / e.span;
    for v in e.sparks.as_chunks::<3>().0 {
        let speed = crate::world::length(*v);
        let speed = if speed == 0.0 || speed.is_nan() {
            1.0
        } else {
            speed
        };
        let lag = age.min(s.length_m * e.size / speed);
        let (t1, t0) = (age, age - lag);
        let at = |t: f64| -> Vec3 {
            std::array::from_fn(|j| {
                let g = if j == 2 { -0.5 * GRAVITY } else { 0.0 };
                e.p[j] + v[j] * t + g * t * t
            })
        };
        let (width, min_px) = (s.width_m * e.size, p.min_px * 0.5);
        batch.streak(
            at(t0),
            at(t1),
            width,
            min_px,
            s.color,
            s.intensity * fade,
            [0.0, 1.0],
            false,
            0.0,
        );
    }
}

/// Where smoke born at `p`, moving at `n` (its own rise and spread), is
/// `age` into a life of `life`: rising, slowing as it spreads, and drifting
/// on its own and more downwind as it climbs.
fn smoke_at(wind: &[f64; 2], p: Vec3, n: Vec3, life: f64, age: f64) -> Vec3 {
    let x = age / life;
    let climb = life * 0.5 * (1.0 - (1.0 - x) * (1.0 - x));
    let drift = age * (0.3 + 0.7 * x);
    [
        p[0] + n[0] * climb + wind[0] * drift,
        p[1] + n[1] * climb + wind[1] * drift,
        p[2] + n[2] * climb,
    ]
}

/// A puff's radius `x` of the way through its life.
fn puff_radius(s: &PuffStyle, x: f64) -> f64 {
    s.size_m[0] + (s.size_m[1] - s.size_m[0]) * x.sqrt()
}

/// A trail's ribbon along the part of its stretch the round has flown: each
/// end where its smoke has drifted since the round passed it, as wide and
/// faded as its middle's age, so stretches meet end to end unbroken.
fn draw_trail(
    p: &EffectPresentation,
    e: &Effect,
    s: &TrailStyle,
    age: f64,
    dt: f64,
    batch: &mut EffectBatch,
) {
    let length = e.length;
    if length <= 1e-6 {
        return;
    }
    let flown = length.min(length * age / dt);
    let puff = &s.puff;
    let rise = [0.0, 0.0, puff.rise_mps];
    for i in 0..e.cum.len().saturating_sub(1) {
        let (lo, hi) = (e.cum[i], e.cum[i + 1].min(flown));
        if hi <= lo {
            continue;
        }
        // Each point's age: since the round passed it.
        let per_m = dt / length;
        let age_lo = age - lo * per_m;
        let age_hi = (age - hi * per_m).max(0.0);
        let a = smoke_at(&p.wind_mps, point_along(e, lo), rise, puff.life_s, age_lo);
        let b = smoke_at(&p.wind_mps, point_along(e, hi), rise, puff.life_s, age_hi);
        let x = ((age_lo + age_hi) / 2.0 / puff.life_s).min(1.0);
        // Thinning before its puffs fade, so an old trail loosens into them.
        let opacity = s.ribbon * ((1.0 - x) * (1.0 - x));
        let width = 2.0 * RIBBON_WIDTH * puff_radius(puff, x);
        batch.ribbon(a, b, width, p.min_px, puff.albedo, opacity);
    }
}

/// A puff: rising, slowing as it spreads, drifting downwind as it climbs;
/// fading in quickly and out slowly.
fn draw_puff(wind: &[f64; 2], e: &Effect, s: &PuffStyle, age: f64, batch: &mut EffectBatch) {
    let x = age / e.span;
    let a = smoke_at(wind, e.p, e.n, s.life_s, age);
    let radius = e.scale * puff_radius(s, x);
    let fade = (age * PUFF_FADE_IN).min(1.0) * libm::pow(1.0 - x, 1.2);
    let turn = e.rotation + e.size * x * 1.5;
    let frame = e.frame + x * PUFF_TURN_FRAMES;
    let opacity = s.opacity * e.alpha * fade;
    batch.flipbook(
        a,
        radius,
        turn,
        DUST,
        frame,
        s.albedo,
        opacity,
        0.0,
        radius * 0.8,
        true,
    );
}

/// A flame: the fire sheet's hot frames, licking up and shrinking away.
fn draw_flame(e: &Effect, s: &FlameStyle, age: f64, batch: &mut EffectBatch) {
    let x = age / e.span;
    let mut a = e.p;
    a[2] += s.size_m * e.scale * (0.3 + 1.2 * x);
    let radius = s.size_m * e.scale * (0.8 + 0.4 * libm::sin(PI * x));
    let opacity = (x * 6.0).min(1.0) * (1.0 - x);
    batch.flipbook(
        a,
        radius,
        e.rotation,
        FIRE,
        e.frame + x * 4.0,
        s.tint,
        opacity,
        s.emissive,
        0.3,
        false,
    );
}

/// A burning source's fire light (its glow in the air and its cast light),
/// flickering on the clock, dying down over the burn's last third, and no
/// longer than the flames the publications taken so far made.
fn draw_fire_light(src: &Source, clock: f64, noted: f64, keep: f64, batch: &mut EffectBatch) {
    let f = &src.style.flame;
    let age = clock - src.start;
    let burn = src.style.burn_s;
    if age < 0.0 || age >= burn || clock > noted + f.life_s || f.light_intensity <= 0.0 {
        return;
    }
    if f64::from(src.seed) / 4_294_967_296.0 >= keep {
        return; // over the budget
    }
    let phase = f64::from(src.seed % 1000) * 0.01;
    let flicker =
        0.7 + 0.3 * libm::sin(clock * 11.3 + phase) * libm::sin(clock * 5.7 + phase * 1.7);
    let strength = ((burn - age) / burn * 3.0).min(age * 4.0).min(1.0);
    let at = [src.x, src.y, src.top + 0.5];
    batch.glow(
        at,
        f.light_m,
        0.0,
        f.light,
        f.light_intensity * flicker * strength,
        phase,
        0.0,
    );
    if let Some(c) = &f.cast {
        let k = c.intensity * flicker * strength;
        batch.lights.offer(at, c.radius_m, c.color, k, &src.cause);
    }
}

/// A burst: its light, the glow in the air while the fire is hot, the
/// fireball's flipbook over the blast's life, and its first flash.
fn draw_blast(p: &EffectPresentation, e: &Effect, age: f64, batch: &mut EffectBatch) {
    let s = &p.blast;
    let x = age / e.span;
    let heat = (1.0 - x * 2.2).max(0.0);
    let c = &s.cast;
    if age < c.duration_s {
        // A burst below the style's lights a smaller pool, and more dimly.
        let share = e.size / s.reference_size_m;
        let at = [e.p[0], e.p[1], e.p[2] + e.size * 0.5];
        offer_burning_out(batch, c, age, at, share.sqrt(), &e.cause, share.min(1.0));
    }
    // The fireball rises through the blast's life; while the fire is hot
    // it sits at the height of the glow round it, as the browser draws it.
    let mut a = e.p;
    a[2] += e.size * (0.55 + 0.5 * x);
    if heat > 0.0 {
        a = e.p;
        a[2] += e.size * 0.5;
        batch.glow(
            a,
            e.size * 1.2,
            0.0,
            s.flash,
            s.spill * heat * heat,
            e.rotation,
            0.0,
        );
    }
    let frame = (x * 1.15).min(1.0) * (FIRE_FRAMES - 1.0);
    let opacity = s.opacity * ((1.0 - x) * 3.0).min(1.0);
    let radius = e.size * (0.75 + 0.5 * x.sqrt());
    let (emissive, soft) = (s.emissive * heat * heat, e.size * 0.5);
    batch.flipbook(
        a, radius, e.rotation, FIRE, frame, s.tint, opacity, emissive, soft, false,
    );
    if age < s.flash_duration_s {
        let k = 1.0 - age / s.flash_duration_s;
        let mut a = e.p;
        a[2] += e.size * 0.4;
        let k = s.flash_intensity * k * k;
        batch.glow(a, e.size * 0.3, p.min_px, s.flash, k, e.rotation, 0.5);
    }
}
