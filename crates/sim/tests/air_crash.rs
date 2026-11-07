//! A downed helicopter (D3, D31): it falls forward, spinning, crumples on
//! what it cannot break, striking it once, and where it strikes the ground it
//! fells the trees and hurts what lies there before its wreck comes to rest
//! on the ground.

use super::air::{battle_with, fixture, order};
use contract::ids::{Side, UnitId};
use contract::observation::{AirframeStrike, FallingAirframe};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::crash::{Crash, Strike};
use sim::math::{V2, V3};
use sim::publication::{self, reader::Reader, Publisher};

/// The helicopter at [100, 200] flying east toward [500, 200], one hit from
/// death, and a gun jeep at `jeep` to bring it down; plus `others`. Returns
/// the battle once it has settled, where it died, and each tick it fell.
fn shoot_down(map: Value, jeep: [f64; 2], others: Value) -> (Battle, V3, Vec<Fell>) {
    let (mut b, last) = bring_down(map, jeep, others, json!([]));
    // Let it fall and settle.
    let mut fell = Vec::new();
    for _ in 0..10 * b.rules().tick_hz {
        fell.extend(b.crashes().iter().map(|c| Fell {
            yaw: c.yaw,
            seen: Side::ALL.map(|s| b.observe(s).crashes.clone()),
        }));
        b.step();
    }
    (b, last, fell)
}

/// `shoot_down`'s battle with the scenario's `events` too, the tick the
/// helicopter dies, and where it was the tick before.
fn bring_down(map: Value, jeep: [f64; 2], others: Value, events: Value) -> (Battle, V3) {
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
    let setup = serde_json::from_value(json!({ "map": map, "rules": fixture,
        "units": units, "events": events, "scripts": [] }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    assert_eq!(b.accept(order(&[0], [500.0, 200.0])).error, None);
    let mut last = b.unit(UnitId(0)).unwrap().position;
    for _ in 0..60 * b.rules().tick_hz {
        b.step();
        let heli = b.unit(UnitId(0)).unwrap();
        if !heli.alive() {
            return (b, last);
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

/// The test airframe's hull, half extents.
const HALF: V3 = V3 {
    x: 7.0,
    y: 1.2,
    z: 1.6,
};

/// Walls of `ruin` 60 m tall whose west face stands at x = 110, one per
/// centre y and `half_y` long each way.
fn walls(ys: &[f64], half_y: f64) -> sim::world::WorldGeometry {
    let props: Vec<Value> = (ys.iter().enumerate())
        .map(|(id, y)| {
            json!({ "id": id, "kind": "ruin", "center": [113, y], "yaw": 0,
            "half_extents": [3, half_y, 30] })
        })
        .collect();
    crate::common::flat([400.0, 400.0], &format!(r#", "props": {}"#, json!(props)))
}

/// A crash from `from` flying `velocity` at `yaw`, one tick at a time until
/// it lands: each tick's state. No part of its spinning hull, nose and tail
/// included, ever enters a wall.
fn fall(world: &sim::world::WorldGeometry, from: V3, velocity: V2, yaw: f64) -> Vec<Crash> {
    let mut crash = Crash::new(UnitId(0), from, velocity, yaw, None, vec![]);
    let gravity = sim::math::v3(0.0, 0.0, -9.81);
    let mut ticks = Vec::new();
    loop {
        let fall = crash.fall(world, gravity, HALF, 1.0 / 30.0);
        let hull = sim::math::Obb2 {
            center: crash.position.xy(),
            yaw: crash.yaw,
            half: HALF.xy(),
        };
        for wall in world.props() {
            let depth = (wall.footprint().separation(&hull)).map_or(0.0, |s| s.length());
            let under = crash.position.z < wall.top_z();
            assert!(
                !under || depth < 1e-6,
                "its hull sank {depth:.2} m into a wall at {crash:?}"
            );
        }
        ticks.push(crash.clone());
        if fall != sim::crash::Fall::Falling {
            return ticks;
        }
        assert!(ticks.len() < 600, "it never landed");
    }
}

/// Each strike over the fall, with the tick it struck on.
fn strikes(fell: &[Crash]) -> Vec<(usize, Strike)> {
    (fell.iter().enumerate())
        .filter_map(|(i, c)| c.strike.map(|s| (i, s)))
        .collect()
}

#[test]
fn it_crumples_on_a_building_it_falls_into_and_drops_at_its_foot() {
    // Falling from 20 m at 30 m/s, nose first, into a face 3 m ahead.
    let fell = fall(
        &walls(&[200.0], 60.0),
        sim::math::v3(100.0, 200.0, 20.0),
        sim::math::v2(30.0, 0.0),
        0.0,
    );
    let struck = strikes(&fell);
    assert_eq!(struck.len(), 1, "it strikes the wall once: {struck:?}");
    let (at, s) = struck[0];
    assert!(
        (s.point.x - 110.0).abs() < 0.1,
        "it strikes the face: {s:?}"
    );
    assert!(
        s.point.z > 15.0 && s.point.z < 22.0,
        "it strikes the face where it hits it: {s:?}"
    );
    assert!(s.normal.x < -0.99, "the face looks back at it: {s:?}");
    assert!(
        (s.speed_mps - 30.0).abs() < 0.5,
        "it closes at its own speed: {s:?}"
    );
    // An airframe crumples on masonry rather than springing off it.
    let back = -fell[at].velocity.x;
    assert!(
        (0.0..0.2 * s.speed_mps).contains(&back),
        "it springs back at {back:.1} m/s from a {:.0} m/s strike",
        s.speed_mps
    );
    let rest = fell.last().unwrap().position;
    assert!(rest.x < 110.0, "it passed the wall: {rest:?}");
}

#[test]
fn it_scrapes_along_a_face_it_strikes_aslant() {
    // At 45° into the face: it keeps going along it, but slower for the scrape.
    let fell = fall(
        &walls(&[200.0], 60.0),
        sim::math::v3(95.0, 170.0, 20.0),
        sim::math::v2(21.0, 21.0),
        std::f64::consts::FRAC_PI_4,
    );
    let struck = strikes(&fell);
    assert_eq!(struck.len(), 1, "it strikes the wall once: {struck:?}");
    let (at, s) = struck[0];
    assert!(
        (s.speed_mps - 21.0).abs() < 0.5,
        "it closes at its speed into the face: {s:?}"
    );
    let along = fell[at].velocity.y;
    assert!(
        along > 0.0 && along < 0.75 * 21.0,
        "it slides on along the face at {along:.1} m/s"
    );
}

#[test]
fn a_wreck_drifting_against_a_face_strikes_nothing() {
    // Nose 1 m off the face, drifting into it at a walk and spinning against it.
    let fell = fall(
        &walls(&[200.0], 60.0),
        sim::math::v3(102.0, 200.0, 20.0),
        sim::math::v2(1.5, 0.0),
        0.0,
    );
    assert!(
        fell.iter().any(|c| c.velocity.x < 1.0),
        "it never came up against the face"
    );
    assert_eq!(strikes(&fell), vec![], "resting against it strikes nothing");
}

#[test]
fn the_parts_of_one_building_it_strikes_together_are_one_strike() {
    // Two walls meet where its nose hits.
    let fell = fall(
        &walls(&[170.0, 230.0], 30.0),
        sim::math::v3(100.0, 200.0, 20.0),
        sim::math::v2(30.0, 0.0),
        0.0,
    );
    assert_eq!(strikes(&fell).len(), 1, "{:?}", strikes(&fell));
}

#[test]
fn a_hull_striking_a_short_wall_broadside_strikes_it_on_the_wall() {
    // 14 m of hull, nearly side-on, into a wall 4 m long: its ends overhang it.
    let fell = fall(
        &walls(&[200.0], 2.0),
        sim::math::v3(108.5, 200.0, 20.0),
        sim::math::v2(30.0, 0.0),
        std::f64::consts::FRAC_PI_2,
    );
    let struck = strikes(&fell);
    assert!(!struck.is_empty(), "it never struck the wall");
    for (_, s) in struck {
        assert!(
            (s.point.x - 110.0).abs() < 0.1 && (s.point.y - 200.0).abs() <= 2.0 + 1e-9,
            "it strikes off the wall's face, out past its end: {s:?}"
        );
    }
}

#[test]
fn rammed_into_a_face_again_and_again_it_strikes_no_more_than_once_a_quarter_second() {
    let world = walls(&[200.0], 60.0);
    let gravity = sim::math::v3(0.0, 0.0, -9.81);
    let dt = 1.0 / 30.0;
    let mut crash = Crash::new(
        UnitId(0),
        sim::math::v3(100.0, 200.0, 40.0),
        sim::math::v2(30.0, 0.0),
        0.0,
        None,
        vec![],
    );
    let mut struck = Vec::new();
    let mut contacts = 0;
    for tick in 0..30 {
        // Driven back into the face at 30 m/s every tick: each a hard strike
        // by speed alone.
        crash.velocity = sim::math::v3(30.0, 0.0, crash.velocity.z);
        crash.fall(&world, gravity, HALF, dt);
        if crash.velocity.x < 30.0 {
            contacts += 1;
        }
        if let Some(s) = crash.strike {
            assert!(s.speed_mps >= 29.0, "{s:?}");
            struck.push(tick);
        }
    }
    assert!(contacts >= 20, "it met the face on only {contacts} ticks");
    assert!(struck.len() >= 2, "it struck again once quiet: {struck:?}");
    for pair in struck.windows(2) {
        let gap_s = (pair[1] - pair[0]) as f64 * dt;
        assert!(
            gap_s >= 0.25,
            "it struck again {gap_s:.3} s after a strike: {struck:?}"
        );
    }
}

#[test]
fn a_hull_overlapping_a_face_it_is_leaving_keeps_its_speed() {
    // Its nose 2 m into the face, backing off it and sliding along it.
    let world = walls(&[200.0], 60.0);
    let gravity = sim::math::v3(0.0, 0.0, -9.81);
    let dt = 1.0 / 30.0;
    let start = sim::math::v3(105.0, 200.0, 20.0);
    let mut crash = Crash::new(
        UnitId(0),
        start,
        sim::math::v2(-10.0, 3.0),
        0.0,
        None,
        vec![],
    );
    let fall = crash.fall(&world, gravity, HALF, dt);
    assert_eq!(fall, sim::crash::Fall::Falling);
    assert!(
        crash.position.x <= 110.0 - HALF.x,
        "it was put clear of the face: {crash:?}"
    );
    assert_eq!(
        crash.velocity,
        sim::math::v3(-10.0, 3.0, 0.0) + gravity * dt,
        "leaving the face, it is neither turned nor slowed"
    );
    assert_eq!(crash.strike, None);
}

#[test]
fn it_strikes_a_roof_it_falls_onto_once_and_slides_off() {
    let world = walls(&[200.0], 60.0);
    let fell = fall(
        &world,
        sim::math::v3(113.0, 200.0, 70.0),
        sim::math::v2(0.0, 0.0),
        0.0,
    );
    let struck = strikes(&fell);
    assert_eq!(struck.len(), 1, "it strikes the roof once: {struck:?}");
    let s = struck[0].1;
    assert!(
        (s.point.z - 60.0).abs() < 1e-6 && s.normal.z > 0.99,
        "{s:?}"
    );
    assert!(
        s.speed_mps > 10.0,
        "a 10 m drop strikes at {:.1} m/s",
        s.speed_mps
    );
    let rest = fell.last().unwrap().position;
    let ground = world.height_at(rest.x, rest.y).unwrap();
    assert!(
        (rest.z - ground).abs() < 1e-6,
        "it slid off onto the ground: {rest:?}"
    );
}

/// The test helicopter shot down as in the open, but into a building: a wall
/// raised across its fall the tick after it died, its face 3 m past its
/// spinning hull's reach. The battle the tick it died, and the wall.
fn into_a_building() -> (Battle, sim::math::Obb2) {
    let map = || open(json!([]), json!([]));
    let (b, _) = bring_down(map(), [300.0, 80.0], json!([]), json!([]));
    let crash = b.crashes()[0].clone();
    let reach = b.unit(UnitId(0)).unwrap().hull.unwrap().xy().length();
    let way = crash.velocity.xy().normalized();
    let wall = sim::math::Obb2 {
        center: crash.position.xy() + way * (reach + 3.0 + 3.0),
        yaw: way.y.atan2(way.x),
        half: sim::math::v2(3.0, 40.0),
    };
    let raised = json!([{ "tick": b.tick() + 1, "add_prop": { "kind": "ruin",
        "center": [wall.center.x, wall.center.y], "yaw": wall.yaw, "half_extents": [3, 40, 30] } }]);
    let (b, _) = bring_down(map(), [300.0, 80.0], json!([]), raised);
    (b, wall)
}

#[test]
fn both_sides_see_it_strike_the_building_it_is_shot_down_into_once() {
    let (mut b, wall) = into_a_building();
    let mut seen: [Vec<AirframeStrike>; 3] = Default::default();
    let mut wired = Vec::new();
    for _ in 0..10 * b.rules().tick_hz {
        b.step();
        for side in [Side::Blue, Side::Red] {
            let crashes = &b.observe(side).crashes;
            seen[side.index()].extend(crashes.iter().filter_map(|c| c.strike));
        }
        // What blue's wire carries, from a fresh publisher: a whole snapshot.
        if b.observe(Side::Blue)
            .crashes
            .iter()
            .any(|c| c.strike.is_some())
        {
            let mut reader = Reader::new(&publication::layout_json(&b)).unwrap();
            let words: Vec<u32> = (Publisher::new().publish(&b, Side::Blue).unwrap().iter())
                .map(|w| w.to_bits())
                .collect();
            let o = reader.decode(&words).unwrap().unwrap();
            wired.extend(o.crashes.iter().filter_map(|c| c.strike.clone()));
        }
    }
    let (blue, red) = (&seen[Side::Blue.index()], &seen[Side::Red.index()]);
    assert_eq!(blue.len(), 1, "blue sees it strike once: {blue:?}");
    assert_eq!(red, blue, "red, which shot it down, sees the same strike");
    let s = blue[0];
    let point = sim::math::v2(s.point[0], s.point[1]);
    let off = wall.distance(point);
    assert!(off < 0.5, "it strikes {off:.1} m off the wall: {s:?}");
    let way = sim::math::v2(wall.yaw.cos(), wall.yaw.sin());
    assert!(
        s.normal[0] * way.x + s.normal[1] * way.y < -0.9,
        "the face it strikes looks back the way it came: {s:?}"
    );
    assert_eq!(wired.len(), 1, "blue's wire carries the strike once");
    let w = &wired[0];
    let off = (0..3)
        .map(|i| (w.point[i] - s.point[i]).abs())
        .fold(0.0, f64::max);
    assert!(
        off < 1e-3 && (w.speed_mps - s.speed_mps).abs() < 1e-3,
        "{w:?} against {s:?}"
    );
    // Its wreck lies at the foot of the face it struck, not past it.
    let (at, _, _) = wreck(&b);
    assert!(
        (at - wall.center).dot(way) < -3.0,
        "its wreck at {at:?} lies past the wall"
    );
}

#[test]
fn a_helicopter_downed_into_a_building_replays_to_the_same_digest() {
    let run = || {
        let (mut b, _) = into_a_building();
        for _ in 0..10 * b.rules().tick_hz {
            b.step();
        }
        b.digest()
    };
    assert_eq!(run(), run());
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
