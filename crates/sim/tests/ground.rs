//! The ground layer: craters give infantry light cover and slow vehicles
//! slightly; scorch, track wear and trampling are recorded only.
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

fn rules() -> Rules {
    serde_json::from_value(common::game()).unwrap()
}

fn num(section: &str, key: &str) -> f64 {
    common::game()[section][key].as_f64().unwrap()
}

/// A flat 600 × 400 field with a forest block in its north-east corner.
fn field() -> String {
    json!({
        "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [],
        "forests": [{ "shape":{"kind":"polygon","ring":[[450.0,250.0],[600.0,250.0],[600.0,400.0],[450.0,400.0]]}}]
    })
    .to_string()
}

/// HE bursts (the lab emitter) on every point of a grid, from tick 1.
fn barrage(x0: f64, x1: f64, y0: f64, y1: f64, step: f64) -> Vec<Value> {
    let mut events = Vec::new();
    let mut x = x0;
    while x <= x1 {
        let mut y = y0;
        while y <= y1 {
            events.push(json!({ "tick": 1, "burst": { "point": [x, y], "weapon": "tank_he" } }));
            y += step;
        }
        x += step;
    }
    events
}

fn battle(units: Value, events: Vec<Value>, scripts: Value, seed: u64) -> Battle {
    Battle::new(
        &common::scenario_with(&field(), units, Value::Array(events), scripts),
        seed,
    )
}

fn drive(unit: u32, goal: [f64; 2]) -> Value {
    json!({ "tick": 1, "side": "blue",
        "order": { "kind": "move", "units": [unit], "gesture": unit + 1, "goal": goal, "route": "shortest" } })
}

#[test]
fn a_crater_is_light_ground_cover_from_every_side_once_it_is_deep_enough() {
    use contract::scenario::CoverTier;
    // One burst digs a crater; a second, 1.5 m off, overlaps its rim.
    let mut b = battle(
        json!([]),
        barrage(100.0, 100.0, 100.0, 100.0, 1.0),
        json!([]),
        1,
    );
    b.step();
    let r = rules();
    let at = |x: f64, y: f64, from: [f64; 2]| {
        let (p, from) = (sim::math::v2(x, y), sim::math::v2(from[0], from[1]));
        sim::cover::at(b.world(), b.ground(), &[], &r, p, from)
    };
    assert!(
        b.ground().crater_fill(100.0, 100.0, &r.ground) >= r.cover.crater_min_fill,
        "the burst dug a full crater"
    );
    for from in [[300.0, 100.0], [0.0, 100.0], [100.0, 300.0]] {
        assert_eq!(
            at(100.0, 100.0, from),
            Some(CoverTier::Light),
            "from {from:?}"
        );
    }
    assert_eq!(
        at(200.0, 100.0, [300.0, 100.0]),
        None,
        "open ground is no cover"
    );
    // The rim, where the crater is shallow, is not yet cover.
    let rim = (1..40)
        .map(|k| 100.0 + k as f64 * 0.1)
        .find(|&x| {
            let fill = b.ground().crater_fill(x, 100.0, &r.ground);
            fill > 0.0 && fill < r.cover.crater_min_fill
        })
        .expect("a shallow rim");
    assert_eq!(at(rim, 100.0, [300.0, 100.0]), None);
}

#[test]
fn craters_are_never_impassable() {
    // A tank drives straight through a field shelled until every crater is full.
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(barrage(150.0, 250.0, 80.0, 120.0, 2.0));
    }
    let mut b = battle(
        json!([{ "side": "blue", "kind": "tank", "position": [100, 100] }]),
        events,
        json!([drive(0, [300.0, 100.0])]),
        1,
    );
    let full = num("ground", "crater_full_depth");
    b.step();
    assert!(
        (b.ground().cell(200.0, 100.0).crater as f64) >= full,
        "the field is saturated"
    );
    for _ in 0..60 * 30 {
        b.step();
    }
    let tank = b.unit(UnitId(0)).unwrap();
    assert!(
        (tank.position.xy() - sim::math::v2(300.0, 100.0)).length() < 1.0,
        "the tank crossed: {:?}",
        tank.position
    );
}

/// Ticks until unit `id` reaches `x`, and the route it planned first.
fn crossing(b: &mut Battle, id: u32, x: f64) -> (u32, Vec<sim::math::V2>) {
    b.step();
    let route = b.unit(UnitId(id)).unwrap().route.clone().unwrap();
    for t in 1..60 * 30 {
        if b.unit(UnitId(id)).unwrap().position.x >= x {
            return (t, route);
        }
        b.step();
    }
    panic!("unit {id} never reached x = {x}");
}

#[test]
fn craters_slow_a_moving_vehicle_slightly_and_never_its_plan() {
    let tank = json!([{ "side": "blue", "kind": "tank", "position": [100, 100] }]);
    let order = json!([drive(0, [300.0, 100.0])]);
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(barrage(150.0, 250.0, 90.0, 110.0, 2.0));
    }
    let (clean, clean_route) = crossing(
        &mut battle(tank.clone(), vec![], order.clone(), 1),
        0,
        280.0,
    );
    let (slowed, slowed_route) = crossing(&mut battle(tank, events, order, 1), 0, 280.0);
    assert_eq!(clean_route, slowed_route, "navigation never reads craters");
    assert!(
        slowed > clean,
        "craters slow the crossing: {slowed} vs {clean}"
    );
    // Slightly: never slower than the full-crater multiplier over the field.
    let mult = num("ground", "crater_vehicle_mult");
    assert!(
        (slowed as f64) <= clean as f64 / mult + 2.0,
        "{slowed} vs {clean} at ×{mult}"
    );
}

#[test]
fn craters_never_slow_a_vehicle_turning_in_place_or_a_squad() {
    // A tank turns in place to face west and a squad walks east, on craters and off.
    let units = json!([
        { "side": "blue", "kind": "tank", "position": [200, 100] },
        { "side": "blue", "kind": "rifle", "position": [150, 140] },
    ]);
    let orders = json!([drive(0, [150.0, 100.0]), drive(1, [260.0, 140.0])]);
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(barrage(140.0, 270.0, 90.0, 150.0, 2.0));
    }
    let mut clean = battle(units.clone(), vec![], orders.clone(), 1);
    let mut cratered = battle(units, events, orders, 1);
    let start = clean.unit(UnitId(0)).unwrap().position;
    let mut turned = 0;
    for _ in 0..40 * 30 {
        clean.step();
        cratered.step();
        let (a, b) = (
            clean.unit(UnitId(1)).unwrap(),
            cratered.unit(UnitId(1)).unwrap(),
        );
        assert_eq!(a.position, b.position, "the squad walks alike");
        let (a, b) = (
            clean.unit(UnitId(0)).unwrap(),
            cratered.unit(UnitId(0)).unwrap(),
        );
        if a.position == start {
            // Turning in place: nothing to slow.
            assert_eq!(b.position, start, "both start driving on the same tick");
            assert_eq!(a.yaw, b.yaw, "the turn is not slowed");
            turned += 1;
        }
    }
    assert!(
        turned > 30,
        "the tank turned in place first ({turned} ticks)"
    );
}

#[test]
fn craters_appearing_never_bump_a_side_navigation_revision_or_replan() {
    // Shells land along the route ahead of a driving tank, then behind it.
    let tank = json!([{ "side": "blue", "kind": "tank", "position": [100, 100] }]);
    let order = json!([drive(0, [400.0, 100.0])]);
    let mut events = Vec::new();
    for (k, x) in (150..380).step_by(10).enumerate() {
        events.push(
            json!({ "tick": 1 + 20 * k, "burst": { "point": [x, 100], "weapon": "tank_he" } }),
        );
        events.push(
            json!({ "tick": 1 + 20 * k, "burst": { "point": [x, 104], "weapon": "grenade" } }),
        );
    }
    let mut clean = battle(tank.clone(), vec![], order.clone(), 1);
    let mut shelled = battle(tank, events, order, 1);
    for _ in 0..90 * 30 {
        clean.step();
        shelled.step();
    }
    assert!(
        shelled.ground().cell(300.0, 100.0).crater > 0,
        "craters appeared"
    );
    for side in Side::ALL {
        assert_eq!(
            shelled.navigation_revision(side),
            clean.navigation_revision(side)
        );
        assert_eq!(shelled.route_searches(side), clean.route_searches(side));
    }
}

#[test]
fn the_cosmetic_channels_leave_combat_identical() {
    // HE on a squad in the open, a squad walking and a tank driving: once with
    // scorch, tracks and trampling recorded and once with them switched off.
    let units = json!([
        { "side": "blue", "kind": "tank", "position": [100, 200] },
        { "side": "blue", "kind": "rifle", "position": [100, 150] },
        { "side": "red", "kind": "rifle", "position": [300, 200], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
        { "side": "blue", "kind": "supply", "position": [100, 300] },
    ]);
    let scripts = json!([
        { "tick": 1, "side": "blue", "order": { "kind": "attack", "units": [0],
            "target": { "kind": "ground", "point": [300, 200, 0] } } },
        drive(1, [220.0, 150.0]),
        drive(3, [250.0, 300.0]),
    ]);
    let setup = |cosmetic: bool| {
        let mut s = common::scenario_with(&field(), units.clone(), json!([]), scripts.clone());
        if !cosmetic {
            s.rules.ground.scorch_per_burst = 0.0;
            s.rules.ground.tracks_per_pass = 0.0;
            s.rules.ground.trampled_per_pass = 0.0;
        }
        Battle::new(&s, 7)
    };
    let (mut marked, mut bare) = (setup(true), setup(false));
    for _ in 0..40 * 30 {
        marked.step();
        bare.step();
        for side in Side::ALL {
            assert_eq!(
                serde_json::to_string(marked.observe(side)).unwrap(),
                serde_json::to_string(bare.observe(side)).unwrap(),
                "tick {}",
                marked.tick()
            );
        }
    }
    let (m, b) = (marked.ground(), bare.ground());
    assert!(
        m.cells().any(|(_, _, c)| c.crater > 0),
        "the HE left craters"
    );
    assert!(
        m.cells().all(|(x, y, c)| b.cell(x, y).crater == c.crater),
        "craters are the same in both"
    );
    assert!(m.cells().any(|(_, _, c)| c.scorch > 0) && m.cells().any(|(_, _, c)| c.trampled > 0));
    assert!(m.cells().any(|(_, _, c)| c.tracks > 0));
    assert!(b
        .cells()
        .all(|(_, _, c)| c.scorch == 0 && c.tracks == 0 && c.trampled == 0));
    assert_ne!(
        marked.digest(),
        bare.digest(),
        "the cosmetic channels are state"
    );
}

#[test]
fn channels_saturate_and_storage_stays_within_the_map_bound() {
    // Forty rounds on one spot: the crater saturates instead of wrapping, and
    // the layer stores no more after the first burst than after the last.
    let one = barrage(300.0, 300.0, 200.0, 200.0, 1.0);
    let mut first = battle(json!([]), one.clone(), json!([]), 1);
    first.step();
    let mut events = Vec::new();
    for _ in 0..40 {
        events.extend(one.clone());
    }
    let mut many = battle(json!([]), events, json!([]), 1);
    many.step();
    assert_eq!(many.ground().cell(300.0, 200.0).crater, u8::MAX);
    assert_eq!(many.ground().bytes(), first.ground().bytes());
    assert!(many.ground().bytes() <= many.ground().bound_bytes());
}

#[test]
fn the_endurance_battle_keeps_the_layer_within_its_bound() {
    let s = sim::endurance::scenario(
        &sim::maps::load("endurance").unwrap().definition,
        &common::game(),
        1,
        false,
    )
    .unwrap();
    let mut b = Battle::new(&s, 1);
    let bound = b.ground().bound_bytes();
    let cell = s.rules.ground.cell_m;
    let area_cells = (s.map.size[0] / cell).ceil() * (s.map.size[1] / cell).ceil();
    // The bound is the map area in bytes per cell (five channels), plus the
    // tile index.
    assert!(
        (bound as f64) < area_cells * 5.0 * 1.1,
        "{bound} for {area_cells} cells"
    );
    for _ in 0..common::tick_hz() * 20 {
        b.step();
    }
    assert!(b.ground().bytes() > 0, "the battle marked the ground");
    assert!(b.ground().bytes() <= bound);
    assert_eq!(b.load().ground_bytes, b.ground().bytes());
}

#[test]
fn craters_enter_the_digest_and_replay_to_it() {
    let units = json!([{ "side": "blue", "kind": "tank", "position": [100, 100] }]);
    let quiet = battle(units.clone(), vec![], json!([]), 1);
    let mut shelled = battle(
        units.clone(),
        barrage(200.0, 200.0, 200.0, 200.0, 1.0),
        json!([]),
        1,
    );
    let mut q = quiet;
    q.step();
    shelled.step();
    assert_ne!(q.digest(), shelled.digest(), "a crater is state");
    let mut digests = vec![shelled.digest()];
    for _ in 0..60 {
        shelled.step();
        digests.push(shelled.digest());
    }
    let setup = common::scenario_with(
        &field(),
        units,
        Value::Array(barrage(200.0, 200.0, 200.0, 200.0, 1.0)),
        json!([]),
    );
    let mut replay = Battle::from_replay(&setup, &shelled.replay()).unwrap();
    for d in digests {
        replay.step();
        assert_eq!(replay.digest(), d);
    }
}
