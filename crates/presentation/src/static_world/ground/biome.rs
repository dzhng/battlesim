//! The parts of the resolved biome (`terrain/biome.ts`, already in its map's
//! regional finish) the ground's shapes are cut by. The rest is the look,
//! which the page still resolves.
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Biome {
    pub seed: u32,
    pub field_rules: FieldRules,
    pub plots: Vec<PlotKind>,
    pub palettes: HashMap<String, Vec<Vec<f64>>>,
    pub roads: HashMap<String, Road>,
}

#[derive(Deserialize)]
pub struct FieldRules {
    pub extent_m: f64,
    pub size_m: [f64; 2],
    pub min_width_m: f64,
    pub strip_chance: f64,
    pub max_aspect: f64,
    pub orientation_deg: f64,
    pub orientation_jitter_deg: f64,
    pub tract_m: f64,
    pub cut_jitter_deg: f64,
    pub cut_range: [f64; 2],
    pub colour_jitter: f64,
    pub settlement_kind: String,
    pub yard_m: f64,
    pub settlement_m: f64,
    pub surround_kind: String,
}

#[derive(Deserialize)]
pub struct PlotKind {
    pub name: String,
    pub weight: f64,
    pub palette: String,
}

#[derive(Deserialize)]
pub struct Road {
    /// How far a town's kind runs on under this one's ends; read only for
    /// the kind a row draws through towns.
    #[serde(default)]
    pub join_m: f64,
    pub town: Option<Town>,
}

/// A paved kind drawn as another (`kind`) through built ground.
#[derive(Deserialize)]
pub struct Town {
    pub kind: String,
    pub beside_m: f64,
    pub gap_m: f64,
}

impl Biome {
    /// A paved kind's row, or the default's.
    pub fn road(&self, kind: &str) -> &Road {
        self.roads.get(kind).unwrap_or(&self.roads["default"])
    }
}
