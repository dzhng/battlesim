//! Admission before light reaches records or GPU resources.
use super::LightSettings;
use crate::settings::{
    read,
    validate::{numbers, range},
};
impl LightSettings {
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let light: Self = read(value, "presentation.light")?;
        light.check()?;
        Ok(light)
    }

    pub fn check(&self) -> Result<(), String> {
        numbers(self, "presentation.light", &[".sun_azimuth", ".depth_bias"])?;
        let within = |p: &str, v, lo, hi| range(&format!("presentation.light.{p}"), v, lo, hi);
        within(
            "sun_azimuth",
            self.sun_azimuth,
            -std::f64::consts::TAU,
            std::f64::consts::TAU,
        )?;
        within(
            "sun_elevation",
            self.sun_elevation,
            0.05,
            std::f64::consts::FRAC_PI_2,
        )?;
        within("sun_intensity", self.sun_intensity, 0.0, 100.0)?;
        within("shadow_floor", self.shadow_floor, 0.0, 0.5)?;
        let sky = &self.sky;
        within("sky.turbidity", sky.turbidity, 1.0, 12.0)?;
        within("sky.radiance", sky.radiance, 0.0, 1000.0)?;
        within("sky.saturation", sky.saturation, 0.0, 3.0)?;
        within("sky.grade_mix", sky.grade_mix, 0.0, 1.0)?;
        for (p, values) in [
            ("sky.fill", sky.fill),
            ("sky.tint", sky.tint),
            ("sky.clouds.shade", sky.clouds.shade),
        ] {
            for (i, v) in values.into_iter().enumerate() {
                within(&format!("{p}[{i}]"), v, 0.0, 4.0)?;
            }
        }
        let c = &sky.clouds;
        for (p, v, lo, hi) in [
            ("coverage", c.coverage, 0.0, 1.0),
            ("height_m", c.height_m, 100.0, 20000.0),
            ("scale_m", c.scale_m, 10.0, 1e5),
            ("softness", c.softness, 0.001, 0.5),
            ("opacity", c.opacity, 0.0, 1.0),
            ("radiance", c.radiance, 0.0, 100.0),
        ] {
            within(&format!("sky.clouds.{p}"), v, lo, hi)?;
        }
        within("exposure", self.exposure, 0.01, 16.0)?;
        let h = &self.haze;
        within("haze.overview_from_m", h.overview_from_m, 1.0, 1e6)?;
        within(
            "haze.range_far_m",
            h.range_far_m,
            h.range_near_m + f64::EPSILON,
            1e6,
        )?;
        for (p, v) in [("range_power", h.range_power), ("sun_power", h.sun_power)] {
            within(&format!("haze.{p}"), v, f64::MIN_POSITIVE, f64::MAX)?;
        }
        for (p, v) in [
            ("mist_opacity", h.mist_opacity),
            ("mist_color_strength", h.mist_color_strength),
        ] {
            within(&format!("haze.{p}"), v, 0.0, 1.0)?;
        }
        for (p, [lo, hi]) in [
            ("mist_height_m", h.mist_height_m),
            ("mist_distance_m", h.mist_distance_m),
        ] {
            within(&format!("haze.{p}[1]"), hi, lo + f64::EPSILON, 1e6)?;
        }
        let c = &self.cascades;
        if ![512, 1024, 2048, 4096].contains(&c.map_size) {
            return Err(
                "presentation.light.cascades.map_size: must be 512, 1024, 2048 or 4096".into(),
            );
        }
        within("cascades.max_far_m", c.max_far_m, 10.0, 20000.0)?;
        within("cascades.split_lambda", c.split_lambda, 0.0, 1.0)?;
        within("cascades.softness_m", c.softness_m, 0.0, 10.0)?;
        within("bloom.radius", self.bloom.radius, 0.0, 1.0)?;
        within("bloom.threshold", self.bloom.threshold, 0.0, 100.0)?;
        within(
            "bloom.smooth_width",
            self.bloom.smooth_width,
            f64::MIN_POSITIVE,
            100.0,
        )?;
        Ok(())
    }
}
