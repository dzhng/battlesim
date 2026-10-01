//! Plan-only preparation; consumers receive the contract's final physical map.
use contract::identity::{GenerationIdentity, Seed};
use contract::map::{AuthoredPropDefinition, BuildingPartReference, MapDefinition, SurfaceArea};
use contract::templates::{PlacementFrame, TemplateGeometryCatalog};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod inspect;
pub mod layout;
pub mod parcels;

use layout::{GenerationRequest, PresetDefinitions};

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
    #[serde(deserialize_with = "contract::numbers::array")]
    pub size: [f64; 2],
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
    /// Roads, tracks and sidewalks, in the contract's shared ground shapes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<SurfaceArea>,
    /// Forest shapes; the one `forests.rule` stands their trees at load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forests: Vec<contract::map::Forest>,
    /// Rivers, in the contract's shared shape: impassable water.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivers: Vec<contract::river::River>,
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
    #[serde(deserialize_with = "contract::numbers::array")]
    pub center: [f64; 2],
    /// Its envelope: a simple ring, star-shaped about `center`.
    #[serde(deserialize_with = "contract::numbers::points")]
    pub outline: Vec<[f64; 2]>,
    /// The built ground: simple rings inside the outline that do not overlap.
    /// What the outline holds beyond them is field or wood.
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
    /// A point inside the ring, near its middle, to place things on. (A ring
    /// sector's centroid can fall outside it.)
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryShare {
    pub category: contract::templates::BuildingCategory,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub weight: f64,
}

/// The half of the playable area north (`Top`) or south of its midline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Half {
    Top,
    Bottom,
}

/// A wedge of ground with no settlement and no forest: every bearing from
/// `from_rad` to `to_rad` (counter-clockwise from +X, about the settlement's
/// centre) is open for `depth_m` beyond the settlement's edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproachPlan {
    /// Index into `settlements`.
    pub settlement: usize,
    pub half: Half,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub from_rad: f64,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub to_rad: f64,
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub depth_m: f64,
    /// Width across the wedge half-way out.
    #[serde(deserialize_with = "contract::numbers::scalar")]
    pub front_m: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileLimits {
    /// Ordinary authored bodies plus materialized template parts only.
    pub max_authored_parts: u32,
    pub max_bay_positions: u64,
    /// Polygon vertices plus rounded stroke samples, over surfaces, forests
    /// and rivers.
    pub max_ground_points: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileRequest {
    pub generator_version: String,
    pub preset_revision: String,
    pub seed: Seed,
    pub template_catalog_hash: String,
    pub plan: MapPlan,
    pub limits: CompileLimits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnsupportedPlanField,
    InvalidRequest,
    InvalidCatalogue,
    MissingTemplate,
    InvalidBounds,
    InvalidPlacement,
    InvalidAuthoredIds,
    /// A river the terrain cannot carry (`contract::river::validate`).
    InvalidRiver,
    ComplexityLimit,
    InvalidPresets,
    /// A bounded search found no layout; the diagnostic names what ran out.
    GenerationFailed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub feature: Option<String>,
    pub location: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct CompileReport {
    pub authored_parts: u32,
    pub bay_positions: u64,
    pub ground_points: u64,
    pub limits: CompileLimits,
}
#[derive(Clone, Debug, Serialize)]
pub struct GeneratedMap {
    pub map: MapDefinition,
    pub identity: GenerationIdentity,
    pub report: CompileReport,
}

/// The native and WASM preparation boundaries emit this same result record.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompileOutcome {
    Ok { result: Box<GeneratedMap> },
    Error { diagnostics: Vec<Diagnostic> },
}

pub fn compile(request_json: &str, descriptors_json: &str) -> CompileOutcome {
    let result = (|| {
        let request: CompileRequest = serde_json::from_str(request_json).map_err(|error| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidRequest,
                feature: None,
                location: "$".into(),
                message: error.to_string(),
            }]
        })?;
        lower(&request, &catalogue(descriptors_json)?)
    })();
    match result {
        Ok(result) => CompileOutcome::Ok {
            result: Box::new(result),
        },
        Err(diagnostics) => CompileOutcome::Error { diagnostics },
    }
}

/// The physical catalogue a list of template descriptors makes.
fn catalogue(descriptors_json: &str) -> Result<TemplateGeometryCatalog, Vec<Diagnostic>> {
    let invalid = |message: String| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidCatalogue,
            feature: None,
            location: "$.catalogue".into(),
            message,
        }]
    };
    let descriptors =
        serde_json::from_str(descriptors_json).map_err(|error| invalid(error.to_string()))?;
    TemplateGeometryCatalog::new(descriptors).map_err(invalid)
}

pub fn compile_json(
    request_json: &str,
    descriptors_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&compile(request_json, descriptors_json))
}

/// The CLI and Wasm boundaries emit this same record.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GenerateOutcome {
    Ok { plan: Box<MapPlan> },
    Error { diagnostics: Vec<Diagnostic> },
}

/// A request's whole plan: the layout, then its districts' streets, parcels
/// and buildings.
fn generate(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> Result<(GenerationRequest, MapPlan, TemplateGeometryCatalog), Vec<Diagnostic>> {
    let request: GenerationRequest = serde_json::from_str(request_json).map_err(|error| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidRequest,
            feature: None,
            location: "$".into(),
            message: error.to_string(),
        }]
    })?;
    let presets = PresetDefinitions::from_json(presets_json)?;
    let catalogue = catalogue(descriptors_json)?;
    let layout = layout::generate_layout(&request, &presets)?;
    let plan = parcels::fill_districts(layout, &request, &catalogue, &presets)?;
    Ok((request, plan, catalogue))
}

/// Generate a plan from request, preset and template JSON.
pub fn generate_plan(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> GenerateOutcome {
    match generate(request_json, presets_json, descriptors_json) {
        Ok((_, plan, _)) => GenerateOutcome::Ok {
            plan: Box::new(plan),
        },
        Err(diagnostics) => GenerateOutcome::Error { diagnostics },
    }
}

/// Generate a plan and compile it, through the same `lower` an authored plan uses.
pub fn generate_map(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> CompileOutcome {
    let result = generate(request_json, presets_json, descriptors_json)
        .and_then(|(request, plan, catalogue)| lower(&request.compile_request(plan), &catalogue));
    match result {
        Ok(result) => CompileOutcome::Ok {
            result: Box::new(result),
        },
        Err(diagnostics) => CompileOutcome::Error { diagnostics },
    }
}

pub fn generate_plan_json(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&generate_plan(request_json, presets_json, descriptors_json))
}

pub fn generate_map_json(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&generate_map(request_json, presets_json, descriptors_json))
}

pub fn validate_plan(plan: &MapPlan) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics: Vec<_> = plan
        .unsupported_fields
        .keys()
        .map(|field| Diagnostic {
            code: DiagnosticCode::UnsupportedPlanField,
            feature: None,
            location: format!("$.plan.{field}"),
            message: format!("{field} has no admitted compiler geometry owner in this pass"),
        })
        .collect();
    let shapes = plan
        .surfaces
        .iter()
        .enumerate()
        .map(|(i, area)| {
            (
                format!("$.plan.surfaces[{i}]"),
                shape_points(&area.shape).to_vec(),
            )
        })
        .chain(plan.forests.iter().enumerate().map(|(i, forest)| {
            (
                format!("$.plan.forests[{i}]"),
                shape_points(&forest.shape).to_vec(),
            )
        }))
        .chain(plan.rivers.iter().enumerate().map(|(i, river)| {
            (
                format!("$.plan.rivers[{i}]"),
                river.points().iter().map(|p| p.xy).collect(),
            )
        }));
    for (location, points) in shapes {
        // A stroke or a river may overhang the edge by its width; its
        // authored points may not.
        let outside = points
            .iter()
            .any(|p| p[0] < 0.0 || p[1] < 0.0 || p[0] > plan.size[0] || p[1] > plan.size[1]);
        if outside {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::InvalidBounds,
                feature: None,
                location,
                message: "ground shape has an authored point outside the map bounds".into(),
            });
        }
    }
    let mut feature_ids = BTreeSet::new();
    for (index, placement) in plan.buildings.iter().enumerate() {
        if placement.id.is_empty() || !feature_ids.insert(&placement.id) {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::InvalidRequest,
                feature: Some(placement.id.clone()),
                location: format!("$.plan.buildings[{index}].id"),
                message: "building feature IDs must be nonempty and unique".into(),
            });
        }
    }
    let header_errors = contract::map::validate_header(
        plan.size,
        plan.fog_cell_m,
        plan.height_grid_m,
        plan.slope_cutoff_deg,
    );
    let header_diagnostic = |error: &contract::map::HeaderError| Diagnostic {
        code: DiagnosticCode::InvalidBounds,
        feature: None,
        location: format!("$.plan.{}", error.field),
        message: error.message.into(),
    };
    diagnostics.extend(
        header_errors
            .iter()
            .filter(|e| e.field == "size")
            .map(header_diagnostic),
    );
    if plan.size.iter().any(|size| *size > 20_000.0) {
        diagnostics.push(Diagnostic {
            code: DiagnosticCode::InvalidBounds,
            feature: None,
            location: "$.plan.size".into(),
            message: "playable map bounds exceed the 20000 metre architecture envelope".into(),
        });
    }
    diagnostics.extend(
        header_errors
            .iter()
            .filter(|e| e.field != "size")
            .map(header_diagnostic),
    );
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

pub fn lower(
    request: &CompileRequest,
    catalogue: &TemplateGeometryCatalog,
) -> Result<GeneratedMap, Vec<Diagnostic>> {
    let diagnostics: Vec<_> = [
        ("generator_version", &request.generator_version),
        ("preset_revision", &request.preset_revision),
    ]
    .into_iter()
    .filter(|(_, value)| contract::identity::validate_version_identifier(value).is_err())
    .map(|(field, _)| Diagnostic {
        code: DiagnosticCode::InvalidRequest,
        feature: None,
        location: format!("$.{field}"),
        message: "generation version identifiers must be nonempty".into(),
    })
    .collect();
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    if request.template_catalog_hash != catalogue.hash() {
        return Err(vec![Diagnostic {
            code: DiagnosticCode::InvalidCatalogue,
            feature: None,
            location: "$.template_catalog_hash".into(),
            message: "requested physical catalogue hash differs from the supplied catalogue".into(),
        }]);
    }
    validate_plan(&request.plan)?;
    let mut authored_parts = request.plan.props.len() as u64;
    let mut bay_positions = 0u64;
    let mut descriptors = Vec::new();
    for placement in &request.plan.buildings {
        let descriptor = catalogue
            .templates()
            .iter()
            .find(|template| template.id == placement.template_id)
            .ok_or_else(|| {
                vec![Diagnostic {
                    code: DiagnosticCode::MissingTemplate,
                    feature: Some(placement.id.clone()),
                    location: "template_id".into(),
                    message: format!("unknown physical template {}", placement.template_id),
                }]
            })?;
        authored_parts = authored_parts
            .checked_add(descriptor.parts.len() as u64)
            .ok_or_else(|| {
                complexity(
                    "$.limits.max_authored_parts",
                    "authored part count overflow",
                )
            })?;
        bay_positions = bay_positions
            .checked_add(descriptor.bay_position_count().map_err(|message| {
                vec![Diagnostic {
                    code: DiagnosticCode::InvalidCatalogue,
                    feature: Some(placement.id.clone()),
                    location: "template_id".into(),
                    message,
                }]
            })? as u64)
            .ok_or_else(|| {
                complexity("$.limits.max_bay_positions", "bay position count overflow")
            })?;
        if authored_parts > u64::from(request.limits.max_authored_parts) {
            return Err(complexity(
                "$.limits.max_authored_parts",
                "compiled authored parts exceed admission",
            ));
        }
        if bay_positions > request.limits.max_bay_positions {
            return Err(complexity(
                "$.limits.max_bay_positions",
                "compiled bay positions exceed admission",
            ));
        }
        descriptors.push(descriptor);
    }
    if authored_parts > u64::from(request.limits.max_authored_parts) {
        return Err(complexity(
            "$.limits.max_authored_parts",
            "compiled authored parts exceed admission",
        ));
    }
    let ground_points = ground_points(&request.plan);
    if ground_points > request.limits.max_ground_points {
        return Err(complexity(
            "$.limits.max_ground_points",
            "compiled ground points exceed admission",
        ));
    }
    let mut map = MapDefinition {
        size: request.plan.size,
        fog_cell_m: request.plan.fog_cell_m,
        height_grid_m: request.plan.height_grid_m,
        slope_cutoff_deg: request.plan.slope_cutoff_deg,
        relief: Vec::new(),
        rivers: request.plan.rivers.clone(),
        surfaces: request.plan.surfaces.clone(),
        bridges: Vec::new(),
        forests: request.plan.forests.clone(),
        props: request.plan.props.clone(),
        buildings: Vec::new(),
        template_catalog_hash: Some(catalogue.hash().into()),
    };
    contract::river::validate(&map).map_err(|message| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidRiver,
            feature: None,
            location: "$.plan.rivers".into(),
            message,
        }]
    })?;
    for (placement, descriptor) in request.plan.buildings.iter().zip(descriptors) {
        let building = contract::map::BuildingDefinition::materialize(
            descriptor,
            placement.frame,
            placement.kind.clone(),
            placement.owner,
            placement.parts.clone(),
        )
        .map_err(|message| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidPlacement,
                feature: Some(placement.id.clone()),
                location: "frame".into(),
                message,
            }]
        })?;
        map.buildings.push(building);
    }
    let authored = map.authored_props().map_err(|message| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidAuthoredIds,
            feature: None,
            location: "$.plan".into(),
            message,
        }]
    })?;
    for (id, prop) in &authored {
        // Named only on refusal: a city has tens of thousands of parts.
        let refuse = |message: &str| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidBounds,
                feature: request
                    .plan
                    .buildings
                    .iter()
                    .find(|building| building.parts.iter().any(|part| part.prop == *id))
                    .map(|building| building.id.clone()),
                location: format!("$.plan.authored_parts[{id}]"),
                message: message.into(),
            }]
        };
        let bounds = prop.footprint_bounds().map_err(refuse)?;
        if bounds[0] < 0.0 || bounds[1] < 0.0 || bounds[2] > map.size[0] || bounds[3] > map.size[1]
        {
            return Err(refuse("physical body extends outside the map bounds"));
        }
    }
    let identity = GenerationIdentity {
        generator_version: request.generator_version.clone(),
        preset_revision: request.preset_revision.clone(),
        seed: request.seed,
        config_hash: contract::identity::json_hash(&request.plan).map_err(|error| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidRequest,
                feature: None,
                location: "$.plan".into(),
                message: error.to_string(),
            }]
        })?,
        template_catalog_hash: catalogue.hash().into(),
        map_hash: contract::identity::json_hash(&map).map_err(|error| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidRequest,
                feature: None,
                location: "$.plan".into(),
                message: error.to_string(),
            }]
        })?,
    };
    Ok(GeneratedMap {
        map,
        identity,
        report: CompileReport {
            authored_parts: authored_parts as u32,
            bay_positions,
            ground_points,
            limits: request.limits,
        },
    })
}

/// A shape's authored points: a polygon's ring or a stroke's controls.
fn shape_points(shape: &contract::ground::GroundShape) -> &[[f64; 2]] {
    match shape {
        contract::ground::GroundShape::Polygon { ring } => ring,
        contract::ground::GroundShape::Stroke { centerline, .. } => centerline.control_points(),
    }
}

/// What the plan's ground costs every consumer: polygon vertices plus the
/// rounded samples of each stroke and river.
pub fn ground_points(plan: &MapPlan) -> u64 {
    let count = |shape: &contract::ground::GroundShape| match shape {
        contract::ground::GroundShape::Polygon { ring } => ring.len() as u64,
        contract::ground::GroundShape::Stroke { centerline, .. } => {
            centerline.samples().len() as u64
        }
    };
    plan.surfaces
        .iter()
        .map(|area| count(&area.shape))
        .sum::<u64>()
        + plan
            .forests
            .iter()
            .map(|forest| count(&forest.shape))
            .sum::<u64>()
        + plan
            .rivers
            .iter()
            .map(|river| river.samples().len() as u64)
            .sum::<u64>()
}

fn complexity(location: &str, message: &str) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: DiagnosticCode::ComplexityLimit,
        feature: None,
        location: location.into(),
        message: message.into(),
    }]
}
