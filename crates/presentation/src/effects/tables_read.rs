//! Strict readers for the flat puff records. Serde flatten cannot combine
//! nested flattened records with deny_unknown_fields; admit the flat shape once.
use super::tables::*;
use serde::{Deserialize, Deserializer};

impl<'de> Deserialize<'de> for TrailStyle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            albedo: [f64; 3],
            opacity: f64,
            size_m: [f64; 2],
            life_s: f64,
            rise_mps: f64,
            spread_mps: f64,
            ribbon: f64,
            spacing_m: f64,
        }
        let f = Fields::deserialize(deserializer)?;
        let puff = PuffStyle {
            albedo: f.albedo,
            opacity: f.opacity,
            size_m: f.size_m,
            life_s: f.life_s,
            rise_mps: f.rise_mps,
            spread_mps: f.spread_mps,
        };
        Ok(Self {
            puff,
            ribbon: f.ribbon,
            spacing_m: f.spacing_m,
        })
    }
}

impl<'de> Deserialize<'de> for PuffBurst {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            albedo: [f64; 3],
            opacity: f64,
            size_m: [f64; 2],
            life_s: f64,
            rise_mps: f64,
            spread_mps: f64,
            count: f64,
            over_s: f64,
        }
        let f = Fields::deserialize(deserializer)?;
        let puff = PuffStyle {
            albedo: f.albedo,
            opacity: f.opacity,
            size_m: f.size_m,
            life_s: f.life_s,
            rise_mps: f.rise_mps,
            spread_mps: f.spread_mps,
        };
        Ok(Self {
            puff,
            count: f.count,
            over_s: f.over_s,
        })
    }
}

impl<'de> Deserialize<'de> for DustStyle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            albedo: [f64; 3],
            opacity: f64,
            size_m: [f64; 2],
            life_s: f64,
            rise_mps: f64,
            spread_mps: f64,
            spacing_m: f64,
            min_speed_mps: f64,
            full_speed_mps: f64,
        }
        let f = Fields::deserialize(deserializer)?;
        let puff = PuffStyle {
            albedo: f.albedo,
            opacity: f.opacity,
            size_m: f.size_m,
            life_s: f.life_s,
            rise_mps: f.rise_mps,
            spread_mps: f.spread_mps,
        };
        Ok(Self {
            puff,
            spacing_m: f.spacing_m,
            min_speed_mps: f.min_speed_mps,
            full_speed_mps: f.full_speed_mps,
        })
    }
}

impl<'de> Deserialize<'de> for DamageSmokeStyle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            albedo: [f64; 3],
            opacity: f64,
            size_m: [f64; 2],
            life_s: f64,
            rise_mps: f64,
            spread_mps: f64,
            rate_hz: f64,
        }
        let f = Fields::deserialize(deserializer)?;
        let puff = PuffStyle {
            albedo: f.albedo,
            opacity: f.opacity,
            size_m: f.size_m,
            life_s: f.life_s,
            rise_mps: f.rise_mps,
            spread_mps: f.spread_mps,
        };
        Ok(Self {
            puff,
            rate_hz: f.rate_hz,
        })
    }
}

impl<'de> Deserialize<'de> for GroundDust {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            albedo: [f64; 3],
            opacity: f64,
            size_m: [f64; 2],
            life_s: f64,
            rise_mps: f64,
            spread_mps: f64,
            count: f64,
            over_s: f64,
            reach_m: f64,
        }
        let f = Fields::deserialize(deserializer)?;
        let puff = PuffStyle {
            albedo: f.albedo,
            opacity: f.opacity,
            size_m: f.size_m,
            life_s: f.life_s,
            rise_mps: f.rise_mps,
            spread_mps: f.spread_mps,
        };
        Ok(Self {
            burst: PuffBurst {
                puff,
                count: f.count,
                over_s: f.over_s,
            },
            reach_m: f.reach_m,
        })
    }
}
