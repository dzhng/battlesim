//! Authored map geometry. Metres; origin at the map's south-west corner; XY
//! ground, +Z up. Rectangles are `[min_x, min_y, width, height]`.
use serde::{Deserialize, Serialize};

pub type Rect = [f64; 4];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDefinition {
    /// Closed ground bounds `[width, height]`.
    pub size: [f64; 2],
    /// Height-sample spacing; triangles use the south-west → north-east diagonal.
    pub height_grid_m: f64,
    /// The one steepness limit shared by every ground unit (M03).
    pub slope_cutoff_deg: f64,
    #[serde(default)]
    pub relief: Vec<Relief>,
    #[serde(default)]
    pub water: Vec<Water>,
    #[serde(default)]
    pub roads: Vec<Road>,
    #[serde(default)]
    pub bridges: Vec<Bridge>,
    #[serde(default)]
    pub forests: Vec<Forest>,
    #[serde(default)]
    pub props: Vec<PropDefinition>,
}

/// Additive height contributions, sampled at grid vertices before triangulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Relief {
    /// `peak * (1 - (d/radius)^2)^2` inside `radius`, zero outside.
    Ridge {
        center: [f64; 2],
        peak_m: f64,
        radius_m: f64,
    },
    /// A flat-topped plateau `height_m` high over `rect`, whose sides fall at
    /// `side_degrees` to the surrounding ground.
    Mesa {
        rect: Rect,
        height_m: f64,
        side_degrees: f64,
    },
}

/// Impassable water: ground inside is lowered to `bed_z`; the surface sits at `surface_z`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Water {
    pub rect: Rect,
    pub bed_z: f64,
    pub surface_z: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Road {
    pub points: Vec<[f64; 2]>,
    pub width_m: f64,
}

/// A traversable deck: an oriented box whose top is walkable ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bridge {
    pub center: [f64; 2],
    /// Half length along the bridge heading and half width across it.
    pub half_extents: [f64; 2],
    pub yaw: f64,
    pub deck_z: f64,
    pub thickness_m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forest {
    pub rect: Rect,
    pub canopy_height_m: f64,
    /// Trunk grid spacing; trunks start `spacing/2` inside the rect's minimum corner.
    pub trunk_spacing_m: f64,
    pub trunk_radius_m: f64,
    pub trunk_height_m: f64,
    /// Trunks are omitted within this distance of roads and props.
    pub trunk_clearance_m: f64,
}

/// A prop's kind: only the key of the fixture's body table (`props.<kind>`,
/// `scenario::PropBody`). What a kind blocks, hides, stops or weighs is data,
/// never code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    Building,
    Wall,
    Crate,
    Trunk,
    BridgeDeck,
    Ruin,
    Fence,
    Sandbags,
    Trench,
    /// One anti-tank tooth: a line of them is a dragon's-teeth wall (Q18).
    Tooth,
    JeepWreck,
    SupplyWreck,
    TankWreck,
}

impl PropKind {
    pub const ALL: [PropKind; 13] = [
        PropKind::Building,
        PropKind::Wall,
        PropKind::Crate,
        PropKind::Trunk,
        PropKind::BridgeDeck,
        PropKind::Ruin,
        PropKind::Fence,
        PropKind::Sandbags,
        PropKind::Trench,
        PropKind::Tooth,
        PropKind::JeepWreck,
        PropKind::SupplyWreck,
        PropKind::TankWreck,
    ];
}

/// What moves on the ground, as far as a body's `blocks` columns care:
/// soldiers, and every vehicle whatever its push class (navigation's classes
/// are infantry plus one per push class, `scenario::PushClass`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoverClass {
    Infantry,
    Vehicle,
}

impl MoverClass {
    pub const ALL: [MoverClass; 2] = [MoverClass::Infantry, MoverClass::Vehicle];

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropDefinition {
    pub kind: PropKind,
    pub center: [f64; 2],
    pub yaw: f64,
    /// Half extents: along heading, across heading, vertical.
    pub half_extents: [f64; 3],
    /// Base height; omitted means the ground height at `center`.
    #[serde(default)]
    pub base_z: Option<f64>,
}
