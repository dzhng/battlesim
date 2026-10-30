//! `presentation.effects`: every curve and size the effects are drawn by,
//! read from the fixture and checked once ([`EffectPresentation::check`]).
use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use super::cook_off::CookOffFeel;
use super::high_air_shadow::HighAirShadowStyle;
use crate::world::Vec3;

/// A tracer on a very short stretch lives at most this past its tick, s.
pub(super) const TRACER_TAIL_MAX_S: f64 = 1.0;

/// A kind's row, or the table's `default`.
pub(super) fn pick<'a, T>(table: &'a HashMap<String, T>, kind: &str) -> &'a T {
    table.get(kind).unwrap_or_else(|| &table["default"])
}

/// Shared effect tuning and the battle's admitted weapon looks.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectPresentation {
    /// Instances the effect pass holds; past it an effect is dropped.
    pub capacity: usize,
    /// The narrowest a streak or smallest a sprite draws, pixels.
    pub min_px: f64,
    #[serde(default, skip_serializing_if = "crate::looks::WeaponLooks::is_empty")]
    pub looks: crate::looks::WeaponLooks,
    pub impacts: HashMap<String, Arc<ImpactStyle>>,

    pub sparks: SparkStyle,
    pub ricochet_sparks: f64,
    /// Sparks a second a hull grinding a metal body throws.
    pub grind_sparks_per_s: f64,
    pub blast: BlastStyle,
    pub cook_off: CookOffFeel,
    pub dust: DustStyle,
    pub wake: WakeStyle,
    /// The dust a heavy gun's shot raises; none where absent.
    pub shot_dust: Option<ShotDustStyle>,
    pub damage_smoke: DamageSmokeStyle,
    pub rotor_blur: RotorBlurStyle,
    /// The shadow beneath a high-air aircraft.
    pub high_air_shadow: HighAirShadowStyle,
    /// Smoke sources' looks by kind; a kind without one draws nothing.
    pub smoke: HashMap<String, Arc<SmokeSourceStyle>>,
    /// Instances every smoke source together may hold; past it each thins alike.
    pub smoke_budget: f64,
    /// How far round a surface a cast light reaches past facing it, 0..1.
    pub cast_wrap: f64,
}

/// A light an effect casts: colour, irradiance at its centre and reach; a
/// burning-out one fades as 1 − (age/duration)² over `duration_s`, and a
/// flash's sits `forward_m` along the shot.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CastStyle {
    pub color: Vec3,
    pub intensity: f64,
    pub radius_m: f64,
    #[serde(default)]
    pub duration_s: f64,
    #[serde(default)]
    pub forward_m: f64,
}

/// A round's light along its tail: a glow at least `min_px` wide, or a hot
/// core over the tail's front `share`.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TracerLight {
    pub color: Vec3,
    pub intensity: f64,
    pub width_m: f64,
    pub min_px: f64,
    #[serde(default)]
    pub share: f64,
}

/// A round in flight: a glow along a tail `tail_s` of its flight long
/// (within `tail_m`), and optionally a hot core, a dark body, a smoke trail
/// and a light.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TracerStyle {
    /// Chance to show a new round; its whole flight keeps the choice.
    pub chance: Option<f64>,
    pub zoom_boost: Option<f64>,
    /// A sharp line of this screen width instead of a world-sized glow.
    pub line_px: Option<f64>,
    /// Light energy kept at the rear, relative to the head.
    pub tail_brightness: Option<f64>,
    pub tail_s: f64,
    pub tail_m: [f64; 2],
    pub glow: TracerLight,
    pub core: Option<TracerLight>,
    pub body: Option<Body>,
    pub smoke: Option<TrailStyle>,
    pub cast: Option<CastStyle>,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub color: Vec3,
    pub size_m: f64,
}

/// A soft sun-lit puff that rises, spreads and drifts downwind.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PuffStyle {
    pub albedo: Vec3,
    pub opacity: f64,
    /// Radius at birth and at death, metres.
    pub size_m: [f64; 2],
    pub life_s: f64,
    pub rise_mps: f64,
    pub spread_mps: f64,
}

/// A smoke trail: an unbroken ribbon at `ribbon` opacity, and a puff every `spacing_m`.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct TrailStyle {
    #[serde(flatten)]
    pub puff: PuffStyle,
    pub ribbon: f64,
    pub spacing_m: f64,
}

/// `count` puffs born over the first `over_s` seconds.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct PuffBurst {
    #[serde(flatten)]
    pub puff: PuffStyle,
    pub count: f64,
    pub over_s: f64,
}

/// Dust off each track every `spacing_m`, thicker with speed.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct DustStyle {
    #[serde(flatten)]
    pub puff: PuffStyle,
    pub spacing_m: f64,
    pub min_speed_mps: f64,
    pub full_speed_mps: f64,
}

/// How a unit the side sees wading in standing water pushes on it: rings
/// round a still or slow wader, `ring_hz` a second each, and at `vee_mps`
/// or faster a V-wake, a push every `spacing_m` along its way, where the
/// water stands `vee_depth_m` deep or more (shallower, a splash's rings).
/// A soldier's push reaches `soldier_push_radius_m`, a hull's its half width.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct WakeStyle {
    pub ring_hz: f64,
    pub vee_mps: f64,
    pub vee_depth_m: f64,
    pub spacing_m: f64,
    pub soldier_push_radius_m: f64,
    pub ring_strength: f64,
    pub vee_strength: f64,
}

/// The dust a heavy gun's shot raises at full strength (`FlashStyle::dust`):
/// shaken off the whole hull as it jolts, and blown off the ground by the
/// muzzle blast, out to `reach_m` ahead of the muzzle where it points.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShotDustStyle {
    pub hull: PuffBurst,
    pub ground: GroundDust,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct GroundDust {
    #[serde(flatten)]
    pub burst: PuffBurst,
    pub reach_m: f64,
}

/// Smoke a damaged hull trails: a puff `rate_hz` a second where it is.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct DamageSmokeStyle {
    #[serde(flatten)]
    pub puff: PuffStyle,
    pub rate_hz: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlashStyle {
    pub color: Vec3,
    pub intensity: f64,
    pub size_m: f64,
    pub tongue_m: f64,
    pub duration_s: f64,
    /// A muzzle fireball's size (0: none) and how long it burns.
    pub fireball_m: f64,
    pub fireball_s: f64,
    pub cast: Option<CastStyle>,
    /// How much dust a hull's shot of this kind shakes off the hull and
    /// blows off the ground, as a share of `shot_dust`'s; none where absent.
    #[serde(default)]
    pub dust: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoundScale {
    pub size: f64,
    pub duration: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactStyle {
    pub color: Vec3,
    pub opacity: f64,
    pub size_m: f64,
    pub duration_s: f64,
    /// Sparks a tracer round throws off the surface, and its flash (0: none).
    pub sparks: f64,
    pub flash: f64,
    pub cast: Option<CastStyle>,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SparkStyle {
    pub color: Vec3,
    pub intensity: f64,
    pub speed_mps: f64,
    pub duration_s: f64,
    pub length_m: f64,
    pub width_m: f64,
}

/// A burst: its fireball per metre of blast radius, and the one the
/// style's numbers describe (`reference_size_m`); smaller ones burn, spark,
/// throw and light less.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlastStyle {
    pub size_per_radius: f64,
    pub reference_size_m: f64,
    pub duration_s: f64,
    pub tint: Vec3,
    pub emissive: f64,
    pub opacity: f64,
    pub flash: Vec3,
    pub flash_intensity: f64,
    pub spill: f64,
    pub spark_scale: f64,
    pub flash_duration_s: f64,
    pub sparks: f64,
    pub plume: PuffBurst,
    pub smoke: PuffBurst,
    pub cast: CastStyle,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlameStyle {
    pub tint: Vec3,
    pub emissive: f64,
    pub size_m: f64,
    pub life_s: f64,
    pub rate_hz: f64,
    /// The fire's glow in the air round it: colour, intensity and radius.
    pub light: Vec3,
    pub light_intensity: f64,
    pub light_m: f64,
    pub cast: Option<CastStyle>,
}

/// A smoke source's look: it burns `burn_s`, smoulders `smoulder_s`, then is out.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeSourceStyle {
    pub burn_s: f64,
    pub smoulder_s: f64,
    pub flame: FlameStyle,
    pub smoke: PuffStyle,
    pub smoke_hz: f64,
    pub smoulder: PuffStyle,
    pub smoulder_hz: f64,
}

/// A rotor as a shutter sees it: each blade smeared over the angle it swept.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RotorBlurStyle {
    pub albedo: f64,
    /// The share of a frame the shutter stays open.
    pub shutter: f64,
    pub hub_share: f64,
    pub min_chord_px: f64,
    /// The least a spinning disc covers, 0..1.
    pub disc: f64,
}

fn ensure(ok: bool, why: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("presentation.effects.{}", why()))
    }
}

impl EffectPresentation {
    /// Read effect tuning: every material table is complete, every light reaches,
    /// and every life is bounded.
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let p: Self = crate::settings::read(value, "presentation.effects")?;
        p.check()?;
        Ok(p)
    }

    pub fn check(&self) -> Result<(), String> {
        crate::settings::validate::numbers(
            self,
            "presentation.effects",
            &[".forward_m", ".spin_turns", ".up_m"],
        )?;
        ensure(self.capacity > 0, || "capacity must be positive".into())?;
        ensure(self.impacts.contains_key("default"), || {
            "impacts needs a default".into()
        })?;
        ensure((0.0..=1.0).contains(&self.cast_wrap), || {
            "cast_wrap must be in [0, 1]".into()
        })?;
        let cast = |at: String, c: &Option<CastStyle>, fades: bool| match c {
            Some(c) => ensure(
                c.radius_m > 0.0 && c.intensity >= 0.0 && (!fades || c.duration_s > 0.0),
                || {
                    format!("{at}.cast needs a positive reach, an intensity ≥ 0 and (fading) a duration")
                },
            ),
            None => Ok(()),
        };
        for (k, look) in &self.looks.looks {
            let Some(t) = &look.tracer else { continue };
            cast(format!("tracers.{k}"), &t.cast, false)?;
        }
        for (k, look) in &self.looks.looks {
            let Some(f) = &look.flash else { continue };
            cast(format!("flashes.{k}"), &f.cast, true)?;
        }
        for (k, s) in &self.smoke {
            cast(format!("smoke.{k}.flame"), &s.flame.cast, false)?;
        }
        cast("blast".into(), &Some(self.blast.cast), true)?;
        for (hit, i) in &self.impacts {
            cast(format!("impacts.{hit}"), &i.cast, true)?;
            // An impact's light burns within its own life.
            let lit = i.cast.map_or(0.0, |c| c.duration_s);
            ensure(lit <= i.duration_s, || {
                format!("impacts.{hit}.cast outlives its impact")
            })?;
            for (kind, look) in &self.looks.looks {
                let Some(s) = look.impact.as_ref().and_then(|i| i.surfaces.get(hit)) else {
                    continue;
                };
                let ok = s.size.is_finite() && s.size > 0.0 && s.duration.is_finite();
                ensure(
                    ok && s.duration > 0.0 && lit <= i.duration_s * s.duration,
                    || {
                        format!("weapon_looks.{kind}.impact.surfaces.{hit} must be positive and outlast its light")
                    },
                )?;
            }
        }
        let b = &self.blast;
        ensure(b.reference_size_m > 0.0 && b.size_per_radius > 0.0, || {
            "blast: reference_size_m and size_per_radius must be positive".into()
        })?;
        ensure(b.cast.duration_s <= b.duration_s, || {
            "blast.cast outlives its blast".into()
        })?;
        self.cook_off.validate()?;
        for (k, look) in &self.looks.looks {
            let Some(f) = &look.flash else { continue };
            ensure(f.dust.is_finite() && f.dust >= 0.0, || {
                format!("flashes.{k}.dust must be ≥ 0")
            })?;
        }
        if let Some(d) = &self.shot_dust {
            for (k, b) in [("hull", &d.hull), ("ground", &d.ground.burst)] {
                ensure(
                    b.count.fract() == 0.0 && b.count >= 0.0 && b.over_s >= 0.0,
                    || format!("shot_dust.{k}: a whole count ≥ 0, over_s ≥ 0"),
                )?;
            }
            ensure(d.ground.reach_m >= 0.0, || {
                "shot_dust.ground.reach_m must be ≥ 0".into()
            })?;
        }
        ensure(self.damage_smoke.rate_hz > 0.0, || {
            "damage_smoke.rate_hz must be positive".into()
        })?;
        let r = &self.rotor_blur;
        ensure(
            (0.0..=1.0).contains(&r.albedo)
                && r.shutter > 0.0
                && r.shutter <= 1.0
                && r.hub_share > 0.0
                && r.hub_share < 1.0
                && r.min_chord_px >= 0.0
                && (0.0..1.0).contains(&r.disc),
            || {
                "rotor_blur: albedo in [0, 1], shutter in (0, 1], hub_share in (0, 1), min_chord_px ≥ 0, disc in [0, 1)".into()
            },
        )?;
        ensure(self.high_air_shadow.valid(), || {
            "high_air_shadow: opacity in [0, 1], fade_half_m > 0, blur_m ≥ 0, blur_per_m ≥ 0, ground_band_m > 0".into()
        })?;
        ensure(self.max_lifetime().is_finite(), || {
            "every life must be finite".into()
        })
    }

    /// The longest any one effect lives, seconds (`maxEffectLifetime`).
    fn max_lifetime(&self) -> f64 {
        let b = &self.blast;
        let burst = |p: &PuffBurst| p.over_s + p.puff.life_s;
        let fire = b.duration_s.max(burst(&b.plume)).max(burst(&b.smoke));
        let c = &self.cook_off;
        let mut longest = fire
            .max(self.dust.puff.life_s)
            .max(
                self.shot_dust
                    .map_or(0.0, |d| burst(&d.hull).max(burst(&d.ground.burst))),
            )
            .max(self.damage_smoke.puff.life_s)
            .max(self.sparks.duration_s * b.spark_scale.max(1.0).sqrt());
        for f in &c.fireballs {
            longest = longest.max(c.delay_s + f.after_s + fire);
        }
        longest = longest.max(c.impact_after() + burst(&b.plume));
        for look in self.looks.looks.values() {
            let Some(f) = &look.flash else { continue };
            let lit = f.cast.map_or(0.0, |c| c.duration_s);
            longest = longest.max(f.duration_s).max(f.fireball_s).max(lit);
        }
        for (hit, i) in &self.impacts {
            longest = longest.max(i.duration_s);
            for look in self.looks.looks.values() {
                if let Some(s) = look.impact.as_ref().and_then(|i| i.surfaces.get(hit)) {
                    longest = longest.max(i.duration_s * s.duration);
                }
            }
        }
        for s in self.smoke.values() {
            longest = longest
                .max(s.flame.life_s)
                .max(s.smoke.life_s)
                .max(s.smoulder.life_s);
        }
        for look in self.looks.looks.values() {
            let Some(t) = &look.tracer else { continue };
            if let Some(s) = &t.smoke {
                longest = longest.max(s.puff.life_s);
            }
        }
        longest + 1.0
    }
}

impl TracerStyle {
    pub(crate) fn validate(&self, kind: &str) -> Result<(), String> {
        let at = |why: &str| format!("tracers.{kind}.{why}");
        let within = |v: Option<f64>, ok: fn(f64) -> bool| v.is_none_or(|v| v.is_finite() && ok(v));
        ensure(within(self.chance, |c| (0.0..=1.0).contains(&c)), || {
            at("chance must be in [0, 1]")
        })?;
        ensure(within(self.zoom_boost, |z| z > 1.0), || {
            at("zoom_boost must be above 1")
        })?;
        ensure(within(self.line_px, |l| l > 0.0), || {
            at("line_px must be positive")
        })?;
        ensure(
            within(self.tail_brightness, |b| (0.0..=1.0).contains(&b)),
            || at("tail_brightness must be in [0, 1]"),
        )?;
        ensure(
            self.tail_s > 0.0 && self.tail_s <= TRACER_TAIL_MAX_S,
            || at("tail_s must be in (0, 1]"),
        )?;
        ensure(
            self.tail_m[0] > 0.0 && self.tail_m[1] >= self.tail_m[0],
            || at("tail_m must be [least, most], least above 0"),
        )?;
        ensure(
            self.core.is_none_or(|c| {
                c.share > 0.0 && c.share <= 1.0 && c.min_px.is_finite() && c.min_px > 0.0
            }),
            || at("core needs a share in (0, 1] and a positive min_px"),
        )?;
        ensure(self.smoke.is_none_or(|s| s.spacing_m > 0.0), || {
            at("smoke.spacing_m must be positive")
        })
    }
}
