//! A jet drawn on its gun run: the pose driver, fed the poses a simulated
//! strafe publishes, draws the nose at the pitch the simulation flies, so
//! the model's nose and the rounds leaving it agree.
use presentation::units::driver::PoseDriver;
use serde_json::{json, Value};

mod common;
use common::*;

/// The shipped pose feel for winged aircraft.
fn shipped_fixed_wing_feel() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/presentation.json"
    );
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    fixture["pose"]["attitude"]["fixed_wing"].clone()
}

#[test]
fn a_strafing_jet_is_drawn_at_the_pitch_it_flies() {
    let mut rules = sim::fixtures::test_rules();
    // The ground point a gun run strafes; the test jet carries only its gun.
    let setup = serde_json::from_value(json!({
        "map": { "size": [4000, 4000], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules.take(),
        "units": [{ "side": "blue", "kind": "test_jet", "position": [1000, 2000], "yaw": 0.0 }],
        "events": [], "scripts": []
    }))
    .unwrap();
    let mut battle = sim::battle::Battle::new(&setup, 1);
    let ack = battle.accept(contract::command::CommandEnvelope {
        side: contract::ids::Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: contract::command::Order::Attack {
            units: vec![contract::ids::UnitId(0)],
            target: contract::command::TargetRef::Ground {
                point: [2700.0, 2000.0, 0.0],
            },
        },
    });
    assert_eq!(ack.error, None);

    let feel = shipped_fixed_wing_feel();
    let units = units_with(|v| {
        let mut jet = v["types"]["capture_us_tank"].clone();
        jet["flies"] = true.into();
        jet["flight"] = "fixed_wing".into();
        v["types"]["test_jet"] = jet;
        v["pose"]["attitude"]["fixed_wing"] = feel;
    });
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let hz = f64::from(battle.rules().tick_hz);
    let mut steady_ticks = 0;
    let mut deepest = 0.0_f64;
    let mut sim_pitch = Vec::new();
    for tick in 0..(40.0 * hz) as u64 {
        battle.step();
        let unit = battle.unit(contract::ids::UnitId(0)).unwrap();
        let mut jet = tank(0.0, unit.yaw, (0.0, 0.0, 0.0), 0.0);
        jet.kind = "test_jet".into();
        jet.mounts.clear();
        jet.position = [unit.position.x, unit.position.y, unit.position.z];
        let frame = frame(tick as f64 / hz, vec![jet], &none);
        let drawn = driver.update(&frame, &units, None).vehicles[0]
            .motion
            .map_or(0.0, |m| m[2].asin());
        sim_pitch.push(unit.pitch());
        deepest = deepest.max(-unit.pitch());
        // Holding a pitch for half a second, the drawn nose has caught up.
        let held = (hz * 0.5) as usize;
        let last = &sim_pitch[sim_pitch.len().saturating_sub(held)..];
        let steady = last.len() == held && last.iter().all(|p| (p - unit.pitch()).abs() < 1e-4);
        if steady && unit.pitch() < -0.1 {
            steady_ticks += 1;
            assert!(
                (drawn - unit.pitch()).abs() < 0.5_f64.to_radians(),
                "drawn at {:.2} deg, flying at {:.2} deg",
                drawn.to_degrees(),
                unit.pitch().to_degrees()
            );
        }
    }
    assert!(
        deepest > 10_f64.to_radians(),
        "the jet never dived: {:.1} deg",
        deepest.to_degrees()
    );
    assert!(steady_ticks > 0, "the jet never held its dive");
}
