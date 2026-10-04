//! Forests as bodies, all by one rule (Q14, Q16, Q21, Q-G8b). A forest's
//! shape only generates trunks; at runtime a forest is those bodies plus the
//! ground a heavy vehicle has cleared, and every forest query (speed,
//! concealment, sight, cover, fog) reads both.
use crate::common;

use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::scenario::PushClass;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{v2, v3, Obb2, V2};
use sim::visibility::{self, OcclusionGrid};
use sim::world::WorldGeometry;

fn forest(rect: [f64; 4]) -> Value {
    let [x, y, w, h] = rect;
    json!({ "shape": {"kind":"polygon","ring":[[x,y],[x+w,y],[x+w,y+h],[x,y+h]]}})
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

/// Q16, Q-G8b: the one rule's spacing sets how many trunks stand and how
/// close, in every forest alike.
#[test]
fn the_rule_sets_trunk_spacing_in_every_forest() {
    let w = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0]),
        forest([150.0, 0.0, 120.0, 120.0])
    ]));
    let d = common::forest_rules().rule;
    for (x0, x1) in [(0.0, 120.0), (150.0, 270.0)] {
        let trunks = trunks_in(&w, x0, x1);
        let per_axis: f64 = ((120.0 - d.trunk_spacing_m / 2.0) / d.trunk_spacing_m).floor() + 1.0;
        let grid = per_axis * per_axis;
        // A jittered trunk never leaves its forest, so a few edge ones go.
        assert!(
            trunks.len() as f64 <= grid && trunks.len() as f64 >= 0.85 * grid,
            "{x0}: {} trunks on a {grid} grid",
            trunks.len()
        );
        let closest = trunks
            .iter()
            .enumerate()
            .flat_map(|(i, a)| trunks[i + 1..].iter().map(move |b| (*a - *b).length()))
            .fold(f64::INFINITY, f64::min);
        let floor = d.trunk_spacing_m * (1.0 - 2.0 * d.trunk_jitter);
        assert!(closest >= floor - 1e-9, "{x0}: {closest:.2} m apart");
    }
    // The same forest places the same trunks.
    let again = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0]),
        forest([150.0, 0.0, 120.0, 120.0])
    ]));
    assert_eq!(trunks_in(&w, 0.0, 300.0), trunks_in(&again, 0.0, 300.0));
}

/// Q21, Q-G8b: every forest hides alike, infantry more than vehicles, and
/// open ground not at all.
#[test]
fn every_forest_conceals_alike() {
    let w = forests(json!([
        forest([0.0, 0.0, 120.0, 120.0]),
        forest([150.0, 0.0, 120.0, 120.0])
    ]));
    // Forest ground grants the same bonus regardless of tree crown coverage.
    let mean = |x0: f64, infantry: bool| {
        let mut sum = 0.0;
        let mut n = 0.0;
        for i in 0..20 {
            for j in 0..20 {
                let p = v2(x0 + 20.0 + 4.0 * i as f64, 20.0 + 4.0 * j as f64);
                sum += w.forest_concealment(infantry, p.x, p.y);
                n += 1.0;
            }
        }
        sum / n
    };
    for infantry in [true, false] {
        let (a, b) = (mean(0.0, infantry), mean(150.0, infantry));
        assert!(a < 1.0 && (a - b).abs() < 0.02, "{infantry}: {a} vs {b}");
    }
    // Infantry hides more than a vehicle in the same foliage (the class rule).
    assert!(mean(150.0, true) < mean(150.0, false));
    assert_eq!(w.forest_concealment(true, 135.0, 60.0), 1.0);
    assert!(!w.surface_at(135.0, 60.0).unwrap().forest);
    assert!(w.surface_at(210.0, 60.0).unwrap().forest);
}

/// Q16: a lane knocked through the trees is open ground to speed, sensing,
/// cover and fog, while the forest beside it is still forest.
#[test]
fn a_cleared_lane_reads_as_open_ground() {
    let mut w = forests(json!([forest([40.0, 0.0, 200.0, 120.0])]));
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
    let mut grid = OcclusionGrid::new(&w, 8.0);
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
        &mut grid,
        &rules.sensors,
        v3(30.0, 60.0, 1.8),
        &sight,
        &mut field,
    );
    assert!(field.visible(228.0, 60.0));
    let mut beside = grid.field();
    visibility::sweep(
        &w,
        &mut grid,
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
    let w = forests(json!([forest([0.0, 0.0, 60.0, 60.0])]));
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
    let map = json!({ "size": [300, 80], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                      "forests": [forest([70.0, 0.0, 60.0, 40.0])] });
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
    assert!(
        truck.x > 140.0,
        "the truck drove past the forest: {truck:?}"
    );
    assert_eq!(b.world().cleared_cells(), 0, "it knocked nothing down");
}

/// A tank through the forest, and a red squad down its lane that did
/// not see the trees fall.
fn carve(watcher: [f64; 2]) -> contract::scenario::ScenarioDefinition {
    let map = json!({ "size": [1000, 80], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                      "forests": [forest([70.0, 0.0, 60.0, 80.0])] });
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
    let list = json!([forest([40.0, 0.0, 120.0, 120.0])]);
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
    let records: std::collections::BTreeMap<usize, [f32; 2]> = known[3..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|r| (r[1] as usize * nx + r[0] as usize, [r[2], r[3]]))
        .collect();
    for k in 0..nx * known[1] as usize {
        let pair = records.get(&k).copied().unwrap_or([0.0, 0.0]);
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
        let mut w = forests(json!([forest([0.0, 0.0, 120.0, 120.0])]));
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

/// The cleared ground is framed by its count of words, so what the digest
/// reads after it can never pass for a cleared word: an uncleared world
/// still writes its (zero) count.
#[test]
fn the_cleared_digest_is_framed_by_its_count() {
    let w = forests(json!([forest([0.0, 0.0, 120.0, 120.0])]));
    assert_eq!(w.cleared_cells(), 0);
    let mut d = sim::digest::Digest::default();
    w.digest_cleared(&mut d);
    assert_ne!(d.finish(), sim::digest::Digest::default().finish());
}

#[test]
fn polygon_forest_uses_its_concave_boundary_for_ground_and_trunks() {
    let w = forests(json!([{
        "shape": {"kind":"polygon","ring":[[0,0],[90,0],[90,27],[27,27],[27,90],[0,90]]}
    }]));
    assert!(w.forest_ground(10.0, 70.0));
    assert!(w.forest_ground(70.0, 10.0));
    assert!(
        w.forest_ground(27.0, 70.0),
        "the authored boundary is closed"
    );
    assert!(
        !w.forest_ground(70.0, 70.0),
        "the concave notch is open ground"
    );
    let trunks = trunks_in(&w, 0.0, 100.0);
    assert!(!trunks.is_empty());
    assert!(
        trunks.iter().all(|p| p.x <= 27.0 || p.y <= 27.0),
        "no trunk in the notch"
    );
}

#[test]
fn stroke_forest_is_its_square_ended_band_instead_of_its_bounds() {
    let w = forests(json!([{
        "shape": {"kind":"stroke","points":[[20,20],[80,80]],"width_m":18}
    }]));
    assert!(w.forest_ground(50.0, 50.0));
    assert!(
        w.forest_ground(17.0, 25.0),
        "beside the first point, within half the width"
    );
    assert!(
        !w.forest_ground(14.0, 20.0),
        "the strip is cut square across its first point: no half-disc of wood behind it"
    );
    assert!(
        !w.forest_ground(20.0, 80.0),
        "bounds must not become physical forest"
    );
    let trunks = trunks_in(&w, 0.0, 100.0);
    assert!(!trunks.is_empty());
    for p in trunks {
        let along = ((p.x - 20.0) + (p.y - 20.0)) / 120.0;
        assert!((0.0..=1.0).contains(&along), "a trunk past the strip's end");
        assert!((p.x - (20.0 + 60.0 * along)).hypot(p.y - (20.0 + 60.0 * along)) <= 9.0);
    }
}

#[test]
fn foliage_line_depth_keeps_its_exact_spans_across_bucket_edges_and_canopies() {
    let map: contract::map::MapDefinition = serde_json::from_value(json!({
        "size":[300,300], "fog_cell_m":8, "height_grid_m":4, "slope_cutoff_deg":35,
        "forests":[forest([66.0,58.0,12.0,12.0]), forest([74.0,62.0,12.0,12.0]),
            forest([-30.0,50.0,12.0,20.0]), forest([96.0,92.0,12.0,12.0])]
    }))
    .unwrap();
    let mut rules = common::game();
    sim::fixtures::patch_catalog(
        &mut rules,
        "props",
        "trunk",
        json!({"body":{"conceals":1.0}}),
    );
    let mut rules: contract::scenario::Rules = serde_json::from_value(rules).unwrap();
    rules.forests.rule.trunk_spacing_m = 12.0;
    rules.forests.rule.trunk_jitter = 0.0;
    rules.forests.rule.canopy_radius_m = 16.0;
    rules.forests.rule.canopy_height_m = 6.0;
    rules.forests.rule.attenuation_per_m = 0.03;
    let world = WorldGeometry::new(&map, &rules);
    let lines = [
        ([40.0, 60.0, 1.5], [120.0, 60.0, 1.5]),
        ([63.99999999, 0.0, 1.5], [63.99999999, 120.0, 1.5]),
        ([63.99999999, 70.0, 1.5], [63.99999999, 85.0, 1.5]),
        ([77.0, 0.0, 1.5], [77.0, 120.0, 1.5]),
        ([-40.0, 62.0, 1.5], [110.0, 62.0, 1.5]),
        ([150.0, 200.0, 1.5], [100.0, 102.0, 1.5]),
        ([200.0, 200.0, 1.5], [280.0, 260.0, 1.5]),
        ([62.0, 62.0, 50.0], [102.0, 98.0, 50.0]),
    ];
    let depths: Vec<_> = lines
        .into_iter()
        .map(|(a, b)| {
            world
                .foliage_depth(v3(a[0], a[1], a[2]), v3(b[0], b[1], b[2]))
                .to_bits()
        })
        .collect();
    // Exact span integration from the exhaustive forest walk, with fixed inputs.
    assert_eq!(
        depths,
        vec![
            0x3ff3333333333337,
            0x3fdeb851eb851ebc,
            0x3faeb851eb851eb8,
            0x3feeb851eb851ebe,
            0x3ff317e4b17e4b1b,
            0x3fd62ba122893e40,
            0x0,
            0x0
        ]
    );
}

/// Sparse floor cover is physical, generated after every trunk, and cannot
/// rearrange a later forest's trees when a density changes.
#[test]
fn sparse_floor_bodies_leave_every_trunk_and_foliage_cell_unchanged() {
    let map: contract::map::MapDefinition = serde_json::from_value(json!({
        "size":[300,240],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "forests":[forest([0.0,0.0,220.0,220.0]),forest([100.0,50.0,180.0,180.0])]
    }))
    .unwrap();
    let mut raw = common::game();
    // Deliberately provide broad navigable gaps for every catalog mover;
    // overlapping dense woods may correctly admit no floor cover.
    raw["forests"]["rule"]["trunk_spacing_m"] = json!(18);
    raw["forests"]["rule"]["logs_per_ha"] = json!(5);
    raw["forests"]["rule"]["boulders_per_ha"] = json!(3);
    raw["forests"]["rule"]["log_half_extents_m"] = json!([2.2, 0.35, 0.35]);
    raw["forests"]["rule"]["boulder_half_extents_m"] = json!([1, 0.8, 0.75]);
    raw["forests"]["log"] = json!("log");
    raw["forests"]["boulder"] = json!("boulder");
    let bodies: contract::scenario::Rules = serde_json::from_value(raw.clone()).unwrap();
    raw["forests"]["rule"]["logs_per_ha"] = json!(0);
    raw["forests"]["rule"]["boulders_per_ha"] = json!(0);
    let empty: contract::scenario::Rules = serde_json::from_value(raw).unwrap();
    let (with, without) = (
        WorldGeometry::new(&map, &bodies),
        WorldGeometry::new(&map, &empty),
    );
    let trunks = |w: &WorldGeometry| {
        w.props()
            .filter(|p| p.forest_tree)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(trunks(&with), trunks(&without));
    assert_eq!(with.export_foliage(), without.export_foliage());
    assert_eq!(
        with.export_forest_trunk_ranges(),
        without.export_forest_trunk_ranges()
    );
    let floor: Vec<_> = with.props().filter(|p| !p.forest_tree).collect();
    assert!(floor.iter().any(|p| with.types().id(p.kind) == "log"));
    assert!(floor.iter().any(|p| with.types().id(p.kind) == "boulder"));
    for body in floor {
        assert!(with.forest_ground(body.center.x, body.center.y));
        for tree in trunks(&with) {
            assert!(!body.footprint().contains(
                tree.center,
                tree.half.x + bodies.forests.rule.trunk_clearance_m
            ));
        }
    }
    let again = WorldGeometry::new(&map, &bodies);
    assert_eq!(with.export_props(), again.export_props());
}

/// A fallen trunk is useful solid cover, not standing foliage; a boulder
/// stops a jeep and cannot be cleared by ordinary shoving or rifle fire.
#[test]
fn floor_bodies_give_real_cover_without_creating_foliage() {
    use contract::scenario::CoverTier::{Heavy, Medium};
    for (kind, tier) in [("log", Medium), ("boulder", Heavy)] {
        let w = common::flat(
            [100.0, 100.0],
            &format!(
                r#","props":[{{"kind":"{kind}","center":[50,50],"yaw":0,"half_extents":[1,1,0.7]}}]"#
            ),
        );
        let p = w.prop(0).unwrap();
        assert!(p.blocks(MoverClass::Infantry) && p.blocks(MoverClass::Vehicle));
        assert!(!w.segment_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)));
        assert!(w.sight_clear(v3(40.0, 50.0, 1.0), v3(60.0, 50.0, 1.0)));
        assert!(w.foliage_at(50.0, 50.0).is_open());
        let r = common::rules();
        let ground = sim::ground::GroundLayer::new(100.0, 100.0, &r.ground);
        assert_eq!(
            sim::cover::at(&w, &ground, &[], &r, v2(48.5, 50.0), v2(60.0, 50.0)),
            Some(tier)
        );
        assert!(!PushClass::Light.pushes(p.body.weight_class));
        assert_eq!(PushClass::Heavy.pushes(p.body.weight_class), kind == "log");
        let mut structures = sim::structures::Structures::default();
        assert_eq!(structures.damage(&w, 0, 1.0e6), kind == "log");
    }
}

#[test]
fn invalid_floor_generation_rules_fail_during_rule_loading() {
    let mut raw = common::game();
    raw["forests"]["log"] = json!("log");
    raw["forests"]["rule"]["logs_per_ha"] = json!(5);
    raw["forests"]["rule"]["log_half_extents_m"] = json!([2.2, 0.35, 0.35]);
    let cases = [
        ("log", json!("missing")),
        ("rule", json!({"logs_per_ha":-1})),
        ("rule", json!({"logs_per_ha":101})),
        ("rule", json!({"logs_per_ha":1e-320})),
        ("rule", json!({"log_half_extents_m":[1,0,1]})),
    ];
    for (key, patch) in cases {
        let mut bad = raw.clone();
        if key == "rule" {
            for (name, value) in patch.as_object().unwrap() {
                bad["forests"][key][name] = value.clone();
            }
        } else {
            bad["forests"][key] = patch;
        }
        let e = serde_json::from_value::<contract::scenario::Rules>(bad)
            .unwrap_err()
            .to_string();
        assert!(e.contains("forests") && e.contains("log"), "{e}");
    }
}

/// Every open navigation cell remains connected to the forest's approaches;
/// floor cover must not close isolated pockets between trunks.
#[test]
fn sparse_floor_cover_keeps_all_open_forest_cells_reachable() {
    use sim::navigation::{Mobility, NavBase, NavGrid};
    use std::collections::VecDeque;
    use std::sync::Arc;
    for offset in [0.0, 7.0, 19.0] {
        let map: contract::map::MapDefinition = serde_json::from_value(json!({
            "size":[200,200],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
            "forests":[forest([10.0+offset,10.0,160.0,160.0])]
        }))
        .unwrap();
        let mut r = common::rules();
        r.forests.log = Some("log".into());
        r.forests.boulder = Some("boulder".into());
        r.forests.rule.logs_per_ha = 5.0;
        r.forests.rule.boulders_per_ha = 3.0;
        r.forests.rule.log_half_extents_m = [2.2, 0.35, 0.35];
        r.forests.rule.boulder_half_extents_m = [1.0, 0.8, 0.75];
        let w = WorldGeometry::new(&map, &r);
        assert!(
            w.props().any(|p| !p.forest_tree),
            "experiment must contain floor bodies"
        );
        let grid = NavGrid::new(Arc::new(NavBase::build(&w, w.props(), 0.3)));
        for (class, push, half_width_m) in [
            (MoverClass::Infantry, PushClass::None, 0.5),
            (MoverClass::Vehicle, PushClass::Light, 1.0),
        ] {
            let m = Mobility {
                off_road_mps: 3.0,
                road_mps: 3.0,
                forest_multiplier: 0.5,
                half_width_m,
                class,
                push,
                drive: None,
            };
            let point = |k: usize| v2((k % 100) as f64 * 2.0 + 1.0, (k / 100) as f64 * 2.0 + 1.0);
            let open: Vec<_> = (0..10000).map(|k| grid.fits_at(point(k), &m)).collect();
            let start = open.iter().position(|p| *p).unwrap();
            let mut visited = vec![false; 10000];
            visited[start] = true;
            let mut queue = VecDeque::from([start]);
            while let Some(k) = queue.pop_front() {
                let x = k % 100;
                let y = k / 100;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (x, y) = (x as isize + dx, y as isize + dy);
                    if !(0..100).contains(&x) || !(0..100).contains(&y) {
                        continue;
                    }
                    let next = y as usize * 100 + x as usize;
                    if open[next] && !visited[next] && grid.route_fits(point(k), &[point(next)], &m)
                    {
                        visited[next] = true;
                        queue.push_back(next);
                    }
                }
            }
            for (k, free) in open.iter().enumerate() {
                assert!(
                    !free || visited[k],
                    "{class:?}: isolated cell {:?} at forest offset {offset}",
                    point(k)
                );
            }
        }
    }
}
/// The tree-line contract uses the ordinary forest rule: a far-field recon
/// squad loses identification through a real strip, with an open arm as control.
#[test]
fn a_real_tree_line_hides_a_recon_squad_from_the_far_field() {
    let battle = |strips: bool, observer_x: f64, target_x: f64| {
        let mut map =
            json!({"size":[1200,300],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35});
        if strips {
            map["forests"] =
                json!([{"shape":{"kind":"stroke","points":[[600,20],[600,280]],"width_m":24}}]);
        }
        let mut setup = common::scenario(
            &map.to_string(),
            json!([
                {"side":"blue","kind":"recon","position":[observer_x,150],"engagement":"return_fire_only"},
                {"side":"red","kind":"recon","position":[target_x,150],"engagement":"return_fire_only"}
            ]),
            json!([]),
        );
        // Keep the open control in optical range independently of recon balance.
        let mut rules = common::game();
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            "recon",
            json!({"sensors":{"ground_m":1000}}),
        );
        setup.rules.catalog = serde_json::from_value::<contract::scenario::Rules>(rules)
            .unwrap()
            .catalog;
        let mut b = Battle::new(&setup, 1);
        b.step();
        b
    };
    let (strip, open) = (battle(true, 100.0, 950.0), battle(false, 100.0, 950.0));
    assert!(
        !open.observe(Side::Blue).identified.is_empty(),
        "control identifies the squad"
    );
    assert!(
        strip.observe(Side::Blue).identified.is_empty(),
        "strip attenuates sight to the far squad"
    );
    let close = battle(true, 500.0, 700.0);
    assert!(
        !close.observe(Side::Blue).identified.is_empty(),
        "a thin strip uses ordinary forest attenuation, not an opaque wall"
    );
}

#[test]
fn tree_line_foliage_follows_trunk_crowns_beyond_the_authored_strip_edge() {
    let map: contract::map::MapDefinition = serde_json::from_value(json!({
        "size":[300,240],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,
        "forests":[{"shape":{"kind":"stroke","points":[[30,30],[150,60],[250,190]],"width_m":18}}]
    }))
    .unwrap();
    let r = common::rules();
    let w = WorldGeometry::new(&map, &r);
    let trunks: Vec<_> = w
        .props()
        .filter(|p| p.forest_tree)
        .map(|p| p.center)
        .collect();
    assert!(!trunks.is_empty());
    let grid = w.export_foliage();
    assert!(grid.len() > 3);
    let mut over_edge = false;
    for record in grid[3..].as_chunks::<4>().0 {
        let center = v2(
            (record[0] as f64 + 0.5) * grid[2] as f64,
            (record[1] as f64 + 0.5) * grid[2] as f64,
        );
        assert!(
            trunks
                .iter()
                .any(|p| (*p - center).length() <= r.forests.rule.canopy_radius_m + 1e-9),
            "foliage at {center:?} needs a real crown"
        );
        over_edge |= !w.forest_ground(center.x, center.y);
    }
    assert!(
        over_edge,
        "canopy reaches past the strip edge instead of being clipped to it"
    );
}

#[test]
fn forest_floor_scaled_integrity_is_refused_without_an_aggregate() {
    let mut raw = common::game();
    sim::fixtures::patch_catalog(
        &mut raw,
        "props",
        "log",
        json!({"body":{"hp_scale":"building_floor_bands"}}),
    );
    raw["forests"]["log"] = json!("log");
    raw["forests"]["rule"]["logs_per_ha"] = json!(5);
    raw["forests"]["rule"]["log_half_extents_m"] = json!([2.2, 0.35, 0.35]);
    assert!(
        serde_json::from_value::<contract::scenario::Rules>(raw).is_err(),
        "an ordinary generated log has no building bulk from which to derive HP"
    );
}
