//! Forests as bodies, with densities (slice 34b, Q14, Q16, Q21). A forest
//! rect and its density only generate trunks; at runtime a forest is those
//! bodies plus the ground a heavy vehicle has cleared, and every forest
//! query (speed, concealment, sight, cover, fog) reads both.
use crate::common;

use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::scenario::{ForestDensity, PushClass};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, v3, Obb2, V2};
use sim::visibility::{self, OcclusionGrid};
use sim::world::WorldGeometry;

fn density(name: &str) -> ForestDensity {
    common::forest_rules().densities[name]
}

fn forest(rect: [f64; 4], density: &str) -> Value {
    json!({ "rect": rect, "density": density, "canopy_height_m": 12, "trunk_radius_m": 0.35,
            "trunk_height_m": 10, "trunk_clearance_m": 2 })
}

fn forests(list: Value) -> WorldGeometry {
    common::flat([300.0, 120.0], &format!(r#","forests":{list}"#))
}

fn trunks_in(w: &WorldGeometry, x0: f64, x1: f64) -> Vec<V2> {
    w.props()
        .filter(|p| p.kind == common::kind("trunk") && p.center.x >= x0 && p.center.x < x1)
        .map(|p| p.center)
        .collect()
}

/// Q16: a density's spacing sets how many trunks stand and how close.
#[test]
fn density_sets_trunk_spacing() {
    let w = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0], "light"),
        forest([150.0, 0.0, 120.0, 120.0], "dense")
    ]));
    for (name, x0, x1) in [("light", 0.0, 120.0), ("dense", 150.0, 270.0)] {
        let d = density(name);
        let trunks = trunks_in(&w, x0, x1);
        let per_axis = ((120.0 - d.trunk_spacing_m / 2.0) / d.trunk_spacing_m).floor() + 1.0;
        let grid = per_axis * per_axis;
        // A jittered trunk never leaves its forest, so a few edge ones go.
        assert!(
            trunks.len() as f64 <= grid && trunks.len() as f64 >= 0.85 * grid,
            "{name}: {} trunks on a {grid} grid",
            trunks.len()
        );
        let closest = trunks
            .iter()
            .enumerate()
            .flat_map(|(i, a)| trunks[i + 1..].iter().map(move |b| (*a - *b).length()))
            .fold(f64::INFINITY, f64::min);
        let floor = d.trunk_spacing_m * (1.0 - 2.0 * d.trunk_jitter);
        assert!(closest >= floor - 1e-9, "{name}: {closest:.2} m apart");
    }
    // The same forest places the same trunks.
    let again = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0], "light"),
        forest([150.0, 0.0, 120.0, 120.0], "dense")
    ]));
    assert_eq!(trunks_in(&w, 0.0, 300.0), trunks_in(&again, 0.0, 300.0));
}

/// Q16, Q21: light forest hides infantry far less than dense, vehicles
/// likewise, and open ground not at all.
#[test]
fn denser_forest_conceals_more() {
    let w = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0], "light"),
        forest([150.0, 0.0, 120.0, 120.0], "dense")
    ]));
    // Averaged over the forests' interiors: a single point may sit in a gap.
    let mean = |x0: f64, infantry: bool| {
        let mut sum = 0.0;
        let mut n = 0.0;
        for i in 0..20 {
            for j in 0..20 {
                let p = v2(x0 + 20.0 + 4.0 * i as f64, 20.0 + 4.0 * j as f64);
                sum += w.foliage_at(p.x, p.y).concealment(infantry);
                n += 1.0;
            }
        }
        sum / n
    };
    for infantry in [true, false] {
        let (light, dense) = (mean(0.0, infantry), mean(150.0, infantry));
        assert!(
            dense < light && light < 1.0,
            "{infantry}: {light} vs {dense}"
        );
    }
    // Infantry hides more than a vehicle in the same foliage (the class rule).
    assert!(mean(150.0, true) < mean(150.0, false));
    assert_eq!(w.foliage_at(135.0, 60.0).concealment(true), 1.0);
    assert!(!w.surface_at(135.0, 60.0).unwrap().forest);
    assert!(w.surface_at(210.0, 60.0).unwrap().forest);
}

/// Q16: a lane knocked through the trees is open ground to speed, sensing,
/// cover and fog, while the forest beside it is still forest.
#[test]
fn a_cleared_lane_reads_as_open_ground() {
    let mut w = forests(json!([forest([40.0, 0.0, 200.0, 120.0], "medium")]));
    let lane = Obb2 {
        center: v2(140.0, 60.0),
        yaw: 0.0,
        half: v2(110.0, 2.5),
    };
    let fell: Vec<_> = w
        .props()
        .filter(|p| p.kind == common::kind("trunk") && lane.contains(p.center, 0.4))
        .map(|p| p.id)
        .collect();
    assert!(!fell.is_empty());
    let before = w.obstacle_revision();
    for id in fell {
        assert!(w.knock_down(id).is_some());
    }
    assert!(
        w.obstacle_revision() > before,
        "a knocked tree bumps the revision"
    );
    let nowhere = Obb2 {
        center: v2(-50.0, -50.0),
        yaw: 0.0,
        half: v2(1.0, 1.0),
    };
    assert!(!w.clear(&lane, &nowhere).is_empty());
    // Beside the lane, in forest: the probe row 20 m north.
    let (on, off) = (v2(140.0, 60.0), v2(140.0, 80.0));
    // Speed: forest ground is gone from the lane only.
    assert!(!w.surface_at(on.x, on.y).unwrap().forest);
    assert!(w.surface_at(off.x, off.y).unwrap().forest);
    // Sensing: no concealment, and no foliage along the lane.
    assert_eq!(
        sim::sensing::concealment_multiplier(true, &w, on.with_z(0.0)),
        1.0
    );
    assert!(sim::sensing::concealment_multiplier(true, &w, off.with_z(0.0)) < 1.0);
    let along = |y: f64| w.foliage_depth(v3(35.0, y, 1.5), v3(230.0, y, 1.5));
    assert_eq!(along(60.0), 0.0);
    assert!(along(80.0) > 0.5);
    // Cover: no body stands in the lane to give any.
    let threat = v2(35.0, 60.0);
    let hulls = Vec::new();
    let rules: contract::scenario::Rules =
        serde_json::from_value(common::scenario_rules()).unwrap();
    let ground = sim::ground::GroundLayer::new(w.width(), w.depth(), &rules.ground);
    assert!(sim::cover::at(&w, &ground, &hulls, &rules, on, threat).is_none());
    // Fog: an eye at the lane's mouth sees down it, not through the trees.
    let grid = OcclusionGrid::new(&w, rules.sensors.fog_cell_m);
    let mut field = grid.field();
    let sight = sim::sight::Sight {
        forward: 0.0,
        shape: contract::scenario::SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 400.0,
    };
    visibility::sweep(
        &w,
        &grid,
        &rules.sensors,
        v3(30.0, 60.0, 1.8),
        &sight,
        &mut field,
    );
    assert!(field.visible(228.0, 60.0));
    let mut beside = grid.field();
    visibility::sweep(
        &w,
        &grid,
        &rules.sensors,
        v3(30.0, 80.0, 1.8),
        &sight,
        &mut beside,
    );
    assert!(!beside.visible(228.0, 80.0));
}

/// Q16, Q27: a trunk is a solid body, so it blocks infantry (soldiers walk
/// between trunks) and every vehicle, and only a heavy push class knocks it
/// down.
#[test]
fn trunks_block_every_mover_and_only_heavy_push_knocks_them() {
    let w = forests(json!([forest([0.0, 0.0, 60.0, 60.0], "medium")]));
    let trunk = w.props().find(|p| p.kind == common::kind("trunk")).unwrap();
    assert!(trunk.body.topples);
    assert!(trunk.blocks(MoverClass::Infantry));
    assert!(trunk.blocks(MoverClass::Vehicle));
    let weight = trunk.body.weight_class;
    assert!(PushClass::Heavy.pushes(weight));
    assert!(!PushClass::Medium.pushes(weight) && !PushClass::Light.pushes(weight));
}

/// Q16: whether a vehicle knocks its way through a forest is the forest's
/// tree against its push class, not any other toppling body the catalog
/// holds: with a light sapling type in the catalog, a supply truck (which
/// can shove a sapling, not a tree) grazing a forest's edge clears nothing.
#[test]
fn only_the_forests_own_tree_decides_who_clears_a_lane() {
    let mut rules = common::scenario_rules();
    let sapling = json!({ "props": { "sapling": {
        "body": { "blocks": { "infantry": false, "vehicle": true }, "stops_rounds": false,
                  "occludes": false, "weight_class": "light", "cover_tier": "light",
                  "topples": true, "hp": 10 },
        "destroyed": "cleared",
        "appearance": { "drawn_by": "forest" } } } });
    rules["catalog"].as_array_mut().unwrap().push(sapling);
    // Trees stand clear of the forest's edge: the truck's hull overlaps the
    // treeless band inside it by a metre.
    let map = json!({ "size": [300, 80], "height_grid_m": 4, "slope_cutoff_deg": 35,
                      "forests": [forest([70.0, 0.0, 60.0, 40.0], "medium")] });
    let setup = serde_json::from_value(json!({
        "map": map, "rules": rules, "events": [],
        "units": [{ "side": "blue", "kind": "supply", "position": [40, 40.4] }],
        "scripts": [{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [170, 40.4], "route": "shortest" } }],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    run(&mut b, 30.0);
    let truck = b.unit(UnitId(0)).unwrap().position.xy();
    assert!(truck.x > 140.0, "the truck drove past the forest: {truck:?}");
    assert_eq!(b.world().cleared_cells(), 0, "it knocked nothing down");
}

/// A tank through medium forest, and a red squad down its lane that did
/// not see the trees fall.
fn carve(watcher: [f64; 2]) -> contract::scenario::ScenarioDefinition {
    let map = json!({ "size": [1000, 80], "height_grid_m": 4, "slope_cutoff_deg": 35,
                      "forests": [forest([70.0, 0.0, 60.0, 80.0], "medium")] });
    common::scenario_with(
        &map.to_string(),
        json!([
            { "side": "blue", "kind": "tank", "position": [15, 40], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": watcher, "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [185, 40], "route": "shortest" } }]),
    )
}

fn run(b: &mut Battle, seconds: f64) {
    for _ in 0..(seconds * b.rules().tick_hz as f64).round() as u64 {
        b.step();
    }
}

/// Q16: a tank knocks trees down and clears the lane it drives, the digest
/// covers both, and a replay reproduces them exactly.
#[test]
fn a_carved_lane_replays_and_is_in_the_digest() {
    let setup = carve([250.0, 70.0]);
    let mut b = Battle::new(&setup, 1);
    let start = b
        .world()
        .props()
        .filter(|p| p.kind == common::kind("trunk"))
        .count();
    let digest0 = b.digest();
    run(&mut b, 60.0);
    let now = b
        .world()
        .props()
        .filter(|p| p.kind == common::kind("trunk"))
        .count();
    assert!(now < start, "the tank knocked trees down");
    assert!(b.world().cleared_cells() > 0, "and cleared its lane");
    assert!(b.world().cleared(100.0, 40.0));
    assert!(
        b.ground().cell(100.0, 40.0).cleared > 0,
        "crushed-ground marks"
    );
    let tank = b.unit(UnitId(0)).unwrap().position.xy();
    assert!(
        (tank - v2(185.0, 40.0)).length() < 3.0,
        "the tank got through: {tank:?}"
    );
    let mut again = Battle::from_replay(&setup, &b.replay()).unwrap();
    run(&mut again, 60.0);
    assert_eq!(again.digest(), b.digest());
    assert_ne!(digest0, b.digest());
}

/// L1 for trees: the pusher's side plans without a tree it knocked down at
/// once; a side that never saw it fall still plans around it, and one that
/// watched the lane open learns it.
#[test]
fn a_side_that_did_not_see_a_tree_fall_keeps_it_standing() {
    let watched = |at: [f64; 2]| {
        let mut b = Battle::new(&carve(at), 1);
        run(&mut b, 60.0);
        b
    };
    // Beyond every red eye's reach, then looking down the lane from its end.
    let (hidden, seen) = (watched([990.0, 40.0]), watched([250.0, 40.0]));
    for b in [&hidden, &seen] {
        assert!(b.navigation_revision(Side::Blue) > 0, "the pusher knows");
        assert!(b.known_ground(Side::Blue).cell(100.0, 40.0).cleared > 0);
    }
    assert_eq!(hidden.navigation_revision(Side::Red), 0);
    assert_eq!(hidden.known_ground(Side::Red).cell(100.0, 40.0).cleared, 0);
    assert!(seen.navigation_revision(Side::Red) > 0);
    assert!(seen.known_ground(Side::Red).cell(100.0, 40.0).cleared > 0);
    // The watcher changes nothing of the battle itself.
    assert_eq!(hidden.world().cleared_cells(), seen.world().cleared_cells());
}

/// 34c: the drawn fog's foliage follows ground a side has seen cleared. The
/// static world's grid, less the trees standing on cleared ground, is the
/// foliage the battle's world has at each cell's centre once those trees are
/// down and that ground cleared.
#[test]
fn foliage_from_known_cleared_ground_matches_the_battle_world() {
    let list = json!([forest([40.0, 0.0, 120.0, 120.0], "medium")]);
    let fixed = forests(list.clone());
    let mut live = forests(list);
    // A lane across the forest, as a tank leaves it.
    let lane = Obb2 {
        center: v2(100.0, 60.0),
        yaw: 0.3,
        half: v2(70.0, 3.0),
    };
    let none = Obb2 {
        center: v2(-10.0, -10.0),
        yaw: 0.0,
        half: v2(0.0, 0.0),
    };
    assert!(!live.clear(&lane, &none).is_empty());
    let fallen: Vec<u32> = live
        .props()
        .filter(|p| p.kind == common::kind("trunk") && live.cleared(p.center.x, p.center.y))
        .map(|p| p.id)
        .collect();
    assert!(!fallen.is_empty());
    for id in fallen {
        live.knock_down(id);
    }
    let known = fixed.export_foliage_cleared(|x, y| live.cleared(x, y));
    let (nx, cell) = (known[0] as usize, known[2] as f64);
    let mut opened = 0;
    for (k, pair) in known[3..].chunks(2).enumerate() {
        let mid = v2((k % nx) as f64 + 0.5, (k / nx) as f64 + 0.5) * cell;
        let truth = live.foliage_at(mid.x, mid.y);
        assert_eq!(pair[1], truth.depth_per_m as f32, "cell {k} at {mid:?}");
        assert_eq!(pair[0], truth.canopy_m as f32, "cell {k} at {mid:?}");
        let before = fixed.foliage_at(mid.x, mid.y);
        opened += usize::from(!before.is_open() && truth.depth_per_m < before.depth_per_m);
    }
    assert!(opened > 0, "the lane opened some cells");
}

/// The digest reads where ground was cleared, not only how much: two lanes
/// of the same size in different places hash apart.
#[test]
fn the_digest_tells_cleared_lanes_apart_by_place() {
    let cleared = |y: f64| {
        let mut w = forests(json!([forest([0.0, 0.0, 120.0, 120.0], "light")]));
        let lane = Obb2 {
            center: v2(60.0, y),
            yaw: 0.0,
            half: v2(10.0, 1.5),
        };
        let none = Obb2 {
            center: v2(-50.0, -50.0),
            yaw: 0.0,
            half: v2(0.1, 0.1),
        };
        w.clear(&lane, &none);
        let mut d = sim::digest::Digest::default();
        w.digest_cleared(&mut d);
        (w.cleared_cells(), d.finish())
    };
    let (a, b) = (cleared(40.5), cleared(80.5));
    assert!(a.0 > 0);
    assert_eq!(a.0, b.0, "the same count of cells");
    assert_ne!(a.1, b.1, "in different places");
}
