//! The battle's light as the shaders read it (`World.light`): the
//! fixture's `presentation.light` resolved into the uniform records of the
//! sky, the haze, the sun and post ([`LightRecords`]). The sky model is a
//! Hillaire-style atmosphere whose turbidity sets the haze, the sun's tint and
//! the overcast, with a cumulus layer over it. Both clients upload the records
//! as they are: a renderer bakes its sky lookup from [`SkyLight`] and derives
//! nothing itself.
mod check;
use serde::Deserialize;

use crate::records::{Aerial, Bloom, Grade, SkyLight, Sun};

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
/// Neutral ground-fog extinction (km⁻¹) per (turbidity − onset)²: heavy
/// weather only; clear turbidities add none.
const FOG_COEFF_KM: f64 = 0.026;
const FOG_TURBIDITY_ONSET: f64 = 4.0;
/// Where the haze's horizon sky takes over from the view's: the fade below
/// and above the horizon, widened as the visibility clears from 8 to 32 km.
const HORIZON_FADE_Z: [f64; 2] = [-0.18, 0.06];
const HORIZON_CLEAR_EXTRA_Z: [f64; 2] = [0.2, 0.08];
const HORIZON_VISIBILITY_KM: [f64; 2] = [8.0, 32.0];

/// `presentation.light`, as far as the records read it.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LightSettings {
    /// About +z from +x, and above the ground (rad).
    pub sun_azimuth: f64,
    pub sun_elevation: f64,
    pub sun_intensity: f64,
    /// The share of the sun a sun-shadowed surface keeps.
    pub shadow_floor: f64,
    pub sky: SkySettings,
    pub haze: HazeSettings,
    pub exposure: f64,
    pub grade: GradeSettings,
    pub bloom: BloomSettings,
    /// The sun's shadow cascades (`shadow.rs`).
    pub cascades: CascadeSettings,
}

/// `presentation.light.cascades`: how the sun's shadow maps are fitted.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CascadeSettings {
    /// Texels a side of each cascade's map.
    pub map_size: u32,
    /// No sun shadow beyond this view depth (m).
    pub max_far_m: f64,
    /// Practical split: 0 uniform, 1 logarithmic.
    pub split_lambda: f64,
    /// Normal bias per cascade in its texels per texel of PCF radius, with
    /// a floor (m).
    pub normal_bias_texels: f64,
    pub normal_bias_min_m: f64,
    /// Normalised depth bias, scaled by the cascade's index.
    pub depth_bias: f64,
    /// The penumbra's width (m): each cascade's PCF radius follows its texel.
    pub softness_m: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkySettings {
    /// 1 is pure Rayleigh air; from 4 to 7 the sky turns overcast.
    pub turbidity: f64,
    /// The single-scattered sky's brightness against the sunlit ground.
    pub radiance: f64,
    /// Multiplies the sky's environment light per linear channel: the shadow fill.
    pub fill: [f64; 3],
    /// The clear sky's saturation before tone mapping: 1 leaves it as the
    /// physical model makes it, 0 is its luminance alone.
    #[serde(default = "one")]
    pub saturation: f64,
    /// Multiplies the clear sky per linear channel before tone mapping.
    #[serde(default = "white")]
    pub tint: [f64; 3],
    /// How much of the grade the sky takes: 1 all of it, 0 none, so a warm
    /// grade can sit on the land under a sky that keeps its own colour.
    #[serde(default = "one")]
    pub grade_mix: f64,
    pub clouds: CloudSettings,
}

fn one() -> f64 {
    1.0
}

fn white() -> [f64; 3] {
    [1.0; 3]
}

/// The haze's curve over the physical extinction.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct HazeSettings {
    pub distance_scale: f64,
    pub clear_radius_m: f64,
    pub overview_from_m: f64,
    pub range_near_m: f64,
    pub range_far_m: f64,
    pub range_power: f64,
    pub range_strength: f64,
    pub sun_tint: [f64; 3],
    pub sun_strength: f64,
    pub sun_power: f64,
    pub mist_color: [f64; 3],
    pub mist_height_m: [f64; 2],
    pub mist_distance_m: [f64; 2],
    pub mist_color_strength: f64,
    pub mist_opacity: f64,
}

/// Every uniform record the light resolves to.
#[derive(Clone, Copy, Debug)]
pub struct LightRecords {
    pub sky: SkyLight,
    pub aerial: Aerial,
    pub sun: Sun,
    pub bloom: Bloom,
    pub grade: Grade,
}

impl LightRecords {
    /// The records' words in order (sky, aerial, sun, bloom, grade): what the
    /// browser's binding hands the page.
    pub fn words(&self) -> Vec<f32> {
        let mut out = Vec::new();
        out.extend_from_slice(bytemuck::cast_slice(&[self.sky]));
        out.extend_from_slice(bytemuck::cast_slice(&[self.aerial]));
        out.extend_from_slice(bytemuck::cast_slice(&[self.sun]));
        out.extend_from_slice(bytemuck::cast_slice(&[self.bloom]));
        out.extend_from_slice(bytemuck::cast_slice(&[self.grade]));
        out
    }
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloudSettings {
    pub coverage: f64,
    pub height_m: f64,
    pub scale_m: f64,
    pub softness: f64,
    pub opacity: f64,
    pub radiance: f64,
    pub shade: [f64; 3],
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GradeSettings {
    /// Whether the grade applies; off leaves only exposure and AgX.
    #[serde(default = "yes")]
    pub enabled: bool,
    pub saturation: f64,
    pub contrast: f64,
    pub split_tone: f64,
    pub shadow_lift: f64,
    pub shadow_tint: [f64; 3],
    pub highlight_tint: [f64; 3],
    pub lift: [f64; 3],
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
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
    let top = -r0 * mu + (r0 * r0 * (mu * mu - 1.0) + ATMOSPHERE_TOP_KM * ATMOSPHERE_TOP_KM).sqrt();
    let dt = top / STEPS as f64;
    let mut depth = [0.0; 3];
    for i in 0..STEPS {
        let t = (i as f64 + 0.5) * dt;
        let h = (r0 * r0 + t * t + 2.0 * r0 * t * mu).sqrt() - PLANET_RADIUS_KM;
        let rayleigh = libm::exp(-h / RAYLEIGH_SCALE_KM);
        let mie_density = libm::exp(-h / MIE_SCALE_KM);
        for (d, beta) in depth.iter_mut().zip(BETA_RAYLEIGH) {
            *d += (beta * rayleigh + mie * mie_density) * dt;
        }
    }
    let t = depth.map(|d| libm::exp(-d));
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
    /// Every record, resolved.
    pub fn records(&self) -> LightRecords {
        LightRecords {
            sky: self.sky(),
            aerial: self.aerial(),
            sun: self.sun(),
            bloom: self.bloom(),
            grade: self.grade(),
        }
    }

    /// The sun as the environment carries it: its tint is the atmosphere's
    /// transmittance toward it, greyed as the overcast diffuses it.
    pub fn sun(&self) -> Sun {
        let sun = self.sun_direction();
        let overcast = smoothstep(4.0, 7.0, self.sky.turbidity);
        let t = transmittance_to_sun(sun, self.sky.turbidity);
        let lum = crate::color::luminance(t);
        let colour = t.map(|c| c + (lum - c) * overcast);
        let [r, g, b] = colour.map(|c| c * self.sun_intensity);
        let fill = self.sky.fill;
        Sun {
            direction: f32s([sun[0], sun[1], sun[2], 0.0]),
            radiance: f32s([r, g, b, 0.0]),
            settings: [0.0, self.shadow_floor as f32, 0.0, 0.0],
            fill: f32s([fill[0], fill[1], fill[2], 0.0]),
        }
    }

    /// The haze: turbidity gives the physical extinction, the haze settings
    /// its curve.
    pub fn aerial(&self) -> Aerial {
        let h = &self.haze;
        let turbidity = self.sky.turbidity;
        let mie = mie_scale(turbidity) * BETA_MIE_EXTINCTION;
        let fog = FOG_COEFF_KM * (turbidity - FOG_TURBIDITY_ONSET).max(0.0).powi(2);
        let extinction = BETA_RAYLEIGH.map(|r| (r + mie) * h.distance_scale + fog);
        // Koschmieder's visibility: a clearer air widens the horizon's fade.
        let visibility_km = 3.912 / ((extinction[0] + extinction[1] + extinction[2]) / 3.0);
        let [near, far] = HORIZON_VISIBILITY_KM;
        let thin = ((visibility_km - near) / (far - near)).clamp(0.0, 1.0);
        Aerial {
            extinction: f32s(extinction),
            clear_radius_km: (h.clear_radius_m / 1000.0) as f32,
            sun_tint: f32s(h.sun_tint),
            sun_strength: h.sun_strength as f32,
            mist_color: f32s(h.mist_color),
            mist_color_strength: h.mist_color_strength as f32,
            overview_from_m: h.overview_from_m as f32,
            range_near_m: h.range_near_m as f32,
            range_far_m: h.range_far_m as f32,
            range_power: h.range_power as f32,
            range_strength: h.range_strength as f32,
            mist_opacity: h.mist_opacity as f32,
            sun_power: h.sun_power as f32,
            pad0: 0.0,
            mist_height_m: f32s(h.mist_height_m),
            mist_distance_m: f32s(h.mist_distance_m),
            horizon_fade: f32s([
                HORIZON_FADE_Z[0] - HORIZON_CLEAR_EXTRA_Z[0] * thin,
                HORIZON_FADE_Z[1] + HORIZON_CLEAR_EXTRA_Z[1] * thin,
            ]),
            pad1: [0.0; 2],
        }
    }

    /// Unit vector toward the sun (z up).
    pub fn sun_direction(&self) -> [f64; 3] {
        let (az, el) = (self.sun_azimuth, self.sun_elevation);
        let cos_el = libm::cos(el);
        [
            cos_el * libm::cos(az),
            cos_el * libm::sin(az),
            libm::sin(el),
        ]
    }

    /// The sky's uniform.
    pub fn sky(&self) -> SkyLight {
        let sun = self.sun_direction();
        let turbidity = self.sky.turbidity;
        let overcast = smoothstep(4.0, 7.0, turbidity);
        let transmittance = transmittance_to_sun(sun, turbidity);
        let c = &self.sky.clouds;
        let across = libm::hypot(sun[0], sun[1]);
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
            sky_saturation: self.sky.saturation as f32,
            sky_tint: f32s(self.sky.tint),
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

    /// The grade's uniform; a disabled grade leaves only exposure and AgX.
    pub fn grade(&self) -> Grade {
        let g = &self.grade;
        let on = if g.enabled { 1.0 } else { 0.0 };
        let rgb = |v: [f64; 3], w: f64| f32s([v[0], v[1], v[2], w]);
        Grade {
            saturation: (g.saturation * on) as f32,
            contrast: (g.contrast * on) as f32,
            split_tone: (g.split_tone * on) as f32,
            shadow_lift: (g.shadow_lift * on) as f32,
            exposure: self.exposure as f32,
            sky_grade_mix: self.sky.grade_mix as f32,
            pad: [0.0; 2],
            shadow_tint: rgb(g.shadow_tint, 1.0),
            highlight_tint: rgb(g.highlight_tint, 1.0),
            lift: rgb(g.lift, 0.0),
        }
    }
}
