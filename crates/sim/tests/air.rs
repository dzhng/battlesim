//! Aircraft: a hull in the air is no ground body. Traffic drives under it,
//! soldiers neither step out of it nor lean on it, forest ground does not hide
//! it, and it is admitted against its own hull limits.

use contract::catalog::AltitudeLayer;
use contract::command::{
    CommandEnvelope, MoveDirection, Order, OrderError, RoutePolicy, TargetRef,
};
use contract::ids::{Side, UnitId};
use contract::observation::ContactSource;
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

/// The test rules, whose catalog holds the test helicopter (`test_heli`,
/// `fixtures/units/test/aircraft.json`).
pub(crate) fn fixture() -> Value {
    sim::fixtures::test_game()
}

pub(crate) fn battle_with(fixture: &Value, map: Value, units: Value) -> Battle {
    scripted(fixture, map, units, json!([]), json!([]))
}

fn scripted(fixture: &Value, map: Value, units: Value, events: Value, scripts: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": map, "rules": fixture, "units": units, "events": events, "scripts": scripts,
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

pub(crate) fn open_map() -> Value {
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

pub(crate) fn order(units: &[u32], goal: [f64; 2]) -> CommandEnvelope {
    CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Move {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            gesture: 1,
            goal,
            route: RoutePolicy::Shortest,
            direction: MoveDirection::Forward,
            facing: None,
        },
        queued: false,
    }
}

#[test]
fn a_helicopter_enters_the_battle_at_cruise_height() {
    let fixture = fixture();
    let b = battle_with(
        &fixture,
        open_map(),
        json!([{ "side": "blue", "kind": "test_heli", "position": [300, 200] }]),
    );
    let heli = b.unit(UnitId(0)).unwrap();
    assert!(heli.airborne());
    assert!(heli.ground_footprint().is_none());
    let cruise = b.rules().air.cruise_agl_m;
    assert!(
        (heli.position.z - cruise).abs() < 1e-9,
        "z {}",
        heli.position.z
    );
}

#[test]
fn a_tank_drives_under_a_hovering_helicopter() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [150, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    assert_eq!(b.accept(order(&[0], [450.0, 200.0])).error, None);
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
    }
    let tank = b.unit(UnitId(0)).unwrap().position.xy();
    let short = (tank - sim::math::v2(450.0, 200.0)).length();
    assert!(
        short < 5.0,
        "the tank waited for the helicopter: {short:.1} m short"
    );
}

#[test]
fn soldiers_are_not_shoved_from_under_it() {
    let fixture = fixture();
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [300, 200] },
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
        ]),
    );
    let before: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
    }
    let after: Vec<_> = b.unit(UnitId(0)).unwrap().member_positions().collect();
    for (a, z) in before.iter().zip(&after) {
        assert!(
            (a.xy() - z.xy()).length() < 1e-9,
            "a soldier was shoved from {a:?} to {z:?}"
        );
    }
}

#[test]
fn a_helicopter_over_forest_is_not_concealed() {
    let fixture = fixture();
    let forest_map = json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "forests": [{ "shape": { "kind": "polygon",
            "ring": [[250, 150], [350, 150], [350, 250], [250, 250]] } }] });
    let b = battle_with(
        &fixture,
        forest_map,
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200] },
            { "side": "blue", "kind": "test_tank", "position": [300, 180] },
        ]),
    );
    let rules = b.rules();
    assert!(!sim::sensing::concealed(
        b.world(),
        b.unit(UnitId(0)).unwrap(),
        rules
    ));
    assert!(
        sim::sensing::concealed(b.world(), b.unit(UnitId(1)).unwrap(), rules),
        "the same forest hides a tank on its ground"
    );
}

#[test]
fn an_identified_enemy_helicopter_below_half_hp_publishes_smoking() {
    // Two blue helicopters in red's plain sight, one under half its health
    // and one just above, and a tank as damaged: red sees which helicopter
    // smokes (a coarse bit, never its health), and blue sees its own alike.
    let fixture = fixture();
    let hp = |kind: &str| {
        let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
        let kind = rules.catalog.index(kind).unwrap();
        rules.catalog.get(kind).hull().unwrap().hp
    };
    let (heli, tank) = (hp("test_heli"), hp("test_tank"));
    let mut b = battle_with(
        &fixture,
        open_map(),
        json!([
            { "side": "blue", "kind": "test_heli", "position": [300, 200], "engagement": "return_fire_only",
                "condition": { "hp": heli * 0.49 } },
            { "side": "blue", "kind": "test_heli", "position": [300, 230], "engagement": "return_fire_only",
                "condition": { "hp": heli * 0.51 } },
            { "side": "blue", "kind": "test_tank", "position": [300, 170], "engagement": "return_fire_only",
                "condition": { "hp": tank * 0.2 } },
            { "side": "red", "kind": "test_recon", "position": [330, 215], "engagement": "return_fire_only" },
        ]),
    );
    for _ in 0..30 {
        b.step();
    }
    let red = b.observe(Side::Red);
    let mut seen: Vec<(f64, bool)> = red
        .identified
        .iter()
        .map(|e| (e.position[1], e.smoking))
        .collect();
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(
        seen.iter().map(|s| s.1).collect::<Vec<_>>(),
        [false, true, false],
        "the tank at y 170, the damaged helicopter at 200, the other at 230: {seen:?}"
    );
    let own: Vec<bool> = b
        .observe(Side::Blue)
        .own
        .iter()
        .map(|u| u.smoking)
        .collect();
    assert_eq!(own, [true, false, false]);
}

#[test]
fn an_air_hull_past_its_limits_is_refused() {
    let mut fixture = fixture();
    let past = fixture["hull_limits"]["air"]["half_length_m"]
        .as_f64()
        .unwrap()
        + 0.5;
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "body": { "hull": { "half_extents_m": [past, 1.2, 1.6] } } }),
    );
    let error = serde_json::from_value::<Rules>(fixture).expect_err("past the air limit");
    assert!(error.to_string().contains("air half length"), "{error}");
}

#[test]
fn an_aircraft_cruising_past_its_limit_is_refused() {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "mobility": { "air": { "cruise_kmh": 400 } } }),
    );
    let error = serde_json::from_value::<Rules>(fixture).expect_err("past the air speed");
    assert!(error.to_string().contains("cruise_kmh"), "{error}");
}

#[test]
fn a_battle_with_a_helicopter_replays_to_the_same_digest() {
    let fixture = fixture();
    let run = || {
        let mut b = battle_with(
            &fixture,
            open_map(),
            json!([
                { "side": "blue", "kind": "test_heli", "position": [200, 200] },
                { "side": "red", "kind": "test_rifle", "position": [400, 200] },
            ]),
        );
        for _ in 0..20 * b.rules().tick_hz {
            b.step();
        }
        b.digest()
    };
    assert_eq!(run(), run());
}

/// A long strip: an observer at one end loses sight of what stands past its
/// sensor range at the other.
fn strip() -> Value {
    json!({ "size": [1600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
}

#[test]
fn a_lost_helicopter_leaves_an_airborne_contact() {
    // Red's squad sees the hovering helicopter 550 m off, then walks out of
    // its 600 m sight: the area it leaves hangs where the airframe was, in
    // the air, not on the ground below it.
    let fixture = fixture();
    let mut b = scripted(
        &fixture,
        strip(),
        json!([
            { "side": "red", "kind": "test_rifle", "position": [200, 200], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "test_heli", "position": [750, 200], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([{ "tick": 20, "side": "red", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [20, 200], "route": "shortest" } }]),
    );
    let heli = b.unit(UnitId(1)).unwrap().position;
    let mut seen = false;
    for _ in 0..240 * b.rules().tick_hz {
        b.step();
        let red = b.observe(Side::Red);
        seen |= !red.identified.is_empty();
        if let Some(c) = red.contacts.first() {
            assert!(seen, "the helicopter was identified before it was lost");
            assert_eq!(c.source, ContactSource::LastSeen);
            assert_eq!(c.layer, AltitudeLayer::LowAir);
            assert_eq!(
                c.center,
                [heli.x, heli.y, heli.z],
                "the area is where it hovered"
            );
            return;
        }
    }
    panic!("the squad never lost the helicopter (seen: {seen})");
}

/// Blue's squad and, 800 m off and out of its sight, red's helicopter,
/// which fires once at tick 5.
fn heard_helicopter(fixture: &Value) -> Battle {
    let mut b = scripted(
        fixture,
        strip(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 200] },
            { "side": "red", "kind": "test_heli", "position": [900, 200], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 5, "fire": { "unit": 1 } }]),
        json!([]),
    );
    for _ in 0..5 {
        b.step();
    }
    b
}

#[test]
fn a_heard_helicopter_report_is_airborne() {
    // A shot from an unseen helicopter is heard overhead: the report's area
    // is at the airframe's height, in the air band.
    let fixture = fixture();
    let b = heard_helicopter(&fixture);
    let heli = b.unit(UnitId(1)).unwrap().position;
    let blue = b.observe(Side::Blue);
    assert!(blue.identified.is_empty(), "the helicopter is out of sight");
    let [c] = blue.contacts.as_slice() else {
        panic!("one report: {:?}", blue.contacts);
    };
    assert_eq!(c.source, ContactSource::Firing);
    assert_eq!(c.layer, AltitudeLayer::LowAir);
    assert_eq!(c.center[2], heli.z, "heard at the airframe's height");
    let off = (c.center[0] - heli.x).hypot(c.center[1] - heli.y);
    assert!(
        off <= c.radius,
        "the airframe lies inside its area: {off:.1} m"
    );
}

/// Blue's tank and, 720 m off and beyond each other's sight, a red `kind`
/// that fires every second: blue holds only its firing area.
fn hidden_fire_at_a_tank(fixture: &Value, kind: &str) -> Battle {
    let fire: Vec<Value> = (0..20)
        .map(|k| json!({ "tick": 5 + k * 30, "fire": { "unit": 1 } }))
        .collect();
    scripted(
        fixture,
        strip(),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 200] },
            { "side": "red", "kind": kind, "position": [820, 200], "engagement": "return_fire_only" },
        ]),
        json!(fire),
        json!([]),
    )
}

/// Rounds blue's tank launches over `ticks`.
fn blue_rounds(b: &mut Battle, ticks: u64) -> usize {
    let mut fired = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        b.step();
        fired.extend(
            b.rounds()
                .filter(|(_, r)| r.unit == UnitId(0))
                .map(|(p, _)| p.id),
        );
    }
    fired.len()
}

#[test]
fn area_fire_refuses_an_air_contact() {
    // A tank crew hears a helicopter's gun but can't see it: shelling the
    // field under where the sound came from would be silly, so neither the
    // player's attack order nor the crew's own fire aims at the report. A
    // squad's report from the same place draws HE as before.
    let fixture = fixture();
    let mut b = hidden_fire_at_a_tank(&fixture, "test_heli");
    let mut ground = hidden_fire_at_a_tank(&fixture, "test_rifle");
    let (air_rounds, ground_rounds) = (blue_rounds(&mut b, 600), blue_rounds(&mut ground, 600));
    assert!(b.observe(Side::Blue).identified.is_empty(), "never seen");
    let [c] = b.observe(Side::Blue).contacts.as_slice() else {
        panic!("one report: {:?}", b.observe(Side::Blue).contacts);
    };
    assert_eq!(c.layer, AltitudeLayer::LowAir);
    assert_eq!(air_rounds, 0, "the crew never shells an air contact");
    assert!(ground_rounds > 0, "a ground report still draws area fire");
    let attack = |id| CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Contact { id },
        },
        queued: false,
    };
    assert_eq!(
        b.accept(attack(c.id)).error,
        Some(OrderError::AirContact),
        "an attack order on an air contact is refused"
    );
    let area = ground.observe(Side::Blue).contacts[0].id;
    assert_eq!(ground.accept(attack(area)).error, None);
}
