//! Authored map geometry. Metres; origin at the map's south-west corner; XY
//! ground, +Z up. Rectangles are `[min_x, min_y, width, height]`.
use serde::{Deserialize, Serialize};

pub type Rect = [f64; 4];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDefinition {
    /// Closed ground bounds `[width, height]`.
    pub size: [f64; 2],
    /// Visibility and foliage cell spacing in metres.
    pub fog_cell_m: f64,
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
    pub props: Vec<AuthoredPropDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_catalog_hash: Option<String>,
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
    /// The prop type its deck is (a `props` catalog entry).
    pub deck: String,
    pub center: [f64; 2],
    /// Half length along the bridge heading and half width across it.
    pub half_extents: [f64; 2],
    pub yaw: f64,
    pub deck_z: f64,
    pub thickness_m: f64,
}

/// Authoring input only (Q16, Q21): a forest generates its trees (the
/// fixture's `forests.tree` prop type), and at runtime it is those bodies
/// plus the ground they leave cleared. `density` names a row of the
/// fixture's `forests.densities` (spacing, jitter, concealment, attenuation,
/// canopy).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forest {
    pub rect: Rect,
    pub density: String,
    pub canopy_height_m: f64,
    pub trunk_radius_m: f64,
    pub trunk_height_m: f64,
    /// Trunks are omitted within this distance of roads and props.
    pub trunk_clearance_m: f64,
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
    pub center: [f64; 2],
    pub yaw: f64,
    /// Half extents: along heading, across heading, vertical.
    pub half_extents: [f64; 3],
    /// Base height; omitted means the ground height at `center`.
    #[serde(default)]
    pub base_z: Option<f64>,
}

/// An authored body can reserve its stable global ID; omitted IDs fill the
/// remaining dense namespace. Dynamic prop definitions contain geometry only.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthoredPropDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(flatten)]
    pub geometry: PropDefinition,
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

impl MapDefinition {
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
