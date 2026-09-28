//! Infantry cover (slice 33): tiers through spread only, for soldiers only
//! (Q5, Q20); spots behind bodies on the far side from the threat (Q7);
//! step-out round a corner (D3); a re-resolve at most once a second (Q11).
use contract::ids::{Side, UnitId};
use contract::scenario::{CoverTier, Rules};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::cover::{self, Body, Known, Place};
use sim::ground::GroundLayer;
use sim::lean::{Lean, LeanSide, Round};
use sim::math::{v2, Obb2, V2};
use sim::world::{Prop, WorldGeometry};

use crate::common;

fn rules() -> Rules {
    serde_json::from_value(common::village()).unwrap()
}

fn map(props: Value) -> String {
    json!({ "size": [200, 120], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

fn world(props: Value) -> WorldGeometry {
    WorldGeometry::new(
        &serde_json::from_str(&map(props)).unwrap(),
        &common::rules(),
    )
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
        { "kind": "tank_wreck", "center": [60, 60], "yaw": 0, "half_extents": common::hull("tank").half_extents_m },
        { "kind": "supply_wreck", "center": [140, 60], "yaw": 0, "half_extents": common::hull("supply").half_extents_m },
    ]));
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let east = v2(190.0, 60.0);
    let tank_wreck = cover::at(&w, &ground, &[], &r, v2(56.0, 60.0), east);
    let truck_wreck = cover::at(&w, &ground, &[], &r, v2(136.5, 60.0), east);
    assert_eq!(
        tank_wreck,
        cover::weight_tier(common::hull("tank").weight_class)
    );
    assert_eq!(
        truck_wreck,
        cover::weight_tier(common::hull("supply").weight_class)
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
                common::hull("tank").half_extents_m[0],
                common::hull("tank").half_extents_m[1],
            ),
        },
        tier: tank_wreck.unwrap(),
        vehicle: Some(UnitId(0)),
        prop: None,
        top: 2.4,
        ground: false,
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
    let radius = r.physics.soldier_radius_m;
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

/// A place at `x` on a line, with its cover and how a soldier fights there.
fn place(x: f64, tier: Option<CoverTier>, direct: bool) -> Place {
    Place {
        at: v2(x, 0.0),
        tier,
        direct,
        leans: Vec::new(),
    }
}

/// A lean from `from` out to `at`, round prop 0.
fn lean_to(from: V2, at: V2) -> Lean {
    Lean {
        from,
        at,
        side: LeanSide::Left,
        body: Round::Prop(0),
    }
}

#[test]
fn the_squad_claims_the_most_fighting_places_then_the_strongest_cover() {
    use CoverTier::{Heavy, Light};
    // A light spot a soldier can fight from, and a heavy one he can't:
    // fighting comes first, so the nearer man takes the light spot and the
    // other the heavy one; nobody shares.
    let spots = [
        place(0.0, Some(Light), true),
        place(4.0, Some(Heavy), false),
    ];
    let from = [v2(1.0, 0.0), v2(3.0, 0.0)];
    let claims = cover::claim(
        &from,
        &[None, None],
        &spots,
        2.0,
        0.0,
        common::rules().cover.lean_apart_m,
    );
    let taken: Vec<_> = claims.iter().map(|c| c.and_then(|c| c.spot)).collect();
    assert_eq!(taken, vec![Some(0), Some(1)]);
    assert_eq!(
        claims
            .iter()
            .map(|c| c.unwrap().engages)
            .collect::<Vec<_>>(),
        vec![true, false]
    );
    // A soldier already fighting from heavy cover stays (no shuffle); the
    // other takes what is left rather than displacing him.
    let from = [v2(6.0, 0.0), v2(3.0, 0.0)];
    let stay = [
        Some(Place {
            at: from[0],
            ..place(0.0, Some(Heavy), true)
        }),
        Some(Place {
            at: from[1],
            ..place(0.0, None, false)
        }),
    ];
    let claims = cover::claim(
        &from,
        &stay,
        &spots,
        2.0,
        0.0,
        common::rules().cover.lean_apart_m,
    );
    let taken: Vec<_> = claims.iter().map(|c| c.and_then(|c| c.spot)).collect();
    assert_eq!(taken, vec![None, Some(0)], "{claims:?}");
    // Walking breaks a tie: two equal spots go to the nearer man each.
    let spots = [
        place(0.0, Some(Light), true),
        place(10.0, Some(Light), true),
    ];
    let from = [v2(9.0, 0.0), v2(1.0, 0.0)];
    let claims = cover::claim(
        &from,
        &[None, None],
        &spots,
        2.0,
        0.0,
        common::rules().cover.lean_apart_m,
    );
    let taken: Vec<_> = claims.iter().map(|c| c.and_then(|c| c.spot)).collect();
    assert_eq!(taken, vec![Some(1), Some(0)]);
}

#[test]
fn a_lean_point_is_claimed_by_one_soldier() {
    // Two spots behind tall cover whose only way to fight is the same lean
    // point: one man claims it and fights; the other takes cover he can't
    // fight from (his step-out comes after).
    let edge = v2(2.0, 1.0);
    let spots: Vec<Place> = [1.0, 3.0]
        .into_iter()
        .map(|x| Place {
            leans: vec![lean_to(v2(x, 0.0), edge)],
            ..place(x, Some(CoverTier::Heavy), false)
        })
        .collect();
    let from = [v2(1.0, -3.0), v2(3.0, -3.0)];
    let claims = cover::claim(
        &from,
        &[None, None],
        &spots,
        2.0,
        0.0,
        common::rules().cover.lean_apart_m,
    );
    let leaning: Vec<_> = claims
        .iter()
        .filter(|c| c.unwrap().lean.is_some())
        .collect();
    assert_eq!(leaning.len(), 1, "{claims:?}");
    assert_eq!(leaning[0].unwrap().lean.unwrap().at, edge);
    assert_eq!(
        claims.iter().filter(|c| c.unwrap().engages).count(),
        1,
        "{claims:?}"
    );
}

/// Sandbags shot to a rubble strip: the squad that lined them stands a
/// step off the strip, 2 m apart, and the strip's spots lie between them.
/// Every one of them steps in: a neighbour who is about to move keeps no
/// spot clear, only one who stays where he is.
#[test]
fn soldiers_beside_free_cover_step_in_together() {
    let spots: Vec<Place> = (0..5)
        .map(|k| Place::quiet(v2(0.0, 2.35 * k as f64), Some(CoverTier::Light)))
        .collect();
    let from: Vec<V2> = (0..5).map(|k| v2(-0.9, 0.5 + 2.0 * k as f64)).collect();
    let stay: Vec<_> = from.iter().map(|&p| Some(Place::quiet(p, None))).collect();
    let claims = cover::claim(
        &from,
        &stay,
        &spots,
        2.0,
        2.0,
        common::rules().cover.lean_apart_m,
    );
    assert!(
        claims.iter().all(|c| c.is_some_and(|c| c.spot.is_some())),
        "all step in: {claims:?}"
    );
    // A soldier who holds heavy cover a step from a spot stays, and keeps it clear.
    let from = [v2(-0.9, 0.5), v2(-0.9, 6.0)];
    let stay = [
        Some(Place::quiet(from[0], Some(CoverTier::Heavy))),
        Some(Place::quiet(from[1], None)),
    ];
    let claims = cover::claim(
        &from,
        &stay,
        &spots[..1],
        2.0,
        2.0,
        common::rules().cover.lean_apart_m,
    );
    let taken: Vec<_> = claims.iter().map(|c| c.and_then(|c| c.spot)).collect();
    assert_eq!(taken, vec![None, None]);
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
            p.with_z(r.physics.infantry_muzzle_m),
            target.with_z(common::physics("infantry_aim_m")),
        )
    };
    let stands = |p: V2| !w.props().any(|q| q.footprint().contains(p, 0.3));
    let fits = |p: V2| (stands(p) && clear(p)).then_some(());
    let from = v2(61.1, 48.0);
    assert!(!clear(from), "the wall blocks him");
    let radius = r.physics.soldier_radius_m;
    let step = r.cover.step_out_m;
    let (out, ()) = cover::step_out(from, target, &known, &r.cover, radius, step, &fits)
        .expect("a clear place round the north end");
    assert!(clear(out));
    assert!((out - from).length() <= r.cover.step_out_m + 1e-9);
    assert!(out.x < 62.0, "he stays on his side of the wall: {out:?}");
    // Deep behind the middle of a long wall: nothing within the step, but
    // the squad's area reaches round its end (27d).
    let long = world(json!([wall([62.0, 45.0], [0.4, 8.0, 1.5])]));
    let clear_long = |p: V2| {
        long.segment_clear(
            p.with_z(r.physics.infantry_muzzle_m),
            target.with_z(common::physics("infantry_aim_m")),
        )
    };
    let fits_long = |p: V2| {
        let stands = !long.props().any(|q| q.footprint().contains(p, 0.3)) && p.x < 62.0;
        (stands && clear_long(p)).then_some(())
    };
    let deep = v2(61.1, 42.0);
    let near = cover::step_out(deep, target, &known, &r.cover, radius, step, &fits_long);
    assert_eq!(near, None, "nothing within the step");
    let (round, ()) = cover::step_out(deep, target, &known, &r.cover, radius, 14.0, &fits_long)
        .expect("round the end, inside the area");
    assert!(
        clear_long(round) && (round - deep).length() > step,
        "{round:?}"
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
    // A new crater lands inside the squad's area twice a second: each is a
    // change of the ground its side sees, so each asks for a re-resolve.
    let events: Vec<Value> = (0..40)
        .map(|k| {
            json!({ "tick": 5 + 15 * k, "burst": { "point": [68.0 + (k % 6) as f64, 52.0 + 1.2 * (k / 6) as f64], "weapon": "tank_he" } })
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

/// The ticks after 300 squad 0 re-resolved its cover at, with a crate added
/// in view at `at` on tick 300 (a 30 Hz battle).
fn resolves_after_a_crate_lands(at: [f64; 2]) -> Vec<u64> {
    let setup = common::scenario_with(
        &map(json!([crate_at([64.0, 60.0])])),
        json!([
            { "side": "blue", "kind": "rifle", "position": [60, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [140, 60], "engagement": "return_fire_only" },
        ]),
        json!([{ "tick": 300, "add_prop": { "kind": "crate", "center": at, "yaw": 0, "half_extents": [0.8, 0.8, 0.6] } }]),
        json!([]),
    );
    let mut b = Battle::new(&setup, 1);
    let mut resolves = Vec::new();
    for _ in 0..600 {
        b.step();
        let at = b.unit(UnitId(0)).unwrap().cover.resolved_at;
        if b.tick() > 300 && resolves.last() != Some(&at) && at > 300 {
            resolves.push(at);
        }
    }
    assert!(
        b.navigation_revision(Side::Blue) > 0,
        "blue learned the crate"
    );
    resolves
}

#[test]
fn a_holding_squad_re_resolves_for_a_body_learned_within_its_reach_only() {
    // A crate beside the squad changes where it can hide: it re-resolves
    // within the second after it is learned.
    let near = resolves_after_a_crate_lands([66.0, 56.0]);
    assert!(!near.is_empty(), "a crate beside it re-resolves: {near:?}");
    assert!(near[0] <= 300 + 2 * 30, "promptly: {near:?}");
    // The same crate 30 m off, in plain view, is none of its business.
    let far = resolves_after_a_crate_lands([60.0, 30.0]);
    assert!(
        far.is_empty(),
        "a crate 30 m off never re-resolves: {far:?}"
    );
}

#[test]
fn a_side_answers_which_planning_changes_came_near_until_its_log_runs_out() {
    let w = world(json!([crate_at([20.0, 20.0]), crate_at([150.0, 20.0])]));
    let props: Vec<&Prop> = w.props().collect();
    let (near, far) = (props[0], props[1]);
    let mut side = sim::movement::SideGeometry::default();
    side.forget(near);
    let here = v2(22.0, 20.0);
    for _ in 0..10 {
        side.forget(far);
    }
    assert!(side.changed_near(0, here, 3.0), "the crate beside it went");
    assert!(!side.changed_near(1, here, 3.0), "since then, only far off");
    assert!(side.changed_near(1, v2(150.0, 23.0), 3.0));
    assert!(
        !side.changed_near(side.revision, here, 1000.0),
        "nothing new"
    );
    // Once the log no longer reaches back to a squad's revision, it can't
    // tell what changed where: any change may be near.
    for _ in 0..5000 {
        side.forget(far);
    }
    assert!(side.changed_near(1, here, 3.0));
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
        json!([{ "tick": 150, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }, { "tick": 150, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }, { "tick": 150, "burst": { "point": [64.0, 45.0], "weapon": "atgm" } }]),
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
