//! A battle's units and effects are resolved from the runtime contents
//! `Fielding::contents` names, and from no others: a page that fetches
//! exactly those has everything Rust reads.
use presentation::assets::Runtime;
use presentation::fielded::{self, Fielding};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn a_battle_reads_only_the_contents_it_names() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let catalog = presentation::static_world::files::rules(&root, "market-town")
        .unwrap()
        .catalog
        .view();
    let runtime = Runtime::open(&root.join("assets/runtime")).unwrap();
    let kinds = fielded::encounter_kinds(&root, "market-town", "menu").unwrap();
    let fielding = Fielding {
        presentation: &serde_json::from_slice(
            &std::fs::read(root.join("fixtures/presentation.json")).unwrap(),
        )
        .unwrap(),
        rules: &game,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    };
    let named: BTreeSet<String> = fielding.contents().unwrap().into_iter().collect();
    let read = RefCell::new(BTreeSet::new());
    let sections = fielding
        .resolve(&|hash| {
            assert!(
                named.contains(hash),
                "content {hash} was read but not named"
            );
            read.borrow_mut().insert(hash.to_owned());
            runtime.read(hash, "bundle.bin")
        })
        .unwrap();
    // Every drawn appearance and wreck is read; a skeleton no body wears
    // may go unread.
    let skeletons: BTreeSet<String> = runtime.catalog["skeletons"]
        .as_object()
        .unwrap()
        .values()
        .map(|h| h.as_str().unwrap().to_owned())
        .collect();
    let unread: Vec<_> = named.difference(&read.borrow()).cloned().collect();
    assert!(
        unread.iter().all(|h| skeletons.contains(h)),
        "named but unread: {unread:?}"
    );
    let typed = sections.typed().unwrap();
    assert!(!typed.units.appearances.is_empty() && !typed.tables.is_empty());
}

#[test]
fn a_ground_hull_model_without_its_gauge_is_refused_before_loading_bundles() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let catalog = rules.catalog.view();
    let rules = serde_json::to_value(rules).unwrap();
    let mut runtime = Runtime::open(&root.join("assets/runtime")).unwrap().catalog;
    let unit = catalog["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "test_tank")
        .unwrap();
    let appearance = unit["appearance"].as_str().unwrap();
    let faction_appearance = "test_faction_hull";
    runtime["appearances"][faction_appearance] = runtime["appearances"][appearance].clone();
    runtime["appearances"][appearance]["factions"] = serde_json::json!({
        "test_faction": faction_appearance
    });
    runtime["appearances"][faction_appearance]
        .as_object_mut()
        .unwrap()
        .remove("track_gauge");
    let presentation =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let kinds = vec!["test_tank".to_owned()];
    let result = Fielding {
        presentation: &presentation,
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime,
        kinds: &kinds,
        factions: Some(["test_faction", "test_faction"]),
    }
    .resolve(&|_| Err("unexpected bundle read".into()));
    let error = result.err().expect("the missing gauge is refused");
    assert!(
        error.contains(faction_appearance) && error.contains("track_gauge"),
        "{error}"
    );
}

#[test]
fn each_factions_gauge_drives_its_own_track_travel() {
    use presentation::units::driver::{FeedFrame, FeedUnit, PoseDriver};
    use serde_json::json;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let catalog = rules.catalog.view();
    let unit = catalog["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "test_tank")
        .unwrap();
    let appearance = unit["appearance"].as_str().unwrap();
    let half_width = unit["body"]["hull"]["half_extents_m"][1].as_f64().unwrap();
    let runtime = Runtime::open(&root.join("assets/runtime")).unwrap();
    let mut art = runtime.catalog.clone();
    for (name, gauge) in [("test_blue_hull", 0.5), ("test_red_hull", 0.9)] {
        art["appearances"][name] = art["appearances"][appearance].clone();
        art["appearances"][name]["track_gauge"] = json!(gauge);
    }
    art["appearances"][appearance]["factions"] =
        json!({"test_blue": "test_blue_hull", "test_red": "test_red_hull"});
    let units = Fielding {
        presentation: &serde_json::from_str(include_str!("../../../fixtures/presentation.json"))
            .unwrap(),
        rules: &serde_json::to_value(rules).unwrap(),
        catalog: &catalog,
        runtime: &art,
        kinds: &["test_tank".into()],
        factions: Some(["test_blue", "test_red"]),
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap()
    .units;
    let mut driver = PoseDriver::new(&units).unwrap();
    for yaw in [0.0, 0.5] {
        driver.update(
            &FeedFrame {
                time: yaw,
                units: ["blue", "red"]
                    .into_iter()
                    .map(|side| FeedUnit {
                        id: 1.0,
                        kind: "test_tank".into(),
                        side: side.into(),
                        yaw,
                        ..FeedUnit::default()
                    })
                    .collect(),
                fallen: Default::default(),
            },
            &units,
            None,
        );
    }
    for pose in &driver.poses().vehicles {
        let gauge = if &*pose.side == "blue" { 0.5 } else { 0.9 };
        let travelled = pose.articulation.travel_r - pose.articulation.travel_l;
        assert!(
            (travelled - half_width * gauge).abs() < 1e-12,
            "{}: {travelled}",
            pose.side
        );
    }
}
