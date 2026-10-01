//! Plan-only preparation; consumers receive the contract's final physical map.
use contract::identity::{GenerationIdentity, Seed};
use contract::map::{AuthoredPropDefinition, BuildingPartReference, MapDefinition, SurfaceArea};
use contract::templates::{PlacementFrame, TemplateGeometryCatalog};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    /// A requested feature without a shared physical owner cannot be discarded.
    #[serde(flatten)]
    pub unsupported_fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileLimits {
    /// Ordinary authored bodies plus materialized template parts only.
    pub max_authored_parts: u32,
    pub max_bay_positions: u64,
    /// Polygon vertices plus rounded stroke samples, over surfaces and forests.
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
    ComplexityLimit,
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
        let descriptors = serde_json::from_str(descriptors_json).map_err(|error| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidCatalogue,
                feature: None,
                location: "$.catalogue".into(),
                message: error.to_string(),
            }]
        })?;
        let catalogue = TemplateGeometryCatalog::new(descriptors).map_err(|message| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidCatalogue,
                feature: None,
                location: "$.catalogue".into(),
                message,
            }]
        })?;
        lower(&request, &catalogue)
    })();
    match result {
        Ok(result) => CompileOutcome::Ok {
            result: Box::new(result),
        },
        Err(diagnostics) => CompileOutcome::Error { diagnostics },
    }
}

pub fn compile_json(
    request_json: &str,
    descriptors_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&compile(request_json, descriptors_json))
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
        .map(|(i, area)| (format!("$.plan.surfaces[{i}]"), &area.shape))
        .chain(
            plan.forests
                .iter()
                .enumerate()
                .map(|(i, forest)| (format!("$.plan.forests[{i}]"), &forest.shape)),
        );
    for (location, shape) in shapes {
        // A stroke may overhang the edge by its width; its authored points may not.
        let outside = shape_points(shape)
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
        water: Vec::new(),
        surfaces: request.plan.surfaces.clone(),
        bridges: Vec::new(),
        forests: request.plan.forests.clone(),
        props: request.plan.props.clone(),
        buildings: Vec::new(),
        template_catalog_hash: Some(catalogue.hash().into()),
    };
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
        let feature = request
            .plan
            .buildings
            .iter()
            .find(|building| building.parts.iter().any(|part| part.prop == *id))
            .map(|building| building.id.clone());
        let location = format!("$.plan.authored_parts[{id}]");
        let bounds = prop.footprint_bounds().map_err(|message| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidBounds,
                feature: feature.clone(),
                location: location.clone(),
                message: message.into(),
            }]
        })?;
        if bounds[0] < 0.0 || bounds[1] < 0.0 || bounds[2] > map.size[0] || bounds[3] > map.size[1]
        {
            return Err(vec![Diagnostic {
                code: DiagnosticCode::InvalidBounds,
                feature,
                location,
                message: "physical body extends outside the map bounds".into(),
            }]);
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
/// rounded samples of each stroke.
fn ground_points(plan: &MapPlan) -> u64 {
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
}

fn complexity(location: &str, message: &str) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: DiagnosticCode::ComplexityLimit,
        feature: None,
        location: location.into(),
        message: message.into(),
    }]
}
