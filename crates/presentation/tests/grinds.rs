//! A hull throws sparks where it grinds a metal body it shoves, and none off
//! one that is not metal. Each battle drives a test tank on a slant through
//! the flank of one body in a row of five, to a goal past it, and its side's
//! publications are read and handed to the effects as a page does.
use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use presentation::effects::publication::{EffectFeed, Grind};
use presentation::fielded;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::reader::Reader;
use sim::publication::{self, Publisher};
use std::path::PathBuf;

const KIND: &str = "test_tank";
/// The body in the row the tank drives through.
const SHOVED: usize = 2;

/// A row of `body` across the tank's way.
fn row(body: &str) -> ScenarioDefinition {
    let bodies: Vec<Value> = [15.0, 20.0, 25.0, 30.0, 35.0]
        .map(|x| json!({ "kind": body, "center": [x, 30.0], "yaw": 0, "half_extents": [2.1, 0.9, 0.75] }))
        .to_vec();
    serde_json::from_value(json!({
        "map": { "size": [60, 60], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": bodies },
        "rules": sim::fixtures::test_rules(),
        "units": [{ "side": "blue", "kind": KIND, "position": [22.0, 5.0], "engagement": "return_fire_only" }],
        "events": [],
        "scripts": [{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [27.0, 38.0], "route": "shortest" } }],
    }))
    .unwrap()
}

/// How far the tank shoves the body, and the grinds blue's effects take
/// from its publications, with the hull's distance from each.
fn shove_through(body: &str) -> (f64, Vec<(Grind, f64)>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let f = fielded::fielded(&root, &row(body).rules, &[KIND.to_owned()]).unwrap();
    let mut battle = Battle::new(&row(body), 1);
    let parked = battle.world().prop(SHOVED as u32).unwrap().center;
    let mut reader = Reader::new(&publication::layout_json(&battle)).unwrap();
    let mut publisher = Publisher::new();
    let mut feed = EffectFeed::new("blue");
    let mut grinds = Vec::new();
    for _ in 0..30 * battle.rules().tick_hz as u64 {
        battle.step();
        let words: Vec<u32> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        let o = reader.decode(&words).unwrap().unwrap();
        let hull = battle
            .unit(contract::ids::UnitId(0))
            .unwrap()
            .ground_footprint()
            .unwrap();
        for g in feed.publication(&o, &f.units, &f.effects, None).grinds {
            let off = hull.distance(sim::math::v2(g.point[0], g.point[1]));
            grinds.push((g, off));
        }
    }
    let moved = (battle.world().prop(SHOVED as u32).unwrap().center - parked).length();
    (moved, grinds)
}

#[test]
fn a_hull_grinding_a_metal_body_throws_sparks_where_they_meet() {
    let (moved, grinds) = shove_through("parked_car");
    assert!(moved > 1.0, "the tank shoves the car {moved:.1} m");
    assert!(
        grinds.len() > 30,
        "it grinds the car for {} ticks",
        grinds.len()
    );
    for (g, off) in &grinds {
        assert!(
            *off < 0.25,
            "sparks {off:.2} m off the hull at {:?}",
            g.point
        );
    }
}

#[test]
fn a_hull_shoving_a_body_that_is_not_metal_throws_no_sparks() {
    let (moved, grinds) = shove_through("sandbags");
    assert!(moved > 1.0, "the tank shoves the sandbags {moved:.1} m");
    assert!(grinds.is_empty(), "{} grinds off sandbags", grinds.len());
}
