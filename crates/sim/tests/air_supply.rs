//! The resupply sink (D10, D27): a helicopter idle at a deployed supply truck
//! sinks to a low hover 2 m over the tallest ground hull in the catalog and is
//! served; any order or engagement lifts it back to cruise. Sinking is not
//! moving, so it never talks itself out of service.

use super::air::{battle_with, fixture, open_map, order};
use contract::catalog::AltitudeLayer;
use contract::command::{CommandEnvelope, Engagement, Order};
use contract::ids::{Side, UnitId};
use contract::observation::ServiceStatus;
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

const HELI: UnitId = UnitId(1);

/// The test catalog with its tank grown to an 8 m hull top, so the tallest
/// ground hull is one this test chose.
fn tall_fixture() -> Value {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_tank",
        json!({ "body": { "hull": { "half_extents_m": [3.5, 1.8, 4.0] } } }),
    );
    fixture
}

/// The top of the tallest ground hull in `rules`' catalog.
fn tallest_ground_top(rules: &Rules) -> f64 {
    let catalog = &rules.catalog;
    catalog
        .indices()
        .map(|i| catalog.get(i))
        .filter(|t| t.mobility.layer() == AltitudeLayer::Ground)
        .filter_map(|t| t.hull())
        .map(|h| 2.0 * h.half_extents_m[2])
        .fold(f64::NEG_INFINITY, f64::max)
}

/// A supply truck (unit 0) and a damaged helicopter over it (unit 1), plus
/// `others`; the helicopter holds fire unless told otherwise.
fn at_the_truck(others: &[Value]) -> Battle {
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_supply", "position": [100, 200] }),
        json!({ "side": "blue", "kind": "test_heli", "position": [110, 200],
            "engagement": "return_fire_only", "condition": { "hp": 100 } }),
    ];
    units.extend_from_slice(others);
    battle_with(&tall_fixture(), open_map(), Value::Array(units))
}

fn heli_z(b: &Battle) -> f64 {
    b.unit(HELI).unwrap().position.z
}

/// Step until the helicopter has sunk to `low` (within 1 cm), at most a
/// minute.
fn sink(b: &mut Battle, low: f64) {
    for _ in 0..60 * b.rules().tick_hz {
        if (heli_z(b) - low).abs() < 0.01 {
            return;
        }
        b.step();
    }
    panic!("never sank to {low:.2} m: at {:.2} m", heli_z(b));
}

fn low_hover(b: &Battle) -> f64 {
    tallest_ground_top(b.rules()) + 2.0
}

#[test]
fn a_helicopter_sinks_at_a_deployed_truck_and_is_served() {
    let mut b = at_the_truck(&[]);
    let low = low_hover(&b);
    assert!(low >= 10.0, "the grown tank sets the hover: {low}");
    sink(&mut b, low);
    // It holds there, and the truck repairs it.
    let hp = b.unit(HELI).unwrap().hp;
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        assert!((heli_z(&b) - low).abs() < 0.01, "left the low hover");
    }
    let heli = b.unit(HELI).unwrap();
    assert!(heli.hp > hp, "not served: hp {hp} -> {}", heli.hp);
    assert!(
        (heli.position.xy() - sim::math::v2(110.0, 200.0)).length() < 1e-9,
        "drifted while sinking"
    );
}

#[test]
fn it_climbs_on_an_order() {
    let mut b = at_the_truck(&[]);
    let low = low_hover(&b);
    sink(&mut b, low);
    assert_eq!(b.accept(order(&[1], [400.0, 200.0])).error, None);
    let cruise = b.rules().air.cruise_agl_m;
    let mut highest = low;
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        highest = highest.max(heli_z(&b));
    }
    assert!(
        (highest - cruise).abs() < 0.01,
        "climbed only to {highest:.2} m"
    );
}

#[test]
fn it_climbs_on_engagement() {
    // A held-fire enemy squad 250 m off, inside the door gun's reach.
    let rifle = json!({ "side": "red", "kind": "test_rifle", "position": [360, 200],
        "engagement": "return_fire_only" });
    let mut b = at_the_truck(&[rifle]);
    let low = low_hover(&b);
    sink(&mut b, low);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::SetEngagement {
            units: vec![HELI],
            policy: Engagement::FireAtWill,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let cruise = b.rules().air.cruise_agl_m;
    let mut engaged = false;
    let mut highest = low;
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        engaged |= b.unit(HELI).unwrap().service == ServiceStatus::Firing;
        highest = highest.max(heli_z(&b));
    }
    assert!(engaged, "it never engaged the squad");
    assert!(
        highest > low + 0.5 * (cruise - low),
        "it stayed low while fighting: {highest:.2} m"
    );
}

#[test]
fn sinking_never_flips_service_to_moving() {
    let mut b = at_the_truck(&[]);
    let low = low_hover(&b);
    let cruise = b.rules().air.cruise_agl_m;
    let mut sank = false;
    for _ in 0..40 * b.rules().tick_hz {
        let before = heli_z(&b);
        b.step();
        let status = b.unit(HELI).unwrap().service;
        assert_ne!(status, ServiceStatus::Moving, "at {:.2} m", heli_z(&b));
        sank |= heli_z(&b) < before;
    }
    assert!(sank, "it never sank");
    assert!(
        (heli_z(&b) - low).abs() < 0.01,
        "it climbed back toward {cruise}"
    );
}
