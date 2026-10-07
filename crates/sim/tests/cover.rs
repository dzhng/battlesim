//! Infantry cover: tiers through spread only, for soldiers only
//! (Q5, Q20); spots behind bodies on the far side from the threat (Q7);
//! step-out round a corner (D3); a re-resolve at most once a second (Q11),
//! against each new enemy that appears, and as an enemy comes into or goes
//! out of the squad's reach.
use contract::ids::{Side, UnitId};
use contract::scenario::{CoverTier, Rules, ScenarioDefinition};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::cover::{self, Known, Place};
use sim::ground::GroundLayer;
use sim::lean::{Lean, LeanSide, Round};
use sim::math::{v2, wrap_angle, V2};
use sim::world::{Prop, WorldGeometry};
use std::f64::consts::FRAC_PI_2;

use crate::common;

fn rules() -> Rules {
    serde_json::from_value(common::game()).unwrap()
}

fn map(props: Value) -> String {
    json!({ "size": [200, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
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
        { "kind": "heavy_wreck", "center": [60, 60], "yaw": 0, "half_extents": common::hull("test_tank").half_extents_m, "wreck_of": "test_tank" },
        { "kind": "medium_wreck", "center": [140, 60], "yaw": 0, "half_extents": common::hull("test_supply").half_extents_m, "wreck_of": "test_supply" },
    ]));
    let ground = GroundLayer::new(w.width(), w.depth(), &r.ground);
    let east = v2(190.0, 60.0);
    let heavy_wreck = cover::at(&w, &ground, &[], &r, v2(56.0, 60.0), east);
    let medium_wreck = cover::at(&w, &ground, &[], &r, v2(136.5, 60.0), east);
    assert_eq!(
        heavy_wreck,
        cover::weight_tier(common::hull("test_tank").weight_class)
    );
    assert_eq!(
        medium_wreck,
        cover::weight_tier(common::hull("test_supply").weight_class)
    );
    assert!(
        heavy_wreck > medium_wreck,
        "a tank is heavier cover than a truck"
    );
    let setup = common::scenario(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [60, 60] },
            { "side": "blue", "kind": "test_supply", "position": [140, 60] }
        ]),
        json!([]),
    );
    let b = Battle::new(&setup, 1);
    let hulls = sim::lean::hulls([b.unit(UnitId(0)).unwrap(), b.unit(UnitId(1)).unwrap()], &r);
    let bodies = cover::hull_bodies(&hulls);
    for (point, wreck) in [
        (v2(56.0, 60.0), heavy_wreck),
        (v2(136.5, 60.0), medium_wreck),
    ] {
        assert_eq!(
            cover::at(b.world(), &ground, &bodies, &r, point, east),
            wreck
        );
    }
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
    // the squad's area reaches round its end.
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
            { "side": "red", "kind": "test_rifle", "position": [125, 60] },
        ]),
        json!([]),
        json!([]),
    );
    // Only rifles take part: launcher selection and its delayed first shot
    // must not let impacts enter the comparison before the full volley.
    let mut documents = sim::fixtures::test_documents();
    for document in &mut documents {
        if let Some(grenadier) = document
            .get_mut("soldiers")
            .and_then(|soldiers| soldiers.get_mut("test_grenadier"))
        {
            grenadier["mounts"] =
                json!([{ "name": "rifles", "weapons": ["rifle"], "squad": true }]);
        }
    }
    setup.rules.catalog = contract::catalog::resolve(&documents).unwrap();
    if !cover {
        let t = &mut setup.rules.cover.tiers;
        (t.light, t.medium, t.heavy) = (1.0, 1.0, 1.0);
    }
    Battle::new(&setup, 3)
}

/// Each rifle's first launch, before impacts can feed back into the experiment.
fn first_volley(mut b: Battle) -> Vec<(u64, [f64; 3])> {
    let mut rounds = std::collections::BTreeMap::new();
    let count = b.unit(UnitId(1)).unwrap().members.len();
    for _ in 0..30 * 20 {
        b.step();
        for (p, r) in b.rounds().filter(|(_, r)| r.side == Side::Red) {
            if b.arsenal().weapons[r.weapon].id != "rifle" {
                continue;
            }
            let body = p.shooter.unwrap().body.0;
            rounds
                .entry(body)
                .or_insert((p.id.0, [p.velocity.x, p.velocity.y, p.velocity.z]));
        }
        if rounds.len() == count {
            return rounds.into_values().collect();
        }
    }
    panic!("not every red rifle fired");
}

#[test]
fn cover_widens_the_spread_of_rounds_at_soldiers_and_never_at_a_vehicle() {
    // The same wall and seed; only the tiers' multipliers differ.
    // Rifles on a tank behind it: nothing changes (Q20).
    assert_eq!(
        first_volley(firefight(false, "test_tank")),
        first_volley(firefight(true, "test_tank")),
        "no cover for a vehicle"
    );
    // Rifles on a squad sheltering behind it: wider rounds.
    let bare = first_volley(firefight(false, "test_rifle"));
    let covered = first_volley(firefight(true, "test_rifle"));
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
            { "side": "blue", "kind": "test_rifle", "position": [60, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [140, 60], "engagement": "return_fire_only" },
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
            { "side": "blue", "kind": "test_rifle", "position": [60, 60], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [140, 60], "engagement": "return_fire_only" },
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
    let map = sim::navigation::NavBase::build(&w, w.props(), 0.3);
    let mut side = sim::movement::SideGeometry::new(std::sync::Arc::new(map));
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
            { "side": "blue", "kind": "test_rifle", "position": [58, 45] },
            { "side": "red", "kind": "test_rifle", "position": [110, 45] },
            { "side": "blue", "kind": "test_tank", "position": [50, 70], "engagement": "return_fire_only" },
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

/// An attack-move that halts on contact on the way and then carries on to
/// its destination holds at the destination: the area it arrives at is the
/// order's, never the one round the halt it left behind.
#[test]
fn an_attack_move_that_halted_on_the_way_holds_where_it_arrives() {
    // Red shows itself beside blue's path, then walks behind a long wall;
    // once its last-seen place fades blue marches on.
    let setup = common::scenario_with(
        &json!({ "size": [320, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": [wall([100.0, 70.0], [90.0, 0.5, 3.0])] })
        .to_string(),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [20, 30] },
            { "side": "red", "kind": "test_rifle", "position": [70, 55], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([
            { "tick": 1, "side": "blue", "order": { "kind": "attack_move", "units": [0], "gesture": 1, "goal": [260.0, 30.0] } },
            { "tick": 90, "side": "red", "order": { "kind": "move", "units": [1], "gesture": 1, "goal": [70.0, 100.0], "route": "shortest" } }
        ]),
    );
    let mut b = Battle::new(&setup, 5);
    let hz = b.rules().tick_hz as u64;
    let own = |b: &Battle| b.observe(Side::Blue).own[0].clone();
    let goal = v2(260.0, 30.0);
    let mut halted = false;
    let mut arrived = None;
    for _ in 0..240 * hz {
        b.step();
        let u = own(&b);
        halted |= u.state == contract::observation::MoveState::Halted;
        let near = (v2(u.position[0], u.position[1]) - goal).length() < 10.0;
        if halted && near && u.goal.is_none() && arrived.is_none() {
            arrived = Some(b.tick());
        }
        if arrived.is_some_and(|t| b.tick() >= t + 10 * hz) {
            break;
        }
    }
    assert!(halted, "the attack-move halted on contact");
    assert!(arrived.is_some(), "and carried on to its destination");
    let u = own(&b);
    let area = u.area.expect("a squad has an area");
    let at = v2(area.anchor[0], area.anchor[1]);
    assert!(
        (at - goal).length() < 3.0,
        "its area is the destination's: {at:?}"
    );
    for m in &u.members {
        let d = (v2(m[0], m[1]) - goal).length();
        assert!(d <= area.radius, "a soldier holds inside it, {d:.1} m off");
    }
}

/// A setup where blue squad 0 holds at (100, 100) among `props`, with the
/// other `units` (none of whom fires first), each of the `moves`
/// (tick, unit, goal) a red move order.
fn holding(props: Value, units: Value, moves: &[(u64, u32, [f64; 2])]) -> ScenarioDefinition {
    let mut all = vec![
        json!({ "side": "blue", "kind": "test_rifle", "position": [100, 100], "engagement": "return_fire_only" }),
    ];
    all.extend(units.as_array().unwrap().iter().map(|u| {
        let mut u = u.clone();
        u["kind"] = json!("test_rifle");
        u["engagement"] = json!("return_fire_only");
        u
    }));
    let scripts: Vec<Value> = moves
        .iter()
        .map(|(tick, unit, goal)| {
            json!({ "tick": tick, "side": "red", "order": { "kind": "move", "units": [unit],
                "gesture": 1, "goal": goal, "route": "shortest" } })
        })
        .collect();
    common::scenario_with(
        &json!({ "size": [300, 240], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
            .to_string(),
        Value::Array(all),
        json!([]),
        Value::Array(scripts),
    )
}

/// A tall screen, 40 m long north to south, no one sees past.
fn screen(center: [f64; 2]) -> Value {
    wall(center, [0.5, 20.0, 2.0])
}

/// Blue squad 0 inside an L of low walls: one along its north side, one
/// along its east. Red squad 2 waits behind a tall screen north-east, out
/// of blue's sight, and at tick `shows` walks out to due east, 75 m off:
/// nearer than red squad 1 (at `red1`). `extra`: more props.
fn a_squad_in_an_l(
    red1: [f64; 2],
    shows: u64,
    extra: &[Value],
    moves: &[(u64, u32, [f64; 2])],
) -> Battle {
    let mut props = vec![
        wall([100.0, 106.0], [6.0, 0.4, 0.5]),
        wall([106.0, 100.0], [0.4, 6.0, 0.5]),
        screen([165.0, 130.0]),
    ];
    props.extend_from_slice(extra);
    let mut moves = moves.to_vec();
    moves.push((shows, 2, [175.0, 100.0]));
    let setup = holding(
        Value::Array(props),
        json!([
            { "side": "red", "position": red1 },
            { "side": "red", "position": [175, 130] },
        ]),
        &moves,
    );
    Battle::new(&setup, 1)
}

fn north_face(p: V2) -> bool {
    (p.y - 105.1).abs() < 0.6 && (93.5..106.5).contains(&p.x)
}

fn east_face(p: V2) -> bool {
    (p.x - 105.1).abs() < 0.6 && (93.5..106.5).contains(&p.y)
}

/// Step until blue's side identifies red unit `id`; the tick it did.
fn until_identified(b: &mut Battle, id: u32, within: u64) -> u64 {
    for _ in 0..within {
        b.step();
        if b.observe(Side::Blue)
            .identified
            .iter()
            .any(|u| u.id.0 == id)
        {
            return b.tick();
        }
    }
    panic!("red {id} never showed itself");
}

/// Blue squad 0 as published: its yaw, and each soldier's place and the
/// cover he has there now against his squad's threat.
fn blue_squad(b: &Battle) -> (f64, Vec<(V2, Option<CoverTier>)>) {
    let u = b.observe(Side::Blue).own[0].clone();
    let soldiers = u
        .members
        .iter()
        .zip(&u.member_orders)
        .map(|(m, o)| (v2(m[0], m[1]), o.cover_now))
        .collect();
    (u.yaw, soldiers)
}

/// How many soldiers are in cover standing where `face` holds.
fn in_cover_at(soldiers: &[(V2, Option<CoverTier>)], face: impl Fn(V2) -> bool) -> usize {
    soldiers
        .iter()
        .filter(|(at, now)| now.is_some() && face(*at))
        .count()
}

/// Once red 2 shows itself east, blue squad 0 turns to it within two
/// seconds, and its soldiers line the east wall in cover against it.
fn rearranges_against_red_2(b: &mut Battle) {
    let hz = b.rules().tick_hz as u64;
    until_identified(b, 2, 30 * hz);
    for _ in 0..2 * hz {
        b.step();
    }
    let (yaw, _) = blue_squad(b);
    let red = b.observe(Side::Blue).identified.clone();
    let red2 = red.iter().find(|u| u.id.0 == 2).unwrap().position;
    let bearing = (red2[1] - 100.0).atan2(red2[0] - 100.0);
    assert!(
        wrap_angle(yaw - bearing).abs() < 0.2,
        "it turns to red 2: yaw {:.0}, red 2 at {:.0}",
        yaw.to_degrees(),
        bearing.to_degrees()
    );
    // Its soldiers walk over to the east wall, in cover against red 2.
    for _ in 0..8 * hz {
        b.step();
    }
    let (_, after) = blue_squad(b);
    assert!(
        in_cover_at(&after, east_face) >= 5 && in_cover_at(&after, north_face) == 0,
        "against red 2 the squad lines the east wall: {after:?}"
    );
}

#[test]
fn a_holding_squad_rearranges_against_a_new_enemy_on_its_flank() {
    // Red 1 stands in the open 100 m north; red 2 steps out due east,
    // nearer: blue's threat swings 90 degrees.
    let mut b = a_squad_in_an_l([100.0, 200.0], 450, &[], &[]);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..10 * hz {
        b.step();
    }
    let (yaw, before) = blue_squad(&b);
    assert!(
        in_cover_at(&before, north_face) >= 5 && in_cover_at(&before, east_face) == 0,
        "against red 1 the squad lines the north wall: {before:?}"
    );
    assert!(wrap_angle(yaw - FRAC_PI_2).abs() < 0.2, "facing north");
    rearranges_against_red_2(&mut b);
}

#[test]
fn a_squad_holding_against_where_an_enemy_was_last_seen_rearranges_against_a_new_one() {
    // Red 1, seen to the north-east, walks behind a long tall wall across
    // the north: blue holds against where it was last seen. Then red 2
    // steps out due east.
    let across = wall([100.0, 205.0], [30.0, 0.5, 2.0]);
    let mut b = a_squad_in_an_l([140.0, 225.0], 900, &[across], &[(1, 1, [100.0, 215.0])]);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..25 * hz {
        b.step();
    }
    let seen = b.observe(Side::Blue).identified.clone();
    assert!(seen.is_empty(), "red 1 is out of sight: {seen:?}");
    let (yaw, before) = blue_squad(&b);
    assert!(
        in_cover_at(&before, north_face) >= 5 && in_cover_at(&before, east_face) == 0,
        "against red 1's last-seen place the squad lines the north wall: {before:?}"
    );
    assert!(
        yaw > 1.0,
        "facing where red 1 was last seen: {:.0}",
        yaw.to_degrees()
    );
    rearranges_against_red_2(&mut b);
}

/// Blue squad 0's soldiers tucked behind the middle of the tall wall along
/// its north side, not leaning out: none of them can fire north.
fn tucked(b: &Battle) -> usize {
    let u = b.observe(Side::Blue).own[0].clone();
    u.members
        .iter()
        .zip(&u.member_leans)
        .filter(|(m, lean)| {
            lean.is_none() && (m[1] - 105.1).abs() < 0.6 && (93.5..106.5).contains(&m[0])
        })
        .count()
}

#[test]
fn a_holding_squad_rearranges_against_a_new_enemy_on_nearly_the_same_bearing() {
    // Blue hides behind a tall wall from red 1, 120 m north and out of
    // its rifles' reach (90 m here). Red 2 comes round a screen to the
    // north-north-east, within reach and only some 20 degrees off red 1's
    // bearing: blue should step out round the wall's ends to fight it.
    // (A second blue squad off to the west watches: blue behind its wall
    // sees little.)
    let mut setup = holding(
        json!([
            wall([100.0, 106.0], [8.0, 0.4, 1.5]),
            screen([130.0, 155.0])
        ]),
        json!([
            { "side": "red", "position": [100, 220] },
            { "side": "red", "position": [138, 160] },
            { "side": "blue", "position": [60, 100] },
        ]),
        &[(450, 2, [112.0, 160.0])],
    );
    let rifle = &mut setup.rules.weapons.get_mut("rifle").unwrap().ballistics;
    (rifle.range_m, rifle.scatter_mrad) = (90.0, 60.0);
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..10 * hz {
        b.step();
    }
    let (yaw, _) = blue_squad(&b);
    assert!(wrap_angle(yaw - FRAC_PI_2).abs() < 0.2, "facing red 1");
    assert!(tucked(&b) >= 5, "out of red 1's reach, blue tucks in");
    until_identified(&mut b, 2, 30 * hz);
    let red = b.observe(Side::Blue).identified.clone();
    let red2 = red.iter().find(|u| u.id.0 == 2).unwrap().position;
    let bearing = (red2[1] - 100.0).atan2(red2[0] - 100.0);
    assert!(
        wrap_angle(bearing - FRAC_PI_2).abs() < 45f64.to_radians(),
        "red 2 shows within 45 degrees of red 1: {:.0}",
        bearing.to_degrees()
    );
    for _ in 0..2 * hz {
        b.step();
    }
    let (yaw, _) = blue_squad(&b);
    assert!(
        wrap_angle(yaw - bearing).abs() < 0.2,
        "it turns to red 2: yaw {:.0}, red 2 at {:.0}",
        yaw.to_degrees(),
        bearing.to_degrees()
    );
    for _ in 0..8 * hz {
        b.step();
    }
    assert_eq!(tucked(&b), 0, "everyone steps out to fight red 2");
}

/// Step until blue's side sees red unit `id` stand within reach of all of
/// blue squad 0's area (`inside`), or beyond reach of all of it: `reach`
/// from its every place, the area's anchor and radius as published.
fn until_reach(b: &mut Battle, id: u32, reach: f64, inside: bool, within: u64) {
    for _ in 0..within {
        b.step();
        let obs = b.observe(Side::Blue);
        let area = obs.own[0].area.expect("a holding squad has an area");
        let at = v2(area.anchor[0], area.anchor[1]);
        let d = obs
            .identified
            .iter()
            .find(|u| u.id.0 == id)
            .map(|u| (v2(u.position[0], u.position[1]) - at).length());
        let crossed = |d: f64| match inside {
            true => d + area.radius <= reach,
            false => d - area.radius > reach,
        };
        if d.is_some_and(crossed) {
            return;
        }
    }
    panic!("red {id} never crossed blue's reach");
}

#[test]
fn a_holding_squad_rearranges_as_the_same_enemy_walks_into_and_out_of_its_reach() {
    // Blue hides behind a tall wall from red 1, 130 m due north and out of
    // its rifles' reach (90 m here). Red 1 walks straight at it, stops
    // within reach, then walks back out: the same enemy on the same
    // bearing throughout. In reach, blue leans and steps out round the
    // wall's ends to fight it; out of reach again, it tucks back in. (A
    // second blue squad off to the west watches: blue behind its wall
    // sees little.)
    let reach = 90.0;
    let mut setup = holding(
        json!([wall([100.0, 106.0], [8.0, 0.4, 1.5])]),
        json!([
            { "side": "red", "position": [100, 230] },
            { "side": "blue", "position": [60, 100] },
        ]),
        &[(300, 1, [100.0, 170.0]), (1500, 1, [100.0, 230.0])],
    );
    let rifle = &mut setup.rules.weapons.get_mut("rifle").unwrap().ballistics;
    (rifle.range_m, rifle.scatter_mrad) = (reach, 60.0);
    let mut b = Battle::new(&setup, 1);
    let hz = b.rules().tick_hz as u64;
    for _ in 0..10 * hz {
        b.step();
    }
    let (yaw, _) = blue_squad(&b);
    assert!(wrap_angle(yaw - FRAC_PI_2).abs() < 0.2, "facing red 1");
    assert!(tucked(&b) >= 5, "out of red 1's reach, blue tucks in");
    until_reach(&mut b, 1, reach, true, 60 * hz);
    for _ in 0..2 * hz {
        b.step();
    }
    let (yaw, _) = blue_squad(&b);
    assert!(
        wrap_angle(yaw - FRAC_PI_2).abs() < 0.2,
        "still facing red 1"
    );
    assert_eq!(tucked(&b), 0, "in reach, everyone leans or steps out");
    until_reach(&mut b, 1, reach, false, 60 * hz);
    // Red's rearmost soldier trails its unit, and the soldiers who
    // stepped out walk a few metres back behind the wall.
    for _ in 0..6 * hz {
        b.step();
    }
    assert!(tucked(&b) >= 5, "out of reach again, blue tucks back in");
}

/// An enemy beyond the whole holding area's weapon and lean reach cannot
/// offer engagement. It must not trigger exhaustive candidate-ring searches.
#[cfg(target_os = "macos")]
#[test]
fn an_unreachable_enemy_does_not_amplify_holding_work() {
    if !common::isolated_cost_test("cover::an_unreachable_enemy_does_not_amplify_holding_work") {
        return;
    }
    fn peak(enemy: bool) -> u64 {
        let mut setup = holding(
            json!([]),
            json!([
                { "side": if enemy { "red" } else { "blue" }, "position": [220,100] }
            ]),
            &[],
        );
        let rifle = &mut setup.rules.weapons.get_mut("rifle").unwrap().ballistics;
        (rifle.range_m, rifle.scatter_mrad) = (90.0, 60.0);
        let mut b = Battle::new(&setup, 1);
        let mut peak = 0;
        for _ in 0..60 {
            let mut previous = common::counters::instructions().expect("native counter");
            let mut movement = 0;
            b.step_profiled(|phase| {
                let now = common::counters::instructions().unwrap();
                if phase == sim::battle::TickPhase::Movement {
                    movement += now - previous;
                }
                previous = now;
            });
            peak = peak.max(movement);
        }
        if enemy {
            assert!(
                b.observe(Side::Blue).identified.iter().any(|u| u.id.0 == 1),
                "the holding squad must know the enemy"
            );
        }
        assert_eq!(
            b.unit(UnitId(0)).unwrap().cover.resolved_at > 0,
            enemy,
            "only the enemy arm resolves against a threat"
        );
        peak
    }
    let quiet = peak(false);
    let distant = peak(true);
    assert!(
        distant <= quiet * 2 + 1_000_000,
        "impossible engagement amplified holding work: quiet {quiet}, distant {distant}"
    );
}
