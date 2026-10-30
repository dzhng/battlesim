//! Battle setup follows sim/tests/common and maps' encounter override test.
use contract::catalog::AltitudeLayer;
use contract::scenario::{EncounterDefinition, Rules, ScenarioDefinition};
use serde_json::{json, Value};

#[path = "../../../sim/tests/common/mod.rs"]
mod common;

pub struct Case {
    pub name: String,
    pub scenario: ScenarioDefinition,
    pub expected: Vec<String>,
    pub hull_trace: bool,
}

fn row<'a>(game: &'a mut Value, section: &str, id: &str) -> &'a mut Value {
    game["catalog"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|doc| doc.get_mut(section)?.get_mut(id))
        .unwrap()
}

pub fn weapon(id: &str, added: bool, soldier: bool) -> Case {
    let mut game = sim::fixtures::test_rules();
    let mut overrides = json!({});
    if added {
        overrides = json!({"weapons": {id: {"extends": "carbine", "name": "Encounter carbine"}}});
    }
    let base: Rules = serde_json::from_value(game.clone()).unwrap();
    let resolved = base.overridden(overrides.as_object().unwrap()).unwrap();
    let weapon = &resolved.weapons[id];
    let bomb = weapon.ballistics.delivery == contract::ballistics::Delivery::Unpowered;
    let layer = if weapon.targets.contains_key(&AltitudeLayer::Ground) {
        AltitudeLayer::Ground
    } else if weapon.targets.contains_key(&AltitudeLayer::LowAir) {
        AltitudeLayer::LowAir
    } else {
        AltitudeLayer::HighAir
    };
    let distance = (weapon.min_range_m + 100.0).max(100.0);
    let shooter = if soldier {
        "test_rifle"
    } else if bomb {
        "test_jet"
    } else {
        "test_tank"
    };
    if soldier {
        let mut gunner = row(&mut game, "soldiers", "test_rifleman").clone();
        gunner["mounts"] =
            json!([{ "id": "rifles", "name": "Probe gun", "weapons": [id], "squad": true }]);
        game["catalog"]
            .as_array_mut()
            .unwrap()
            .push(json!({"soldiers": {"proof_gunner": gunner}}));
        row(&mut game, "units", shooter)["body"]["squad"]["slots"] = json!(["proof_gunner"]);
    } else {
        row(&mut game, "units", shooter)["mounts"] = json!([{
            "id": if bomb { "gun" } else { "cannon" }, "name": "Probe gun", "weapons": [id],
            "turret": !bomb, "pivot_m": [0, 0, 1.45], "muzzle_m": [5.9, 0, 0.55]
        }]);
    }
    row(&mut game, "units", shooter)["sensors"] = json!({
        "ground_m": 3000, "low_air_m": 3000, "high_air_m": 3000,
        "sight_shape": {"front": 1, "side": 1, "rear": 1}
    });
    let target = match layer {
        AltitudeLayer::Ground => "test_supply",
        AltitudeLayer::LowAir => "test_heli",
        AltitudeLayer::HighAir => "test_jet",
    };
    let target_row = row(&mut game, "units", target);
    target_row["mounts"] = json!([]);
    target_row["body"]["hull"]["hp"] = json!(1e8);
    target_row["body"]["hull"]["armor"] = json!({
        "front": 0, "side": 0, "rear": 0, "roof": 0,
        "ricochet": {"front": 0, "side": 0, "rear": 0, "roof": 0}
    });
    // SEAD needs an emitting target; no weapon's authored mechanics are retuned.
    if weapon.anti_radiation {
        target_row["capabilities"]["radar_emitter"] = json!(true);
    }
    let units = json!([
        {"side": "blue", "kind": shooter, "position": if bomb {json!([1000, 2000])} else {json!([100, 200])}},
        {"side": "red", "kind": target, "position": if bomb {json!([2000, 2000])} else {json!([100.0 + distance, 200.0])},
         "yaw": std::f64::consts::PI, "engagement": "return_fire_only"}
    ]);
    let map = json!({"size": [if bomb {4000.0} else {(distance + 400.0).max(1000.0)}, if bomb {4000} else {1000}], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35});
    // Build the same analytic map and scenario as the sim harness, then use
    // the saved encounter entry point to exercise the added row's extends.
    let mut scenario = common::scenario(&map.to_string(), units.clone(), json!([]));
    if added {
        // The base catalog must load before the encounter adds its weapon.
        let mut base_game = game.clone();
        row(&mut base_game, "soldiers", "proof_gunner")["mounts"][0]["weapons"] =
            json!(["carbine"]);
        let encounter: EncounterDefinition = serde_json::from_value(json!({
            "units": units, "overrides": {
                "weapons": overrides["weapons"],
                "soldiers": {"proof_gunner": {"mounts": [
                    {"id": "rifles", "weapons": [id]}
                ]}}
            }
        }))
        .unwrap();
        scenario = encounter
            .on(scenario.map, serde_json::from_value(base_game).unwrap())
            .unwrap();
    } else {
        scenario.rules = serde_json::from_value(game.clone()).unwrap();
    }
    Case {
        name: if soldier {
            format!("soldier-{id}")
        } else {
            id.into()
        },
        scenario,
        expected: vec![id.into()],
        hull_trace: false,
    }
}

pub fn bmp() -> Case {
    let mut game = sim::fixtures::rules();
    game["catalog"] = json!(sim::fixtures::catalog_documents(
        sim::fixtures::CatalogSet::Test
    ));
    // The actual mount chooses HE at a soft vehicle, then Bastion once the
    // last HE round is exhausted. Both travel through the same publication.
    let mut scenario = common::scenario(
        r#"{"size":[1000,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#,
        json!([
            {"side":"blue", "kind":"eastern_bmp_ifv_family_bmp_3", "position":[200,300],
             "condition":{"spent":{"ifv_he":19}}},
            {"side":"red", "kind":"test_supply", "position":[400,300], "engagement":"return_fire_only"}
        ]),
        json!([]),
    );
    row(&mut game, "units", "test_supply")["body"]["hull"]["hp"] = json!(1e8);
    scenario.rules = serde_json::from_value(game.clone()).unwrap();
    Case {
        name: "bmp-3".into(),
        scenario,
        expected: vec!["ifv_he".into(), "bastion".into()],
        hull_trace: true,
    }
}
