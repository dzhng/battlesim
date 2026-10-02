//! The endurance load (slice 16): the synthetic stress battle is the size the
//! scale verdict names, and remains are kept, never pruned, as it runs.
use contract::ids::Side;
use sim::battle::Battle;
use sim::endurance::{scenario, LATE_CORPSES, LATE_WRECKS};

use crate::common;

/// The battle's saved field, the catalogue's `endurance` map.
fn field() -> contract::map::MapDefinition {
    sim::maps::load("endurance").unwrap().definition
}

#[test]
fn each_side_fields_100_units_half_of_them_rifle_squads() {
    let s = scenario(&field(), &common::game(), 1, false).unwrap();
    for side in Side::ALL {
        let mine: Vec<_> = s.units.iter().filter(|u| u.side == side).collect();
        assert_eq!(mine.len(), 100);
        assert_eq!(mine.iter().filter(|u| u.kind == "rifle").count(), 50);
    }
    assert!(!s.scripts.is_empty(), "seeded waves drive the battle");
    // Same seed, same battle.
    let again = scenario(&field(), &common::game(), 1, false).unwrap();
    assert_eq!(
        serde_json::to_string(&s).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn the_late_state_starts_with_its_remains_and_keeps_every_one() {
    let s = scenario(&field(), &common::game(), 1, true).unwrap();
    let mut battle = Battle::new(&s, 1);
    let start = battle.load();
    assert!(start.corpses >= LATE_CORPSES);
    assert!(start.wrecks >= LATE_WRECKS);
    assert_eq!(start.living_units, 200);
    let mut last = start;
    for _ in 0..common::tick_hz() * 20 {
        battle.step();
        let now = battle.load();
        assert!(now.corpses >= last.corpses && now.wrecks >= last.wrecks);
        last = now;
    }
}

/// Full-world allocation and local street combat are distinct measurements.
#[test]
fn city_stress_brings_both_forces_and_remains_to_the_same_arena() {
    let mut map = field();
    map.size = [10_000.0, 10_000.0];
    let s = sim::endurance::city_scenario(&map, &common::game(), 1, true).unwrap();
    assert_eq!(
        s.map.size, map.size,
        "the full physical world remains loaded"
    );
    for side in Side::ALL {
        let fighters: Vec<_> = s
            .units
            .iter()
            .filter(|u| u.side == side && u.condition.is_none())
            .collect();
        assert_eq!(fighters.len(), 100);
        assert!(fighters.iter().all(|u| u.position[0] > 3500.0
            && u.position[0] < 6500.0
            && u.position[1] > 4000.0
            && u.position[1] < 6000.0));
        let goal = s
            .scripts
            .iter()
            .find_map(|script| match &script.order {
                contract::command::Order::AttackMove { goal, .. } if script.side == side => {
                    Some(*goal)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            (goal[0] - 5000.0).abs() <= 500.0,
            "both sides advance to contact"
        );
    }
    assert!(s
        .units
        .iter()
        .filter(|u| u.condition.is_some())
        .all(|u| u.position[0] >= 3900.0 && u.position[0] <= 6100.0));
    assert!(s.map.props[map.props.len()..]
        .iter()
        .all(|p| p.center[0] >= 3900.0 && p.center[0] <= 6100.0));
}

#[test]
fn city_stress_places_living_units_on_usable_ground() {
    let mut map = field();
    map.size = [10_000.0, 10_000.0];
    let wreck = serde_json::from_value(serde_json::json!({
        "kind": "heavy_wreck", "center": [4600, 4150], "yaw": 0,
        "half_extents": [12, 12, 2]
    }))
    .unwrap();
    map.props.push(wreck);
    let s = sim::endurance::city_scenario(&map, &common::game(), 1, false).unwrap();
    let world = sim::world::WorldGeometry::new(&map, &s.rules);
    let grid = sim::navigation::NavGrid::new(std::sync::Arc::new(sim::navigation::NavBase::build(
        &world,
        world.props(),
        s.rules.physics.soldier_radius_m,
    )));
    for u in &s.units {
        assert!(
            grid.placement_fits(
                sim::math::v2(u.position[0], u.position[1]),
                &sim::units::mobility(s.rules.catalog.by_id(&u.kind), &s.rules)
            ),
            "{} starts in an obstacle at {:?}",
            u.kind,
            u.position
        );
    }
}

#[test]
fn late_stress_remains_do_not_overwrite_building_parts() {
    let mut map = field();
    map.props.clear();
    map.buildings.clear();
    let template: contract::templates::BuildingTemplateDescriptor = serde_json::from_str(
        include_str!("../../../fixtures/parity/templates/asymmetric.json"),
    )
    .unwrap();
    let parts = template
        .parts
        .iter()
        .enumerate()
        .map(|(i, part)| contract::map::BuildingPartReference {
            part: part.id.clone(),
            prop: i as u32,
        })
        .collect();
    map.buildings.push(
        contract::map::BuildingDefinition::materialize(
            &template,
            contract::templates::PlacementFrame {
                translation: [1500.0, 1000.0, 0.0],
                yaw: 0.0,
            },
            "house".into(),
            0,
            parts,
        )
        .unwrap(),
    );
    let original = map.authored_props().unwrap();
    let s = scenario(&map, &common::game(), 1, true).unwrap();
    let loaded = s
        .map
        .authored_props()
        .expect("late remains must retain a legal dense body namespace");
    for (id, body) in original {
        let found = loaded.iter().find(|(other, _)| *other == id).unwrap();
        assert_eq!(found.1.center, body.center);
        assert_eq!(found.1.half_extents, body.half_extents);
    }
    assert_eq!(
        loaded
            .iter()
            .filter(|(_, p)| p.kind == "heavy_wreck")
            .count(),
        LATE_WRECKS
    );
}

#[cfg(target_os = "macos")]
#[path = "../examples/common/instructions.rs"]
mod counters;

/// Remains are not traffic. Increasing vehicles must not multiply the work
/// added by thousands of fallen squads; count instructions, not loaded clocks.
#[cfg(target_os = "macos")]
#[test]
fn fallen_squads_do_not_multiply_vehicle_traffic_work() {
    // Process counters include every thread. Run the measurement alone even
    // when the surrounding test suite runs battles concurrently.
    if std::env::var_os("SIM_TRAFFIC_COST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "endurance::fallen_squads_do_not_multiply_vehicle_traffic_work",
                "--test-threads=1",
            ])
            .env("SIM_TRAFFIC_COST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "isolated traffic-cost regression failed");
        return;
    }
    fn movement_cost(vehicles: usize, fallen: usize) -> u64 {
        let mut units =
            vec![serde_json::json!({ "side": "blue", "kind": "rifle", "position": [900,900] })];
        for k in 0..vehicles {
            units.push(serde_json::json!({ "side": "blue", "kind": "tank", "position": [50 + k * 10, 50] }));
        }
        for _ in 0..fallen {
            units.push(
                serde_json::json!({ "side": "blue", "kind": "rifle", "position": [500,500],
                "condition": { "casualties": common::rules().catalog.by_id("rifle").squad_size() } }),
            );
        }
        let setup = common::scenario(
            r#"{ "size": [1000,1000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }"#,
            serde_json::Value::Array(units),
            serde_json::json!([]),
        );
        let mut b = Battle::new(&setup, 1);
        let mut previous = counters::instructions().expect("native instruction counter");
        let mut movement = 0;
        b.step_profiled(|phase| {
            let now = counters::instructions().unwrap();
            if phase == sim::battle::TickPhase::Movement {
                movement += now - previous;
            }
            previous = now;
        });
        movement
    }
    let one = movement_cost(1, 2000).saturating_sub(movement_cost(1, 0));
    let fifty = movement_cost(50, 2000).saturating_sub(movement_cost(50, 0));
    assert!(one > 0, "instruction counter must observe the added work");
    assert!(fifty <= one * 2 + 1_000_000,
        "fallen-squad traffic amplification: one vehicle adds {one} instructions, fifty add {fifty}");
}
