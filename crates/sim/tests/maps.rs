//! The saved-map catalogue (`fixtures/maps/<id>/`) as native readers meet it:
//! every map resolves by id through the one resolver, every saved encounter
//! makes a battle on its map, and a folder that does not resolve is refused
//! by name, with nothing standing in for it.
use std::path::PathBuf;

use contract::maps::{MapCategory, ResolveCode};
use sim::battle::Battle;
use sim::maps::Catalogue;

use crate::common;

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
fn every_saved_map_says_whether_it_is_a_tests_or_the_menus() {
    let catalogue = Catalogue::shipped();
    let menu: Vec<String> = catalogue
        .ids()
        .into_iter()
        .filter(|id| catalogue.category(id).unwrap_or_else(|e| panic!("{e}")) == MapCategory::Menu)
        .collect();
    assert_eq!(menu, ["market-town", "paris-corner"]);
}

#[test]
fn a_listing_that_does_not_say_test_or_menu_is_refused() {
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
    let rules = common::rules();
    let mut encounters = 0;
    for id in catalogue.ids() {
        let map = catalogue.load(&id).unwrap().definition;
        for name in catalogue.encounters(&id).unwrap() {
            let encounter = catalogue
                .encounter(&id, &name)
                .unwrap_or_else(|e| panic!("{e}"));
            let units = encounter.units.len();
            assert!(units > 0, "{id}/{name} fields no unit");
            let setup = encounter.on(map.clone(), rules.clone());
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
            "../fixtures/building-templates.json",
            ResolveCode::InvalidSources,
        ),
        ("absent-templates.json", ResolveCode::MissingDocument),
    ] {
        std::fs::write(
            scratch.file("SOURCES.json"),
            sources.replacen("building-templates.json", library, 1),
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
