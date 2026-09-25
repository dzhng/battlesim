//! Consequences of physical fire (slice 09), driven through real scenarios.
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::PropKind;
use contract::observation::OwnUnit;
use serde_json::{json, Value};
use sim::battle::Battle;

mod common;

fn map(props: Value, forests: Value) -> String {
    json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props, "forests": forests })
        .to_string()
}

fn battle(props: Value, units: Value, seed: u64) -> Battle {
    Battle::new(
        &common::scenario_with(&map(props, json!([])), units, json!([]), json!([])),
        seed,
    )
}

fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

fn order(b: &mut Battle, side: Side, seq: u64, order: Order) {
    let ack = b.accept(CommandEnvelope {
        side,
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

fn rules() -> Value {
    common::village()
}

#[test]
fn penetrating_hits_take_fixed_damage_and_a_failed_one_takes_none() {
    // Blue's tank faces red's tank front (140) with AP (180): each hit costs 40.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "tank", "position": [300, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
        ]),
        1,
    );
    let damage = rules()["weapons"]["tank_ap"]["damage"].as_f64().unwrap();
    let full = rules()["health"]["tank"].as_f64().unwrap();
    let mut seen = vec![full];
    for _ in 0..400 {
        b.step();
        let hp = own(&b, Side::Red, 1).map_or(0.0, |u| u.hp);
        if hp != *seen.last().unwrap() {
            seen.push(hp);
        }
    }
    assert!(seen.len() >= 2, "the tank was hit: {seen:?}");
    for w in seen.windows(2) {
        assert!(
            (w[0] - w[1] - damage).abs() < 1e-9 || w[1] <= 0.0,
            "fixed damage per hit: {seen:?}"
        );
    }
}

#[test]
fn rounds_that_cannot_penetrate_do_nothing() {
    // Scouts' rifles (5) against a supply truck, whose every face is 5 or
    // more: the default gun keeps firing (W09) and never scratches it.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "recon", "position": [100, 300] },
            { "side": "red", "kind": "supply", "position": [200, 300] },
        ]),
        2,
    );
    run(&mut b, 300);
    assert_eq!(
        own(&b, Side::Red, 1).unwrap().hp,
        rules()["health"]["supply"].as_f64().unwrap()
    );
}

#[test]
fn a_destroyed_tank_leaves_a_wreck_that_reroutes_the_side_that_sees_it() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "blue", "kind": "supply", "position": [100, 340] },
            { "side": "red", "kind": "tank", "position": [300, 300], "engagement": "return_fire_only" },
        ]),
        3,
    );
    let mut t = 0;
    while own(&b, Side::Red, 2).is_some() {
        b.step();
        t += 1;
        assert!(t < 2000, "red's tank survived");
    }
    run(&mut b, 12);
    let wreck = b
        .observe(Side::Blue)
        .known_props
        .iter()
        .find(|p| p.kind == PropKind::Wreck)
        .cloned()
        .expect("blue saw the wreck");
    assert!((wreck.center[0] - 300.0).abs() < 1.0);
    // A death blue watched: the attack is complete and leaves no last-seen area.
    assert_eq!(own(&b, Side::Blue, 0).unwrap().goal, None);
    assert!(
        b.observe(Side::Blue).contacts.is_empty(),
        "{:?}",
        b.observe(Side::Blue).contacts
    );
    // Red has no eyes left there: it knows of no wreck.
    assert!(b.observe(Side::Red).known_props.is_empty());
    // Blue's truck is sent straight through the wreck's spot.
    order(
        &mut b,
        Side::Blue,
        1,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [500.0, 300.0],
            route: contract::command::RoutePolicy::Shortest,
        },
    );
    run(&mut b, 2);
    let truck = own(&b, Side::Blue, 1).unwrap();
    let mut from = [truck.position[0], truck.position[1]];
    for p in truck.route {
        for k in 0..=20 {
            let t = k as f64 / 20.0;
            let q = [
                from[0] + (p[0] - from[0]) * t,
                from[1] + (p[1] - from[1]) * t,
            ];
            let inside = (q[0] - wreck.center[0]).abs() < wreck.half_extents[0]
                && (q[1] - wreck.center[1]).abs() < wreck.half_extents[1];
            assert!(!inside, "the route crosses the wreck at {q:?}");
        }
        from = p;
    }
    // The destroyed tank is gone from red's own list, and can take no orders.
    let ack = b.accept(CommandEnvelope {
        side: Side::Red,
        seq: 1,
        order: Order::Stop {
            units: vec![UnitId(2)],
        },
        queued: false,
    });
    assert!(ack.error.is_some());
}

#[test]
fn near_misses_suppress_without_damage() {
    // Blue's scouts fire at the ground past red's squad, whose nearest soldiers
    // stand a couple of metres off their line of fire.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "recon", "position": [180, 300] },
            { "side": "red", "kind": "rifle", "position": [200, 306.5], "engagement": "return_fire_only" },
        ]),
        4,
    );
    order(
        &mut b,
        Side::Blue,
        1,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [320.0, 300.0, 0.0],
            },
        },
    );
    let mut peak: f64 = 0.0;
    for _ in 0..120 {
        b.step();
        let red = own(&b, Side::Red, 1).unwrap();
        peak = peak.max(red.suppression);
        assert!(red.suppression <= 1.0);
        assert!(
            red.member_hp.iter().all(|&hp| hp == 100.0),
            "a near miss never damages"
        );
        assert_eq!(red.member_hp.len(), 8);
    }
    assert!(peak > 0.0, "suppressed");
}

/// Two red squads walk the same way; blue's scouts fire past one of them.
fn walkers(suppress: bool) -> Battle {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "recon", "position": [180, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [200, 306.5], "engagement": "return_fire_only" },
        ]),
        5,
    );
    if suppress {
        order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [320.0, 300.0, 0.0],
                },
            },
        );
    }
    run(&mut b, 60);
    order(
        &mut b,
        Side::Red,
        1,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [200.0, 500.0],
            route: contract::command::RoutePolicy::Shortest,
        },
    );
    b
}

#[test]
fn suppression_slows_but_never_turns_a_squad_back() {
    let (mut calm, mut pinned) = (walkers(false), walkers(true));
    let mut last = own(&pinned, Side::Red, 1).unwrap().position[1];
    for _ in 0..60 {
        calm.step();
        pinned.step();
        let y = own(&pinned, Side::Red, 1).unwrap().position[1];
        assert!(y >= last - 1e-9, "never retreats");
        last = y;
    }
    let s = own(&pinned, Side::Red, 1).unwrap().suppression;
    assert!(s > 0.0);
    let (a, c) = (
        own(&pinned, Side::Red, 1).unwrap(),
        own(&calm, Side::Red, 1).unwrap(),
    );
    assert!(
        a.position[1] < c.position[1],
        "suppressed squads move slower"
    );
    assert!(a.position[1] > 306.5, "but still advance");
}

#[test]
fn suppression_recovers_after_a_lull() {
    let mut b = walkers(true);
    order(
        &mut b,
        Side::Blue,
        2,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    run(&mut b, 2);
    let s0 = own(&b, Side::Red, 1).unwrap().suppression;
    assert!(s0 > 0.0);
    let delay = rules()["suppression"]["recovery_delay_s"].as_f64().unwrap();
    let rate = rules()["suppression"]["decay_per_s"].as_f64().unwrap();
    // Nothing fades during the delay (red may answer, but blue stopped).
    run(&mut b, (delay * 30.0) as u64 - 10);
    assert_eq!(own(&b, Side::Red, 1).unwrap().suppression, s0);
    run(&mut b, 10 + 30);
    let s1 = own(&b, Side::Red, 1).unwrap().suppression;
    assert!(s1 < s0 && s1 >= s0 - rate * 1.4, "{s0} → {s1}");
}

#[test]
fn blast_is_sampled_per_soldier_and_spares_no_team() {
    // HE into ground between a red squad and a blue squad standing close by:
    // soldiers take different damage by distance, and blue's own are hit too.
    let (mut red_losses, mut blue_losses, mut distinct) = (0.0, 0.0, false);
    for seed in 0..12 {
        let mut b = battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "tank", "position": [100, 300] },
                { "side": "red", "kind": "rifle", "position": [300, 303], "engagement": "return_fire_only" },
                { "side": "blue", "kind": "rifle", "position": [300, 290], "engagement": "return_fire_only" },
            ]),
            seed,
        );
        order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [300.0, 297.0, 0.0],
                },
            },
        );
        // AP starts loaded: HE needs a full reload first.
        run(&mut b, 240);
        let red = own(&b, Side::Red, 1).map_or(vec![], |u| u.member_hp);
        let blue = own(&b, Side::Blue, 2).map_or(vec![], |u| u.member_hp);
        red_losses += 800.0 - red.iter().sum::<f64>();
        blue_losses += 800.0 - blue.iter().sum::<f64>();
        let mut hurt: Vec<f64> = red.iter().copied().filter(|&h| h < 100.0).collect();
        hurt.sort_by(f64::total_cmp);
        hurt.dedup();
        distinct |= hurt.len() > 1 || (red.len() < 8 && !red.is_empty());
    }
    assert!(
        red_losses > 0.0 && blue_losses > 0.0,
        "red {red_losses}, blue {blue_losses}"
    );
    assert!(distinct, "per-soldier outcomes differ");
}

#[test]
fn the_fallen_stay_where_they_fell_and_block_nothing() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "rifle", "position": [250, 300], "engagement": "return_fire_only" },
        ]),
        6,
    );
    let props = b.world().props().count();
    let mut first = None;
    for _ in 0..900 {
        b.step();
        if let Some(c) = b.observe(Side::Red).corpses.first() {
            first = Some(c.position);
            break;
        }
    }
    let at = first.expect("a soldier fell");
    run(&mut b, 120);
    assert!(
        b.observe(Side::Red)
            .corpses
            .iter()
            .any(|c| c.position == at && c.own),
        "it stays put"
    );
    assert!(
        b.observe(Side::Blue)
            .corpses
            .iter()
            .any(|c| c.position == at && !c.own),
        "blue saw it fall"
    );
    assert_eq!(
        b.world().props().count(),
        props,
        "a corpse is not an obstacle"
    );
    // Blue's tank is sent straight over the spot: no detour.
    let goal = [at[0] + 40.0, at[1]];
    order(
        &mut b,
        Side::Blue,
        1,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: contract::command::RoutePolicy::Shortest,
        },
    );
    run(&mut b, 2);
    let route = own(&b, Side::Blue, 0).unwrap().route;
    assert_eq!(route.len(), 1, "a straight route: {route:?}");
}

#[test]
fn damage_from_untargeted_fire_grants_return_fire() {
    // Red's squad holds fire; blue's HE aimed at the ground beside it still
    // earns an answer.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "red", "kind": "at", "position": [300, 303], "engagement": "return_fire_only" },
        ]),
        7,
    );
    order(
        &mut b,
        Side::Blue,
        1,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [300.0, 297.0, 0.0],
            },
        },
    );
    let mut answered = false;
    for _ in 0..240 {
        b.step();
        answered |= own(&b, Side::Red, 1).is_some_and(|u| {
            u.mounts
                .iter()
                .any(|m| matches!(m.target, Some(TargetRef::Identified { .. })))
        });
    }
    assert!(answered, "the attacked squad may answer (W13)");
}

#[test]
fn damage_replays_identically() {
    let make = || {
        battle(
            json!([]),
            json!([
                { "side": "blue", "kind": "tank", "position": [100, 300] },
                { "side": "blue", "kind": "rifle", "position": [120, 330] },
                { "side": "red", "kind": "rifle", "position": [300, 303] },
                { "side": "red", "kind": "tank", "position": [320, 260] },
            ]),
            9,
        )
    };
    let (mut a, mut c) = (make(), make());
    for _ in 0..600 {
        a.step();
        c.step();
        assert_eq!(a.digest(), c.digest());
    }
}

#[test]
fn rounds_aimed_into_forest_spread_wider_from_its_first_metre() {
    let forest = json!([{ "rect": [400, 200, 200, 200], "canopy_height_m": 12, "trunk_spacing_m": 20,
        "trunk_radius_m": 0.35, "trunk_height_m": 10, "trunk_clearance_m": 2 }]);
    let map: contract::map::MapDefinition = serde_json::from_str(&map(json!([]), forest)).unwrap();
    let world = sim::world::WorldGeometry::new(&map);
    let rules: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let at = |x: f64| sim::damage::cover_spread(&world, &rules, sim::math::v3(x, 300.0, 0.0), 0.0);
    let full = rules.cover.forest_spread_multiplier;
    assert_eq!(at(300.0), 1.0, "open ground");
    assert!(
        at(401.0) > 1.0 && at(401.0) < full,
        "the edge already covers: {}",
        at(401.0)
    );
    assert!((at(480.0) - full).abs() < 1e-9, "deep forest is full cover");
}

#[test]
fn a_hit_meets_the_face_it_struck() {
    use contract::scenario::Face;
    use sim::math::v3;
    use sim::units::face_toward;
    let half = v3(3.5, 1.8, 1.2);
    // Just outside each face, and at the corners where two compete.
    assert_eq!(face_toward(v3(3.6, 0.0, 0.0), half), Face::Front);
    assert_eq!(face_toward(v3(-3.6, 0.5, 0.0), half), Face::Rear);
    assert_eq!(face_toward(v3(1.0, 1.9, 0.0), half), Face::Side);
    assert_eq!(face_toward(v3(1.0, -1.9, 0.3), half), Face::Side);
    assert_eq!(face_toward(v3(0.5, 0.5, 1.3), half), Face::Roof);
    // A corner is judged on the box's scale: nearer the side than the front.
    assert_eq!(face_toward(v3(3.4, 1.85, 0.0), half), Face::Side);
}

#[test]
fn a_wall_shields_soldiers_from_a_blast_beside_it() {
    // HE bursts on blue's side of a tall wall; red's squad just behind it is
    // inside the blast radius but out of its line.
    for seed in 0..6 {
        let mut b = battle(
            json!([{ "kind": "wall", "center": [300, 300], "yaw": 0, "half_extents": [0.5, 20, 4] }]),
            json!([
                { "side": "blue", "kind": "tank", "position": [100, 300] },
                { "side": "red", "kind": "rifle", "position": [306, 300], "engagement": "return_fire_only" },
            ]),
            seed,
        );
        order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [296.0, 300.0, 0.0],
                },
            },
        );
        run(&mut b, 420);
        let red = own(&b, Side::Red, 1).unwrap();
        assert!(
            red.member_hp.iter().all(|&hp| hp == 100.0),
            "seed {seed}: {:?}",
            red.member_hp
        );
    }
}

#[test]
fn cover_lowers_losses_to_the_same_fire_over_many_seeds() {
    // The same HE fire at the same squad, once in the open and once 45 m into
    // a forest: paired seeds, total losses compared (never one lucky seed).
    let forest = json!([{ "rect": [330, 230, 90, 90], "canopy_height_m": 12, "trunk_spacing_m": 20,
        "trunk_radius_m": 0.35, "trunk_height_m": 10, "trunk_clearance_m": 2 }]);
    let losses = |forests: &Value, seed: u64| {
        let setup = common::scenario_with(
            &map(json!([]), forests.clone()),
            json!([
                { "side": "blue", "kind": "tank", "position": [230, 275], "engagement": "return_fire_only" },
                { "side": "red", "kind": "rifle", "position": [375, 275], "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        let mut b = Battle::new(&setup, seed);
        order(
            &mut b,
            Side::Blue,
            1,
            Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [375.0, 272.0, 0.0],
                },
            },
        );
        // Short of wiping either squad out, so the comparison does not saturate.
        run(&mut b, 300);
        own(&b, Side::Red, 1).map_or(800.0, |u| 800.0 - u.member_hp.iter().sum::<f64>())
    };
    let (mut open, mut covered) = (0.0, 0.0);
    for seed in 0..16 {
        open += losses(&json!([]), seed);
        covered += losses(&forest, seed);
    }
    assert!(covered < open * 0.85, "open {open}, forest {covered}");
}
