//! The saved-map catalogue (`fixtures/maps/<id>/`) as native readers meet it:
//! every map resolves by id through the one resolver, every saved encounter
//! makes a battle on its map, and a folder that does not resolve is refused
//! by name, with nothing standing in for it.
use std::path::PathBuf;

use contract::maps::{MapCategory, ResolveCode};
use sim::battle::Battle;
use sim::maps::Catalogue;

#[test]
fn every_saved_map_resolves_by_id_to_the_identity_its_sources_pin() {
    let catalogue = Catalogue::shipped();
    let ids = catalogue.ids();
    assert!(!ids.is_empty(), "the catalogue has maps");
    for id in &ids {
        let resolved = catalogue.load(id).unwrap_or_else(|e| panic!("{e}"));
        let sources: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(catalogue.maps.join(id).join("SOURCES.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&resolved.identity).unwrap(),
            sources["identity"],
            "{id}"
        );
        assert!(
            catalogue.maps.join(id).join("meta.json").is_file(),
            "{id} has its listing metadata"
        );
    }
}

#[test]
fn every_saved_map_says_whether_it_is_a_tests_or_the_capture_maps() {
    let catalogue = Catalogue::shipped();
    let capture: Vec<String> = catalogue
        .ids()
        .into_iter()
        .filter(|id| {
            catalogue.category(id).unwrap_or_else(|e| panic!("{e}")) == MapCategory::Capture
        })
        .collect();
    assert_eq!(capture, ["market-town", "paris-corner", "wetlands"]);
}

#[test]
fn a_listing_that_does_not_say_test_or_capture_is_refused() {
    let dir = Scratch::new("listing");
    for other in [r#""lab""#, r#""playable""#, r#""benchmark""#, "null"] {
        std::fs::write(
            dir.file("meta.json"),
            format!(r#"{{"category": {other}, "label": "Geometry"}}"#),
        )
        .unwrap();
        let error = dir.catalogue().category("geometry").unwrap_err();
        assert_eq!(error.code, ResolveCode::InvalidListing, "{other}");
        assert_eq!(error.location, "geometry/meta.json");
    }
    std::fs::write(dir.file("meta.json"), r#"{"category": "test"}"#).unwrap();
    assert_eq!(
        dir.catalogue().category("geometry").unwrap(),
        MapCategory::Test
    );
    std::fs::remove_file(dir.file("meta.json")).unwrap();
    let missing = dir.catalogue().category("geometry").unwrap_err();
    assert_eq!(missing.code, ResolveCode::MissingDocument);
}

#[test]
fn every_saved_encounter_makes_a_battle_on_its_map() {
    let catalogue = Catalogue::shipped();
    // Each map's encounters run on the catalog set its pages run: a test
    // map's on the test set (labs field the game's units too, the Apache),
    // the menu's on the menu's set.
    let rules_of = |set| {
        let mut game = sim::fixtures::game();
        game["catalog"] = serde_json::Value::Array(sim::fixtures::catalog_documents(set));
        serde_json::from_value::<contract::scenario::Rules>(game).unwrap()
    };
    let test_rules = rules_of(sim::fixtures::CatalogSet::Test);
    let capture_rules = rules_of(sim::fixtures::CatalogSet::Capture);
    let mut encounters = 0;
    for id in catalogue.ids() {
        let map = catalogue.load(&id).unwrap().definition;
        let rules = match catalogue.category(&id).unwrap() {
            contract::maps::MapCategory::Test => &test_rules,
            contract::maps::MapCategory::Capture => &capture_rules,
        };
        for name in catalogue.encounters(&id).unwrap() {
            let encounter = catalogue
                .encounter(&id, &name)
                .unwrap_or_else(|e| panic!("{e}"));
            let units = encounter.units.len();
            assert!(units > 0, "{id}/{name} fields no unit");
            let setup = encounter
                .on(map.clone(), rules.clone())
                .unwrap_or_else(|e| panic!("{id}/{name}: {e}"));
            // Through the first ticks, so its opening events and orders land.
            let mut battle = Battle::new(&setup, 1);
            battle.step();
            battle.step();
            assert_eq!(battle.load().living_units, units, "{id}/{name}");
            encounters += 1;
        }
    }
    assert!(encounters > 0, "the catalogue has saved encounters");
}

/// Production generation fits the released catalogue envelope; tuning a lower work allowance never lowers saved-map admission.
#[test]
fn generated_geometry_allowances_fit_the_released_catalogue_envelope() {
    let game: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(sim::fixtures::dir().join("generated-battle.json")).unwrap(),
    )
    .unwrap();
    let admission = serde_json::to_value(contract::maps::MapAdmission::CATALOGUE).unwrap();
    for limit in ["max_authored_parts", "max_bay_positions"] {
        assert!(game["limits"][limit].as_u64().unwrap() > 0);
        assert!(
            game["limits"][limit].as_u64().unwrap() <= admission[limit].as_u64().unwrap(),
            "{limit}"
        );
    }
}

/// A scratch catalogue holding a copy of the shipped `geometry` folder.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("battlegame-maps-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let shipped = Catalogue::shipped();
        for file in ["map.json", "SOURCES.json", "encounters/authority.json"] {
            let to = root.join("maps/geometry").join(file);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(shipped.maps.join("geometry").join(file), to).unwrap();
        }
        Scratch(root)
    }

    fn catalogue(&self) -> Catalogue {
        Catalogue {
            maps: self.0.join("maps"),
            libraries: Catalogue::shipped().libraries,
        }
    }

    fn file(&self, path: &str) -> PathBuf {
        self.0.join("maps/geometry").join(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_map_that_does_not_resolve_is_refused_naming_the_document_at_fault() {
    let scratch = Scratch::new("refusals");
    let catalogue = scratch.catalogue();
    catalogue.load("geometry").expect("the copy resolves");

    // An address is one folder name: a path is refused before anything is read.
    let error = catalogue.load("../geometry").unwrap_err();
    assert_eq!(error.code, ResolveCode::InvalidId);

    let error = catalogue.load("nowhere").unwrap_err();
    assert_eq!(error.code, ResolveCode::MissingDocument);
    assert_eq!(error.location, "nowhere/map.json");

    // A map edited without its sources no longer is the map they pin.
    let map = std::fs::read_to_string(scratch.file("map.json")).unwrap();
    std::fs::write(
        scratch.file("map.json"),
        map.replacen("\"fog_cell_m\": 8", "\"fog_cell_m\": 16", 1),
    )
    .unwrap();
    let error = catalogue.load("geometry").unwrap_err();
    assert_eq!(error.code, ResolveCode::IdentityMismatch);
    assert_eq!(error.location, "geometry/SOURCES.json.identity.map_hash");
    std::fs::write(scratch.file("map.json"), map).unwrap();

    // A map names its physical library by file name: a path is refused
    // before it is read, and a library the catalogue lacks is a missing
    // document.
    let sources = std::fs::read_to_string(scratch.file("SOURCES.json")).unwrap();
    for (library, code) in [
        (
            "../fixtures/prototype-building-templates.json",
            ResolveCode::InvalidSources,
        ),
        ("absent-templates.json", ResolveCode::MissingDocument),
    ] {
        std::fs::write(
            scratch.file("SOURCES.json"),
            sources.replacen(
                "\"prototype-building-templates.json\"",
                &format!("{library:?}"),
                1,
            ),
        )
        .unwrap();
        let error = catalogue.load("geometry").unwrap_err();
        assert_eq!(error.code, code, "{library}");
        assert_eq!(error.location, "geometry/SOURCES.json.catalogue.library");
    }

    std::fs::remove_file(scratch.file("SOURCES.json")).unwrap();
    let error = catalogue.load("geometry").unwrap_err();
    assert_eq!(error.code, ResolveCode::MissingDocument);
    assert_eq!(error.location, "geometry/SOURCES.json");
    assert!(
        error.message.contains("SOURCES.json"),
        "the diagnostic names the file it could not read: {error}"
    );
}

#[test]
fn an_encounter_that_is_not_one_is_refused_naming_its_file() {
    let scratch = Scratch::new("encounters");
    let catalogue = scratch.catalogue();
    assert_eq!(catalogue.encounters("geometry").unwrap(), ["authority"]);
    catalogue
        .encounter("geometry", "authority")
        .expect("the copy parses");

    let error = catalogue.encounter("geometry", "absent").unwrap_err();
    assert_eq!(error.code, ResolveCode::MissingDocument);
    assert_eq!(error.location, "geometry/encounters/absent.json");

    // A misspelt section is not silently dropped.
    std::fs::write(
        scratch.file("encounters/authority.json"),
        r#"{ "units": [], "script": [] }"#,
    )
    .unwrap();
    let error = catalogue.encounter("geometry", "authority").unwrap_err();
    assert_eq!(error.code, ResolveCode::InvalidEncounter);
    assert_eq!(error.location, "geometry/encounters/authority.json");
    assert!(error.message.contains("script"), "{error}");
}

/// The test rules a scratch encounter plays.
fn test_rules() -> contract::scenario::Rules {
    serde_json::from_value(sim::fixtures::test_game()).unwrap()
}

#[test]
fn an_encounters_overrides_retune_its_own_battle_and_no_other() {
    let scratch = Scratch::new("overrides");
    let catalogue = scratch.catalogue();
    let rules = test_rules();
    let shared = rules.weapons["tank_ap"].clone();
    let reload_s = shared.reload_s + 1.25;
    // A blue tank fielding a weapon only this scenario has, a test tank
    // whose shared gun this scenario retunes, and a red tank to shoot at.
    let encounter = serde_json::json!({
        "units": [
            { "side": "blue", "kind": "test_scenario_tank", "position": [60, 120] },
            { "side": "blue", "kind": "test_tank", "position": [60, 100] },
            { "side": "red", "kind": "test_tank", "position": [260, 150], "yaw": std::f64::consts::PI }
        ],
        "overrides": {
            "weapons": {
                "tank_ap": { "reload_s": reload_s },
                "test_scenario_gun": { "extends": "tank_ap", "name": "Scenario Gun" }
            },
            // A finite row is priced for resupply, as a shipped one is.
            "service": { "round_costs": { "test_scenario_gun": 5 } },
            "units": {
                "test_scenario_tank": {
                    "extends": "test_tank",
                    "name": "Scenario Tank",
                    "mounts": [{ "id": "cannon", "weapons": ["test_scenario_gun"] }]
                }
            }
        }
    });
    std::fs::write(
        scratch.file("encounters/retuned.json"),
        encounter.to_string(),
    )
    .unwrap();

    let scenario = catalogue
        .scenario("geometry", "retuned", rules.clone())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut battle = Battle::new(&scenario, 1);
    // The retuned row changes the field it names and keeps the rest; the
    // added row extends the retuned one.
    let played = &battle.rules().weapons["tank_ap"];
    assert_eq!(played.reload_s, reload_s);
    assert_eq!(played.aim_s, shared.aim_s);
    assert_eq!(
        battle.rules().weapons["test_scenario_gun"].reload_s,
        reload_s
    );
    // The scenario-only tank's cannon fires the scenario-only gun, never the
    // shared one it replaced.
    let mut fired = std::collections::BTreeSet::new();
    for _ in 0..600 {
        battle.step();
        for (_, round) in battle.rounds().filter(|(_, r)| r.unit.0 == 0) {
            fired.insert(battle.arsenal().weapons[round.weapon].id.clone());
        }
    }
    assert!(fired.contains("test_scenario_gun"), "{fired:?}");
    assert!(!fired.contains("tank_ap"), "{fired:?}");

    // The rules it was laid on, and every other scenario on them, keep the
    // shared row.
    assert_eq!(rules.weapons["tank_ap"].reload_s, shared.reload_s);
    let other = catalogue
        .scenario("geometry", "authority", rules)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(other.rules.weapons["tank_ap"].reload_s, shared.reload_s);
}

#[test]
fn an_override_is_held_to_the_shipped_rules_checks_naming_its_encounter_and_field() {
    let scratch = Scratch::new("bad-overrides");
    let catalogue = scratch.catalogue();
    for (overrides, field) in [
        (
            serde_json::json!({ "weapons": { "tank_ap": { "reload_s": -1 } } }),
            "weapons.tank_ap: reload_s",
        ),
        // A mount naming a row the rules lack, as a shipped unit would be.
        (
            serde_json::json!({ "units": { "test_tank": { "mounts": [
                { "id": "cannon", "weapons": ["absent_gun"] }
            ] } } }),
            "absent_gun",
        ),
        // A misspelt section is not silently dropped.
        (serde_json::json!({ "weapon": {} }), "weapon"),
    ] {
        std::fs::write(
            scratch.file("encounters/authority.json"),
            serde_json::json!({ "units": [], "overrides": overrides }).to_string(),
        )
        .unwrap();
        let error = catalogue
            .scenario("geometry", "authority", test_rules())
            .unwrap_err();
        assert_eq!(error.code, ResolveCode::InvalidEncounter, "{error}");
        assert_eq!(error.location, "geometry/encounters/authority.json");
        assert!(error.message.contains(field), "{error}");
    }
}

/// The native half of the scenario-override paired record: an encounter
/// that retunes a shared weapon and fields a scenario-only unit and weapon,
/// played on the test set's rules, as its battle's digests.
/// `web/tests/scenarioOverrides.test.ts` holds the Wasm build, which makes
/// the scenario through the same `EncounterDefinition::on`, to them.
#[test]
fn an_overridden_encounter_plays_its_paired_record() {
    let path = "scenario-overrides.json";
    let mut record: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/scenario-overrides.json"
    ))
    .unwrap();
    let mut game = sim::fixtures::game();
    game["catalog"] = serde_json::Value::Array(sim::fixtures::catalog_documents(
        sim::fixtures::CatalogSet::Test,
    ));
    let rules = serde_json::from_value(game).unwrap();
    let map = sim::maps::load(record["map"].as_str().unwrap())
        .unwrap()
        .definition;
    let encounter: contract::scenario::EncounterDefinition =
        serde_json::from_value(record["encounter"].clone()).unwrap();
    let scenario = encounter.on(map, rules).unwrap();
    let mut battle = Battle::new(&scenario, record["seed"].as_u64().unwrap());
    let every = record["every"].as_u64().unwrap();
    let mut digests = vec![format!("{:016x}", battle.digest())];
    while battle.tick() < record["ticks"].as_u64().unwrap() {
        battle.step();
        if battle.tick().is_multiple_of(every) {
            digests.push(format!("{:016x}", battle.digest()));
        }
    }
    let recorded = std::mem::replace(&mut record["digests"], serde_json::json!(digests));
    if crate::common::bless_parity(path, &record) {
        return;
    }
    assert_eq!(recorded, record["digests"]);
}
