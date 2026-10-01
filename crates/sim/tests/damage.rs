//! Consequences of physical fire (slice 09), driven through real scenarios.
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::OwnUnit;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{BodyId, FlightEvent, Struck};
use sim::math::v2;

use crate::common;

fn map(props: Value, forests: Value) -> String {
    json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props, "forests": forests })
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

fn run(b: &mut Battle, ticks: u64) {
    for _ in 0..ticks {
        b.step();
    }
}

fn rules() -> Value {
    common::village()
}

/// A squad's hidden suppression level, as the authority holds it.
fn level(b: &Battle, id: u32) -> f64 {
    b.unit(UnitId(id)).unwrap().suppression
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
    let full = crate::common::hull("tank").hp;
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
    let mut setup = common::scenario_with(
        &map(json!([]), json!([])),
        json!([
            { "side": "blue", "kind": "recon", "position": [100, 300] },
            { "side": "red", "kind": "supply", "position": [200, 300] }
        ]),
        json!([]),
        json!([]),
    );
    let hull = common::hull("supply");
    let full = hull.hp;
    for penetrates in [false, true] {
        if penetrates {
            let a = &hull.armor;
            setup.rules.weapons.get_mut("rifle").unwrap().penetration =
                a.front.max(a.side).max(a.rear).max(a.roof) + 1.0;
        }
        let mut b = Battle::new(&setup, 2);
        let mut hits = 0;
        for _ in 0..300 {
            b.step();
            hits += b
                .flight_events()
                .iter()
                .filter(|e| {
                    matches!(e,
                FlightEvent::Impact(i) if i.struck == Struck::Body(
                    BodyId(sim::weapons::VEHICLE_BODY_BASE + 1)))
                })
                .count();
        }
        assert!(hits > 0, "rifle rounds reached the truck");
        let hp = b.unit(UnitId(1)).unwrap().hp;
        if penetrates {
            assert!(hp < full, "penetrating rounds hurt it");
        } else {
            assert_eq!(hp, full, "nonpenetrating hits do nothing");
        }
    }
}

#[test]
fn a_destroyed_tank_leaves_a_wreck_that_reroutes_the_side_that_sees_it() {
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "tank", "position": [100, 300] },
            { "side": "blue", "kind": "supply", "position": [100, 340] },
            { "side": "red", "kind": "tank", "position": [300, 300], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
        ]),
        // A seed on which blue's tank wins the even duel (on seed 3 red's
        // return fire now does).
        1,
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
        .find(|p| p.kind == common::kind("heavy_wreck"))
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
    common::order(
        &mut b,
        Side::Blue,
        1,
        Order::Move {
            units: vec![UnitId(1)],
            gesture: 1,
            goal: [500.0, 300.0],
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
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
fn a_round_suppresses_a_squad_once_however_long_it_takes_to_pass() {
    // Blue's scouts fire along red's squad, lengthwise: a round takes more
    // than a tick to pass its soldiers. Each round adds at most its weapon's
    // near-miss strength (its strongest pass), never once per tick.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "recon", "position": [150, 300] },
            { "side": "red", "kind": "rifle", "position": [200, 302], "engagement": "return_fire_only" },
        ]),
        4,
    );
    common::order(
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
    let strength = rules()["weapons"]["rifle"]["near_miss_suppression"]
        .as_f64()
        .unwrap();
    let mut rounds = std::collections::BTreeSet::new();
    for _ in 0..90 {
        b.step();
        rounds.extend(
            b.rounds()
                .filter(|(_, r)| r.unit == UnitId(0))
                .map(|(p, _)| p.id),
        );
        let red = level(&b, 1);
        assert!(
            red <= rounds.len() as f64 * strength + 1e-12,
            "{red} from {} rounds",
            rounds.len()
        );
    }
    assert!(level(&b, 1) > 0.0, "suppressed");
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
    common::order(
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
        peak = peak.max(level(&b, 1));
        assert!(level(&b, 1) <= 1.0);
        assert!(
            red.member_hp.iter().all(|&hp| hp == 100.0),
            "a near miss never damages"
        );
        assert_eq!(red.member_hp.len(), 8);
    }
    assert!(peak > 0.0, "suppressed");
}

#[test]
fn blast_is_sampled_per_soldier_and_spares_no_team() {
    // HE into ground between a red squad and a blue squad standing close by:
    // soldiers take different damage by distance, and blue's own are hit too.
    let (mut red_losses, mut blue_losses, mut distinct) = (0.0, 0.0, false);
    for seed in 0..12 {
        let mut setup = common::scenario_with(
            &map(json!([]), json!([])),
            json!([
                { "side": "blue", "kind": "tank", "position": [100, 300] },
                { "side": "red", "kind": "rifle", "position": [300, 303], "engagement": "return_fire_only" },
                { "side": "blue", "kind": "rifle", "position": [300, 290], "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        // One HE shell, without gunfire changing the victims before its arrival.
        for (name, weapon) in &mut setup.rules.weapons {
            setup
                .rules
                .service
                .round_costs
                .entry(name.clone())
                .or_insert(1);
            weapon.ammo = contract::weapons::AmmoCapacity::Rounds(u32::from(name == "tank_he"));
        }
        let mut b = Battle::new(&setup, seed);
        common::order(
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
        // Observe the single shell's consequences.
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
    common::order(
        &mut b,
        Side::Blue,
        1,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal,
            route: contract::command::RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
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
    common::order(
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
fn damage_repeats_from_its_seed() {
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
    for wall in [true, false] {
        let mut losses = 0.0;
        for seed in 0..6 {
            let props = if wall {
                json!([{ "kind": "wall", "center": [300, 300], "yaw": 0, "half_extents": [0.5, 20, 4] }])
            } else {
                json!([])
            };
            let mut setup = common::scenario_with(
                &map(props, json!([])),
                json!([
                    { "side": "blue", "kind": "tank", "position": [100, 300] },
                    { "side": "red", "kind": "rifle", "position": [306, 300], "engagement": "return_fire_only" }
                ]),
                json!([]),
                json!([]),
            );
            // Only the shells can hurt red; rifle-calibre fire is not the control.
            setup.rules.weapons.get_mut("hmg").unwrap().damage = 0.0;
            setup.rules.physics.flight.min_spread_at_max_range_m = 0.0;
            setup
                .rules
                .weapons
                .get_mut("tank_he")
                .unwrap()
                .ballistics
                .scatter_mrad = 0.0;
            let mut b = Battle::new(&setup, seed);
            let full: f64 = b
                .unit(UnitId(1))
                .unwrap()
                .members
                .iter()
                .map(|s| s.hp)
                .sum();
            common::order(
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
            let mut bursts = 0;
            for _ in 0..420 {
                b.step();
                bursts += b
                    .flight_events()
                    .iter()
                    .filter(|e| {
                        matches!(e,
                    FlightEvent::Impact(i) if i.detonated &&
                    (i.point.xy() - v2(296.0, 300.0)).length() < 2.0)
                    })
                    .count();
            }
            assert!(
                bursts > 0,
                "seed {seed} wall {wall}: a shell burst beside the wall"
            );
            losses += full - own(&b, Side::Red, 1).map_or(0.0, |u| u.member_hp.iter().sum());
        }
        if wall {
            assert_eq!(losses, 0.0, "the wall shields the squad");
        } else {
            assert!(
                losses > 0.0,
                "without the wall the same shelling hurts soldiers"
            );
        }
    }
}

#[test]
fn cover_lowers_losses_to_the_same_fire_over_many_seeds() {
    // The same rifle fire at the same squad behind the same low wall and
    // crates, once with the cover tiers' multipliers and once with every
    // tier at 1: paired seeds, total losses compared (never one lucky seed).
    let losses = |tiers: bool, seed: u64| {
        let mut setup = common::scenario_with(
            &map(
                json!([
                    { "kind": "wall", "center": [381, 275], "yaw": 0, "half_extents": [0.4, 8, 0.5] },
                    { "kind": "crate", "center": [381, 262], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] },
                    { "kind": "crate", "center": [381, 288], "yaw": 0, "half_extents": [0.8, 0.8, 0.6] },
                ]),
                json!([]),
            ),
            json!([
                { "side": "blue", "kind": "rifle", "position": [650, 275] },
                { "side": "red", "kind": "rifle", "position": [375, 275], "engagement": "return_fire_only" },
            ]),
            json!([]),
            json!([]),
        );
        let mut rules = common::scenario_rules();
        // Compare cover tiers at one incoming trajectory. Projectile tuning
        // otherwise changes how much of the fire the low wall physically screens.
        rules["weapons"]["rifle"]["speed_mps"] = json!(510.0);
        sim::fixtures::patch_catalog(&mut rules, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
        setup.rules = serde_json::from_value(rules).unwrap();
        if !tiers {
            let t = &mut setup.rules.cover.tiers;
            (t.light, t.medium, t.heavy) = (1.0, 1.0, 1.0);
        }
        let mut b = Battle::new(&setup, seed);
        // Keep every operator alive so this compares loss rates without wipeout saturation.
        let full: f64 = own(&b, Side::Red, 1).unwrap().member_hp.iter().sum();
        run(&mut b, 600);
        full - own(&b, Side::Red, 1).unwrap().member_hp.iter().sum::<f64>()
    };
    let (mut open, mut covered) = (0.0, 0.0);
    for seed in 0..16 {
        open += losses(false, seed);
        covered += losses(true, seed);
    }
    assert!(open > 0.0, "the fire hurts");
    assert!(
        covered < open * 0.85,
        "without cover {open}, with {covered}"
    );
}
