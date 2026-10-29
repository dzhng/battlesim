//! Looks are fielded from the battle's resolved weapon rows.
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("../../fixtures/{name}.json")).unwrap())
        .unwrap()
}

#[test]
fn unused_shipped_looks_are_refused() {
    presentation::looks::check_shipped(&fixture("presentation"), &fixture("rules")["weapons"])
        .unwrap();
    let mut presentation = fixture("presentation");
    presentation["weapon_looks"]["unused"] = presentation["weapon_looks"]["tank_sabot"].clone();
    let result = presentation::looks::check_shipped(&presentation, &fixture("rules")["weapons"]);
    assert!(result.unwrap_err().contains("unused"));
}

#[test]
fn encounter_weapons_inherit_their_parents_look() {
    let mut rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    rules = rules
        .overridden(
            json!({"weapons": {"encounter_gun": {"extends":"tank_ap", "name":"Encounter gun"}}})
                .as_object()
                .unwrap(),
        )
        .unwrap();
    let looks = presentation::looks::WeaponLooks::from_value(
        &fixture("presentation"),
        &serde_json::to_value(&rules.weapons).unwrap(),
    )
    .unwrap();
    assert_eq!(looks.name("encounter_gun"), looks.name("tank_ap"));
}

#[path = "../../sim/tests/common/mod.rs"]
mod common;

fn fired_look(added: bool) {
    use contract::ids::Side;
    use presentation::{
        assets::Runtime,
        effects::{frame::EffectFrame, publication::EffectFeed, tables::EffectPresentation},
        fielded::Fielding,
    };
    use sim::{
        battle::Battle,
        publication::{reader::Reader, Publisher},
    };
    let mut scenario = common::scenario(
        r#"{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([
            {"side":"blue", "kind":"test_tank", "position":[100,200], "engagement":"fire_at_will"},
            {"side":"red", "kind":"test_supply", "position":[250,200], "engagement":"return_fire_only"}
        ]),
        json!([]),
    );
    let id = if added {
        "encounter_gun"
    } else {
        "light_tank_ap"
    };
    let mut overrides = json!({"units":{"test_tank":{"sensors":{"ground_m":3000,"low_air_m":3000,"high_air_m":3000,"sight_shape":{"front":1,"side":1,"rear":1}}, "mounts":[{"id":"cannon", "weapons":[id]}]}}});
    if added {
        overrides["weapons"] = json!({id:{"extends":"tank_ap", "name":"Encounter gun"}});
    }
    overrides["units"]["test_supply"] = json!({"mounts": [], "body":{"hull":{"hp":1000000, "armor":{"front":100,"side":100,"rear":100,"roof":100}}}});
    scenario.rules = scenario
        .rules
        .overridden(overrides.as_object().unwrap())
        .unwrap();
    let mut battle = Battle::new(&scenario, 7);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = Runtime::open(&root.join("assets/runtime")).unwrap();
    let rules = serde_json::to_value(battle.rules()).unwrap();
    let settings = fixture("presentation");
    let catalog = battle.rules().catalog.view();
    let kinds = vec!["test_tank".into(), "test_supply".into()];
    let fielded = Fielding {
        presentation: &settings,
        weapons: &rules["weapons"],
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    let p = EffectPresentation::from_value(&fielded.effects.presentation).unwrap();
    let expected = if added { "tank_ap" } else { id };
    assert_eq!(
        p.looks.name(id),
        battle.rules().weapons[expected].appearance
    );
    let mut frame = EffectFrame::new(f64::from(battle.rules().tick_hz), p);
    let mut publisher = Publisher::new();
    let mut reader = Reader::new(&sim::publication::layout_json(&battle)).unwrap();
    let mut feed = EffectFeed::new("blue");
    for _ in 0..600 {
        let words: Vec<_> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect();
        let observation = reader.decode(&words).unwrap().unwrap();
        let publication = feed.publication(&observation, &fielded.units, &fielded.effects, None);
        if frame
            .note(&publication)
            .iter()
            .any(|launch| &*launch.kind == id)
        {
            return;
        }
        battle.step();
    }
    panic!("{id} never presented a launch");
}

#[test]
fn a_derived_weapon_fired_through_a_battle_presents_its_look() {
    fired_look(false);
}
#[test]
fn an_encounter_added_tank_gun_fired_through_a_battle_presents_the_tank_look() {
    fired_look(true);
}

#[test]
fn a_look_cannot_silently_assign_a_recording_to_a_misspelled_surface() {
    let mut presentation = fixture("presentation");
    presentation["weapon_looks"]["rifle"]["sound"]["impacts"]["hulll"] = json!("impact_hull");
    let result = presentation::sound::tables::SoundTables::from_parts(
        fixture("audio"),
        fixture("sounds"),
        &presentation,
        &fixture("rules")["weapons"],
    );
    assert!(result
        .expect_err("unknown surface must be refused")
        .contains("hulll"));
}
