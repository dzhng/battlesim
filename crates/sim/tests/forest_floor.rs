//! A forest's floor cover: logs and boulders generated after every trunk,
//! physical bodies that give cover and never make foliage or seal ground.
use crate::common;
use crate::forest::forest;

use contract::map::MoverClass;
use contract::scenario::PushClass;
use serde_json::json;
use sim::math::{v2, v3};
use sim::world::WorldGeometry;

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
                swamp_multiplier: 0.5,
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
