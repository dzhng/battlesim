//! The battle's light as the shaders read it (18b's `World.light`): the
//! fixture's `presentation.light` resolved into the sky's, the bloom's and the
//! grade's uniform records. The sky model is the browser's
//! (`skyParameters.ts`, `skyLight` in `physicalSky.ts`): a Hillaire-style
//! atmosphere whose turbidity sets the haze and the overcast, and a cumulus
//! layer over it. A renderer bakes its sky lookup from [`SkyLight`] and
//! derives nothing itself.
use serde::Deserialize;

use crate::records::{Bloom, Grade, SkyLight};

// Earth's atmosphere (Hillaire, EGSR 2020), in km, z up: the bake's constants.
const PLANET_RADIUS_KM: f64 = 6360.0;
const ATMOSPHERE_TOP_KM: f64 = 6460.0;
const RAYLEIGH_SCALE_KM: f64 = 8.0;
const MIE_SCALE_KM: f64 = 1.2;
const BETA_RAYLEIGH: [f64; 3] = [5.802e-3, 13.558e-3, 33.1e-3];
const BETA_MIE_SCATTER: f64 = 3.996e-3;
const BETA_MIE_EXTINCTION: f64 = 4.44e-3;
const EYE_ALTITUDE_KM: f64 = 0.2;
/// The multiple-scattering floor (Hillaire's ψms as one constant).
const MS_FLOOR: f64 = 0.32;
/// A low sun's warm aureole, gated to nothing at noon and under overcast.
const LOW_SUN_AUREOLE_RADIANCE: f64 = 0.55;
/// The lower hemisphere's ground bounce (dry earth).
const GROUND_BOUNCE_TINT: [f64; 3] = [0.34, 0.3, 0.25];
const SUN_DISC_RADIANCE: f64 = 2.5;

/// `presentation.light`, as far as the sky and post read it.
#[derive(Clone, Debug, Deserialize)]
pub struct LightSettings {
    /// About +z from +x, and above the ground (rad).
    pub sun_azimuth: f64,
    pub sun_elevation: f64,
    pub sky: SkySettings,
    pub exposure: f64,
    pub grade: GradeSettings,
    pub bloom: BloomSettings,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkySettings {
    /// 1 is pure Rayleigh air; from 4 to 7 the sky turns overcast.
    pub turbidity: f64,
    /// The single-scattered sky's brightness against the sunlit ground.
    pub radiance: f64,
    pub clouds: CloudSettings,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CloudSettings {
    pub coverage: f64,
    pub height_m: f64,
    pub scale_m: f64,
    pub softness: f64,
    pub opacity: f64,
    pub radiance: f64,
    pub shade: [f64; 3],
}

#[derive(Clone, Debug, Deserialize)]
pub struct GradeSettings {
    pub saturation: f64,
    pub contrast: f64,
    pub split_tone: f64,
    pub shadow_lift: f64,
    pub shadow_tint: [f64; 3],
    pub highlight_tint: [f64; 3],
    pub lift: [f64; 3],
}

#[derive(Clone, Debug, Deserialize)]
pub struct BloomSettings {
    pub strength: f64,
    pub radius: f64,
    pub threshold: f64,
    pub smooth_width: f64,
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mie_scale(turbidity: f64) -> f64 {
    (turbidity - 1.0).max(0.05)
}

/// Beer–Lambert transmittance from the eye toward `direction` through the
/// exponential atmosphere, normalised to its brightest channel: the sun's tint.
fn transmittance_to_sun(direction: [f64; 3], turbidity: f64) -> [f64; 3] {
    const STEPS: usize = 64;
    let mie = mie_scale(turbidity) * BETA_MIE_EXTINCTION;
    let r0 = PLANET_RADIUS_KM + EYE_ALTITUDE_KM;
    let mu = direction[2].max(0.0);
    let top = -r0 * mu
        + (r0 * r0 * (mu * mu - 1.0) + ATMOSPHERE_TOP_KM * ATMOSPHERE_TOP_KM).sqrt();
    let dt = top / STEPS as f64;
    let mut depth = [0.0; 3];
    for i in 0..STEPS {
        let t = (i as f64 + 0.5) * dt;
        let h = (r0 * r0 + t * t + 2.0 * r0 * t * mu).sqrt() - PLANET_RADIUS_KM;
        let rayleigh = (-h / RAYLEIGH_SCALE_KM).exp();
        let mie_density = (-h / MIE_SCALE_KM).exp();
        for (d, beta) in depth.iter_mut().zip(BETA_RAYLEIGH) {
            *d += (beta * rayleigh + mie * mie_density) * dt;
        }
    }
    let t = depth.map(|d| (-d).exp());
    let peak = t[0].max(t[1]).max(t[2]).max(1e-6);
    t.map(|v| v / peak)
}

/// `a · b · k` per channel in f32, left to right, as a shader compiler folds
/// the literals these values once were.
fn folded(a: [f64; 3], b: f64, k: f64) -> [f32; 3] {
    a.map(|v| v as f32 * b as f32 * k as f32)
}

fn f32s<const N: usize>(v: [f64; N]) -> [f32; N] {
    v.map(|x| x as f32)
}

impl LightSettings {
    /// Unit vector toward the sun (z up).
    pub fn sun_direction(&self) -> [f64; 3] {
        let (az, el) = (self.sun_azimuth, self.sun_elevation);
        [el.cos() * az.cos(), el.cos() * az.sin(), el.sin()]
    }

    /// The sky's uniform.
    pub fn sky(&self) -> SkyLight {
        let sun = self.sun_direction();
        let turbidity = self.sky.turbidity;
        let overcast = smoothstep(4.0, 7.0, turbidity);
        let transmittance = transmittance_to_sun(sun, turbidity);
        let c = &self.sky.clouds;
        let across = sun[0].hypot(sun[1]);
        let to_sun = if across > 1e-6 {
            [sun[0] / across, sun[1] / across]
        } else {
            [1.0, 0.0]
        };
        let low = 1.0 - c.coverage;
        SkyLight {
            sun_direction: f32s(sun),
            overcast: overcast as f32,
            sun_transmittance: f32s(transmittance),
            radiance: self.sky.radiance as f32,
            ground_tint: f32s(GROUND_BOUNCE_TINT.map(|v| v + (0.88 - v) * overcast)),
            mie_scatter: (BETA_MIE_SCATTER * mie_scale(turbidity)) as f32,
            mie_extinction: (BETA_MIE_EXTINCTION * mie_scale(turbidity)) as f32,
            multiple_scatter: (MS_FLOOR * sun[2].max(0.0).sqrt() / (4.0 * std::f64::consts::PI))
                as f32,
            aureole: ((1.0 - overcast).powi(2)
                * (1.0 - smoothstep(0.25, 0.85, sun[2]))
                * LOW_SUN_AUREOLE_RADIANCE) as f32,
            overcast_scale: sun[2].max(0.05).sqrt() as f32,
            disc_radiance: (SUN_DISC_RADIANCE * (1.0 - overcast)) as f32,
            cloud_reach: (c.height_m / c.scale_m) as f32,
            cloud_edge: f32s([low - c.softness, low + c.softness]),
            cloud_to_sun: f32s([to_sun[0] * 0.07, to_sun[1] * 0.07]),
            cloud_low: low as f32,
            cloud_shadow_top: (low + 0.3) as f32,
            cloud_core_top: (low + 0.35) as f32,
            // The cloud layer gives way to the overcast dome.
            cloud_opacity: (c.opacity * (1.0 - overcast)) as f32,
            pad0: [0.0; 2],
            cloud_lit: folded(transmittance, c.radiance, 1.0),
            cloud_silver: (c.radiance * 0.6) as f32,
            cloud_base: folded(c.shade, c.radiance, 0.7),
            pad1: 0.0,
        }
    }

    /// The bloom's uniform: `radius` moves weight from the fine levels to the
    /// broad ones.
    pub fn bloom(&self) -> Bloom {
        let b = &self.bloom;
        let w = [1.0, 0.8, 0.6, 0.4, 0.2].map(|f: f64| f * (1.0 - b.radius) + (1.2 - f) * b.radius);
        Bloom {
            weights: f32s([w[0], w[1], w[2], w[3]]),
            weight4: w[4] as f32,
            strength: b.strength as f32,
            threshold: b.threshold as f32,
            knee: (b.threshold + b.smooth_width) as f32,
        }
    }

    /// The grade's uniform.
    pub fn grade(&self) -> Grade {
        let g = &self.grade;
        let rgb = |v: [f64; 3], w: f64| f32s([v[0], v[1], v[2], w]);
        Grade {
            saturation: g.saturation as f32,
            contrast: g.contrast as f32,
            split_tone: g.split_tone as f32,
            shadow_lift: g.shadow_lift as f32,
            exposure: self.exposure as f32,
            pad: [0.0; 3],
            shadow_tint: rgb(g.shadow_tint, 1.0),
            highlight_tint: rgb(g.highlight_tint, 1.0),
            lift: rgb(g.lift, 0.0),
        }
    }
}
