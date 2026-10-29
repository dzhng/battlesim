use super::model::{MapSource, Row};
use contract::catalog::Faction;
use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
use contract::ids::Side;
use contract::players::{ControllerKind, Player};
use contract::scenario::{Rules, ScenarioDefinition};
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
    /// Whether each match is replayed from its commands to check its digest.
    pub(crate) check_replay: bool,
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
        let rules: Rules = serde_json::from_value(sim::fixtures::rules())?;
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
            check_replay: true,
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
    let players: Vec<Player> = Side::COMBATANTS
        .into_iter()
        .flat_map(|side| {
            (0..row.players_per_side.max(1)).map(move |index| Player {
                id: format!(
                    "{}-{}",
                    if side == Side::Blue { "blue" } else { "red" },
                    index + 1
                ),
                side,
                faction: Some(inputs.factions[side.index()]),
                controller: ControllerKind::ActiveAi,
                human_capable: false,
            })
        })
        .collect();
    let (map, prepared, sites, identity) = match &row.map {
        MapSource::Saved(id) => {
            let value = sim::fixtures::with_map(id);
            let map: contract::map::MapDefinition =
                serde_json::from_value(value["map"].clone()).map_err(|error| error.to_string())?;
            let prepared = sim::encounter::PreparedMap::new(&map, &inputs.rules);
            let sites = sim::skirmish::synthetic_sites(
                map.size,
                contract::players::entries_per_side(&players),
            );
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
                entries_per_side: contract::players::entries_per_side(&players),
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
                &players,
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
        players: players.clone(),
        skirmish: Some(sim::skirmish::setup_for(&players, sites)),
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
