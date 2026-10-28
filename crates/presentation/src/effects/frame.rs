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
//!   smoke behind a smoking one, by time; in standing water, wake impulses
//!   instead of dust, from a hull or a squad ([`WakeImpulse`]);
//! - flames and a smoke column over every smoke source the side knows,
//!   burning, then smouldering, then out, all thinned alike past `smoke_budget`;
//! - a watched hull's cook-off: fireballs out of its ring, then sparks and
//!   dust where its thrown turret lands;
//! - every drawn rotor as a film camera's shutter sees it turn;
//! - a high-air aircraft's shadow on the ground beneath it
//!   ([`high_air_shadow`](super::high_air_shadow)), under everything else.
//!
//! Every effect has a published cause and a bounded life, and runs on the
//! presentation clock, so a pause holds them and a jump of the clock makes
//! only what would still be alive. A publication is taken once: the same
//! tick again is ignored, an earlier one starts over. Each effect is seeded
//! and summed as the browser does it, so both clients draw the same floats.
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use super::batch::{v4, DUST, DUST_FRAMES, FIRE, FIRE_FRAMES, ROTOR};
pub use super::batch::{EffectBatch, EFFECT_FLOATS, SHAPE_NAMES};
use super::cook_off::GRAVITY;
use super::high_air_shadow::GroundShadow;
use super::launches::{Launch, LaunchTracker};
use super::publication::{CookOff, EffectPublication, Segment, Shooter};
use super::tables::{
    pick, CastStyle, EffectPresentation, FlameStyle, FlashStyle, ImpactStyle, PuffStyle,
    RotorBlurStyle, ShotDustStyle, SmokeSourceStyle, SparkStyle, TracerLight, TracerStyle,
    TrailStyle, TRACER_TAIL_MAX_S,
};
use super::{DrawnRotor, MuzzleSource};
use crate::seed::{Key, Mulberry32, GOLDEN};
use crate::wave::WaveClock;
use crate::wind::WindSettings;
use crate::world::Vec3;

mod draw;
mod pool;
mod smoke;
mod wake;
use draw::*;
use pool::Pool;
use smoke::{draw_puff, draw_trail};
pub use wake::WaveFrame;

// ---- Effects in flight. ----

/// Each effect kind's name, in [`EffectCounts::running`] order.
pub const EFFECT_KINDS: [&str; 8] = [
    "tracer", "flash", "impact", "sparks", "blast", "puff", "flame", "trail",
];

/// What a frame drew: effects running by kind, instances by shape, lights.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EffectCounts {
    pub running: [usize; 8],
    pub instances: [usize; 5],
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

/// How a wake impulse spreads: rings round a still or slow wader, a V
/// behind a moving one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeKind {
    Rings,
    Vee,
}

/// A push on the water's surface: where, how wide (metres) and hard, its
/// kind, and when it is made on the presentation clock (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WakeImpulse {
    pub xy: [f64; 2],
    pub radius: f64,
    pub strength: f64,
    pub kind: WakeKind,
    pub at: f64,
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
    /// The wake impulses made within `WAKE_KEEP_S` of the last tick taken.
    wakes: Vec<WakeImpulse>,
    /// The wave field's clock, which splats them (`waves`).
    waves: WaveClock,
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
            wakes: Vec::new(),
            waves: WaveClock::new(),
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
        self.wakes.clear();
        self.waves.restart();
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
        for g in &publication.grinds {
            // The tick's share, rounded by chance.
            let n = (p.grind_sparks_per_s * dt + rng.sample()).floor();
            if n > 0.0 {
                let dir = [g.heading[0], g.heading[1], 0.0];
                self.pool
                    .sparks(&p.sparks, t1, g.point, UP, dir, n, &mut rng, 1.0);
            }
        }
        self.note_movers(publication, gap, t0);
        self.note_smokers(publication, gap, t0, t1);
        self.note_sources(publication, t0, t1);
        launches
    }

    /// Dust behind every hull the side sees move on the ground: a puff off
    /// each track for every `spacing_m` it covers, when and where it
    /// passed; in standing water, a wake instead (`wake`), from a hull or a
    /// squad alike. A unit first seen, or seen again after a gap, starts clean.
    fn note_movers(&mut self, publication: &EffectPublication, gap: bool, t0: f64) {
        let style = &self.p.dust;
        let dt = self.pool.dt;
        for m in self.movers.values_mut() {
            m.seen = false;
        }
        self.wakes.retain(|w| w.at >= t0 - wake::WAKE_KEEP_S);
        for u in &publication.shooters {
            let wading = !u.wading.is_empty();
            if u.airborne || (u.half.is_none() && !wading) {
                continue;
            }
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
            if wading {
                let d = [dx, dy];
                wake::wake(
                    &mut self.wakes,
                    &self.p.wake,
                    u,
                    d,
                    &mut m.carry,
                    t0,
                    dt,
                    gap,
                );
            } else if let Some([hl, hw, _]) = u
                .half
                .filter(|_| !gap && speed >= style.min_speed_mps && dist > 1e-6)
            {
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
    /// `clock`, for the flashes to sit on; smoke and dust drift on `wind`,
    /// calm without one.
    pub fn build(
        &mut self,
        clock: f64,
        batch: &mut EffectBatch,
        mut muzzles: Option<&mut dyn MuzzleSource>,
        rotors: &[DrawnRotor],
        shadows: &[GroundShadow],
        wind: Option<&WindSettings>,
    ) {
        batch.count = 0;
        batch.dropped = 0;
        batch.lights.clear(self.p.cast_wrap);
        let p = &self.p;
        // High-air shadows first: they lie on the ground, under everything.
        for s in shadows {
            batch.ground_shadow(s, p.high_air_shadow.ground_band_m);
        }
        // Rotor blurs first, under every puff: smoke drifts across them.
        draw_rotors(&p.rotor_blur, rotors, batch);
        let dt = self.pool.dt;
        let drift = wind.map_or([0.0; 2], |w| w.record().drift());
        // Trails' ribbons next, under every puff: drawn in their turn, a
        // later stretch's ribbon would cover an earlier stretch's puffs.
        for e in &self.pool.live {
            if let (Look::Trail(s), true) = (&e.look, e.start <= clock && clock < e.end) {
                draw_trail(p, &drift, e, s, clock - e.start, dt, batch);
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
                Look::Puff(s) => draw_puff(&drift, e, s, age, batch),
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

    /// The wake impulses made from `from` up to `to` on the presentation
    /// clock, kept a second (`WAKE_KEEP_S`) past the last tick taken.
    pub fn wakes(&self, from: f64, to: f64) -> impl Iterator<Item = &WakeImpulse> {
        self.wakes.iter().filter(move |w| from <= w.at && w.at < to)
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
