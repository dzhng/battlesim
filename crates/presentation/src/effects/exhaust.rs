//! A jet's engines and wings as a film shows them: each nozzle's hot glow,
//! brighter the faster it flies, with an afterburner's flame and shock
//! diamonds once it flies faster than its cruise; and the trails its engines
//! leave, widening and fading behind it over a few seconds. Its wingtips
//! leave vapour only while its wings pull hard: in a tight turn or a pull-up,
//! never in level flight however fast.
//!
//! What it draws from is data. Each airframe's nozzles and wingtips are
//! authored on its appearance (`airframes`), in its model's own frame, and
//! its engine kind names a look (`engines`): a look without a `flame` (a
//! high-bypass turbofan's) only ever glows. Its throttle is how fast it is
//! published flying against its type's cruise, and its wings' load how its
//! published flight bends ([`trails`]); a downed airframe's engines are out. Glows and flames sit on the drawn model; trails are laid tick by
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
    /// When the wingtips leave vapour.
    pub vapour: VapourLook,
    /// Each engine kind's look.
    pub engines: BTreeMap<String, EngineLook>,
    /// The engine trails drawn: one of `engine_styles`.
    pub engine_style: String,
    /// Looks the engines' trails can take, each one or more layers laid
    /// from the same points behind each nozzle (a haze, a condensation
    /// trail, a smoky core), drawn in their order.
    pub engine_styles: BTreeMap<String, Vec<TrailLook>>,
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

/// When wingtip vapour forms, from the load the wings carry: the lift they
/// make over the jet's weight (1 in level flight, 2 in a 60° banked turn),
/// read from how its published flight bends.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct VapourLook {
    /// The load where vapour forms, and where it is thickest.
    pub load: [f64; 2],
    /// Once formed, it holds until the load falls below this.
    pub off_load: f64,
    /// How thick it forms (a share of its thickest), and holds while formed.
    pub onset: f64,
    /// How long it takes to form, and to stop forming, s.
    pub attack_s: f64,
    pub release_s: f64,
}

/// A trail: lit, blended over, widening and fading as it ages.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrailLook {
    pub albedo: Vec3,
    /// Its opacity at its weakest and strongest (an engine: standing and
    /// at cruise; a wingtip: no vapour and its thickest).
    pub opacity: [f64; 2],
    pub life_s: f64,
    /// Its width when made and when it has lived its life, m.
    pub width_m: [f64; 2],
    /// It thickens from nothing over this, s.
    pub fade_in_s: f64,
    /// How sharply it thins toward its end: its opacity falls as the
    /// share of its life left to this power.
    #[serde(default = "square")]
    pub taper: f64,
    /// Its added opacity at full afterburner (an engine's).
    #[serde(default)]
    pub reheat: f64,
    /// Vapour broken into wisps rather than one even ribbon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakup: Option<Breakup>,
}

/// How a trail breaks into wisps: anchored where it was laid, so the wisps
/// hang in the air as it ages rather than sliding along it.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Breakup {
    /// How much of it the gaps take away, 0 to 1.
    pub amount: f64,
    /// About how long one wisp is, m.
    pub length_m: f64,
}

fn square() -> f64 {
    2.0
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
        let v = &self.vapour;
        if !ordered(self.reheat_share) || !ordered(v.load) {
            return fail("reheat_share and vapour.load must each rise".into());
        }
        if !(1.0..=v.load[0]).contains(&v.off_load)
            || !(0.0..=1.0).contains(&v.onset)
            || v.attack_s <= 0.0
            || v.release_s <= 0.0
        {
            return fail(
                "vapour: off_load from 1 to load[0], onset in [0, 1], attack_s and release_s positive"
                    .into(),
            );
        }
        let Some(layers) = self.engine_styles.get(&self.engine_style) else {
            return fail(format!(
                "engine_style: no engine_styles.{}",
                self.engine_style
            ));
        };
        if layers.is_empty() {
            return fail(format!(
                "engine_styles.{}: needs a layer",
                self.engine_style
            ));
        }
        let looks = self
            .engine_styles
            .iter()
            .flat_map(|(name, layers)| {
                layers
                    .iter()
                    .map(move |t| (format!("engine_styles.{name}"), t))
            })
            .chain([("wingtip_trail".to_string(), &self.wingtip_trail)]);
        for (name, t) in looks {
            let opaque = t.opacity.iter().all(|o| (0.0..=1.0).contains(o));
            let broken = t
                .breakup
                .is_none_or(|b| (0.0..=1.0).contains(&b.amount) && b.length_m > 0.0);
            if t.life_s <= 0.0
                || t.width_m[0] <= 0.0
                || t.width_m[1] < t.width_m[0]
                || !opaque
                || t.taper <= 0.0
                || !broken
                || !(0.0..=1.0).contains(&t.reheat)
            {
                return fail(format!(
                    "{name}: life_s, width_m and taper must be positive, width_m must not narrow, opacity and reheat in [0, 1], breakup amount in [0, 1] over a positive length_m"
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

    /// The layers an emitter's trail is drawn in.
    fn layers(&self, kind: Emitter) -> &[TrailLook] {
        match kind {
            Emitter::Nozzle { .. } => self
                .engine_styles
                .get(&self.engine_style)
                .map_or(&[], Vec::as_slice),
            Emitter::Wingtip => std::slice::from_ref(&self.wingtip_trail),
        }
    }

    /// How long an emitter's trail is kept: its longest layer's life, s.
    fn life(&self, kind: Emitter) -> f64 {
        self.layers(kind)
            .iter()
            .map(|t| t.life_s)
            .fold(0.0, f64::max)
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
                for layer in style.layers(kind) {
                    draw_ring(style, layer, ring, head, scale, clock, drift, min_px, batch);
                }
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

/// The pieces a still-thickening stretch of trail is drawn in.
const FADE_IN_PARTS: usize = 4;

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
    let mut stretch = |a: Sample, b: Sample| {
        let (age_a, age_b) = (clock - a.t, clock - b.t);
        let age = (age_a + age_b) / 2.0;
        let x = (age / look.life_s).min(1.0);
        let strength = (a.strength + b.strength) / 2.0;
        let reheat = (a.reheat + b.reheat) / 2.0;
        let fade_in = smooth(age / look.fade_in_s.max(1e-6));
        let opacity = scale
            * (look.opacity[0]
                + (look.opacity[1] - look.opacity[0]) * strength
                + look.reheat * reheat)
                .min(1.0)
            * fade_in
            * libm::pow(1.0 - x, look.taper);
        if opacity <= 1e-4 {
            return;
        }
        let width = look.width_m[0] + (look.width_m[1] - look.width_m[0]) * x.sqrt();
        let (a_at, b_at) = (point(a.p, age_a), point(b.p, age_b));
        match look.breakup {
            Some(w) => batch.wisp(
                a_at,
                b_at,
                width,
                min_px,
                look.albedo,
                opacity,
                [a.along / w.length_m, b.along / w.length_m],
                w.amount,
            ),
            None => batch.ribbon(a_at, b_at, width, min_px, look.albedo, opacity),
        }
    };
    // One stretch per tick holds one opacity: where the trail is still
    // thickening from nothing, split it so it fades in smoothly rather than
    // in a step a tick long.
    let mut piece = |a: Sample, b: Sample| {
        let young = clock - a.t < 2.0 * look.fade_in_s;
        let parts = if young { FADE_IN_PARTS } else { 1 };
        let at = |k: usize| {
            let f = k as f64 / parts as f64;
            let mix = |x: f64, y: f64| x + (y - x) * f;
            Sample {
                t: mix(a.t, b.t),
                p: [0, 1, 2].map(|i| mix(a.p[i], b.p[i])),
                strength: mix(a.strength, b.strength),
                reheat: mix(a.reheat, b.reheat),
                joined: true,
                along: mix(a.along, b.along),
            }
        };
        for k in 0..parts {
            stretch(at(k), at(k + 1));
        }
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
                along: p.along + distance(p.p, head),
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
