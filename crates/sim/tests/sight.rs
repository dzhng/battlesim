//! The one sight shape: a unit sees farthest along its forward
//! direction, and spotting, the fog sweep and the publication read the same
//! shape. Vehicles look along the turret (the truck along its hull);
//! infantry see an even 360°.
use std::f64::consts::{FRAC_PI_2, PI};

use contract::ids::{Side, UnitId};
use contract::observation::OwnUnit;
use contract::scenario::Rules;
use serde_json::json;
use sim::battle::Battle;
use sim::sight::sight_range;

use crate::common;

/// Open, flat ground big enough for a recon's full range in every direction.
const FLAT: &str =
    r#"{ "size": [2600, 2600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }"#;

fn rules() -> Rules {
    serde_json::from_value(common::game()).unwrap()
}

fn battle(map: &str, units: serde_json::Value) -> Battle {
    Battle::new(&common::scenario(map, units, json!([])), 1)
}

fn own(b: &Battle, side: Side, id: u32) -> OwnUnit {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
        .unwrap()
}

fn base_range(kind: &str) -> f64 {
    common::rules().catalog.by_id(kind).sensors.ground_m
}

fn shape(kind: &str) -> [f64; 3] {
    let s = common::rules().catalog.by_id(kind).sensors.sight_shape;
    [s.front, s.side, s.rear]
}

#[test]
fn range_follows_the_vehicle_shape_at_front_side_rear_and_across_the_wrap() {
    let rules = rules();
    for (kind, yaw) in [("tank", 0.7), ("supply", -2.9)] {
        let b = battle(
            FLAT,
            json!([{ "side": "blue", "kind": kind, "position": [1300, 1300], "yaw": yaw }]),
        );
        let unit = b.unit(UnitId(0)).unwrap();
        let [front, side, rear] = shape(kind);
        let range = base_range(kind);
        let at = |rel: f64| sight_range(unit, &rules, yaw + rel);
        assert!((at(0.0) - range * front).abs() < 1e-9, "{kind} ahead");
        assert!((at(FRAC_PI_2) - range * side).abs() < 1e-9, "{kind} left");
        assert!((at(-FRAC_PI_2) - range * side).abs() < 1e-9, "{kind} right");
        assert!((at(PI) - range * rear).abs() < 1e-9, "{kind} astern");
        // The same bearing reached either way round, and no seam astern.
        assert!((at(PI - 1e-6) - at(-PI + 1e-6)).abs() < 1e-3, "{kind} wrap");
        assert!((at(0.3) - at(0.3 + 2.0 * PI)).abs() < 1e-9, "{kind} turn");
        // Continuous through the side boundary, and never rising away from
        // the front.
        assert!((at(FRAC_PI_2 - 1e-6) - at(FRAC_PI_2 + 1e-6)).abs() < 1e-3);
        let mut last = f64::INFINITY;
        for k in 0..=180 {
            let r = at((k as f64).to_radians());
            assert!(r <= last + 1e-9, "{kind} rises at {k}°");
            assert!(
                (r - at(-(k as f64).to_radians())).abs() < 1e-9,
                "{kind} mirror {k}°"
            );
            last = r;
        }
    }
}

#[test]
fn infantry_sight_is_isotropic_whatever_the_yaw() {
    let rules = rules();
    for kind in ["rifle", "recon", "at"] {
        let b = battle(
            FLAT,
            json!([{ "side": "blue", "kind": kind, "position": [1300, 1300], "yaw": 1.1 }]),
        );
        let unit = b.unit(UnitId(0)).unwrap();
        for k in 0..36 {
            let r = sight_range(unit, &rules, (k as f64 * 10.0).to_radians());
            assert!((r - base_range(kind)).abs() < 1e-9, "{kind} at {}°", k * 10);
        }
        let sight = own(&b, Side::Blue, 0).sight;
        assert_eq!(
            [sight.shape.front, sight.shape.side, sight.shape.rear],
            [1.0; 3]
        );
    }
    // A squad identifies a tank just inside its range behind it as well as ahead.
    let r = base_range("rifle");
    for dx in [r - 5.0, -(r - 5.0)] {
        let b = battle(
            FLAT,
            json!([
                { "side": "blue", "kind": "rifle", "position": [1300, 1300] },
                { "side": "red", "kind": "tank", "position": [1300.0 + dx, 1300] },
            ]),
        );
        assert_eq!(b.observe(Side::Blue).identified.len(), 1, "dx {dx}");
    }
}

#[test]
fn a_vehicle_looks_along_its_turret_and_a_truck_along_its_hull() {
    // The tank faces east. A squad 120 m north is inside its side reach, so
    // it locks on and the turret swings north; a second tank 300 m north is
    // outside the side reach and inside the front reach.
    let side_reach = base_range("tank") * shape("tank")[1];
    let front_reach = base_range("tank") * shape("tank")[0];
    assert!(side_reach < 300.0 && 300.0 < front_reach - 20.0);
    let mut b = battle(
        FLAT,
        json!([
            { "side": "blue", "kind": "tank", "position": [1300, 1300], "yaw": 0 },
            { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [1300, 1600], "engagement": "return_fire_only" },
        ]),
    );
    let far_seen = |b: &Battle| own(b, Side::Blue, 0).sees.len() == 2;
    assert!(
        !far_seen(&b),
        "the far tank is abeam of the hull-forward turret"
    );
    assert!(own(&b, Side::Blue, 0).sight.forward.abs() < 1e-9);
    let mut turned = None;
    for t in 0..600 {
        b.step();
        if far_seen(&b) {
            turned = Some(t);
            break;
        }
    }
    assert!(turned.is_some(), "the turret brings the far tank into view");
    let tank = own(&b, Side::Blue, 0);
    // Swung from east toward the squad in the north, not past it.
    assert!(
        0.5 < tank.sight.forward && tank.sight.forward < FRAC_PI_2 + 0.05,
        "{}",
        tank.sight.forward
    );
    assert!(tank.yaw.abs() < 1e-9, "the hull never turned");

    let truck = battle(
        FLAT,
        json!([{ "side": "blue", "kind": "supply", "position": [1300, 1300], "yaw": 2.0 }]),
    );
    assert!((own(&truck, Side::Blue, 0).sight.forward - 2.0).abs() < 1e-9);
}

/// Every fog cell centre around `eye` at `bearing`, `margin` metres inside and
/// outside the directional range, as (inside, outside) visibility.
fn fog_at(b: &Battle, centre: [f64; 2], bearing: f64, reach: f64, margin: f64) -> (bool, bool) {
    let fog = &b.observe(Side::Blue).ground_visibility;
    let at = |d: f64| fog.visible(centre[0] + bearing.cos() * d, centre[1] + bearing.sin() * d);
    (at(reach - margin), at(reach + margin))
}

#[test]
fn sight_shape_consumers_agree() {
    // One tank on open, flat ground; red tanks placed around it just inside
    // and just outside its directional range. Spotting and the fog sweep
    // follow the same shape, and the publication carries it.
    let rules = rules();
    let yaw = 0.4;
    let centre = [1300.0, 1300.0];
    let cell = common::game()["map"]["fog_cell_m"].as_f64().unwrap();
    let observer = battle(
        FLAT,
        json!([{ "side": "blue", "kind": "tank", "position": centre, "yaw": yaw }]),
    );
    let unit = observer.unit(UnitId(0)).unwrap();
    let published = own(&observer, Side::Blue, 0).sight;
    assert_eq!(published.forward, yaw);
    assert_eq!(published.range, base_range("tank"));
    let [front, side, rear] = shape("tank");
    assert_eq!(
        [
            published.shape.front,
            published.shape.side,
            published.shape.rear
        ],
        [front, side, rear]
    );
    for k in 0..16 {
        let bearing = yaw + k as f64 * PI / 8.0;
        let reach = sight_range(unit, &rules, bearing);
        for (d, seen) in [(reach - 3.0, true), (reach + 3.0, false)] {
            let at = [centre[0] + bearing.cos() * d, centre[1] + bearing.sin() * d];
            let b = battle(
                FLAT,
                json!([
                    { "side": "blue", "kind": "tank", "position": centre, "yaw": yaw },
                    { "side": "red", "kind": "tank", "position": at },
                ]),
            );
            assert_eq!(
                b.observe(Side::Blue).identified.len() == 1,
                seen,
                "spotting at {}°, {d:.0} m of {reach:.0}",
                k * 360 / 16
            );
        }
        let (inside, outside) = fog_at(&observer, centre, bearing, reach, 2.0 * cell);
        assert!(inside, "fog clear inside {reach:.0} m at {}°", k * 360 / 16);
        assert!(!outside, "fogged past {reach:.0} m at {}°", k * 360 / 16);
    }
}

#[test]
fn a_garrison_sees_from_one_eye_per_facade_it_holds() {
    // garrison-lab's one building: 24 m square round (360, 250), yaw 0, on
    // flat ground.
    let map = common::saved_map("garrison");
    let mut b = battle(
        map,
        json!([
            { "side": "blue", "kind": "rifle", "position": [300, 250], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "tank", "position": [200, 250] },
        ]),
    );
    let eye = |key: &str| common::game()["physics"][key].as_f64().unwrap();
    let tank = own(&b, Side::Blue, 1);
    assert_eq!(
        tank.sight.eyes,
        vec![[
            tank.position[0],
            tank.position[1],
            tank.position[2] + common::hull("tank").eye_m
        ]]
    );
    let ack = b.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: serde_json::from_value(json!({ "kind": "garrison", "units": [0], "building": 0 }))
            .unwrap(),
        queued: false,
    });
    assert_eq!(ack.error, None);
    for _ in 0..1200 {
        b.step();
        if own(&b, Side::Blue, 0)
            .garrison
            .is_some_and(|g| g.phase == contract::observation::GarrisonPhase::Inside)
        {
            break;
        }
    }
    // Let the fog sweep run from inside.
    for _ in 0..30 {
        b.step();
    }
    let squad = own(&b, Side::Blue, 0);
    assert!(squad.garrison.is_some());
    let standoff = common::game()["garrison"]["slot_standoff_m"]
        .as_f64()
        .unwrap();
    let out = 12.0 + standoff;
    // Facades in order +x, +y, -x, -y: each occupied facade gives one
    // exterior eye; the published soldier body stands just inside it.
    let facades = [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]];
    assert_eq!(
        squad.sight.eyes.len(),
        4,
        "the squad holds all four directions"
    );
    for eye_point in &squad.sight.eyes {
        let normal = facades
            .iter()
            .find(|n| {
                (eye_point[0] - (360.0 + n[0] * out)).abs() < 1e-9 && n[0] != 0.0
                    || (eye_point[1] - (250.0 + n[1] * out)).abs() < 1e-9 && n[1] != 0.0
            })
            .expect("an eye belongs to an exterior facade");
        assert!(
            squad.members.iter().any(|m| {
                (eye_point[0] - (m[0] + normal[0] * 2.0 * standoff)).abs() < 1e-9
                    && (eye_point[1] - (m[1] + normal[1] * 2.0 * standoff)).abs() < 1e-9
                    && eye_point[2] == m[2] + eye("infantry_eye_m")
            }),
            "each exterior eye belongs to a living soldier inside its facade"
        );
    }
    assert_eq!(squad.sight.range, base_range("rifle"));
    // The side's fog is the union of those eyes: open ground 30 m straight
    // out from every facade is seen.
    let fog = &b.observe(Side::Blue).ground_visibility;
    for n in facades {
        let (x, y) = (360.0 + n[0] * (out + 30.0), 250.0 + n[1] * (out + 30.0));
        assert!(fog.visible(x, y), "seen 30 m out of facade {n:?}");
    }
}

#[test]
fn a_turning_turret_replays_to_identical_digests() {
    let setup = common::scenario(
        FLAT,
        json!([
            { "side": "blue", "kind": "tank", "position": [1300, 1300] },
            { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            { "side": "blue", "kind": "supply", "position": [1200, 1200] },
        ]),
        json!([]),
    );
    let mut live = Battle::new(&setup, 9);
    let mut digests = Vec::new();
    for _ in 0..240 {
        live.step();
        digests.push(live.digest());
    }
    let mut replay = Battle::from_replay(&setup, &live.replay()).unwrap();
    for d in digests {
        replay.step();
        assert_eq!(replay.digest(), d);
    }
}

/// The optics' mount is data (`sensors.on`): the jeep's sight stays on its
/// hull as shipped, and turns with its pedestal HMG once its type says the
/// optics sit there.
#[test]
fn a_jeeps_sight_turns_with_its_hmg_only_when_its_sensors_sit_on_it() {
    let run = |on: Option<&str>| {
        let mut rules = common::game();
        if let Some(on) = on {
            sim::fixtures::patch_catalog(
                &mut rules,
                "units",
                "jeep",
                json!({ "sensors": { "on": on, "sight_shape": { "front": 1.0, "side": 0.6, "rear": 0.4 } } }),
            );
        }
        let map: serde_json::Value = serde_json::from_str(FLAT).unwrap();
        let setup = serde_json::from_value(json!({
            "map": map, "rules": rules, "events": [], "scripts": [],
            "units": [
                { "side": "blue", "kind": "jeep", "position": [1300, 1300], "yaw": 0 },
                { "side": "red", "kind": "rifle", "position": [1300, 1420], "engagement": "return_fire_only" },
            ],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        for _ in 0..90 {
            b.step();
        }
        let jeep = own(&b, Side::Blue, 0);
        (jeep.yaw, jeep.sight.forward)
    };
    let (yaw, forward) = run(None);
    assert!(
        (forward - yaw).abs() < 1e-9,
        "on its hull: {forward} vs {yaw}"
    );
    let (yaw, forward) = run(Some("HMG"));
    assert!(yaw.abs() < 1e-9, "the hull never turned");
    assert!(
        forward > 0.5,
        "the sight swung north with the HMG toward the squad: {forward}"
    );
}

/// Ground visibility follows live body changes even when a previously visited
/// tile is used again, and when the changed footprint crosses a tile boundary.
#[test]
fn a_fog_sweep_tracks_added_moved_and_removed_occluders() {
    use sim::math::{v2, v3};
    use sim::visibility::{self, OcclusionGrid};
    let mut world = common::flat([200.0; 2], "");
    let rules = common::rules();
    let mut grid = OcclusionGrid::new(&world, 8.0);
    let sight = sim::sight::Sight {
        forward: 0.0,
        shape: contract::scenario::SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 160.0,
    };
    let sweep = |world: &sim::world::WorldGeometry, grid: &mut OcclusionGrid| {
        let mut field = grid.field();
        visibility::sweep(
            world,
            grid,
            &rules.sensors,
            v3(32.0, 100.0, 1.8),
            &sight,
            &mut field,
        );
        field
    };
    assert!(sweep(&world, &mut grid).visible(120.0, 100.0));
    let prop = world.add_prop(
        &serde_json::from_value(json!({
            "kind": "wall", "center": [68, 100], "yaw": 0,
            "half_extents": [8, 24, 6]
        }))
        .unwrap(),
    );
    assert!(!sweep(&world, &mut grid).visible(120.0, 100.0));
    world.move_prop(prop, v2(68.0, 180.0), 0.4, 1);
    assert!(sweep(&world, &mut grid).visible(120.0, 100.0));
    world.move_prop(prop, v2(100.0, 100.0), 0.2, 2);
    let blocked = sweep(&world, &mut grid);
    assert!(!blocked.visible(160.0, 100.0));
    assert_eq!(
        blocked.bits,
        sweep(&world, &mut OcclusionGrid::new(&world, 8.0)).bits
    );
    world.remove_prop(prop);
    assert!(sweep(&world, &mut grid).visible(160.0, 100.0));
}

#[test]
fn overlapping_eyes_mark_exactly_the_union_of_their_separate_fog_sweeps() {
    use sim::math::v3;
    use sim::visibility::{self, OcclusionGrid};
    let world = common::flat(
        [400.0; 2],
        r#",
        "relief":[{"kind":"ridge","center":[280,160],"radius_m":60,"peak_m":10}],
        "forests":[{"shape":{"kind":"polygon","ring":[[160,200],[240,200],[240,300],[160,300]]}}],
        "props":[{"kind":"wall","center":[100,120],"yaw":0.2,"half_extents":[8,30,4]}]"#,
    );
    let rules = common::rules();
    let mut grid = OcclusionGrid::new(&world, 8.0);
    let mut joint = grid.field();
    let mut union = grid.field();
    for (at, forward) in [
        ([60.0, 80.0], 0.0),
        ([92.0, 88.0], 0.4),
        ([150.0, 240.0], -0.5),
    ] {
        let sight = sim::sight::Sight {
            forward,
            shape: contract::scenario::SightShape {
                front: 1.0,
                side: 0.7,
                rear: 0.4,
            },
            range: 120.0,
        };
        let eye = v3(at[0], at[1], world.height_at(at[0], at[1]).unwrap() + 1.8);
        let mut separate = grid.field();
        visibility::sweep(
            &world,
            &mut grid,
            &rules.sensors,
            eye,
            &sight,
            &mut separate,
        );
        for (bits, own) in union.bits.iter_mut().zip(separate.bits) {
            *bits |= own;
        }
        visibility::sweep(&world, &mut grid, &rules.sensors, eye, &sight, &mut joint);
    }
    assert_eq!(joint.bits, union.bits);
}

/// A distant prop update must not rebuild all cached occlusion around an eye.
#[cfg(target_os = "macos")]
#[test]
fn distant_body_changes_do_not_repeat_active_fog_raster_work() {
    if !common::isolated_cost_test(
        "sight::distant_body_changes_do_not_repeat_active_fog_raster_work",
    ) {
        return;
    }
    use sim::math::{v2, v3};
    use sim::visibility::{self, OcclusionGrid};
    let mut world = common::flat([2000.0; 2], "");
    // Dense small bodies stress the index's candidate sorting, but are too thin
    // to cover fog-cell centres and alter this eye's ray traversal.
    for y in 0..30 {
        for x in 0..30 {
            world.add_prop(
                &serde_json::from_value(json!({
                    "kind":"wall", "center":[200.0+x as f64*8.0,200.0+y as f64*8.0],
                    "yaw":0.2,"half_extents":[0.1,0.1,2.0]
                }))
                .unwrap(),
            );
        }
    }
    let far = world.add_prop(
        &serde_json::from_value(json!({
            "kind":"wall","center":[1600,1600],"yaw":0,"half_extents":[5,5,3]
        }))
        .unwrap(),
    );
    let rules = common::rules();
    let mut grid = OcclusionGrid::new(&world, 8.0);
    let sight = sim::sight::Sight {
        forward: 0.0,
        shape: contract::scenario::SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 240.0,
    };
    let sweep = |world: &sim::world::WorldGeometry, grid: &mut OcclusionGrid| {
        let mut field = grid.field();
        visibility::sweep(
            world,
            grid,
            &rules.sensors,
            v3(320.0, 320.0, 1.8),
            &sight,
            &mut field,
        );
        field
    };
    let expected = sweep(&world, &mut grid).bits;
    let before = common::counters::instructions().unwrap();
    for _ in 0..8 {
        assert_eq!(sweep(&world, &mut grid).bits, expected);
    }
    let steady = common::counters::instructions().unwrap() - before;
    let mut changing = 0;
    for tick in 1..=8 {
        world.move_prop(far, v2(1600.0 + tick as f64, 1600.0), 0.1, tick);
        let before = common::counters::instructions().unwrap();
        assert_eq!(sweep(&world, &mut grid).bits, expected);
        changing += common::counters::instructions().unwrap() - before;
    }
    assert!(steady > 0);
    assert!(
        changing < steady * 2,
        "distant changes: {changing} vs steady {steady}"
    );
}

/// Retained fog rasters match fresh sweeps through changed heights, old and
/// new moved footprints, removed overlapping bodies and partial edge tiles.
#[test]
fn locally_invalidated_fog_matches_fresh_sweeps() {
    use sim::math::{v2, v3};
    use sim::visibility::{self, OcclusionGrid};
    let mut world = common::flat(
        [253.0, 237.0],
        r#",
        "props":[{"kind":"wall","center":[64,64],"yaw":0.3,"half_extents":[8,28,4]},
                 {"kind":"wall","center":[70,68],"yaw":-0.1,"half_extents":[13,18,2]}]"#,
    );
    let mut grid = OcclusionGrid::new(&world, 8.0);
    let rules = common::rules();
    let sight = sim::sight::Sight {
        forward: 0.0,
        shape: contract::scenario::SightShape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 320.0,
    };
    let assert_same = |world: &sim::world::WorldGeometry, grid: &mut OcclusionGrid| {
        let mut fresh = OcclusionGrid::new(world, 8.0);
        for eye in [
            v3(16.0, 64.0, 1.8),
            v3(128.0, 24.0, 3.0),
            v3(232.0, 208.0, 1.8),
            v3(32.0, 144.0, 12.0),
        ] {
            let mut retained = grid.field();
            let mut rebuilt = fresh.field();
            visibility::sweep(world, grid, &rules.sensors, eye, &sight, &mut retained);
            visibility::sweep(world, &mut fresh, &rules.sensors, eye, &sight, &mut rebuilt);
            assert_eq!(retained.bits, rebuilt.bits, "eye {eye:?}");
        }
    };
    // Authored setup is revision zero, before tracking begins.
    assert_same(&world, &mut grid);
    world.remove_prop(0);
    assert_same(&world, &mut grid);
    let mut prop = None;
    for case in 0..32 {
        let at = [
            24.0 + (case * 61 % 210) as f64,
            16.0 + (case * 43 % 210) as f64,
        ];
        match case % 4 {
            0 => {
                prop = Some(
                    world.add_prop(
                        &serde_json::from_value(json!({
                            "kind":"wall","center":at,"yaw":case as f64*0.37,
                            "half_extents":[8+case%13,13+case%17,2+case%9],"base_z":case%3
                        }))
                        .unwrap(),
                    ),
                );
            }
            1 => world.move_prop(
                prop.unwrap(),
                v2(at[0], at[1]),
                case as f64 * 0.23,
                case as u64,
            ),
            2 => {
                // A replacement can change height while overlapping the old
                // footprint. Check the empty interval as well as the new body.
                let was = world.prop(prop.unwrap()).unwrap().center;
                world.remove_prop(prop.take().unwrap());
                assert_same(&world, &mut grid);
                prop = Some(
                    world.add_prop(
                        &serde_json::from_value(json!({
                            "kind":"wall","center":[was.x,was.y],"yaw":0.2,
                            "half_extents":[20,25,1+case%7],"base_z":4
                        }))
                        .unwrap(),
                    ),
                );
            }
            _ => {
                world.remove_prop(prop.take().unwrap());
            }
        }
        assert_same(&world, &mut grid);
    }
}

/// Overlapping observers must not repeatedly canonicalize the same dense town.
#[cfg(target_os = "macos")]
#[test]
fn overlapping_observers_share_fog_candidate_collection_work() {
    if !common::isolated_cost_test(
        "sight::overlapping_observers_share_fog_candidate_collection_work",
    ) {
        return;
    }
    let mut props = Vec::new();
    for y in 0..100 {
        for x in 0..100 {
            props.push(
                json!({"kind":"wall","center":[200.3+x as f64*8.0,200.3+y as f64*8.0],
                "yaw":0,"half_extents":[0.1,0.1,2.0]}),
            );
        }
    }
    let map = json!({"size":[1600,1600],"fog_cell_m":32,"height_grid_m":4,
        "slope_cutoff_deg":35,"props":props});
    let units: Vec<_> = (0..12)
        .map(|i| {
            json!({"side":"blue","kind":"rifle",
        "position":[560+(i%3)*20,560+(i/3)*20],"engagement":"return_fire_only"})
        })
        .collect();
    let setup = common::scenario(&map.to_string(), json!(units), json!([]));
    let mut battle = Battle::new(&setup, 1);
    let mut collection = 0;
    for _ in 0..6 {
        let mut previous = common::counters::instructions().unwrap();
        battle.step_profiled(|phase| {
            let now = common::counters::instructions().unwrap();
            if matches!(
                phase,
                sim::battle::TickPhase::Fog | sim::battle::TickPhase::Learning
            ) {
                collection += now - previous;
            }
            previous = now;
        });
    }
    eprintln!("whole Fog+Learning instructions: {collection}");
    assert!(
        collection < 60_000_000,
        "Fog+Learning repeated collection: {collection}"
    );
}

/// All observers contribute body knowledge; unseen bodies remain unknown.
#[test]
fn separate_observers_learn_bodies_without_leaking_between_views() {
    let map = r#"{"size":[2200,800],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35}"#;
    let setup = common::scenario(
        map,
        json!([
            {"side":"blue","kind":"rifle","position":[200,400],"engagement":"return_fire_only"},
            {"side":"blue","kind":"rifle","position":[1800,400],"engagement":"return_fire_only"}
        ]),
        json!([
            {"tick":1,"add_prop":{"kind":"wall","center":[100,400],"yaw":0,"half_extents":[45,4,2]}},
            {"tick":1,"add_prop":{"kind":"wall","center":[2000,400],"yaw":0,"half_extents":[45,4,2]}},
            {"tick":1,"add_prop":{"kind":"wall","center":[1000,400],"yaw":0,"half_extents":[3,3,2]}}
        ]),
    );
    let mut battle = Battle::new(&setup, 1);
    let mut digests = Vec::new();
    for _ in 0..12 {
        battle.step();
        digests.push(battle.digest());
    }
    assert_eq!(
        battle
            .observe(Side::Blue)
            .known_props
            .iter()
            .map(|p| p.center)
            .collect::<Vec<_>>(),
        vec![[100.0, 400.0], [2000.0, 400.0]]
    );
    assert!(battle.observe(Side::Red).known_props.is_empty());
    let mut grid = sim::visibility::OcclusionGrid::new(battle.world(), 8.0);
    let mut fresh = grid.field();
    for id in [UnitId(0), UnitId(1)] {
        let u = battle.unit(id).unwrap();
        let sight = sim::sight::of(u, battle.rules());
        for eye in sim::sensing::eyes(u, battle.rules()) {
            sim::visibility::sweep(
                battle.world(),
                &mut grid,
                &battle.rules().sensors,
                eye,
                &sight,
                &mut fresh,
            );
        }
    }
    assert_eq!(
        battle.observe(Side::Blue).ground_visibility.bits,
        fresh.bits
    );
    let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
    for expected in digests {
        replay.step();
        assert_eq!(replay.digest(), expected);
    }
}
