//! The biome's water: the bands of the water's edge (`Shore`) and the
//! surface's look (`Water`).
use super::{range, within, Biome, Range};
use serde::Deserialize;

/// The water's edge along every river, in bands by distance from it: wet
/// silt for `wet_m`, then bare earth out to `mud_m`, whose outer line
/// wanders in by up to `wander` of `mud_m` over `wander_scale_m`.
#[derive(Clone, Deserialize)]
pub struct Shore {
    /// Two colours: the wet silt, the bare earth.
    pub palette: String,
    pub wet_m: f64,
    pub mud_m: f64,
    pub wander: f64,
    pub wander_scale_m: f64,
    /// Both bands are drawn at least this many times as light as the ground
    /// they cover.
    pub lift: f64,
    /// The share of a bank's true slope its shading shows.
    pub relief: f64,
}

/// The water surface's look: its opacity at its edge and out in the channel,
/// how far in it takes 63% of the channel's, and its streaks' light.
#[derive(Clone, Deserialize)]
pub struct Water {
    pub opacity: Range,
    pub shallows_m: f64,
    pub streak: f64,
}

impl Biome {
    /// Checks the shore's and the water's numbers.
    pub(super) fn check_water(&self) -> Result<(), String> {
        let s = &self.shore;
        self.has_palette("shore.palette", &s.palette, 2)?;
        within("shore.wet_m", s.wet_m, 0.0, 20.0)?;
        within("shore.wander", s.wander, 0.0, 0.9)?;
        // The earth's line never wanders in over the wet bank.
        within("shore.mud_m", s.mud_m, s.wet_m / (1.0 - s.wander), 20.0)?;
        within("shore.wander_scale_m", s.wander_scale_m, 0.5, 1000.0)?;
        within("shore.lift", s.lift, 1.0, 3.0)?;
        within("shore.relief", s.relief, 0.0, 1.0)?;
        range("water.opacity", self.water.opacity, 0.0, 1.0)?;
        within("water.shallows_m", self.water.shallows_m, 0.05, 50.0)?;
        within("water.streak", self.water.streak, 0.0, 1.0)?;
        Ok(())
    }
}
