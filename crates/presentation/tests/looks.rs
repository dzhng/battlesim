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
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
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

fn fired_look(added: bool, require_kick: bool, second: bool) {
    use contract::ids::Side;
    use presentation::{
        assets::Runtime,
        effects::{
            frame::{EffectBatch, EffectFrame},
            publication::EffectFeed,
            tables::EffectPresentation,
        },
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
    let mut settings = fixture("presentation");
    let mut overrides = json!({"units":{"test_tank":{"sensors":{"ground_m":3000,"low_air_m":3000,"high_air_m":3000,"sight_shape":{"front":1,"side":1,"rear":1}}, "mounts":[{"id":"cannon", "weapons":[id]}]}}});
    if second {
        let source = &scenario.rules.weapons[id].appearance;
        settings["weapon_looks"]["second_round"] = settings["weapon_looks"][source].clone();
        settings["weapon_looks"]["second_round"]["kick"]["hull"] = json!(1);
        overrides["weapons"] = json!({id:{"appearance":"second_round"}});
        overrides["units"]["test_tank"]["mounts"][0]["weapons"] = json!(["tank_ap", id]);
        scenario.units[0].condition =
            Some(serde_json::from_value(json!({"spent":{"tank_ap":10000}})).unwrap());
    }
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
    if second {
        // Distinct authored kicks make a first-row recoil regression observable.
        let first = &battle.rules().weapons["tank_ap"].appearance;
        settings["weapon_looks"][first]["kick"]["hull"] = json!(0);
    }
    let catalog = battle.rules().catalog.view();
    let kinds = vec!["test_tank".into(), "test_supply".into()];
    let fielded = Fielding {
        presentation: &settings,
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
    let expected_cause = format!("flash:{}", p.looks.name(id));
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(f64::from(battle.rules().tick_hz), p);
    let mut publisher = Publisher::new();
    let mut reader = Reader::new(&sim::publication::layout_json(&battle)).unwrap();
    let mut feed = EffectFeed::new("blue");
    let mut pose_feed = presentation::units::feed::ObservationFeed::new("blue");
    let mut interpolator =
        presentation::units::interpolate::TickInterpolator::new(f64::from(battle.rules().tick_hz));
    let mut driver = presentation::units::driver::PoseDriver::new(&fielded.units).unwrap();
    for _ in 0..600 {
        let words: Vec<_> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect();
        let observation = reader.decode(&words).unwrap().unwrap();
        let now = observation.tick / f64::from(battle.rules().tick_hz);
        interpolator.push(std::sync::Arc::new(observation.clone()), now);
        let sample = interpolator
            .frame(now + 1.0 / f64::from(battle.rules().tick_hz))
            .unwrap();
        driver.update(
            pose_feed.frame(sample, &fielded.units, &fielded.effects),
            &fielded.units,
            None,
        );
        let publication = feed.publication(&observation, &fielded.units, &fielded.effects, None);
        if frame
            .note(&publication)
            .iter()
            .any(|launch| &*launch.kind == id)
        {
            frame.build(
                observation.tick / f64::from(battle.rules().tick_hz),
                &mut batch,
                None,
                &[],
                &[],
                None,
            );
            assert!(
                batch
                    .lights
                    .causes
                    .iter()
                    .any(|cause| cause.as_ref() == expected_cause),
                "the fired weapon's admitted look must reach the rendered flash"
            );
            if require_kick {
                let sample = interpolator.frame(now + 0.1).unwrap();
                let poses = driver.update(
                    pose_feed.frame(sample, &fielded.units, &fielded.effects),
                    &fielded.units,
                    None,
                );
                assert!(
                    poses
                        .vehicles
                        .iter()
                        .find(|v| v.unit == 0.0)
                        .unwrap()
                        .motion
                        .is_some(),
                    "a derived tank round must rock its stationary hull after firing"
                );
            }
            return;
        }
        battle.step();
    }
    panic!("{id} never presented a launch");
}

#[test]
fn a_derived_weapon_fired_through_a_battle_presents_its_look() {
    fired_look(false, false, false);
}
#[test]
fn an_encounter_added_tank_gun_fired_through_a_battle_presents_the_tank_look() {
    fired_look(true, false, false);
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

#[test]
fn a_look_cannot_scale_a_misspelled_impact_material() {
    let mut presentation = fixture("presentation");
    presentation["weapon_looks"]["rifle"]["impact"]["surfaces"]["hulll"] =
        json!({"size": 1, "duration": 1});
    let result =
        presentation::looks::WeaponLooks::from_value(&presentation, &fixture("rules")["weapons"]);
    assert!(result
        .expect_err("misspelled impact material")
        .contains("impact.surfaces.hulll"));
}

#[test]
fn fielding_names_the_weapon_when_its_look_lacks_a_required_part() {
    for (weapon, part) in [
        ("atgm", "motor"),
        ("tank_he", "blast"),
        ("tank_ap", "shot"),
        ("tank_ap", "flash"),
        ("tank_ap", "appearance"),
        ("tank_ap", "tracer"),
        ("tank_ap", "impact"),
        ("tank_ap", "kick"),
    ] {
        let mut rules = sim::fixtures::test_rules();
        let mut settings = fixture("presentation");
        let name = rules["weapons"][weapon]["appearance"]
            .as_str()
            .unwrap()
            .to_owned();
        if part == "appearance" {
            rules["weapons"][weapon]["appearance"] = json!("unknown_look");
        } else if ["motor", "shot"].contains(&part) {
            settings["weapon_looks"][&name]["sound"]
                .as_object_mut()
                .unwrap()
                .remove(part);
        } else {
            settings["weapon_looks"][&name]
                .as_object_mut()
                .unwrap()
                .remove(part);
        }
        let rules: contract::scenario::Rules = serde_json::from_value(rules).unwrap();
        let catalog = rules.catalog.view();
        let rules = serde_json::to_value(rules).unwrap();
        let result = presentation::fielded::Fielding {
            presentation: &settings,
            rules: &rules,
            catalog: &catalog,
            runtime: &json!({}),
            kinds: &[],
            factions: None,
        }
        .resolve(&|_| panic!("bad fielding must not read assets"));
        let error = result
            .err()
            .expect("missing physics-required look part was accepted");
        assert!(
            error.contains(weapon) && error.contains(part),
            "{weapon}/{part}: {error}"
        );
    }
}

#[test]
fn an_unmounted_weapon_may_have_only_blast_parts_whatever_its_name() {
    let mut rules = sim::fixtures::test_rules();
    rules["weapons"]["environment_blast"] = json!({
        "extends": "tank_he", "name": "Ambient blast", "appearance": "blast_only"
    });
    let mut settings = fixture("presentation");
    let original = &settings["weapon_looks"]["tank_he"];
    settings["weapon_looks"]["blast_only"] = json!({
        "blast": original["blast"], "sound": {"blast": original["sound"]["blast"]}
    });
    let rules: contract::scenario::Rules = serde_json::from_value(rules).unwrap();
    let catalog = rules.catalog.view();
    presentation::fielded::Fielding {
        presentation: &settings,
        rules: &serde_json::to_value(rules).unwrap(),
        catalog: &catalog,
        runtime: &json!({}),
        kinds: &[],
        factions: None,
    }
    .resolve(&|_| panic!("no units need assets"))
    .unwrap();
}

#[test]
fn a_crash_look_needs_no_firing_parts() {
    let mut settings = fixture("presentation");
    let original = settings["weapon_looks"]["helicopter_crash"].clone();
    settings["weapon_looks"]["helicopter_crash"] = json!({
        "blast": original["blast"], "sound": {"blast":original["sound"]["blast"]}
    });
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let catalog = rules.catalog.view();
    let rules = serde_json::to_value(rules).unwrap();
    presentation::fielded::Fielding {
        presentation: &settings,
        rules: &rules,
        catalog: &catalog,
        runtime: &json!({}),
        kinds: &[],
        factions: None,
    }
    .resolve(&|_| panic!("no units need assets"))
    .unwrap();
}

#[test]
fn a_crash_mounted_as_a_gun_must_supply_firing_parts() {
    let mut settings = fixture("presentation");
    let original = settings["weapon_looks"]["helicopter_crash"].clone();
    settings["weapon_looks"]["helicopter_crash"] = json!({
        "blast":original["blast"],"sound":{"blast":original["sound"]["blast"]}
    });
    let base: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    for weapons in [
        json!(["helicopter_crash"]),
        json!(["tank_ap", "helicopter_crash"]),
    ] {
        let rules = base
            .clone()
            .overridden(
                json!({"units":{"test_tank":{"mounts":[{"id":"cannon","weapons":weapons}]}}})
                    .as_object()
                    .unwrap(),
            )
            .unwrap();
        let catalog = rules.catalog.view();
        let rules = serde_json::to_value(rules).unwrap();
        let result = presentation::fielded::Fielding {
            presentation: &settings,
            rules: &rules,
            catalog: &catalog,
            runtime: &json!({}),
            kinds: &["test_tank".into()],
            factions: None,
        }
        .resolve(&|_| panic!("invalid firing look must be refused before reading art"));
        let error = result
            .err()
            .expect("every ammunition kind needs firing parts");
        assert!(error.contains("helicopter_crash"), "{error}");
    }
}

#[test]
fn a_derived_tank_round_fired_through_a_battle_kicks_its_hull() {
    fired_look(false, true, false);
}

#[test]
fn a_two_weapon_mount_presents_the_second_rounds_look() {
    fired_look(false, true, true);
}
