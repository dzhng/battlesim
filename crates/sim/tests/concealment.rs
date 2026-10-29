//! Authored concealment through real battle sensing, HIDDEN and replay.
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

use crate::common;

fn setup(observer: &str, target: &str, forest: bool, moving: bool) -> ScenarioDefinition {
    let mut rules = common::game();
    for (id, concealment, spotting) in [
        ("test_rifle", 1.0, 1.0),
        ("test_at", 0.5, 1.0),
        ("test_recon", 1.0, 2.0),
        ("test_tank", 2.8, 0.8),
        ("test_heli", 0.05, 1.0),
    ] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({
                "concealment": concealment,
                "sensors": { "ground_m": 600, "low_air_m": 600, "high_air_m": 600,
                    "spotting": spotting, "sight_shape": {"front": 1, "side": 1, "rear": 1} }
            }),
        );
    }
    rules["sensors"]["moving_concealment"] = json!(1.5);
    rules["sensors"]["building_range_multiplier"] = json!(0.4);
    rules["forests"]["rule"]["concealment"] = json!(0.25);
    rules["forests"]["rule"]["attenuation_per_m"] = json!(0);
    serde_json::from_value(json!({
        "map": {"size": [2000, 240], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "forests": if forest { json!([{
                "shape": {"kind": "polygon", "ring": [[900,0],[1100,0],[1100,240],[900,240]]}
            }]) } else { json!([]) } },
        "rules": rules,
        "units": [
            {"side": "blue", "kind": observer, "position": [1800,120], "engagement": "return_fire_only"},
            {"side": "red", "kind": target, "position": [1000,120], "engagement": "return_fire_only"}
        ], "events": [], "scripts": if moving { json!([{
            "tick": 1, "side": "red", "order": {"kind": "move", "units": [1],
                "gesture": 1, "goal": [1000,200], "route": "shortest"}
        }]) } else {json!([])}
    })).unwrap()
}

/// Probe identical eyes against the battle's actual target samples and geometry.
/// Only observer distance changes; the coarse sweep avoids pinning exact metres.
fn reach(b: &Battle) -> f64 {
    let mut units: Vec<_> = (0..2).map(|i| b.unit(UnitId(i)).unwrap().clone()).collect();
    let at = units[1].position;
    (20..=900)
        .step_by(20)
        .filter(|&distance| {
            units[0].position = sim::math::v3(at.x + distance as f64, at.y, 0.0);
            !sim::sensing::evaluate(b.world(), &units, b.rules(), Side::Blue, |_| true).is_empty()
        })
        .max()
        .expect("identified up close") as f64
}

fn run(setup: &ScenarioDefinition, ticks: usize) -> Battle {
    let mut battle = Battle::new(setup, 7);
    let mut frames = Vec::new();
    for _ in 0..ticks {
        battle.step();
        frames.push((
            battle.digest(),
            serde_json::to_value(battle.observe(Side::Red)).unwrap(),
        ));
    }
    let mut replay = Battle::from_replay(setup, &battle.replay()).unwrap();
    for (digest, frame) in frames {
        replay.step();
        assert_eq!(
            replay.digest(),
            digest,
            "replay digest at {}",
            replay.tick()
        );
        assert_eq!(
            serde_json::to_value(replay.observe(Side::Red)).unwrap(),
            frame
        );
    }
    battle
}

#[test]
fn concealment_controls_identification_and_hidden() {
    let battle =
        |observer, target, forest, moving| run(&setup(observer, target, forest, moving), 4);
    let ordinary = reach(&battle("test_rifle", "test_rifle", false, false));
    let sniper = battle("test_rifle", "test_at", false, false);
    let still = reach(&sniper);
    assert!(
        still < ordinary,
        "a hidden team is identified closer in the open"
    );
    assert!(
        !sniper.observe(Side::Red).own[0].concealed,
        "open ground cannot grant HIDDEN"
    );
    let scout = battle("test_recon", "test_at", false, false);
    let scout_reach = reach(&scout);
    assert!(scout_reach > still, "scouts cancel concealment");
    assert!(scout_reach <= scout.rules().catalog.by_id("test_recon").sensors.ground_m);
    assert!(
        reach(&battle("test_tank", "test_at", false, false)) < still,
        "poor spotting needs to be closer"
    );
    let moving = battle("test_rifle", "test_at", false, true);
    assert!(
        moving.unit(UnitId(1)).unwrap().moving(),
        "the order is in progress"
    );
    assert!(reach(&moving) > still, "moving exposes the team");

    let forest = battle("test_rifle", "test_at", true, false);
    let ordinary_forest = reach(&battle("test_rifle", "test_rifle", true, false));
    assert!(
        reach(&forest) < ordinary_forest && ordinary_forest < still,
        "forest and unit concealment multiply"
    );
    assert!(forest.observe(Side::Red).own[0].concealed);
    let parked = battle("test_rifle", "test_tank", true, false);
    let driving = battle("test_rifle", "test_tank", true, true);
    assert!(
        parked.observe(Side::Red).own[0].concealed,
        "parked tank is HIDDEN in forest"
    );
    assert!(
        !driving.observe(Side::Red).own[0].concealed,
        "driving tank loses HIDDEN"
    );
    assert!(
        reach(&driving) >= reach(&parked),
        "moving tank is never harder to see"
    );

    let air = battle("test_rifle", "test_heli", false, false);
    let air_moving_forest = battle("test_rifle", "test_heli", true, true);
    assert!(air.unit(UnitId(1)).unwrap().airborne());
    assert_eq!(
        reach(&air),
        reach(&air_moving_forest),
        "air ignores ground, own and moving concealment"
    );
    assert!(
        reach(&air) > still,
        "own concealment does not hide an aircraft"
    );
    assert!(!air_moving_forest.observe(Side::Red).own[0].concealed);

    let sheltered = garrisoned(0.5);
    let ordinary_shelter = garrisoned(1.0);
    assert!(
        reach(&sheltered) < reach(&ordinary_shelter),
        "unit concealment still applies inside a building"
    );
    assert!(
        reach(&sheltered) < still,
        "shelter also reduces identification reach"
    );
}

fn garrisoned(concealment: f64) -> Battle {
    let mut sheltered = setup("test_rifle", "test_at", false, false);
    sheltered.map = common::physical_map(&json!({
        "size": [2000,240], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "buildings": [common::building(0, "china-apartment-point-20x20-7f", [1000.0,120.0], 0.0)]
    }).to_string());
    let mut rules = serde_json::to_value(&sheltered.rules).unwrap();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_at",
        json!({"concealment": concealment}),
    );
    sheltered.rules = serde_json::from_value(rules).unwrap();
    sheltered.units[1].position = [970.0, 120.0];
    let mut garrison = Battle::new(&sheltered, 7);
    let ack = garrison.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Garrison {
            units: vec![UnitId(1)],
            building: 0,
        },
    });
    assert_eq!(ack.error, None);
    let mut frames = Vec::new();
    for _ in 0..900 {
        garrison.step();
        frames.push((
            garrison.digest(),
            serde_json::to_value(garrison.observe(Side::Red)).unwrap(),
        ));
        if garrison.unit(UnitId(1)).unwrap().garrisoned() {
            break;
        }
    }
    assert!(garrison.unit(UnitId(1)).unwrap().garrisoned());
    assert!(sim::garrison::shelter(garrison.unit(UnitId(1)).unwrap()) > 0.0);
    assert!(garrison.observe(Side::Red).own[0].concealed);
    let mut replay = Battle::from_replay(&sheltered, &garrison.replay()).unwrap();
    for (digest, frame) in frames {
        replay.step();
        assert_eq!(replay.digest(), digest);
        assert_eq!(
            serde_json::to_value(replay.observe(Side::Red)).unwrap(),
            frame
        );
    }
    garrison
}
