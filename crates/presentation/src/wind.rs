//! The one wind: the biome's `wind` block resolved into the [`Wind`] record
//! every camera carries (`Camera.wind`, the sun's cascades included), as
//! grass, water and trees move in it, and the effects read on the CPU, as
//! smoke and dust drift on it. Its clock is the camera's (`Camera.time`):
//! the record holds none.
use serde::Deserialize;

use crate::records::Wind;
use crate::static_world::ground::biome::within;

/// The biome's `wind` block (`fixtures/biomes/<name>.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct WindSettings {
    /// Where it blows toward, degrees counter-clockwise from +x.
    pub heading_deg: f64,
    /// How fast it carries smoke and dust (m/s).
    pub drift_mps: f64,
    /// The spacing of the gust fronts rolling through the grass (m), and
    /// how fast they roll downwind (m/s).
    pub gust_m: f64,
    pub gust_mps: f64,
}

impl WindSettings {
    /// Refuses a number out of its range, by its path in the biome.
    pub(crate) fn check(&self) -> Result<(), String> {
        within("wind.heading_deg", self.heading_deg, -360.0, 360.0)?;
        within("wind.drift_mps", self.drift_mps, 0.0, 30.0)?;
        within("wind.gust_m", self.gust_m, 1.0, 10000.0)?;
        within("wind.gust_mps", self.gust_mps, 0.0, 100.0)
    }

    /// The record every shader and the effects read.
    pub fn record(&self) -> Wind {
        let heading = self.heading_deg * std::f64::consts::PI / 180.0;
        let f = |v: f64| v as f32;
        Wind {
            toward: [f(libm::cos(heading)), f(libm::sin(heading)), 0.0],
            drift_mps: f(self.drift_mps),
            gust_mps: f(self.gust_mps),
            gust_per_m: f(1.0 / self.gust_m),
            pad: [0.0; 2],
        }
    }
}

impl Wind {
    /// How smoke and dust drift on it (m/s, x and y), as the shaders read
    /// its direction.
    pub fn drift(&self) -> [f64; 2] {
        let d = f64::from(self.drift_mps);
        [f64::from(self.toward[0]) * d, f64::from(self.toward[1]) * d]
    }
}
