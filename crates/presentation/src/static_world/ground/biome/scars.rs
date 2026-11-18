//! The biome's battle scars: how craters, scorch, tracks and trampling mark
//! the ground, and how the grass answers them.
use super::{within, Biome};
use serde::Deserialize;

/// The side's learned ground on the terrain, and the grass's answer.
#[derive(Clone, Deserialize)]
pub struct Scars {
    pub crater: Crater,
    pub scorch: ScarMark,
    pub tracks: ScarMark,
    pub trampled: ScarMark,
    pub grass: ScarGrass,
}

/// How one channel marks the ground: toward a palette colour, whole at the
/// cell's byte `full`, by `strength`.
#[derive(Clone, Deserialize)]
pub struct ScarMark {
    pub palette: String,
    pub full: f64,
    pub strength: f64,
}

/// A crater: a bowl `relief_m` deep at full weight, a rim `rim` of that,
/// and its soil thrown onto the rim.
#[derive(Clone, Deserialize)]
pub struct Crater {
    #[serde(flatten)]
    pub mark: ScarMark,
    pub relief_m: f64,
    pub rim: f64,
    pub ejecta_palette: String,
    pub ejecta: f64,
}

/// The share of clumps a full crater or scorch leaves out, full tracks'
/// share of it, and how far tracks or trampling lay the grass over.
#[derive(Clone, Deserialize)]
pub struct ScarGrass {
    pub thin: f64,
    pub tracks_thin: f64,
    pub flatten: f64,
}

impl Biome {
    pub(super) fn check_scars(&self) -> Result<(), String> {
        let sc = &self.scars;
        for (key, m) in [
            ("crater", &sc.crater.mark),
            ("scorch", &sc.scorch),
            ("tracks", &sc.tracks),
            ("trampled", &sc.trampled),
        ] {
            self.has_palette(&format!("scars.{key}.palette"), &m.palette, 1)?;
            within(&format!("scars.{key}.full"), m.full, 1.0, 255.0)?;
            within(&format!("scars.{key}.strength"), m.strength, 0.0, 1.0)?;
        }
        within("scars.crater.relief_m", sc.crater.relief_m, 0.0, 5.0)?;
        within("scars.crater.rim", sc.crater.rim, 0.0, 2.0)?;
        self.has_palette("scars.crater.ejecta_palette", &sc.crater.ejecta_palette, 1)?;
        within("scars.crater.ejecta", sc.crater.ejecta, 0.0, 1.0)?;
        within("scars.grass.thin", sc.grass.thin, 0.0, 1.0)?;
        within("scars.grass.tracks_thin", sc.grass.tracks_thin, 0.0, 1.0)?;
        within("scars.grass.flatten", sc.grass.flatten, 0.0, 1.0)
    }
}
