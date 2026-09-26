//! Infantry cover (slice 33): tiers through spread only, for soldiers only
//! (Q5, Q20); spots behind bodies on the far side from the threat (Q7);
//! step-out round a corner (D3); a re-resolve at most once a second (Q11).
use contract::ids::{Side, UnitId};
use contract::scenario::{CoverTier, Rules};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::cover::{self, Body, Known};
use sim::ground::GroundLayer;
use sim::math::{v2, Obb2, V2};
use sim::world::{Prop, WorldGeometry};

mod common;

fn rules() -> Rules {
    serde_json::from_value(common::village()).unwrap()
}

fn map(props: Value) -> String {
    json!({ "size": [200, 120], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

fn world(props: Value) -> WorldGeometry {
    WorldGeometry::new(&serde_json::from_str(&map(props)).unwrap())
}

fn wall(center: [f64; 2], half: [f64; 3]) -> Value {
    json!({ "kind": "wall", "center": center, "yaw": 0, "half_extents": half })
}

fn crate_at(center: [f64; 2]) -> Value {
    json!({ "kind": "crate", "center": center, "yaw": 0, "half_extents": [0.8, 0.8, 0.6] })
}

#[test]
fn a_body_covers_a_soldier_only_from_its_far_side_and_within_reach() {
    let r = rules();
    let w = world(json!([crate_at([100.0, 60.0])]));
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let at = |p: V2, from: V2| cover::at(&w, &ground, &[], &r, p, from);
    // Just west of the crate, shot at from the east: behind it.
    let behind = v2(98.7, 60.0);
    assert_eq!(at(behind, v2(160.0, 60.0)), Some(CoverTier::Light));
    assert_eq!(
        at(behind, v2(160.0, 75.0)),
        Some(CoverTier::Light),
        "shot from a little north of east"
    );
    // The same place shot at from the west, north or south: in the open.
    for from in [v2(40.0, 60.0), v2(98.7, 110.0), v2(98.7, 10.0)] {
        assert_eq!(at(behind, from), None, "from {from:?}");
    }
    // Beyond reach of the crate: no cover, even straight behind it.
    let far = v2(99.2 - r.cover.reach_m - 0.2, 60.0);
    assert_eq!(at(far, v2(160.0, 60.0)), None);
}

#[test]
fn a_vehicle_covers_by_its_weight_class_and_its_wreck_keeps_the_tier() {
    let r = rules();
    let w = world(json!([
        { "kind": "wreck", "center": [60, 60], "yaw": 0, "half_extents": r.bodies.tank_half_extents_m },
        { "kind": "wreck", "center": [140, 60], "yaw": 0, "half_extents": r.bodies.supply_half_extents_m },
    ]));
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let east = v2(190.0, 60.0);
    let tank_wreck = cover::at(&w, &ground, &[], &r, v2(56.0, 60.0), east);
    let truck_wreck = cover::at(&w, &ground, &[], &r, v2(136.5, 60.0), east);
    assert_eq!(
        tank_wreck,
        cover::vehicle_tier(contract::scenario::UnitKind::Tank, &r.cover)
    );
    assert_eq!(
        truck_wreck,
        cover::vehicle_tier(contract::scenario::UnitKind::Supply, &r.cover)
    );
    assert!(
        tank_wreck > truck_wreck,
        "a tank is heavier cover than a truck"
    );
    // A live hull covers like its wreck.
    let open = world(json!([]));
    let hull = Body {
        rect: Obb2 {
            center: v2(60.0, 60.0),
            yaw: 0.0,
            half: v2(
                r.bodies.tank_half_extents_m[0],
                r.bodies.tank_half_extents_m[1],
            ),
        },
        tier: tank_wreck.unwrap(),
        vehicle: Some(UnitId(0)),
    };
    assert_eq!(
        cover::at(&open, &ground, &[hull], &r, v2(56.0, 60.0), east),
        tank_wreck
    );
}

/// Every spot offered behind `props` against `threat`.
fn spots_behind(props: Value, threat: V2) -> (WorldGeometry, Vec<cover::Spot>) {
    let r = rules();
    let w = world(props);
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let known_ground = sim::ground::KnownGround::new(&ground);
    let knows = |_: &Prop| true;
    let known = Known::gather(&w, &known_ground, &r, &[], &knows, v2(100.0, 60.0), 30.0);
    let radius = r.bodies.soldier_radius_m;
    let stands = |_: V2| true;
    let spots = cover::spots(&known, threat, &r.cover, radius, 2.0, &stands);
    (w, spots)
}

#[test]
fn cover_spots_lie_on_the_far_side_of_each_body_from_the_threat() {
    let props = json!([wall([100.0, 60.0], [0.4, 4.0, 0.5]), crate_at([90.0, 50.0])]);
    for threat in [v2(180.0, 60.0), v2(20.0, 62.0), v2(170.0, 110.0)] {
        let (w, spots) = spots_behind(props.clone(), threat);
        assert!(!spots.is_empty(), "spots against {threat:?}");
        for s in &spots {
            // The segment from the spot toward the threat meets a body first.
            let toward = (threat - s.at).normalized();
            let hits = w
                .props()
                .any(|p| p.footprint().meets_segment(s.at, s.at + toward * 2.0, 0.3));
            assert!(
                hits,
                "spot {:?} against {threat:?} hides behind nothing",
                s.at
            );
        }
    }
    // Spots spread along a face, at least the spacing apart.
    let (_, spots) = spots_behind(
        json!([wall([100.0, 60.0], [0.4, 4.0, 0.5])]),
        v2(180.0, 60.0),
    );
    assert!(spots.len() >= 4, "an 8 m wall holds several: {spots:?}");
    for (i, a) in spots.iter().enumerate() {
        assert!(a.at.x < 100.0, "west of the wall: {:?}", a.at);
        for b in &spots[i + 1..] {
            assert!((a.at - b.at).length() >= 2.0 - 1e-9);
        }
    }
}

#[test]
fn claims_give_each_spot_to_one_soldier_best_tier_first_and_keep_those_already_covered() {
    let spot = |x: f64, tier| cover::Spot {
        at: v2(x, 0.0),
        tier,
    };
    let spots = [spot(0.0, CoverTier::Light), spot(4.0, CoverTier::Heavy)];
    // Both soldiers could reach both: the nearer to the heavy spot takes it,
    // the other the light one; nobody shares.
    let from = [v2(1.0, 0.0), v2(3.0, 0.0)];
    let claims = cover::claim(&from, &[None, None], &spots, 8.0, 2.0, 0.0);
    assert_eq!(claims, vec![Some(0), Some(1)]);
    // A soldier already in heavy cover stays (no shuffle); the other takes
    // what is left rather than displacing him.
    let stay = [Some(Some(CoverTier::Heavy)), Some(None)];
    let claims = cover::claim(&from, &stay, &spots, 8.0, 2.0, 0.0);
    assert_eq!(claims, vec![None, Some(1)]);
    // Out of search range: nobody claims, whoever it is scatters.
    let claims = cover::claim(&[v2(30.0, 0.0)], &[None], &spots, 8.0, 2.0, 0.0);
    assert_eq!(claims, vec![None]);
}

#[test]
fn a_soldier_whose_line_is_blocked_steps_out_round_the_nearest_corner() {
    let r = rules();
    // A tall wall along y = 40..50 at x = 62; the enemy north-east.
    let w = world(json!([wall([62.0, 45.0], [0.4, 5.0, 1.5])]));
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let known_ground = sim::ground::KnownGround::new(&ground);
    let knows = |_: &Prop| true;
    let known = Known::gather(&w, &known_ground, &r, &[], &knows, v2(60.0, 45.0), 20.0);
    let target = v2(100.0, 72.0);
    let clear = |p: V2| {
        w.segment_clear(
            p.with_z(r.bodies.infantry_muzzle_m),
            target.with_z(sim::weapons::SOLDIER_AIM_M),
        )
    };
    let stands = |p: V2| !w.props().any(|q| q.footprint().contains(p, 0.3));
    let from = v2(61.1, 48.0);
    assert!(!clear(from), "the wall blocks him");
    let radius = r.bodies.soldier_radius_m;
    let out = cover::step_out(from, target, &known, &r.cover, radius, &stands, &clear)
        .expect("a clear place round the north end");
    assert!(clear(out));
    assert!((out - from).length() <= r.cover.step_out_m + 1e-9);
    assert!(out.x < 62.0, "he stays on his side of the wall: {out:?}");
    // Deep behind the middle of a long wall, nothing within reach: he sits out.
    let long = world(json!([wall([62.0, 45.0], [0.4, 20.0, 1.5])]));
    let clear_long = |p: V2| {
        long.segment_clear(
            p.with_z(r.bodies.infantry_muzzle_m),
            target.with_z(sim::weapons::SOLDIER_AIM_M),
        )
    };
    let stands_long = |p: V2| !long.props().any(|q| q.footprint().contains(p, 0.3)) && p.x < 62.0;
    assert_eq!(
        cover::step_out(
            v2(61.1, 40.0),
            target,
            &known,
            &r.cover,
            radius,
            &stands_long,
            &clear_long
        ),
        None
    );
}

/// A rifle squad of each side, 60 m apart, a low wall one metre in front
/// of blue (a `blue` unit kind); with `cover` false every tier widens by 1.
fn firefight(cover: bool, blue: &str) -> Battle {
    let mut setup = common::scenario_with(
        &map(json!([wall([66.5, 60.0], [0.4, 12.0, 0.5])])),
        json!([
            { "side": "blue", "kind": blue, "position": [60, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [125, 60] },
        ]),
        json!([]),
        json!([]),
    );
    if !cover {
        let t = &mut setup.rules.cover.tiers;
        (t.light, t.medium, t.heavy) = (1.0, 1.0, 1.0);
    }
    Battle::new(&setup, 3)
}

/// Every round launched at blue in its first volley: velocity by id.
fn first_volley(mut b: Battle) -> Vec<(u64, [f64; 3])> {
    for _ in 0..30 * 20 {
        b.step();
        let rounds: Vec<_> = b
            .rounds()
            .filter(|(_, r)| r.side == Side::Red)
            .map(|(p, _)| (p.id.0, [p.velocity.x, p.velocity.y, p.velocity.z]))
            .collect();
        if !rounds.is_empty() {
            return rounds;
        }
    }
    panic!("red never fired");
}

#[test]
fn cover_widens_the_spread_of_rounds_at_soldiers_and_never_at_a_vehicle() {
    // The same wall and seed; only the tiers' multipliers differ.
    // Rifles on a tank behind it: nothing changes (Q20).
    assert_eq!(
        first_volley(firefight(false, "tank")),
        first_volley(firefight(true, "tank")),
        "no cover for a vehicle"
    );
    // Rifles on a squad sheltering behind it: wider rounds.
    let bare = first_volley(firefight(false, "rifle"));
    let covered = first_volley(firefight(true, "rifle"));
    assert_eq!(bare.len(), covered.len(), "the same volley");
    assert_ne!(bare, covered, "cover widens the spread");
}

#[test]
fn a_holding_squad_re_resolves_its_cover_at_most_once_a_second() {
    // Craters land beside a squad at rest every few ticks: each is a change
    // of the ground its side sees, so each asks for a re-resolve.
    let events: Vec<Value> = (0..200)
        .map(|k| {
            json!({ "tick": 5 + 3 * k, "burst": { "point": [70.0 + (k % 7) as f64, 50.0 + (k % 5) as f64], "weapon": "tank_he" } })
        })
        .collect();
    let setup = common::scenario_with(
        &map(json!([crate_at([64.0, 60.0])])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [60, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [140, 60], "engagement": "return_fire_only" },
        ]),
        Value::Array(events),
        json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    let mut resolves = Vec::new();
    let mut last = 0;
    for _ in 0..20 * hz {
        b.step();
        let at = b.unit(UnitId(0)).unwrap().cover.resolved_at;
        if at != last {
            resolves.push(at);
            last = at;
        }
    }
    assert!(resolves.len() >= 10, "it keeps re-resolving: {resolves:?}");
    for w in resolves.windows(2) {
        assert!(w[1] - w[0] >= hz, "at most once a second: {resolves:?}");
    }
}

#[test]
fn a_fight_over_cover_replays_to_the_same_digest() {
    let setup = common::scenario_with(
        &map(json!([
            wall([64.0, 45.0], [0.4, 5.0, 0.6]),
            crate_at([57.0, 38.0]),
            crate_at([57.0, 52.0])
        ])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [58, 45] },
            { "side": "red", "kind": "rifle", "position": [110, 45] },
            { "side": "blue", "kind": "tank", "position": [50, 70], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 150, "remove_prop": { "at": [64.0, 45.0] } }]),
        json!([{ "tick": 60, "side": "blue", "order": { "kind": "move", "units": [0], "gesture": 1,
            "goal": [52.0, 66.0], "route": "shortest" } }]),
    );
    let mut live = Battle::new(&setup, 9);
    let mut digests = Vec::new();
    for _ in 0..600 {
        live.step();
        digests.push(live.digest());
    }
    let mut replayed = Battle::from_replay(&setup, &live.replay()).unwrap();
    for (i, want) in digests.iter().enumerate() {
        replayed.step();
        assert_eq!(replayed.digest(), *want, "tick {}", i + 1);
    }
}
