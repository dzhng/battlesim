//! Authored map geometry. Metres; origin at the map's south-west corner; XY
//! ground, +Z up. Rectangles are `[min_x, min_y, width, height]`.
use crate::ground::GroundShape;
use crate::river::River;
use serde::{Deserialize, Serialize};

pub type Rect = [f64; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderError {
    pub field: &'static str,
    pub message: &'static str,
}

/// Physical validity, independent of a caller's playable-size/resource policy.
pub fn validate_header(
    size: [f64; 2],
    fog_cell_m: f64,
    height_grid_m: f64,
    slope_cutoff_deg: f64,
) -> Vec<HeaderError> {
    let mut errors = Vec::new();
    if size.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        errors.push(HeaderError {
            field: "size",
            message: "map bounds must be finite positive metres",
        });
    }
    for (field, value) in [("fog_cell_m", fog_cell_m), ("height_grid_m", height_grid_m)] {
        if !value.is_finite() || value <= 0.0 {
            errors.push(HeaderError {
                field,
                message: "map resolution must be finite positive metres",
            });
        }
    }
    if !slope_cutoff_deg.is_finite() || !(0.0..=90.0).contains(&slope_cutoff_deg) {
        errors.push(HeaderError {
            field: "slope_cutoff_deg",
            message: "ground slope cutoff must lie in 0..=90 degrees",
        });
    }
    errors
}

/// A physical map. `B` is what a building is: the resolved building every
/// consumer reads (the default), or the compact one a saved map stores
/// ([`SavedMap`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    bound(serialize = "B: Serialize", deserialize = "B: Deserialize<'de>")
)]
pub struct MapDefinition<B = BuildingDefinition> {
    /// Closed ground bounds `[width, height]`.
    #[serde(deserialize_with = "crate::numbers::array")]
    pub size: [f64; 2],
    /// Visibility and foliage cell spacing in metres.
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub fog_cell_m: f64,
    /// Height-sample spacing; triangles use the south-west → north-east diagonal.
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub height_grid_m: f64,
    /// The one steepness limit shared by every ground unit (M03).
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub slope_cutoff_deg: f64,
    #[serde(default)]
    pub relief: Vec<Relief>,
    /// Impassable water, crossed at bridges.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivers: Vec<River>,
    #[serde(default)]
    pub surfaces: Vec<SurfaceArea>,
    #[serde(default)]
    pub bridges: Vec<Bridge>,
    #[serde(default)]
    pub forests: Vec<Forest>,
    #[serde(default)]
    pub props: Vec<AuthoredPropDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<B>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_catalog_hash: Option<String>,
}

/// A map as its folder stores it (`fixtures/maps/<id>/map.json`): everything
/// a resolved map holds, with each building as its template and frame. Only
/// the resolver (`maps::resolve`) reads one.
pub type SavedMap = MapDefinition<SavedBuilding>;

impl<B> MapDefinition<B> {
    /// This map with `buildings` in place of its own, which are handed back.
    pub fn with_buildings<C>(self, buildings: Vec<C>) -> (MapDefinition<C>, Vec<B>) {
        let map = MapDefinition {
            size: self.size,
            fog_cell_m: self.fog_cell_m,
            height_grid_m: self.height_grid_m,
            slope_cutoff_deg: self.slope_cutoff_deg,
            relief: self.relief,
            rivers: self.rivers,
            surfaces: self.surfaces,
            bridges: self.bridges,
            forests: self.forests,
            props: self.props,
            buildings,
            template_catalog_hash: self.template_catalog_hash,
        };
        (map, self.buildings)
    }
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

impl Relief {
    /// This feature's height at (x, y).
    pub fn height_at(&self, x: f64, y: f64) -> f64 {
        match *self {
            Relief::Ridge {
                center,
                peak_m,
                radius_m,
            } => {
                let d = (x - center[0]).hypot(y - center[1]);
                if d >= radius_m {
                    0.0
                } else {
                    let q = 1.0 - (d / radius_m).powi(2);
                    peak_m * q * q
                }
            }
            Relief::Mesa {
                rect,
                height_m,
                side_degrees,
            } => {
                let dx = (rect[0] - x).max(x - (rect[0] + rect[2])).max(0.0);
                let dy = (rect[1] - y).max(y - (rect[1] + rect[3])).max(0.0);
                (height_m - dx.hypot(dy) * side_degrees.to_radians().tan()).max(0.0)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceArea {
    pub kind: SurfaceKind,
    pub shape: GroundShape,
}

/// What a paved or worn surface is. Its speed is a row of the rules'
/// `surfaces` table; where kinds overlap, the earlier one here wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    Road,
    CountryRoad,
    DirtTrack,
    Sidewalk,
}

impl SurfaceKind {
    pub const ALL: [SurfaceKind; 4] = [
        SurfaceKind::Road,
        SurfaceKind::CountryRoad,
        SurfaceKind::DirtTrack,
        SurfaceKind::Sidewalk,
    ];

    /// A carriageway: trunks keep clear of it and it draws as a road.
    pub fn is_road(self) -> bool {
        self != SurfaceKind::Sidewalk
    }
}

/// A traversable deck: an oriented box whose top is walkable ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bridge {
    /// The prop type its deck is (a `props` catalog entry).
    pub deck: String,
    pub center: [f64; 2],
    /// Half length along the bridge heading and half width across it.
    pub half_extents: [f64; 2],
    pub yaw: f64,
    pub deck_z: f64,
    pub thickness_m: f64,
}

impl Bridge {
    /// The deck's two end edges, where movers step on and off it.
    pub fn ends(&self) -> [[[f64; 2]; 2]; 2] {
        let (s, c) = libm::sincos(self.yaw);
        let [length, width] = self.half_extents;
        [-1.0, 1.0].map(|end| {
            [-1.0, 1.0].map(|side| {
                [
                    self.center[0] + c * length * end - s * width * side,
                    self.center[1] + s * length * end + c * width * side,
                ]
            })
        })
    }
}

/// Authoring input only (Q16, Q21): a forest is its shape. It generates its
/// trees (the fixture's `forests.tree` prop type) by the one `forests.rule`,
/// and at runtime it is those bodies plus the ground they leave cleared.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forest {
    pub shape: GroundShape,
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
    /// Its prop type: an id of the catalog's `props`.
    pub kind: String,
    #[serde(deserialize_with = "crate::numbers::array")]
    pub center: [f64; 2],
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub yaw: f64,
    /// Half extents: along heading, across heading, vertical.
    #[serde(deserialize_with = "crate::numbers::array")]
    pub half_extents: [f64; 3],
    /// Base height; omitted means the ground height at `center`.
    #[serde(default, deserialize_with = "crate::numbers::optional_scalar")]
    pub base_z: Option<f64>,
}

impl PropDefinition {
    /// World XY bounds `[min_x, min_y, max_x, max_y]` of the existing solid box.
    /// Physical preparation uses the same pinned software rotation evaluator as
    /// templates; this does not change simulation arithmetic.
    pub fn footprint_bounds(&self) -> Result<[f64; 4], &'static str> {
        let height = 2.0 * self.half_extents[2];
        if self
            .center
            .iter()
            .chain(&self.half_extents)
            .any(|v| !v.is_finite())
            || !self.yaw.is_finite()
            || !height.is_finite()
            || self.half_extents.iter().any(|v| *v <= 0.0)
            || self
                .base_z
                .is_some_and(|z| !z.is_finite() || !(z + height).is_finite())
        {
            return Err("physical box requires finite positive extents and a finite pose");
        }
        let (s, c) = libm::sincos(self.yaw);
        let reach_x = self.half_extents[0] * c.abs() + self.half_extents[1] * s.abs();
        let reach_y = self.half_extents[0] * s.abs() + self.half_extents[1] * c.abs();
        let bounds = [
            self.center[0] - reach_x,
            self.center[1] - reach_y,
            self.center[0] + reach_x,
            self.center[1] + reach_y,
        ];
        if bounds.iter().any(|v| !v.is_finite()) {
            return Err("physical box footprint cannot be represented");
        }
        Ok(bounds)
    }
}

/// An authored body can reserve its stable global ID; omitted IDs fill the
/// remaining dense namespace. Dynamic prop definitions contain geometry only.
#[derive(Clone, Debug, Serialize)]
pub struct AuthoredPropDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(flatten)]
    pub geometry: PropDefinition,
}
impl<'de> Deserialize<'de> for AuthoredPropDefinition {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Serde's flattened ContentDeserializer discards numeric tokens before
        // the physical reader sees them. Keep the one flattened wire record
        // intact; its geometry is still decoded by PropDefinition itself.
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        #[derive(Deserialize)]
        struct AuthoredId {
            #[serde(default)]
            id: Option<u32>,
        }
        let id: AuthoredId = serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        let geometry = serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        Ok(Self {
            id: id.id,
            geometry,
        })
    }
}

impl std::ops::Deref for AuthoredPropDefinition {
    type Target = PropDefinition;
    fn deref(&self) -> &Self::Target {
        &self.geometry
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingPartReference {
    pub part: String,
    pub prop: u32,
}

/// One placed physical building. Final geometry is trusted preparation output;
/// catalogue/source-fit verification belongs to that preparation boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingDefinition {
    pub owner: u32,
    pub kind: String,
    pub category: crate::templates::BuildingCategory,
    pub regional_family: String,
    pub parts: Vec<BuildingPartReference>,
    pub geometry: crate::templates::MaterializedBuilding,
}

/// One placed building as a saved map stores it: which template of the map's
/// pinned library stands at which frame, and the ids that are the building's
/// own. Its geometry, category and regional family are the template's, so
/// they are not stored: the resolver materializes them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBuilding {
    pub owner: u32,
    pub kind: String,
    pub template_id: String,
    pub frame: crate::templates::PlacementFrame,
    pub parts: Vec<BuildingPartReference>,
}

impl From<&BuildingDefinition> for SavedBuilding {
    fn from(building: &BuildingDefinition) -> Self {
        Self {
            owner: building.owner,
            kind: building.kind.clone(),
            template_id: building.geometry.template_id.clone(),
            frame: building.geometry.frame,
            parts: building.parts.clone(),
        }
    }
}

impl BuildingDefinition {
    /// Preparation materializes physical geometry once; consumers receive only
    /// this final record, never an appearance or a second template geometry.
    pub fn materialize(
        template: &crate::templates::BuildingTemplateDescriptor,
        frame: crate::templates::PlacementFrame,
        kind: String,
        owner: u32,
        parts: Vec<BuildingPartReference>,
    ) -> Result<Self, String> {
        Ok(Self {
            owner,
            kind,
            category: template.category,
            regional_family: template.regional_family.clone(),
            parts,
            geometry: template.materialize(frame)?,
        })
    }
}

/// The height of land shaped by `relief` at (x, y), before any river is
/// carved into it.
pub fn relief_height(relief: &[Relief], x: f64, y: f64) -> f64 {
    relief.iter().map(|r| r.height_at(x, y)).sum()
}

impl MapDefinition {
    /// The form this map is saved in. The resolver gives this map back from
    /// it, because a building it admits is always its template materialized
    /// at its frame.
    pub fn saved(&self) -> SavedMap {
        let buildings = self.buildings.iter().map(SavedBuilding::from).collect();
        self.clone().with_buildings(buildings).0
    }

    /// The land's height at (x, y) before any river is carved into it.
    pub fn relief_height(&self, x: f64, y: f64) -> f64 {
        relief_height(&self.relief, x, y)
    }

    /// Validate global IDs before allocating indexed storage. Sparse or repeated
    /// explicit IDs cannot request an unbounded vector or overwrite a body.
    pub fn authored_props(&self) -> Result<Vec<(u32, PropDefinition)>, String> {
        use std::collections::BTreeSet;
        let count = self
            .buildings
            .iter()
            .try_fold(self.props.len(), |n, b| n.checked_add(b.parts.len()))
            .ok_or("authored prop count overflow")?;
        let count = u32::try_from(count).map_err(|_| "authored prop count exceeds u32 IDs")?;
        let mut reserved = BTreeSet::new();
        let mut reserve = |id| {
            if id >= count || !reserved.insert(id) {
                Err("authored prop IDs must be unique and dense".to_string())
            } else {
                Ok(())
            }
        };
        for prop in &self.props {
            if let Some(id) = prop.id {
                reserve(id)?;
            }
        }
        for building in &self.buildings {
            building.geometry.validate()?;
            if building.kind.is_empty()
                || building.regional_family.is_empty()
                || building.geometry.template_id.is_empty()
                || building.parts.is_empty()
                || building.parts.len() != building.geometry.parts.len()
                || !building.parts.iter().any(|p| p.prop == building.owner)
            {
                return Err("building requires one owner and every named physical part".into());
            }
            let names: BTreeSet<_> = building.parts.iter().map(|p| p.part.as_str()).collect();
            if names.len() != building.parts.len()
                || building
                    .geometry
                    .parts
                    .iter()
                    .any(|p| !names.contains(p.id.as_str()))
            {
                return Err(
                    "building part references must uniquely cover physical geometry".into(),
                );
            }
            for part in &building.parts {
                reserve(part.prop)?;
            }
        }
        if !self.buildings.is_empty()
            && !self.template_catalog_hash.as_ref().is_some_and(|h| {
                h.len() == 64
                    && h.bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
        {
            return Err("placed buildings require a canonical physical catalogue hash".into());
        }
        let mut free = (0..count).filter(|id| !reserved.contains(id));
        let mut result = Vec::with_capacity(count as usize);
        for prop in &self.props {
            result.push((
                prop.id.unwrap_or_else(|| free.next().unwrap()),
                prop.geometry.clone(),
            ));
        }
        for building in &self.buildings {
            for reference in &building.parts {
                let part = building
                    .geometry
                    .parts
                    .iter()
                    .find(|p| p.id == reference.part)
                    .unwrap();
                result.push((
                    reference.prop,
                    PropDefinition {
                        kind: building.kind.clone(),
                        center: part.center,
                        yaw: part.yaw,
                        half_extents: part.half_extents,
                        base_z: Some(part.base_z),
                    },
                ));
            }
        }
        result.sort_by_key(|(id, _)| *id);
        Ok(result)
    }
}
