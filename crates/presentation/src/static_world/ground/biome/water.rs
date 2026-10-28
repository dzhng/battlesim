//! The biome's water: the bands of the water's edge (`Shore`) and the
//! surface's looks (`Water`).
use super::{bad, within, Biome, Rgb};
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

/// The most water looks a biome holds, every kind of water together: the
/// `TerrainParams.waterLooks` array's length.
pub const WATER_LOOKS_MAX: usize = 8;

/// The water's looks, by the kind of water they dress: each river draws one
/// of `rivers`, picked by weight (`water::look`).
#[derive(Clone, Deserialize)]
pub struct Water {
    pub rivers: Vec<WaterLook>,
}

/// One look of a water surface: its body's colours (`palette`: where little
/// light survives the depth, then where most does), the share of each of
/// red, green and blue it takes per metre of water the light crosses
/// (`extinction_per_m`, as `e` in `exp(-e·d)`), its streaks' light, and how
/// often it is picked against its kind's other looks.
#[derive(Clone, Deserialize)]
pub struct WaterLook {
    pub name: String,
    pub weight: f64,
    pub palette: String,
    pub extinction_per_m: Rgb,
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
        self.check_water_looks("water.rivers", &self.water.rivers)?;
        if self.water.rivers.len() > WATER_LOOKS_MAX {
            return bad("water", format!("holds at most {WATER_LOOKS_MAX} looks"));
        }
        Ok(())
    }

    /// One kind of water's looks: each with a palette of two colours, and
    /// one with some weight to pick by.
    fn check_water_looks(&self, at: &str, looks: &[WaterLook]) -> Result<(), String> {
        for (i, l) in looks.iter().enumerate() {
            let at = format!("{at}[{i}]");
            self.has_palette(&format!("{at}.palette"), &l.palette, 2)?;
            within(&format!("{at}.weight"), l.weight, 0.0, 1e6)?;
            for (c, e) in ["r", "g", "b"].iter().zip(l.extinction_per_m) {
                within(&format!("{at}.extinction_per_m.{c}"), e, 0.01, 20.0)?;
            }
            within(&format!("{at}.streak"), l.streak, 0.0, 1.0)?;
        }
        if !looks.iter().any(|l| l.weight > 0.0) {
            return bad(at, "needs a look with some weight");
        }
        Ok(())
    }
}
