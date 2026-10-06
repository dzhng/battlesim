//! Captured-input developer boundary. No operation reads the repository's current fixtures.
use contract::encounter::EncounterRecipes;
use contract::generation::{CompileLimits, GenerationRequest, MapSize, MapType};
use contract::identity::Seed;
use contract::scenario::Rules;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::PresetDefinitions;
use mapgen::{Diagnostic, DiagnosticCode, MapPlan};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[path = "workbench_fields.rs"]
mod fields;
use sim::map_analysis::{self as sight, AnalysisPolicy};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    presets: String,
    defaults: String,
    templates: String,
    rules: String,
    catalog: String,
    recipes: String,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    #[serde(rename = "type")]
    map_type: MapType,
    size: MapSize,
    #[serde(default)]
    profile: contract::generation::GenerationProfile,
    seed: Seed,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Validate {
        inputs: Inputs,
    },
    Generate {
        inputs: Inputs,
        choice: Choice,
        #[serde(rename = "artifactDir")]
        artifact_dir: PathBuf,
    },
    Inspect {
        #[serde(rename = "artifactDir")]
        artifact_dir: PathBuf,
        crop: Option<String>,
    },
    Sight {
        #[serde(rename = "artifactDir")]
        artifact_dir: PathBuf,
    },
}

struct Checked {
    presets: PresetDefinitions,
    catalogue: TemplateGeometryCatalog,
    rules: Rules,
    resolved_rules: String,
    recipes: EncounterRecipes,
    defaults: Value,
    limits: CompileLimits,
    analysis: AnalysisPolicy,
}
fn diagnostic(location: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::InvalidRequest,
        feature: None,
        location: location.into(),
        message: message.into(),
    }
}
fn parse<T: serde::de::DeserializeOwned>(text: &str, location: &str) -> Result<T, Vec<Diagnostic>> {
    serde_json::from_str(text).map_err(|error| vec![diagnostic(location, error.to_string())])
}
fn checked(inputs: &Inputs) -> Result<Checked, Vec<Diagnostic>> {
    let defaults: Value = parse(&inputs.defaults, "$.defaults")?;
    let mut errors = Vec::new();
    // Report policy has its own owner and never changes generation admission.
    let analysis: AnalysisPolicy = serde_json::from_value(defaults["analysis"].clone())
        .map_err(|error| vec![diagnostic("$.defaults.analysis", error.to_string())])?;
    errors.extend(
        analysis
            .errors()
            .into_iter()
            .map(|(field, message)| diagnostic(format!("$.defaults.analysis.{field}"), message)),
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    let limits: CompileLimits = serde_json::from_value(defaults["limits"].clone())
        .map_err(|error| vec![diagnostic("$.defaults.limits", error.to_string())])?;
    let catalogue_limits = contract::maps::MapAdmission::CATALOGUE;
    for (field, valid) in [
        (
            "max_authored_parts",
            limits.max_authored_parts > 0
                && limits.max_authored_parts <= catalogue_limits.max_authored_parts,
        ),
        (
            "max_bay_positions",
            limits.max_bay_positions > 0
                && limits.max_bay_positions <= catalogue_limits.max_bay_positions,
        ),
        ("max_ground_points", limits.max_ground_points > 0),
    ] {
        if !valid {
            errors.push(diagnostic(
                format!("$.defaults.limits.{field}"),
                "Allowance must be positive and fit the released work envelope",
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let presets = PresetDefinitions::from_json(&inputs.presets)?;
    let descriptors = parse(&inputs.templates, "$.templates")?;
    let catalogue = TemplateGeometryCatalog::new(descriptors)
        .map_err(|error| vec![diagnostic("$.templates", error)])?;
    let mut raw_rules: Value = parse(&inputs.rules, "$.rules")?;
    let catalog: Value = parse(&inputs.catalog, "$.catalog")?;
    raw_rules["catalog"] = catalog
        .get("documents")
        .ok_or_else(|| {
            vec![diagnostic(
                "$.catalog.documents",
                "Captured catalog has no documents",
            )]
        })?
        .clone();
    let resolved_rules = serde_json::to_string(&raw_rules)
        .map_err(|error| vec![diagnostic("$.rules", error.to_string())])?;
    let rules: Rules = parse(&resolved_rules, "$.rules")?;
    let recipes = EncounterRecipes::from_json(&inputs.recipes)
        .map_err(|error| vec![diagnostic("$.recipes", error.to_string())])?;
    let recipe = defaults["encounter"]["recipe"].as_str().ok_or_else(|| {
        vec![diagnostic(
            "$.defaults.encounter.recipe",
            "Encounter recipe must be text",
        )]
    })?;
    if !recipes.recipes.contains_key(recipe) {
        return Err(vec![diagnostic(
            "$.defaults.encounter.recipe",
            "Default recipe is missing",
        )]);
    }
    let _: Seed = serde_json::from_value(defaults["encounter"]["seed"].clone())
        .map_err(|error| vec![diagnostic("$.defaults.encounter.seed", error.to_string())])?;
    Ok(Checked {
        presets,
        catalogue,
        rules,
        resolved_rules,
        recipes,
        defaults,
        limits,
        analysis,
    })
}
#[derive(Serialize, Deserialize)]
struct Artifact {
    inputs: Inputs,
    choice: Choice,
    plan: MapPlan,
    map: contract::map::MapDefinition,
    sites: contract::encounter::EncounterSites,
}
fn inspection(artifact: &Artifact, checked: &Checked, crop: Option<&str>) -> Result<Value, String> {
    let metrics = mapgen::layout::measure(&artifact.plan, &checked.presets);
    let picture = mapgen::inspect::picture(
        &artifact.plan,
        &checked.catalogue,
        &format!(
            "{} {} seed {}",
            artifact.choice.map_type.name(),
            artifact.choice.size.name(),
            artifact.choice.seed.value()
        ),
        &metrics,
        crop,
    )?;
    let mut features = vec![
        json!({"id":"approach","group":"approach","label":"Open approach policy"}),
        json!({"id":"parcels","group":"parcels","label":"Parcel placement"}),
        json!({"id":"roads","group":"roads","label":"Road network"}),
        json!({"id":"forests","group":"forests","label":"Forest placement"}),
        json!({"id":"rivers","group":"rivers","label":"River placement"}),
        json!({"id":"open_country","group":"open_country","label":"Country furnishing"}),
        json!({"id":"street_props","group":"street_props","label":"Street furniture"}),
    ];
    for (index, _) in artifact.plan.bridges.iter().enumerate() {
        features.push(json!({"id":format!("bridge-{index}"),"group":"crossings","label":"Bridge crossing rules","crop":format!("bridge-{index}")}));
    }
    for settlement in &artifact.plan.settlements {
        features.push(json!({"id":settlement.id,"group":format!("classes.{}",settlement.class),"label":format!("{} settlement rules",settlement.class),"crop":settlement.id}));
        for district in &settlement.districts {
            features.push(json!({"id":district.id,"group":format!("districts.{}",district.kind),"label":format!("{} district rules",district.kind),"crop":district.id}));
        }
    }
    Ok(
        json!({"svg":picture.svg,"features":features,"summary":picture.summary,"legend":picture.legend}),
    )
}
fn generated(inputs: Inputs, choice: Choice, directory: PathBuf) -> Result<Value, String> {
    let started = Instant::now();
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("request.json"),serde_json::to_vec(&json!({"inputs":inputs,"choice":choice,"receipts":{"presets":contract::identity::bytes_hash(inputs.presets.as_bytes()),"defaults":contract::identity::bytes_hash(inputs.defaults.as_bytes()),"rules":contract::identity::bytes_hash(inputs.rules.as_bytes()),"catalog":contract::identity::bytes_hash(inputs.catalog.as_bytes()),"templates":contract::identity::bytes_hash(inputs.templates.as_bytes()),"recipes":contract::identity::bytes_hash(inputs.recipes.as_bytes())}})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let finish = |report: Value| -> Result<Value, String> {
        std::fs::write(
            directory.join("report.json"),
            serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(report)
    };
    let c = match checked(&inputs) {
        Ok(c) => c,
        Err(diagnostics) => {
            return finish(
                json!({"status":"refused","choice":choice,"stage":"input","diagnostics":diagnostics}),
            )
        }
    };
    let request = GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: c.presets.revision.clone(),
        seed: choice.seed,
        template_catalog_hash: c.catalogue.hash().into(),
        map_type: choice.map_type,
        size: choice.size,
        profile: choice.profile,
        region: None,
        limits: c.limits,
    };
    let (plan, result) = match mapgen::generate_with_plan(
        &serde_json::to_string(&request).map_err(|e| e.to_string())?,
        &inputs.presets,
        &inputs.templates,
        &c.resolved_rules,
    ) {
        Ok(value) => value,
        Err(failure) => {
            return finish(
                json!({"status":"refused","choice":choice,"stage":failure.stage,"diagnostics":failure.diagnostics}),
            )
        }
    };
    let generation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let metrics = json!({"layout":mapgen::layout::measure(&plan,&c.presets),"country":mapgen::open_country::measure(&plan,&c.presets),"compile":result.report});
    let counts = json!({"buildings":result.map.buildings.len(),"parts":result.report.authored_parts,"bay_positions":result.report.bay_positions,"ground_points":result.report.ground_points,"props":result.map.props.len(),"surfaces":result.map.surfaces.len(),"forests":result.map.forests.len()});
    let encounter_started = Instant::now();
    let world = sim::encounter::PreparedMap::new(&result.map, &c.rules);
    let recipe = c.defaults["encounter"]["recipe"]
        .as_str()
        .ok_or("Default recipe is missing")?;
    let encounter_seed = serde_json::from_value(c.defaults["encounter"]["seed"].clone())
        .map_err(|e| e.to_string())?;
    let encounter = match sim::encounter::plan_encounter(
        &world.queries(&result.map, &result.sites),
        &c.rules,
        &c.recipes.recipes[recipe],
        encounter_seed,
    ) {
        Ok(encounter) => json!({"status":"ok","placement":encounter.placement}),
        Err(diagnostics) => json!({"status":"refused","diagnostics":diagnostics}),
    };
    let encounter_ms = encounter_started.elapsed().as_secs_f64() * 1000.0;
    let artifact = Artifact {
        inputs,
        choice: choice.clone(),
        plan,
        map: result.map,
        sites: result.sites,
    };
    let picture = inspection(&artifact, &c, None)?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("artifact.json"),
        serde_json::to_vec(&artifact).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    finish(
        json!({"status":"ok","choice":choice,"diagnostics":[],"svg":picture["svg"],"features":picture["features"],"summary":picture["summary"],"legend":picture["legend"],"identity":result.identity,"metrics":metrics,"counts":counts,"encounter":encounter,"timings":{"generation_ms":generation_ms,"encounter_ms":encounter_ms,"total_ms":started.elapsed().as_secs_f64()*1000.0}}),
    )
}
fn artifact(directory: &Path) -> Result<Artifact, String> {
    serde_json::from_slice(
        &std::fs::read(directory.join("artifact.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn measured(a: &Artifact, c: &Checked) -> Value {
    let started = Instant::now();
    // The caller's spacing stays exact. Refuse an unsupported analysis workload instead of silently thinning it.
    let grid_points =
        (a.map.size[0] / c.analysis.step_m).ceil() * (a.map.size[1] / c.analysis.step_m).ceil();
    if !grid_points.is_finite() || grid_points > 1_000_000.0 {
        return json!({"status":"unavailable","samples":0,"target":c.analysis.min_median_open,"step_m":c.analysis.step_m,"bearings":0,"elapsed_ms":started.elapsed().as_secs_f64()*1000.0,"reason":"Requested sample grid exceeds the one-million-position analysis envelope"});
    }
    let world = sim::world::WorldGeometry::new(&a.map, &c.rules);
    let eye = sight::infantry_sight(&c.rules);
    let bearings = sight::bearings(a.map.fog_cell_m, &eye);
    let mut shares: Vec<_> = sight::sample_points(&world, &a.sites, c.analysis.step_m)
        .into_iter()
        .filter_map(|p| sight::open_share(&world, &c.rules, &eye, p, bearings))
        .collect();
    shares.sort_by(f64::total_cmp);
    let mut result = json!({"status":"unavailable","samples":shares.len(),"target":c.analysis.min_median_open,"step_m":c.analysis.step_m,"bearings":bearings,"elapsed_ms":started.elapsed().as_secs_f64()*1000.0});
    if shares.is_empty() {
        result["reason"] = json!("No eligible open-ground sample points");
    } else {
        result["status"] = json!("measured");
        result["median"] = json!(sight::quantile(&shares, 0.5));
        result["belowTarget"] = json!(shares
            .iter()
            .filter(|s| **s < c.analysis.min_median_open)
            .count());
    }
    result
}
pub fn run(request: Request) -> Result<Value, String> {
    match request {
        Request::Validate { inputs } => {
            let diagnostics = checked(&inputs).err().unwrap_or_default();
            Ok(
                json!({"status":if diagnostics.is_empty(){"valid"}else{"invalid"},"diagnostics":diagnostics,"fields":fields::describe(&inputs.presets,&inputs.defaults)}),
            )
        }
        Request::Generate {
            inputs,
            choice,
            artifact_dir,
        } => generated(inputs, choice, artifact_dir),
        Request::Inspect { artifact_dir, crop } => {
            let a = artifact(&artifact_dir)?;
            let c = checked(&a.inputs).map_err(|e| format!("{e:?}"))?;
            inspection(&a, &c, crop.as_deref())
        }
        Request::Sight { artifact_dir } => {
            if !artifact_dir.join("artifact.json").exists() {
                let capture: Value = serde_json::from_slice(
                    &std::fs::read(artifact_dir.join("request.json")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                let defaults: Value = serde_json::from_str(
                    capture["inputs"]["defaults"]
                        .as_str()
                        .ok_or("Missing captured defaults")?,
                )
                .map_err(|e| e.to_string())?;
                return Ok(
                    json!({"status":"unavailable","samples":0,"target":defaults["analysis"]["min_median_open"],"step_m":defaults["analysis"]["step_m"],"bearings":0,"elapsed_ms":0,"reason":"The requested map was refused; no compiled map exists"}),
                );
            }
            let a = artifact(&artifact_dir)?;
            let c = checked(&a.inputs).map_err(|e| format!("{e:?}"))?;
            Ok(measured(&a, &c))
        }
    }
}
