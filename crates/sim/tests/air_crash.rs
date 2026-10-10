//! A downed helicopter (D3, D31): it falls forward, spinning, glances off
//! what it cannot break, and where it strikes the ground it fells the trees
//! and hurts what lies there before its wreck comes to rest on the ground.

use super::air::{battle_with, fixture, order};
use contract::ids::UnitId;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{V2, V3};

/// The helicopter at [100, 200] flying east toward [500, 200], one hit from
/// death, and a gun jeep at `jeep` to bring it down; plus `others`. Returns
/// the battle once it has settled, where it died, and its crash's yaw each
/// tick it fell.
fn shoot_down(map: Value, jeep: [f64; 2], others: Value) -> (Battle, V3, Vec<f64>) {
    let mut fixture = fixture();
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_heli",
        json!({ "body": { "hull": { "hp": 1.0 } } }),
    );
    let mut units = vec![
        json!({ "side": "blue", "kind": "test_heli", "position": [100, 200], "engagement": "return_fire_only" }),
        json!({ "side": "red", "kind": "test_gun_jeep", "position": jeep }),
    ];
    units.extend(others.as_array().unwrap().iter().cloned());
    let mut b = battle_with(&fixture, map, Value::Array(units));
    assert_eq!(b.accept(order(&[0], [500.0, 200.0])).error, None);
    let mut last = b.unit(UnitId(0)).unwrap().position;
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        if !heli.alive() {
            // Let it fall and settle.
            let mut spin = Vec::new();
            for _ in 0..10 * b.rules().tick_hz {
                spin.extend(b.crashes().iter().map(|c| c.yaw));
                b.step();
            }
            return (b, last, spin);
        }
        last = heli.position;
    }
    panic!("the helicopter was never brought down");
}

/// The helicopter's wreck: (centre, base z, yaw).
fn wreck(b: &Battle) -> (V2, f64, f64) {
    let kind = b.rules().catalog.index("test_heli").unwrap();
    let w = b
        .world()
        .props()
        .find(|p| p.wreck_of == Some(kind))
        .expect("a wreck where it fell");
    (w.center, w.base_z, w.yaw)
}

fn open(props: Value, forests: Value) -> Value {
    json!({ "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": props, "forests": forests })
}

#[test]
fn a_shot_down_helicopter_falls_forward_spinning_onto_the_ground() {
    let (b, died, spin) = shoot_down(open(json!([]), json!([])), [300.0, 80.0], json!([]));
    assert!(died.z > 15.0, "it died in the air ({:.1} m)", died.z);
    let (at, base, _) = wreck(&b);
    assert!(
        at.x > died.x + 5.0,
        "it fell straight down: {at:?} from {died:?}"
    );
    let ground = b.world().height_at(at.x, at.y).unwrap();
    assert!(
        (base - ground).abs() < 1e-6,
        "the wreck rests at {base:.1}, ground {ground:.1}"
    );
    assert!(spin.len() > 30, "it fell for only {} ticks", spin.len());
    let turned: f64 = spin
        .windows(2)
        .map(|w| sim::math::wrap_angle(w[1] - w[0]).abs())
        .sum();
    assert!(turned > 3.0, "it turned only {turned:.2} rad as it fell");
}

#[test]
fn its_crash_fells_the_trees_it_lands_on() {
    let forest = json!([{ "shape": { "kind": "polygon",
        "ring": [[120, 140], [420, 140], [420, 260], [120, 260]] } }]);
    let (b, _, _) = shoot_down(open(json!([]), forest), [300.0, 40.0], json!([]));
    let (at, _, _) = wreck(&b);
    let tree = b.world().types().index(&b.rules().forests.tree).unwrap();
    let standing = b
        .world()
        .props_near(at, 5.0)
        .into_iter()
        .filter(|p| p.kind == tree && p.footprint().distance(at) < 5.0)
        .count();
    assert_eq!(standing, 0, "{standing} trees still stand under the wreck");
}

#[test]
fn it_glances_off_a_building_it_falls_against() {
    // Falling at 20 m toward a wall 24 m tall whose face is 10 m ahead.
    let world = crate::common::flat(
        [400.0, 400.0],
        r#", "props": [{ "id": 0, "kind": "ruin", "center": [113, 200], "yaw": 0,
            "half_extents": [3, 60, 12] }]"#,
    );
    let mut crash = sim::crash::Crash::new(
        UnitId(0),
        sim::math::v3(100.0, 200.0, 20.0),
        sim::math::v2(30.0, 0.0),
        0.0,
        None,
        vec![],
    );
    let half = sim::math::v3(7.0, 1.2, 1.6);
    let gravity = sim::math::v3(0.0, 0.0, -9.81);
    let mut ticks = 0;
    while !crash.fall(&world, gravity, half, 1.0 / 30.0) {
        assert!(
            crash.position.x < 110.0,
            "it passed into the wall at {:?}",
            crash.position
        );
        ticks += 1;
        assert!(ticks < 300, "it never landed");
    }
    assert!(
        crash.position.x < 110.0 - half.y,
        "it came to rest against the wall"
    );
}

#[test]
fn what_lies_under_it_is_hurt_less_than_by_a_shell() {
    let (clear, _, _) = shoot_down(open(json!([]), json!([])), [300.0, 80.0], json!([]));
    let (at, _, _) = wreck(&clear);
    let squad = json!([{ "side": "red", "kind": "test_rifle", "position": [at.x, at.y],
        "engagement": "return_fire_only" }]);
    let (b, _, _) = shoot_down(open(json!([]), json!([])), [300.0, 80.0], squad);
    let squad = b.unit(UnitId(2)).unwrap();
    let lost: f64 = squad.members.iter().map(|s| 100.0 - s.hp.max(0.0)).sum();
    assert!(lost > 0.0, "the crash hurt nobody under it");
    assert!(squad.alive(), "the crash wiped out the squad");
}

#[test]
fn a_downed_helicopter_replays_to_the_same_digest() {
    let run = || {
        shoot_down(open(json!([]), json!([])), [300.0, 80.0], json!([]))
            .0
            .digest()
    };
    assert_eq!(run(), run());
}
