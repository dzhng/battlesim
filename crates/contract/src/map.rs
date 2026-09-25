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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    Building,
    Wall,
    Crate,
    Trunk,
    BridgeDeck,
    Wreck,
    Ruin,
}

impl PropKind {
    /// Solid to ground movement. Trunks stop projectiles but forests stay
    /// traversable (M02); a bridge deck is walked on, not around (M09).
    pub fn blocks_movement(self) -> bool {
        !matches!(self, PropKind::Trunk | PropKind::BridgeDeck)
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
