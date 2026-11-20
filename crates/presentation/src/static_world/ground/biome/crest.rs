//! The biome's dry crests: high ground facing the sun parches toward straw,
//! fading back to the field's own green lower down and in the shade.
use super::{within, Biome, Range};
use serde::Deserialize;

/// How far high, sunlit ground dries. Its weight is `strength`, times a
/// ramp over `height_m` above the map's datum, times the ground's facing to
/// the sun mixed in by `sun` (0 ignores it, 1 leaves a slope turned away
/// green). At that weight the ground's colour turns toward the palette's
/// straw at its own luminance, loses `desaturate` of its remaining colour
/// and lightens by `lift`. Flat ground below the ramp is untouched.
#[derive(Clone, Deserialize)]
pub struct CrestDry {
    pub palette: String,
    pub strength: f64,
    pub height_m: Range,
    pub sun: f64,
    pub desaturate: f64,
    pub lift: f64,
}

impl Biome {
    pub(super) fn check_crest(&self) -> Result<(), String> {
        let c = &self.crest_dry;
        self.has_palette("crest_dry.palette", &c.palette, 1)?;
        within("crest_dry.strength", c.strength, 0.0, 1.0)?;
        within("crest_dry.height_m[0]", c.height_m[0], -100.0, 1000.0)?;
        within(
            "crest_dry.height_m[1]",
            c.height_m[1],
            c.height_m[0] + 0.1,
            1000.0,
        )?;
        within("crest_dry.sun", c.sun, 0.0, 1.0)?;
        within("crest_dry.desaturate", c.desaturate, 0.0, 1.0)?;
        within("crest_dry.lift", c.lift, 0.0, 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::summer;

    #[test]
    fn a_crest_ramp_that_falls_or_a_missing_palette_is_refused() {
        let e = summer(|b| b["crest_dry"]["height_m"] = serde_json::json!([5.0, 2.0]));
        assert!(e.err().unwrap().starts_with("biome.crest_dry.height_m"));
        let e = summer(|b| b["crest_dry"]["palette"] = "nowhere".into());
        assert!(e.err().unwrap().starts_with("biome.crest_dry.palette"));
    }
}
