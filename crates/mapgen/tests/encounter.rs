//! The encounter planner on generated maps: the map a request generates,
//! with the sites generation hands out beside it, takes the shipped recipe.
//! The planner is the simulation's; this crate only makes the map.
use contract::encounter::EncounterRecipes;
use contract::ids::Side;
use sim::encounter::{plan_encounter, PreparedMap};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const CATALOGUE: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const RECIPES: &str = include_str!("../../../fixtures/encounters.json");

/// A medium skirmish generation request of `kind` from `seed`.
fn request(kind: &str, seed: u64) -> String {
    serde_json::json!({
        "generator_version": mapgen::layout::GENERATOR_VERSION,
        "preset_revision": "layout-presets-19",
        "seed": seed.to_string(),
        "template_catalog_hash": "9af8fb1279fe26d83ce33f24180655f4f1f1115caa455c0b1cad57cccd99bea7",
        "type": kind,
        "size": "medium",
        "limits": {
            "max_authored_parts": 250000,
            "max_bay_positions": 600000,
            "max_ground_points": 200000
        },
        "profile": "skirmish"
    })
    .to_string()
}

#[test]
fn the_shipped_recipes_are_valid() {
    let recipes = EncounterRecipes::from_json(RECIPES).unwrap();
    assert!(recipes.recipes.contains_key("assault"));
}

#[test]
fn generated_maps_plan_the_assault_with_the_sides_at_opposite_edges() {
    let mut rules_json = sim::fixtures::test_game();
    rules_json["catalog"] = serde_json::Value::Array(sim::fixtures::catalog_documents(
        sim::fixtures::CatalogSet::Test,
    ));
    let rules_json = rules_json.to_string();
    let rules: contract::scenario::Rules = serde_json::from_str(&rules_json).unwrap();
    let recipe = &EncounterRecipes::from_json(RECIPES).unwrap().recipes["assault"];
    for (kind, seed) in [("mixed", 1), ("open", 2), ("metro", 3)] {
        let mapgen::CompileOutcome::Ok { result } =
            mapgen::generate_map(&request(kind, seed), PRESETS, CATALOGUE, &rules_json)
        else {
            panic!("{kind} {seed}: the request generates no map")
        };
        let prepared = PreparedMap::new(&result.map, &rules);
        let plan = || {
            plan_encounter(
                &prepared.queries(&result.map, &result.sites),
                &rules,
                recipe,
                1.into(),
            )
            .unwrap_or_else(|refusal| panic!("{kind} {seed}: no assault: {refusal:?}"))
        };
        let encounter = plan();
        // The same inputs plan the same encounter again.
        assert_eq!(
            serde_json::to_string(&encounter).unwrap(),
            serde_json::to_string(&plan()).unwrap(),
            "{kind} {seed}"
        );
        let depth = result.map.size[1];
        for d in &encounter.placement.deployments {
            let near = match d.side {
                Side::Blue => d.head[1],
                Side::Red => depth - d.head[1],
            };
            assert!(
                near < depth / 3.0,
                "{kind} {seed}: {:?} starts {near} m from its edge",
                d.side
            );
        }
    }
}
