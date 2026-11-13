//! Every fixed-wing jet in the roster flies with its landing gear stowed:
//! the pose driver puts a jet in flight gear up from the first frame it is
//! seen, and its model's gear legs fold away about their tops, while a model
//! at rest (a lab, a bake) still stands on its gear. The shipped
//! presentation, roster and runtime art; synthetic flights.
use presentation::assets::Runtime;
use presentation::fielded::Fielding;
use presentation::units::articulation::{articulate, rest_locals};
use presentation::units::driver::{FeedFrame, FeedUnit, PoseDriver};
use presentation::units::pose::Articulation;
use presentation::world::Appearance;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}

/// Every fixed-wing jet of the roster, by unit id.
fn roster_jets() -> Vec<String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root().join("fixtures/units/roster")).unwrap() {
        let doc: Value =
            serde_json::from_slice(&std::fs::read(entry.unwrap().path()).unwrap()).unwrap();
        for (id, unit) in doc["units"].as_object().into_iter().flatten() {
            if unit["extends"] == "roster_profile_fixed_wing" {
                out.push(id.clone());
            }
        }
    }
    out.sort();
    assert!(out.len() > 1, "the roster flies fixed-wing jets");
    out
}

/// The roster's jets, fielded from the shipped presentation and runtime art.
fn roster_jet_units() -> (Vec<String>, presentation::world::UnitsSection) {
    let kinds = roster_jets();
    let catalog = presentation::static_world::files::rules(&root(), "market-town")
        .unwrap()
        .catalog
        .view();
    let runtime = Runtime::open(&root().join("assets/runtime")).unwrap();
    let fielded = Fielding {
        presentation: &read("fixtures/presentation.json"),
        rules: &read("fixtures/rules.json"),
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    (kinds, fielded.units)
}

#[test]
fn a_parked_jet_stands_on_its_gear() {
    let (kinds, units) = roster_jet_units();
    let mut driver = PoseDriver::new(&units).unwrap();
    let parked = |time: f64| FeedFrame {
        time,
        units: kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| FeedUnit {
                id: i as f64,
                kind: kind.as_str().into(),
                side: "blue".into(),
                position: [0.0, i as f64 * 100.0, 0.0],
                grounded: true,
                ..Default::default()
            })
            .collect(),
        fallen: Arc::new(Vec::new()),
    };
    for time in [0.0, 5.0] {
        for (kind, v) in kinds
            .iter()
            .zip(driver.update(&parked(time), &units, None).vehicles.iter())
        {
            assert_eq!(
                v.articulation.gear_up, 0.0,
                "{kind}: parked, it stands on its gear"
            );
        }
    }
}

#[test]
fn every_roster_jet_flies_gear_up_and_stands_on_its_gear_at_rest() {
    let (kinds, units) = roster_jet_units();
    let mut driver = PoseDriver::new(&units).unwrap();
    let flying = |time: f64| FeedFrame {
        time,
        units: kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| FeedUnit {
                id: i as f64,
                kind: kind.as_str().into(),
                side: "blue".into(),
                position: [time * 150.0, i as f64 * 100.0, 80.0],
                ..Default::default()
            })
            .collect(),
        fallen: Arc::new(Vec::new()),
    };
    let first = driver.update(&flying(0.0), &units, None).vehicles.clone();
    let later = driver.update(&flying(0.5), &units, None).vehicles.clone();
    assert_eq!(first.len(), kinds.len());
    for (i, kind) in kinds.iter().enumerate() {
        for v in [&first[i], &later[i]] {
            assert_eq!(
                v.articulation.gear_up, 1.0,
                "{kind}: in flight from the first frame it is seen, gear up"
            );
        }
        let name = units
            .resolve(kind, "blue", i as f64, 0, None, None)
            .unwrap();
        let Some(Appearance::Articulated { nodes, rig, .. }) = units.appearances.get(name) else {
            panic!("{kind}: an articulated model");
        };
        assert!(
            rig.gear.len() >= 3,
            "{kind}: its model's rig carries its gear legs, found {:?}",
            rig.gear
        );
        let posed = |input: &Articulation| {
            let mut locals = rest_locals(nodes);
            articulate(&mut locals, nodes, rig, input);
            locals
        };
        let rest = posed(&Articulation::default());
        let half = posed(&Articulation {
            gear_up: 0.5,
            ..Default::default()
        });
        let flown = posed(&later[i].articulation);
        for &leg in &rig.gear {
            let bind = nodes[leg].bind;
            assert_eq!(rest[leg].s, bind.s, "{kind}: at rest it stands on its gear");
            assert_eq!(
                flown[leg].s, [0.0; 3],
                "{kind}: in flight its gear is stowed"
            );
            assert_eq!(
                flown[leg].t, bind.t,
                "{kind}: each leg folds about its own top"
            );
            assert!(
                (0..3).all(|k| half[leg].s[k] > 0.0 && half[leg].s[k] < bind.s[k]),
                "{kind}: half stowed, a leg shows part way"
            );
        }
        for (n, node) in nodes.iter().enumerate() {
            if !rig.gear.contains(&n) {
                assert_eq!(flown[n].s, node.bind.s, "{kind}: only the gear folds away");
            }
        }
    }
}
