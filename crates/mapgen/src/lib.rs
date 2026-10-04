//! Plan-only preparation; consumers receive the contract's final physical map.
use contract::identity::{GenerationIdentity, Seed};
use contract::map::{AuthoredPropDefinition, BuildingPartReference, MapDefinition, SurfaceArea};
use contract::templates::{PlacementFrame, TemplateGeometryCatalog};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod inspect;
mod joints;
pub mod layout;
pub mod open_country;
pub mod parcels;
pub mod street_props;

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
    /// The paved block interiors of the districts that pave theirs, each
    /// also laid as a `Paving` surface. Plan-only: where court amenities
    /// stand.
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

/// A district's court: its block interior, paved between its buildings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourtPlan {
    /// `settlement-3/district-2/court`, stable for a request.
    pub id: String,
    /// The district it is the interior of.
    pub district: String,
    /// A convex ring inside the district's, counter-clockwise.
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

pub use contract::generation::CompileLimits;

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

impl CompileRequest {
    /// The compiler request for a plan generated from `request`.
    pub fn generated(request: &GenerationRequest, plan: MapPlan) -> Self {
        CompileRequest {
            generator_version: request.generator_version.clone(),
            preset_revision: request.preset_revision.clone(),
            seed: request.seed,
            template_catalog_hash: request.template_catalog_hash.clone(),
            plan,
            limits: request.limits,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnsupportedPlanField,
    InvalidRequest,
    InvalidCatalogue,
    /// Missing or unsound explicit physical generation rules.
    InvalidPhysicalRules,
    MissingTemplate,
    InvalidBounds,
    InvalidPlacement,
    InvalidAuthoredIds,
    /// A river the terrain cannot carry, or a bridge whose ends no ramp
    /// reaches (`contract::river::validate`).
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
    /// Where the plan's settlements and open approaches are, for the
    /// encounter planner: the map itself carries neither.
    pub sites: contract::encounter::EncounterSites,
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
/// and buildings, then what stands in the open country between them, then
/// the street furniture that stands among the buildings, the cover that
/// certifies the open country's sight, and last the gardens.
/// `rules_json` is the explicit battle rules record. The contract extracts
/// only its catalog, forest rules and ground eye/target heights.
fn generate(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> Result<(GenerationRequest, MapPlan, TemplateGeometryCatalog, String), Vec<Diagnostic>> {
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
    let physics = contract::generation_physics::GenerationPhysics::from_rules_json(rules_json)
        .map_err(|message| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidPhysicalRules,
                feature: None,
                location: "$.rules".into(),
                message,
            }]
        })?;
    parcels::admit_region(&request, &presets)?;
    open_country::admit_coverage(&request, &presets, &physics)?;
    let layout = layout::generate_layout(&request, &presets)?;
    let plan = parcels::fill_districts(layout, &request, &catalogue, &presets)?;
    // The open country first: it settles the plan's approach corridors, which
    // street furniture keeps clear.
    let mut plan = open_country::furnish(plan, &request, &catalogue, &presets)?;
    let props =
        street_props::place_street_props(&plan, &request, &catalogue, &physics.catalog, &presets)?;
    plan.props.extend(props);
    let mut plan = open_country::cover(plan, &request, &catalogue, &presets, &physics)?;
    // Gardens last: the cover's sight certificate needs open ground in the
    // suburbs to stand copses on, and gardens give way to it.
    let gardens = street_props::place_gardens(
        &plan,
        &request,
        &catalogue,
        &physics.catalog,
        &presets,
        physics.forests.rule.trunk_clearance_m,
    )?;
    plan.props.extend(gardens);
    let hash = physics.hash().map_err(|error| {
        vec![Diagnostic {
            code: DiagnosticCode::InvalidPhysicalRules,
            feature: None,
            location: "$.rules".into(),
            message: error.to_string(),
        }]
    })?;
    Ok((request, plan, catalogue, hash))
}

/// Generate a plan from request, preset, template and resolved battle rules JSON.
pub fn generate_plan(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> GenerateOutcome {
    match generate(request_json, presets_json, descriptors_json, rules_json) {
        Ok((_, plan, _, _)) => GenerateOutcome::Ok {
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
    rules_json: &str,
) -> CompileOutcome {
    let result = generate_with_plan(request_json, presets_json, descriptors_json, rules_json)
        .map(|(_, compiled)| compiled)
        .map_err(|failure| failure.diagnostics);
    match result {
        Ok(result) => CompileOutcome::Ok {
            result: Box::new(result),
        },
        Err(diagnostics) => CompileOutcome::Error { diagnostics },
    }
}

/// Where the shared generation pipeline refused the requested seed.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStage {
    Generation,
    Compile,
}
#[derive(Debug)]
pub struct GenerationFailure {
    pub stage: GenerationStage,
    pub diagnostics: Vec<Diagnostic>,
}

/// Developer inspection and normal generation share one generation/lowering pipeline.
/// Returns the exact retained plan beside its compiled outcome, without generating twice.
pub fn generate_with_plan(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> Result<(MapPlan, GeneratedMap), GenerationFailure> {
    let (request, plan, catalogue, physical_inputs_hash) =
        generate(request_json, presets_json, descriptors_json, rules_json).map_err(
            |diagnostics| {
                // Physical certification lowers the preliminary map too. Its part/bay allowances
                // have the compiler's named locations even when that lower precedes final generation.
                let compiled = diagnostics.iter().any(|d| {
                    d.code == DiagnosticCode::ComplexityLimit
                        && matches!(
                            d.location.as_str(),
                            "$.limits.max_authored_parts" | "$.limits.max_bay_positions"
                        )
                });
                GenerationFailure {
                    stage: if compiled {
                        GenerationStage::Compile
                    } else {
                        GenerationStage::Generation
                    },
                    diagnostics,
                }
            },
        )?;
    let request = CompileRequest::generated(&request, plan);
    let mut compiled = lower(&request, &catalogue).map_err(|diagnostics| GenerationFailure {
        stage: GenerationStage::Compile,
        diagnostics,
    })?;
    // Borrow the large plan rather than materializing a second JSON tree.
    #[derive(Serialize)]
    struct GeneratedConfiguration<'a> {
        physical_inputs_hash: &'a str,
        plan: &'a MapPlan,
    }
    compiled.identity.config_hash = contract::identity::json_hash(&GeneratedConfiguration {
        physical_inputs_hash: &physical_inputs_hash,
        plan: &request.plan,
    })
    .map_err(|error| GenerationFailure {
        stage: GenerationStage::Compile,
        diagnostics: vec![Diagnostic {
            code: DiagnosticCode::InvalidPhysicalRules,
            feature: None,
            location: "$.rules".into(),
            message: error.to_string(),
        }],
    })?;
    Ok((request.plan, compiled))
}

pub fn generate_plan_json(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&generate_plan(
        request_json,
        presets_json,
        descriptors_json,
        rules_json,
    ))
}

pub fn generate_map_json(
    request_json: &str,
    presets_json: &str,
    descriptors_json: &str,
    rules_json: &str,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&generate_map(
        request_json,
        presets_json,
        descriptors_json,
        rules_json,
    ))
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
    for (index, bridge) in plan.bridges.iter().enumerate() {
        let sound = bridge
            .half_extents
            .iter()
            .chain([&bridge.thickness_m])
            .all(|v| v.is_finite() && *v > 0.0)
            && bridge.yaw.is_finite()
            && bridge.deck_z.is_finite();
        let inside =
            bridge.ends().iter().flatten().all(|p| {
                p[0] >= 0.0 && p[1] >= 0.0 && p[0] <= plan.size[0] && p[1] <= plan.size[1]
            });
        if !sound || !inside {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::InvalidBounds,
                feature: None,
                location: format!("$.plan.bridges[{index}]"),
                message: "a bridge deck is a finite positive box inside the map bounds".into(),
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
        render_margin_m: request.plan.render_margin_m,
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
    let unsound = |location: String| {
        move |message| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidRiver,
                feature: None,
                location,
                message,
            }]
        }
    };
    contract::river::validate(&map).map_err(unsound("$.plan.rivers".into()))?;
    for (index, bridge) in request.plan.bridges.iter().enumerate() {
        contract::river::validate_bridge(&map, bridge)
            .map_err(unsound(format!("$.plan.bridges[{index}]")))?;
    }
    map.bridges = request.plan.bridges.clone();
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
        sites: request.plan.sites(),
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
