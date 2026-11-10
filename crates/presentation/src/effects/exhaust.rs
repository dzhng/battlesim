//! A jet's engines and wings as a film shows them: each nozzle's hot glow,
//! brighter the faster it flies, with an afterburner's flame and shock
//! diamonds once it flies faster than its cruise; and the trails its engines
//! and wingtips leave, widening and fading behind it over a few seconds.
//!
//! What it draws from is data. Each airframe's nozzles and wingtips are
//! authored on its appearance (`airframes`), in its model's own frame, and
//! its engine kind names a look (`engines`): a look without a `flame` (a
//! high-bypass turbofan's) only ever glows. Its throttle is how fast it is
//! published flying against its type's cruise; a downed airframe's engines
//! are out. Glows and flames sit on the drawn model; trails are laid tick by
//! tick ([`trails`]).
use std::collections::{BTreeMap, VecDeque};

mod trails;
pub use trails::JetTrails;
use trails::Sample;

use serde::Deserialize;

use super::batch::EffectBatch;
use crate::world::Vec3;

/// `presentation.effects.jet_exhaust`.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct JetExhaustStyle {
    /// Aircraft whose trails are kept at once; one past them draws none.
    pub max_aircraft: usize,
    /// An aircraft published moving faster than this between two ticks has
    /// jumped, m/s: its trail breaks there.
    pub break_mps: f64,
    /// How long after it was last drawn running a jet still lays its
    /// trails, s.
    pub max_gap_s: f64,
    /// The published flight a throttle reads its speed over, s.
    pub speed_window_s: f64,
    /// The share of its cruise a jet flies at where its afterburner lights
    /// and where it burns full.
    pub reheat_share: [f64; 2],
    /// The load (1 level, 2 in a 60° banked turn) where wingtip vapour
    /// starts to thicken, and where it is thickest.
    pub vapour_load: [f64; 2],
    /// Each engine kind's look.
    pub engines: BTreeMap<String, EngineLook>,
    pub engine_trail: TrailLook,
    pub wingtip_trail: TrailLook,
    /// Each jet appearance's engines and wingtips.
    pub airframes: BTreeMap<String, Airframe>,
}

/// How an engine kind glows and burns.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EngineLook {
    pub glow: GlowLook,
    /// An afterburner's flame; none: it never burns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flame: Option<FlameLook>,
    /// Its trail's opacity over the trail look's.
    pub trail: f64,
}

/// The hot glow in a nozzle.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GlowLook {
    pub color: Vec3,
    /// Its brightness standing still and at cruise.
    pub intensity: [f64; 2],
    /// Its added brightness at full afterburner.
    pub reheat_intensity: f64,
    /// Its radius over the nozzle's.
    pub radius: f64,
    pub min_px: f64,
}

/// An afterburner's flame out of a nozzle at full reheat: a soft outer
/// plume, a hot core, and shock diamonds down its length.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlameLook {
    pub color: Vec3,
    pub core_color: Vec3,
    pub intensity: f64,
    pub core_intensity: f64,
    /// Its length and width over the nozzle's radius.
    pub length: f64,
    pub width: f64,
    /// Shock diamonds down it, and each one's brightness.
    pub diamonds: usize,
    pub diamond_intensity: f64,
    /// How much its length flickers, a share.
    pub flicker: f64,
}

/// A trail: lit, blended over, widening and fading as it ages.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrailLook {
    pub albedo: Vec3,
    /// Its opacity at its weakest and strongest (an engine: standing and
    /// at cruise; a wingtip: level and at its fullest vapour).
    pub opacity: [f64; 2],
    pub life_s: f64,
    /// Its width when made and when it has lived its life, m.
    pub width_m: [f64; 2],
    /// It thickens from nothing over this, s.
    pub fade_in_s: f64,
}

/// One jet appearance: its engine kind, and where its nozzles and wingtips
/// are in its model's frame (+X its nose, +Z up).
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Airframe {
    pub engine: String,
    pub nozzles: Vec<Nozzle>,
    #[serde(default)]
    pub wingtips: Vec<Point>,
}

/// A nozzle's exit: its middle and radius.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nozzle {
    pub at: Vec3,
    pub radius_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub at: Vec3,
}

/// The paths of signed numbers under this section: model-frame points.
pub(super) const SIGNED: [&str; 3] = [".at[0]", ".at[1]", ".at[2]"];

impl JetExhaustStyle {
    pub(super) fn check(&self) -> Result<(), String> {
        let fail = |what: String| Err(format!("jet_exhaust: {what}"));
        let ordered = |p: [f64; 2]| p[0] < p[1];
        if self.max_aircraft == 0 || self.break_mps <= 0.0 {
            return fail("max_aircraft and break_mps must be positive".into());
        }
        if self.max_gap_s <= 0.0 || self.speed_window_s <= 0.0 {
            return fail("max_gap_s and speed_window_s must be positive".into());
        }
        if !ordered(self.reheat_share) || !ordered(self.vapour_load) {
            return fail("reheat_share and vapour_load must each rise".into());
        }
        for (name, t) in [
            ("engine_trail", &self.engine_trail),
            ("wingtip_trail", &self.wingtip_trail),
        ] {
            let opaque = t.opacity.iter().all(|o| (0.0..=1.0).contains(o));
            if t.life_s <= 0.0 || t.width_m[0] <= 0.0 || t.width_m[1] < t.width_m[0] || !opaque {
                return fail(format!(
                    "{name}: life_s and width_m must be positive, width_m must not narrow, opacity in [0, 1]"
                ));
            }
        }
        for (name, look) in &self.engines {
            if look.glow.radius <= 0.0
                || look
                    .flame
                    .is_some_and(|f| f.length <= 0.0 || f.width <= 0.0)
            {
                return fail(format!(
                    "engines.{name}: glow radius and flame size must be positive"
                ));
            }
        }
        for (name, a) in &self.airframes {
            if !self.engines.contains_key(&a.engine) {
                return fail(format!("airframes.{name}: no engine look {}", a.engine));
            }
            if a.nozzles.is_empty() || a.nozzles.iter().any(|n| n.radius_m <= 0.0) {
                return fail(format!(
                    "airframes.{name}: needs nozzles of positive radius"
                ));
            }
        }
        Ok(())
    }

    fn trail(&self, kind: Emitter) -> &TrailLook {
        match kind {
            Emitter::Nozzle { .. } => &self.engine_trail,
            Emitter::Wingtip => &self.wingtip_trail,
        }
    }
}

/// One emitter on a drawn jet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Emitter {
    /// A nozzle's exit, its radius.
    Nozzle {
        radius: f64,
    },
    Wingtip,
}

/// One running jet as its model is drawn this frame
/// ([`DrawnModels::jets`](crate::units::drawn::DrawnModels::jets)).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawnJet {
    /// Its key as the effect publication keys it (`Shooter::key`), as bits.
    pub key: u64,
    /// Its engine kind (`engines`).
    pub engine: String,
    /// Where its model stands (its foot), and its heading.
    pub at: Vec3,
    pub yaw: f64,
    /// Its tail's way: the model's −X.
    pub aft: Vec3,
    /// Its load: 1 flying level, more in a banked turn.
    pub load: f64,
    /// Its type's cruise speed, m/s.
    pub cruise_mps: f64,
    /// Each emitter and where it is drawn.
    pub emitters: Vec<(Emitter, Vec3)>,
}

fn distance(a: Vec3, b: Vec3) -> f64 {
    crate::world::length([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl JetTrails {
    /// Every trail's ribbons at `clock`, drifting on `drift` (m/s), from
    /// what was laid by then; then each jet drawn now's glows and flames.
    pub fn draw(
        &self,
        style: &JetExhaustStyle,
        clock: f64,
        drift: [f64; 2],
        min_px: f64,
        batch: &mut EffectBatch,
    ) {
        let now = |t: &&trails::Track| t.drawn == Some(clock);
        for t in &self.tracks {
            let seen = now(&t);
            for (i, (ring, &kind)) in t.rings.iter().zip(&t.kinds).enumerate() {
                let scale = match kind {
                    Emitter::Nozzle { .. } => style.engines.get(&t.engine).map_or(0.0, |e| e.trail),
                    Emitter::Wingtip => 1.0,
                };
                let head = t.heads.get(i).copied().filter(|_| seen);
                draw_ring(
                    style,
                    style.trail(kind),
                    ring,
                    head,
                    scale,
                    clock,
                    drift,
                    min_px,
                    batch,
                );
            }
        }
        for t in self.tracks.iter().filter(now) {
            let Some(look) = style.engines.get(&t.engine) else {
                continue;
            };
            let (heat, reheat) = t.heat(style);
            for (&at, kind) in t.heads.iter().zip(&t.kinds) {
                if let Emitter::Nozzle { radius } = *kind {
                    draw_nozzle(look, at, t.aft, radius, heat, reheat, clock, t.key, batch);
                }
            }
        }
    }
}

/// One emitter's trail: a ribbon between each two joined samples laid by
/// `clock`, as wide and faint as their age, and one from the newest to
/// `head`, where it is drawn now, unless that is further than it could fly.
#[allow(clippy::too_many_arguments)]
fn draw_ring(
    style: &JetExhaustStyle,
    look: &TrailLook,
    ring: &VecDeque<Sample>,
    head: Option<Vec3>,
    scale: f64,
    clock: f64,
    drift: [f64; 2],
    min_px: f64,
    batch: &mut EffectBatch,
) {
    if scale <= 0.0 {
        return;
    }
    let point = |p: Vec3, age: f64| [p[0] + drift[0] * age, p[1] + drift[1] * age, p[2]];
    let mut piece = |a: Sample, b: Sample| {
        let (age_a, age_b) = (clock - a.t, clock - b.t);
        let age = (age_a + age_b) / 2.0;
        let x = (age / look.life_s).min(1.0);
        let strength = (a.strength + b.strength) / 2.0;
        let fade_in = smooth(age / look.fade_in_s.max(1e-6));
        let opacity = scale
            * (look.opacity[0] + (look.opacity[1] - look.opacity[0]) * strength)
            * fade_in
            * (1.0 - x)
            * (1.0 - x);
        if opacity <= 1e-4 {
            return;
        }
        let width = look.width_m[0] + (look.width_m[1] - look.width_m[0]) * x.sqrt();
        batch.ribbon(
            point(a.p, age_a),
            point(b.p, age_b),
            width,
            min_px,
            look.albedo,
            opacity,
        );
    };
    let mut prev: Option<Sample> = None;
    for &s in ring.iter().take_while(|s| s.t <= clock) {
        if let Some(p) = prev.filter(|_| s.joined) {
            piece(p, s);
        }
        prev = Some(s);
    }
    if let (Some(p), Some(head)) = (prev, head) {
        let reach = style.break_mps * (clock - p.t) + 1.0;
        if p.t < clock && distance(p.p, head) <= reach {
            let head = Sample {
                t: clock,
                p: head,
                joined: true,
                ..p
            };
            piece(p, head);
        }
    }
}

/// A nozzle's glow, and past its reheat point its flame: a soft plume, a
/// hot core and shock diamonds along its tail's way, flickering by `clock`.
#[allow(clippy::too_many_arguments)]
fn draw_nozzle(
    look: &EngineLook,
    at: Vec3,
    aft: Vec3,
    radius: f64,
    heat: f64,
    reheat: f64,
    clock: f64,
    key: u64,
    batch: &mut EffectBatch,
) {
    let g = &look.glow;
    let k = g.intensity[0] + (g.intensity[1] - g.intensity[0]) * heat + g.reheat_intensity * reheat;
    let back = |m: f64| [at[0] + aft[0] * m, at[1] + aft[1] * m, at[2] + aft[2] * m];
    // Just aft of the exit, so the airframe hides it from ahead.
    batch.glow(
        back(radius * 0.15),
        radius * g.radius,
        g.min_px,
        g.color,
        k,
        0.0,
        0.0,
    );
    let Some(f) = look.flame.filter(|_| reheat > 0.0) else {
        return;
    };
    // Each nozzle flickers on its own phase.
    let phase = (key % 997) as f64 * 0.37 + radius * 13.0;
    let flicker = 1.0
        + f.flicker
            * 0.5
            * (libm::sin(clock * 31.0 + phase) * 0.6 + libm::sin(clock * 53.0 + phase * 1.7) * 0.4);
    let length = radius * f.length * (0.35 + 0.65 * reheat) * flicker;
    let width = radius * f.width;
    // Out of the exit, so the plume never paints over the nozzle's lip.
    let start = back(radius * 0.35);
    batch.streak(
        back(length),
        start,
        width,
        g.min_px,
        f.color,
        f.intensity * reheat,
        [0.0, 1.0],
        true,
        0.0,
    );
    let core = length * 0.45;
    batch.streak(
        back(core),
        start,
        width * 0.45,
        g.min_px,
        f.core_color,
        f.core_intensity * reheat,
        [0.0, 1.0],
        true,
        0.0,
    );
    for d in 0..f.diamonds {
        let u = (d as f64 + 1.0) / (f.diamonds as f64 + 1.0);
        let fade = (1.0 - u) * reheat;
        batch.glow(
            back(length * u * 0.8),
            width * 0.32 * (1.0 - 0.3 * u),
            0.0,
            f.core_color,
            f.diamond_intensity * fade,
            0.0,
            0.0,
        );
    }
}
