//! A downed helicopter (D3, D31): it falls forward, spinning, glances off
//! what it cannot break, and where it strikes the ground it fells the trees
//! and hurts what lies there before its wreck comes to rest on the ground.

use super::air::{battle_with, fixture, order};
use contract::ids::{Side, UnitId};
use contract::observation::FallingAirframe;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::math::{V2, V3};

/// The helicopter at [100, 200] flying east toward [500, 200], one hit from
/// death, and a gun jeep at `jeep` to bring it down; plus `others`. Returns
/// the battle once it has settled, where it died, and each tick it fell.
fn shoot_down(map: Value, jeep: [f64; 2], others: Value) -> (Battle, V3, Vec<Fell>) {
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
            let mut fell = Vec::new();
            for _ in 0..10 * b.rules().tick_hz {
                fell.extend(b.crashes().iter().map(|c| Fell {
                    yaw: c.yaw,
                    seen: Side::ALL.map(|s| b.observe(s).crashes.clone()),
                }));
                b.step();
            }
            return (b, last, fell);
        }
        last = heli.position;
    }
    panic!("the helicopter was never brought down");
}

/// A tick its crash fell: its yaw, and what each side's observation showed.
struct Fell {
    yaw: f64,
    seen: [Vec<FallingAirframe>; 3],
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

fn bridge_map() -> Value {
    json!({
        "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "rivers": [{
            "points": [{ "xy": [318, 0], "width_m": 24, "depth_m": 2 },
                        { "xy": [318, 400], "width_m": 24, "depth_m": 2 }],
            "surface_z": -0.5
        }],
        "bridges": [{
            "deck": "bridge_deck", "center": [318, 200], "half_extents": [18, 5],
            "yaw": 0, "deck_z": 0.1, "thickness_m": 0.8
        }],
        "props": [], "forests": []
    })
}

#[test]
fn a_shot_down_helicopter_falls_forward_spinning_onto_the_ground() {
    let (b, died, fell) = shoot_down(open(json!([]), json!([])), [300.0, 80.0], json!([]));
    let spin: Vec<f64> = fell.iter().map(|f| f.yaw).collect();
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
fn a_helicopter_shot_down_over_a_bridge_rests_on_its_deck() {
    let map: contract::map::MapDefinition = serde_json::from_value(bridge_map()).unwrap();
    let world = sim::world::WorldGeometry::new(&map, &crate::common::rules());
    let deck = world.surface_at(317.0, 200.0).unwrap();
    assert_eq!(deck.kind, sim::world::SurfaceKind::Bridge);
    let mut crash = sim::crash::Crash::new(
        UnitId(0),
        V3 {
            x: 317.0,
            y: 200.0,
            z: 20.0,
        },
        V2 { x: 0.0, y: 0.0 },
        0.0,
        None,
        vec![],
    );
    for _ in 0..300 {
        if crash.fall(
            &world,
            V3 {
                x: 0.0,
                y: 0.0,
                z: -9.81,
            },
            V3 {
                x: 7.0,
                y: 1.2,
                z: 1.6,
            },
            1.0 / 30.0,
        ) == sim::crash::Fall::Landed
        {
            break;
        }
    }
    assert!(
        (crash.position.z - deck.z).abs() < 1e-6,
        "impact {:?}, deck {}",
        crash.position,
        deck.z
    );

    let (b, _, _) = shoot_down(bridge_map(), [300.0, 80.0], json!([]));
    let (_, base, _) = wreck(&b);
    let wreck = b
        .world()
        .props()
        .find(|p| p.wreck_of.is_some())
        .expect("a wreck where the helicopter fell");
    let surface = b
        .world()
        .surface_at(wreck.center.x, wreck.center.y)
        .unwrap();
    assert_eq!(surface.kind, sim::world::SurfaceKind::Bridge);
    assert!(
        (base - surface.z).abs() < 1e-6,
        "wreck base {base}, deck {}",
        surface.z
    );
}

#[test]
fn both_sides_that_saw_it_go_down_see_it_fall_to_its_wreck() {
    // Blue flew it and red shot it: each sees the airframe fall, blue as its
    // own unit 0, red as the enemy it had identified, from where it died
    // down to the ground where its wreck then lies.
    let (b, died, fell) = shoot_down(open(json!([]), json!([])), [300.0, 80.0], json!([]));
    let kind = b.rules().catalog.index("test_heli").unwrap();
    let (at, base, _) = wreck(&b);
    for (side, own) in [(Side::Blue, true), (Side::Red, false)] {
        let path: Vec<&FallingAirframe> = fell
            .iter()
            .map(|f| {
                let seen = &f.seen[side.index()];
                assert_eq!(seen.len(), 1, "{side:?} sees the one falling airframe");
                &seen[0]
            })
            .collect();
        assert!(path.iter().all(|c| c.own == own && c.kind == kind));
        if own {
            assert!(path.iter().all(|c| c.id == 0), "blue knows it as unit 0");
        }
        let (first, last) = (path[0], path[path.len() - 1]);
        assert!(
            (first.position[2] - died.z).abs() < 2.0,
            "{side:?} sees it start falling where it died: {:.1} vs {:.1}",
            first.position[2],
            died.z
        );
        assert!(
            last.position[2] - base < 2.0,
            "{side:?} sees it fall to the ground, last at {:.1} m",
            last.position[2]
        );
        assert!(
            (last.position[0] - at.x).hypot(last.position[1] - at.y) < 3.0,
            "{side:?} sees it land where its wreck lies"
        );
        // Its nose drops and it leans into its spin as it falls.
        assert!(last.pitch < -0.1 && last.roll.abs() > 0.1, "{last:?}");
    }
    assert!(
        b.observe(Side::Blue).crashes.is_empty(),
        "a landed airframe is its wreck"
    );
}

/// A wood across the helicopter's flight, and the helicopter shot down over
/// it by a jeep placed so the wreck lands on trees, not in a gap between them.
fn down_in_the_wood() -> Battle {
    let forest = json!([{ "shape": { "kind": "polygon",
        "ring": [[120, 140], [420, 140], [420, 260], [120, 260]] } }]);
    let (b, _, _) = shoot_down(open(json!([]), forest.clone()), [320.0, 40.0], json!([]));
    // Trees stood where it came down: the wood before anything flew over it.
    let (at, _, _) = wreck(&b);
    let wood = battle_with(&fixture(), open(json!([]), forest), json!([]));
    assert!(trees_within(&wood, at, 5.0) > 0, "no tree stood at {at:?}");
    b
}

/// The trees standing within `r` of `at`.
fn trees_within(b: &Battle, at: V2, r: f64) -> usize {
    let tree = b.world().types().index(&b.rules().forests.tree).unwrap();
    (b.world().props_near(at, r).into_iter())
        .filter(|p| p.kind == tree && p.footprint().distance(at) < r)
        .count()
}

#[test]
fn its_crash_fells_the_trees_it_lands_on() {
    let b = down_in_the_wood();
    let (at, _, _) = wreck(&b);
    let standing = trees_within(&b, at, 5.0);
    assert_eq!(standing, 0, "{standing} trees still stand under the wreck");
}

#[test]
fn the_side_that_saw_it_go_down_sees_the_trees_it_felled() {
    // Blue's only unit was the helicopter: with no eyes left on the wood,
    // blue still saw it come down there, so it knows the trees its crash
    // felled as it knows its wreck, and no longer plans with them standing.
    let b = down_in_the_wood();
    assert!(b.unit(UnitId(0)).is_some_and(|u| !u.alive()));
    let (at, _, _) = wreck(&b);
    let felled: Vec<[f64; 2]> = (b.observe(Side::Blue).fallen_bodies.iter())
        .filter(|f| (f.at[0] - at.x).hypot(f.at[1] - at.y) < 9.0)
        .map(|f| f.at)
        .collect();
    assert!(
        !felled.is_empty(),
        "blue sees no tree felled by its wreck at {at:?}"
    );
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
    let wall = world.props().next().unwrap().footprint();
    // No part of its spinning hull, nose and tail included, enters the wall.
    let inside = |crash: &sim::crash::Crash| {
        let hull = sim::math::Obb2 {
            center: crash.position.xy(),
            yaw: crash.yaw,
            half: half.xy(),
        };
        wall.separation(&hull).map_or(0.0, |s| s.length())
    };
    let mut ticks = 0;
    while crash.fall(&world, gravity, half, 1.0 / 30.0) == sim::crash::Fall::Falling {
        let depth = inside(&crash);
        assert!(
            depth < 1e-6,
            "its hull sank {depth:.2} m into the wall at {crash:?}"
        );
        ticks += 1;
        assert!(ticks < 300, "it never landed");
    }
    let depth = inside(&crash);
    assert!(depth < 1e-6, "it came to rest {depth:.2} m into the wall");
    assert!(crash.position.x < 110.0, "it passed the wall: {crash:?}");
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
