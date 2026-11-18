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
    pub ruts: ScarRuts,
}

/// How tracks' ruts read up close: soft edges `feather_m` wide, the track
/// palette's first soil broken by patches `patch_m` across of its last by
/// `mottle`, and the tread pressed in, pads every `tread_pitch_m` across
/// the track darkening it by `tread_dark`, with `tread_grass` of the
/// field's own ground showing between them. The tread fades out before a
/// pad is a few pixels deep.
#[derive(Clone, Deserialize)]
pub struct ScarRuts {
    pub feather_m: f64,
    pub mottle: f64,
    pub patch_m: f64,
    pub tread_pitch_m: f64,
    pub tread_dark: f64,
    pub tread_grass: f64,
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
        within("scars.grass.flatten", sc.grass.flatten, 0.0, 1.0)?;
        let r = &sc.ruts;
        within("scars.ruts.feather_m", r.feather_m, 0.0, 2.0)?;
        within("scars.ruts.mottle", r.mottle, 0.0, 1.0)?;
        within("scars.ruts.patch_m", r.patch_m, 0.01, 10.0)?;
        within("scars.ruts.tread_pitch_m", r.tread_pitch_m, 0.05, 1.0)?;
        within("scars.ruts.tread_dark", r.tread_dark, 0.0, 1.0)?;
        within("scars.ruts.tread_grass", r.tread_grass, 0.0, 1.0)
    }
}
