//! The plan a map is generated as: where its settlements, lots, courts and
//! features lie, before `lower` compiles it into the contract's physical map.
use contract::map::{AuthoredPropDefinition, BuildingPartReference, SurfaceArea};
use contract::templates::PlacementFrame;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingPlacement {
    pub id: String,
    pub template_id: String,
    pub kind: String,
    pub owner: u32,
    pub parts: Vec<BuildingPartReference>,
    pub frame: PlacementFrame,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skirmish: Option<contract::encounter::SkirmishSites>,
    #[serde(deserialize_with = "contract::numbers::array")]
    pub size: [f64; 2],
    #[serde(
        default,
        deserialize_with = "contract::map::render_margin",
        skip_serializing_if = "contract::map::no_render_margin"
    )]
    pub render_margin_m: f64,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub fog_cell_m: f64,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub height_grid_m: f64,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub slope_cutoff_deg: f64,
    #[serde(default)]
    pub props: Vec<AuthoredPropDefinition>,
    #[serde(default)]
    pub buildings: Vec<BuildingPlacement>,
    /// Roads, tracks and paving, in the contract's shared ground shapes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<SurfaceArea>,
    /// Forest shapes; the one `forests.rule` stands their trees at load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forests: Vec<contract::map::Forest>,
    /// Rivers, in the contract's shared shape: impassable water.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivers: Vec<contract::river::River>,
    /// Decks that carry movers over the rivers, in the contract's shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bridges: Vec<contract::map::Bridge>,
    /// Standing shallow water, each in the dip it carves, in the contract's
    /// shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swamps: Vec<contract::swamp::Swamp>,
    /// Where towns stand and what each district is built from, the main
    /// settlement first. Plan-only: the parcel pass turns them into
    /// `buildings`; nothing here reaches the map.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settlements: Vec<SettlementPlan>,
    /// Measured open ground beside settlements. Plan-only, like `settlements`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approaches: Vec<ApproachPlan>,
    /// The parcels the parcel pass cut, in the order it cut them. Plan-only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lots: Vec<LotPlan>,
    /// The regional family its buildings are of, which the map names
    /// (`MapDefinition.regional_family`): set by the parcel pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regional_family: Option<String>,
    /// The paved pieces of the districts that pave their blocks (yards,
    /// car parks, and the paths and lanes across their lawns), each also
    /// laid as a `Paving` surface. Plan-only: where court amenities stand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub courts: Vec<CourtPlan>,
    /// A requested feature without a shared physical owner cannot be discarded.
    #[serde(flatten)]
    pub unsupported_fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementPlan {
    pub id: String,
    /// The preset size class it was drawn from (`village`, `town`, ...).
    pub class: String,
    /// Where its main streets meet: a point of its roads, on its ground.
    #[serde(deserialize_with = "contract::numbers::array")]
    pub center: [f64; 2],
    /// The edge of its districts, and of any open ground they enclose: a
    /// simple ring, which a ray from `center` may cross more than once.
    #[serde(deserialize_with = "contract::numbers::points")]
    pub outline: Vec<[f64; 2]>,
    /// The built ground, nearest `center` first: convex blocks bounded by
    /// roads, streets and the settlement's edge, that do not overlap.
    pub districts: Vec<DistrictPlan>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistrictPlan {
    /// Stable for a request: `settlement-3/district-2`.
    pub id: String,
    /// The preset district it is (`garden_suburb`, `industrial`, ...).
    pub kind: String,
    #[serde(deserialize_with = "contract::numbers::points")]
    pub ring: Vec<[f64; 2]>,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub area_m2: f64,
    /// A point inside the ring, at its middle, to place things on.
    #[serde(deserialize_with = "contract::numbers::array")]
    pub anchor: [f64; 2],
    /// Tallest building the map type admits here; absent means no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_floors: Option<u32>,
    /// What it is built from: its one dominant building category first, then
    /// at most one minor category, with their shares of its ground.
    pub categories: Vec<CategoryShare>,
}

/// One parcel: a rectangle of a district fronting a street, cut to the
/// template that stands on it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LotPlan {
    /// `settlement-3/district-2/lot-14`, stable for a request. The building
    /// on it has the same id; a parcel without one is open ground.
    pub id: String,
    /// Its corners, counter-clockwise from the street side.
    #[serde(deserialize_with = "contract::numbers::points")]
    pub ring: Vec<[f64; 2]>,
}

/// A paved piece of a dense district's block: a built parcel's yard, a car
/// park, or a path across its lawn. The district's ground under none, off
/// its carriageways, is its lawn.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourtPlan {
    /// `<parcel id>/yard`, `<district id>/parking-<n>` or
    /// `<district id>/path-<n>`, stable for a request.
    pub id: String,
    /// The district it lies in.
    pub district: String,
    pub kind: CourtKind,
    /// Its corners, counter-clockwise: a yard's from its street side, as its
    /// parcel's; a car park's from the corner left of its mouth on the
    /// street; a path's from the corner left of the end it starts from (a
    /// street or a yard gate).
    #[serde(deserialize_with = "contract::numbers::points")]
    pub ring: Vec<[f64; 2]>,
}

/// What follows a yard's parcel id in its own.
const YARD: &str = "/yard";

impl CourtPlan {
    /// The yard of the built parcel `lot`, of `district`, whose ring it is.
    pub(crate) fn yard(lot: &str, district: &DistrictPlan, ring: Vec<[f64; 2]>) -> Self {
        Self {
            id: format!("{lot}{YARD}"),
            district: district.id.clone(),
            kind: CourtKind::Yard,
            ring,
        }
    }

    /// The parcel a yard paves, by its id; `None` for any other court.
    pub(crate) fn parcel(&self) -> Option<&str> {
        match self.kind {
            CourtKind::Yard => self.id.strip_suffix(YARD),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourtKind {
    /// A built parcel's own ground, its building standing on it.
    Yard,
    /// Bays either side of an aisle that opens on a carriageway.
    Parking,
    /// A footpath across the lawn, from a street or a yard's gate to the
    /// lawn's middle.
    Path,
    /// A path from a street that a vehicle drives: the lawn either side of
    /// it is kept open the widest hull's way.
    Lane,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryShare {
    pub category: contract::templates::BuildingCategory,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub weight: f64,
}

/// The contract's half and measured open approach: the layout writes them,
/// and the encounter planner reads them (`MapPlan::sites`).
pub use contract::encounter::{Approach as ApproachPlan, Half};

impl MapPlan {
    /// What the encounter planner reads of a plan: where its settlements
    /// and their districts stand, and the measured open approaches. The
    /// compiled map carries neither.
    pub fn sites(&self) -> contract::encounter::EncounterSites {
        use contract::encounter::{DistrictSite, EncounterSites, SettlementSite};
        EncounterSites {
            skirmish: self.skirmish.clone(),
            settlements: self
                .settlements
                .iter()
                .map(|s| SettlementSite {
                    id: s.id.clone(),
                    center: s.center,
                    outline: s.outline.clone(),
                    districts: s
                        .districts
                        .iter()
                        .map(|d| DistrictSite {
                            id: d.id.clone(),
                            ring: d.ring.clone(),
                            area_m2: d.area_m2,
                        })
                        .collect(),
                })
                .collect(),
            approaches: self.approaches.clone(),
        }
    }
}
