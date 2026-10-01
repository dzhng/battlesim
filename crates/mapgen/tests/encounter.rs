//! The encounter planner on generated maps: the map a request generates,
//! with the sites generation hands out beside it, takes the shipped recipe.
//! The planner is the simulation's; this crate only makes the map.
use contract::encounter::{EncounterOutcome, EncounterRecipes};
use contract::ids::Side;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const CATALOGUE: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const RECIPES: &str = include_str!("../../../fixtures/encounters.json");
const RECORDS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/parity/encounter/paired-records.json"
);

#[derive(serde::Serialize, serde::Deserialize)]
struct Corpus {
    cases: Vec<Record>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Record {
    name: String,
    /// The generation request whose map the encounter is planned on.
    request_json: String,
    /// A recipe of `fixtures/encounters.json`.
    recipe: String,
    /// Recipe sections merged over that recipe, field by field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recipe_patch: Option<serde_json::Value>,
    encounter_seed: String,
    /// The outcome's hash: the placement or the diagnostics, to the byte.
    native_sha256: String,
    status: String,
}

/// The recipe a record names, with its patch merged over it.
fn recipe_json(record: &Record) -> String {
    let file: serde_json::Value = serde_json::from_str(RECIPES).unwrap();
    let mut recipe = file["recipes"][&record.recipe].clone();
    assert!(recipe.is_object(), "no recipe {}", record.recipe);
    if let Some(patch) = &record.recipe_patch {
        for (section, fields) in patch.as_object().unwrap() {
            for (field, value) in fields.as_object().unwrap() {
                recipe[section][field] = value.clone();
            }
        }
    }
    recipe.to_string()
}

/// The outcome of planning `record`: the map its request generates, then
/// the simulation's planner over the same JSON the Wasm boundary is given.
fn outcome(record: &Record) -> String {
    let mapgen::CompileOutcome::Ok { result } =
        mapgen::generate_map(&record.request_json, PRESETS, CATALOGUE)
    else {
        panic!("{}: the request generates no map", record.name)
    };
    sim::encounter::plan_encounter_json(
        &serde_json::to_string(&result.map).unwrap(),
        &serde_json::to_string(&result.sites).unwrap(),
        &sim::fixtures::game().to_string(),
        &recipe_json(record),
        &record.encounter_seed,
    )
}

#[test]
fn the_shipped_recipes_are_valid() {
    let recipes = EncounterRecipes::from_json(RECIPES).unwrap();
    assert!(recipes.recipes.contains_key("assault"));
}

/// The native half of the native/Wasm proof: `web/tests/encounter.test.ts`
/// holds the Wasm export to these same outcome hashes.
#[test]
fn native_planning_replays_the_frozen_encounter_records() {
    let mut corpus: Corpus =
        serde_json::from_str(&std::fs::read_to_string(RECORDS).unwrap()).unwrap();
    // `BLESS_PARITY=1` rewrites the native half for a named planner, recipe,
    // generator or rules change; the web test then holds Wasm to it.
    let bless = std::env::var_os("BLESS_PARITY").is_some();
    for record in &mut corpus.cases {
        let outcome = outcome(record);
        let hash = contract::identity::bytes_hash(outcome.as_bytes());
        let parsed: EncounterOutcome = serde_json::from_str(&outcome).unwrap();
        let status = match &parsed {
            EncounterOutcome::Ok { .. } => "ok",
            EncounterOutcome::Error { .. } => "error",
        };
        if bless {
            record.native_sha256 = hash;
            record.status = status.into();
            continue;
        }
        assert_eq!(status, record.status, "{}: {outcome:.600}", record.name);
        assert_eq!(
            hash, record.native_sha256,
            "{}: {outcome:.600}",
            record.name
        );
        // The same inputs plan the same bytes again.
        assert_eq!(outcome, self::outcome(record), "{}", record.name);
        if let EncounterOutcome::Ok { encounter } = parsed {
            assert_eq!(
                encounter.encounter_seed.value().to_string(),
                record.encounter_seed
            );
            // The sides start at opposite edges of the map.
            let depth = 6000.0;
            for d in &encounter.placement.deployments {
                let near = match d.side {
                    Side::Blue => d.head[1],
                    Side::Red => depth - d.head[1],
                };
                assert!(
                    near < depth / 3.0,
                    "{}: {:?} starts {near} m from its edge",
                    record.name,
                    d.side
                );
            }
        }
    }
    if bless {
        std::fs::write(
            RECORDS,
            serde_json::to_string_pretty(&corpus).unwrap() + "\n",
        )
        .unwrap();
    }
}
