use super::model::{MapSource, Row};
use contract::catalog::Faction;
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
use contract::ids::Side;
use contract::scenario::{Rules, ScenarioDefinition};
use contract::skirmish::{MatchRules, SkirmishSetup};
use contract::templates::TemplateGeometryCatalog;

pub(crate) struct Inputs {
    pub(crate) presets: String,
    pub(crate) templates: String,
    pub(crate) rules: Rules,
    pub(crate) rules_json: String,
    pub(crate) preset_revision: String,
    pub(crate) template_hash: String,
    pub(crate) limits: contract::generation::CompileLimits,
    /// Blue's and Red's factions.
    pub(crate) factions: [Faction; 2],
    /// A side's scripted purchases, bought strictly in order as its wallet
    /// allows, in place of its controller's own; None leaves it shopping.
    pub(crate) buy: [Option<Vec<String>>; 2],
    /// A jet keeps its attack target until the side loses sight of it, as a
    /// player's would, instead of taking whatever target is nearest it now.
    pub(crate) sticky_jets: bool,
}

impl Inputs {
    pub(crate) fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let root = sim::fixtures::dir();
        let presets = std::fs::read_to_string(root.join("map-presets.json"))?;
        let preset_revision = serde_json::from_str::<serde_json::Value>(&presets)?["revision"]
            .as_str()
            .ok_or("map presets have no revision")?
            .to_owned();
        let templates = std::fs::read_to_string(root.join("prototype-building-templates.json"))?;
        let catalog = TemplateGeometryCatalog::new(serde_json::from_str(&templates)?)?;
        let rules: Rules = serde_json::from_value(sim::fixtures::game())?;
        let config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
            root.join("generated-battle.json"),
        )?)?;
        Ok(Self {
            presets,
            templates,
            rules_json: serde_json::to_string(&rules)?,
            rules,
            preset_revision,
            template_hash: catalog.hash().into(),
            limits: serde_json::from_value(config["limits"].clone())?,
            factions: [Faction::Us, Faction::Europe],
            buy: [None, None],
            sticky_jets: false,
        })
    }
}

pub(crate) fn prepare_row(
    inputs: &Inputs,
    row: &Row,
) -> Result<
    (
        ScenarioDefinition,
        sim::encounter::PreparedMap,
        Option<String>,
    ),
    String,
> {
    let (map, prepared, sites, identity) = match &row.map {
        MapSource::Saved(id) => {
            let value = sim::fixtures::with_map(id);
            let map: contract::map::MapDefinition =
                serde_json::from_value(value["map"].clone()).map_err(|error| error.to_string())?;
            let prepared = sim::encounter::PreparedMap::new(&map, &inputs.rules);
            let sites = synthetic_sites(&map);
            let identity = contract::identity::json_hash(&map).map_err(|e| e.to_string())?;
            (map, prepared, sites, Some(identity))
        }
        MapSource::Generated(spec) => {
            if spec.kind != "generated" {
                return Err("generated map source must name kind=generated".into());
            }
            let map_type = match spec.map_type.as_str() {
                "open" => MapType::Open,
                "mixed" => MapType::Mixed,
                "metro" => MapType::Metro,
                other => return Err(format!("unknown generated map type {other}")),
            };
            let size = match spec.size.as_str() {
                "small" => MapSize::Small,
                "medium" => MapSize::Medium,
                "large" => MapSize::Large,
                "xl" => MapSize::Xl,
                other => return Err(format!("unknown generated map size {other}")),
            };
            let request = GenerationRequest {
                generator_version: mapgen::layout::GENERATOR_VERSION.into(),
                preset_revision: inputs.preset_revision.clone(),
                seed: row.seed.into(),
                template_catalog_hash: inputs.template_hash.clone(),
                map_type,
                size,
                profile: GenerationProfile::Skirmish,
                region: None,
                limits: inputs.limits,
            };
            let generated = mapgen::generate_map(
                &serde_json::to_string(&request).map_err(|e| e.to_string())?,
                &inputs.presets,
                &inputs.templates,
                &inputs.rules_json,
            );
            let mapgen::CompileOutcome::Ok { result } = generated else {
                return Err("map generation refused the pinned request".into());
            };
            let prepared = sim::encounter::PreparedMap::new(&result.map, &inputs.rules);
            let admitted = sim::map_analysis::admit_skirmish(
                &prepared.queries(&result.map, &result.sites),
                &inputs.rules,
            )
            .map_err(|error| format!("skirmish admission: {error}"))?;
            let sites = admitted
                .sites
                .skirmish
                .ok_or("admission returned no skirmish sites")?;
            let identity = serde_json::to_string(&result.identity).map_err(|e| e.to_string())?;
            (result.map, prepared, sites, Some(identity))
        }
    };
    let setup = ScenarioDefinition {
        skirmish: Some(SkirmishSetup {
            factions: inputs.factions,
            ai_side: None,
            rules: MatchRules {
                credits_per_minute: 200,
                starting_minutes: 2,
                preparation_s: 20,
                max_units: 30,
                dispatch_interval_s: 1,
            },
            sites,
        }),
        map,
        rules: inputs.rules.clone(),
        units: vec![],
        events: vec![],
        scripts: vec![],
        authored_scripts: vec![],
        victory: None,
        opponent: None,
        encounter: None,
    };
    Ok((setup, prepared, identity))
}

fn synthetic_sites(map: &contract::map::MapDefinition) -> SkirmishSites {
    let [width, depth] = map.size;
    SkirmishSites {
        entries: [
            EntrySite {
                side: Side::Blue,
                center: [width * 0.5, 12.0],
                yaw: 1.57,
            },
            EntrySite {
                side: Side::Red,
                center: [width * 0.5, depth - 12.0],
                yaw: -1.57,
            },
        ],
        objectives: vec![
            ObjectiveSite {
                id: "center".into(),
                center: [width * 0.5, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Junction,
                counterpart: None,
            },
            ObjectiveSite {
                id: "west".into(),
                center: [width * 0.25, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: Some("east".into()),
            },
            ObjectiveSite {
                id: "east".into(),
                center: [width * 0.75, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: Some("west".into()),
            },
        ],
        candidates: vec![],
    }
}
