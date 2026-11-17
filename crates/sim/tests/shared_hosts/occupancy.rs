//! Squads sharing a house fight as one garrison and are reinforced only into
//! the host's free seats.
use std::collections::{BTreeMap, BTreeSet};

use contract::command::{Order, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::{HoldPhase, ServiceStatus};
use contract::scenario::{Rules, ScenarioDefinition};
use serde_json::json;
use sim::battle::Battle;
use sim::garrison::seats_for_state;
use sim::hold::Phase;

use super::{living, occupy, phase, refused, BUILDING, TEMPLATE};
use crate::common;

/// Each living garrisoned soldier's window, by (unit, member), of `ids`.
fn window_holders(b: &Battle, ids: &[u32]) -> BTreeMap<(u32, usize), usize> {
    let mut held = BTreeMap::new();
    for &id in ids {
        let Some(unit) = b.unit(UnitId(id)).filter(|u| u.garrisoned()) else {
            continue;
        };
        let seats = &unit.hold.as_ref().unwrap().building().unwrap().seats;
        for (k, _) in unit.members.iter().enumerate().filter(|(_, s)| s.alive()) {
            held.insert(
                (id, k),
                seats[k].expect("a living soldier inside holds a window"),
            );
        }
    }
    held
}

/// A duration in whole ticks, as the rules count it.
fn hold_ticks(seconds: f64, rules: &Rules) -> u64 {
    ((seconds * rules.tick_hz as f64).round() as u64).max(1)
}

fn attack_ground(unit: u32, p: [f64; 2]) -> Order {
    Order::Attack {
        units: vec![UnitId(unit)],
        target: TargetRef::Ground {
            point: [p[0], p[1], 0.0],
        },
    }
}

#[test]
fn squads_sharing_a_house_fight_as_one_garrison() {
    // A rifle squad two men short and an AT team fill a house between them;
    // a supply truck waits out of reach, and a tank waits far north, unseen.
    let (rifles, at, truck, tank) = (0u32, 1u32, 2u32, 3u32);
    let map = json!({ "size": [800, 1100], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [370.0, 300.0],
          "engagement": "return_fire_only", "condition": { "casualties": 2 } },
        { "side": "blue", "kind": "test_at", "position": [360.0, 280.0], "engagement": "return_fire_only" },
        { "side": "blue", "kind": "test_supply", "position": [280.0, 240.0] },
        { "side": "red", "kind": "test_tank", "position": [400.0, 960.0], "engagement": "return_fire_only" },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    let size = |id: u32| living(&Battle::new(&setup, 1), id) as u32;
    setup.rules.buildings.capacity_soldiers = size(rifles) + size(at);
    // A fragment of the tank's shell that reaches a soldier kills him, the
    // house's cover or not.
    setup.rules.weapons.get_mut("tank_he").unwrap().damage = 1e6;
    // Keep the tank available for the later garrison commands.
    setup.rules.weapons.get_mut("atgm").unwrap().damage = 0.0;
    setup.rules.cover.building_fragment_probability_multiplier = 1.0;
    let plan = {
        let b = Battle::new(&setup, 1);
        seats_for_state(b.world(), b.world().prop(BUILDING).unwrap(), b.rules())
    };
    let mut b = Battle::new(&setup, 1);
    let seats = plan.len();
    let short = b
        .rules()
        .catalog
        .get(b.unit(UnitId(rifles)).unwrap().kind)
        .squad_size()
        - living(&b, rifles);
    assert!(
        living(&b, rifles) + living(&b, at) == seats && short > 0,
        "the two squads fill the {seats} seats and the rifles are short"
    );
    let mut digests = Vec::new();
    let mut step = |b: &mut Battle| {
        b.step();
        digests.push(b.digest());
    };
    let mut c = common::Commander::new();
    let both = [rifles, at];

    // The rifles go in first, then the AT team beside them.
    c.ok(&mut b, Side::Blue, occupy(rifles));
    while phase(&b, rifles) != Some(HoldPhase::Inside) {
        assert!(b.tick() < 1500, "the rifles inside");
        step(&mut b);
    }
    c.ok(&mut b, Side::Blue, occupy(at));
    while phase(&b, at) != Some(HoldPhase::Inside) {
        assert!(b.tick() < 1500, "the AT team inside");
        step(&mut b);
    }
    let held = window_holders(&b, &both);
    assert_eq!(held.len(), seats, "every seat taken");
    let windows: BTreeSet<_> = held.values().collect();
    assert_eq!(windows.len(), seats, "two soldiers at one window");

    // The tank drives into view, 250 m north. Every window facing it is the
    // rifles': the AT gunner takes one from a rifleman, who goes to his.
    let (gunner, facing) = (
        (at, 0usize),
        b.rules().garrison.slot_facing_min_deg.to_radians(),
    );
    let tank_at = sim::math::v2(400.0, 550.0);
    let faces_tank = |s: usize| plan[s].slot.faces(tank_at, facing);
    assert!(
        held.iter()
            .filter(|(_, &s)| faces_tank(s))
            .all(|((id, _), _)| *id == rifles)
            && (0..seats).any(faces_tank)
            && !faces_tank(held[&gunner]),
        "only the rifles hold windows facing the tank"
    );
    c.ok(
        &mut b,
        Side::Red,
        Order::Move {
            units: vec![UnitId(tank)],
            gesture: 1,
            goal: [tank_at.x, tank_at.y],
            route: RoutePolicy::Shortest,
            direction: Default::default(),
            facing: None,
        },
    );
    let mut was = window_holders(&b, &both);
    let swapped = loop {
        assert!(b.tick() < 2000, "the gunner faces the tank");
        step(&mut b);
        let now = window_holders(&b, &both);
        if now[&gunner] != was[&gunner] {
            let (from, to) = (was[&gunner], now[&gunner]);
            assert!(
                faces_tank(to),
                "the gunner moved to a window facing the tank"
            );
            let (&rifleman, _) = was.iter().find(|(_, &s)| s == to).unwrap();
            assert_eq!(rifleman.0, rifles, "he took a rifleman's window");
            assert_eq!(now[&rifleman], from, "the rifleman took the gunner's");
            break [from, to];
        }
        was = now;
    };

    // The rifles are ordered to fire east and west in turn, far faster
    // than a soldier may change windows again. Riflemen trade windows across
    // both squads, but no window changes hands within its hold, a soldier
    // who changes doesn't fire until he has got there, and nobody displaces
    // the gunner.
    let rules = b.rules().clone();
    let (hold, swap) = (
        hold_ticks(rules.garrison.window_hold_s, &rules),
        hold_ticks(rules.garrison.window_swap_s, &rules),
    );
    let mut last_change = vec![None; seats];
    for s in swapped {
        last_change[s] = Some(b.tick());
    }
    let mut was = window_holders(&b, &both);
    let mut at_moved = false;
    for t in 0..900 {
        if t % 20 == 0 {
            let x = if (t / 20) % 2 == 0 { 550.0 } else { 250.0 };
            c.ok(&mut b, Side::Blue, attack_ground(rifles, [x, 300.0]));
        }
        step(&mut b);
        let now = window_holders(&b, &both);
        assert_eq!(now.len(), seats, "tick {}: the house stays full", b.tick());
        assert_eq!(
            now.values().collect::<BTreeSet<_>>().len(),
            seats,
            "tick {}: two soldiers at one window",
            b.tick()
        );
        assert!(
            faces_tank(now[&gunner]),
            "tick {}: the gunner was displaced",
            b.tick()
        );
        for (soldier, &s) in &now {
            if was[soldier] == s {
                continue;
            }
            at_moved |= soldier.0 == at;
            let tick = b.tick();
            for window in [was[soldier], s] {
                let since = last_change[window].map(|t| tick - t);
                assert!(
                    since.is_none_or(|n| n >= hold),
                    "tick {tick}: window {window} changed hands {since:?} ticks after it last did"
                );
            }
            let unit = b.unit(UnitId(soldier.0)).unwrap();
            let changing = |t| b.windows().changing_window(unit, soldier.1, &rules, t);
            assert!(
                changing(tick) && changing(tick + swap - 1) && !changing(tick + swap),
                "tick {tick}: {soldier:?} holds fire while changing windows, and only then"
            );
        }
        for (soldier, &s) in &now {
            if was[soldier] != s {
                last_change[was[soldier]] = Some(b.tick());
                last_change[s] = Some(b.tick());
            }
        }
        was = now;
    }
    assert!(
        at_moved,
        "a rifleman of the AT team traded windows with the rifles"
    );
    c.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(rifles)],
        },
    );

    // The truck drives up and sets up beside the full house: the rifles get
    // no replacement, and the truck spends nothing.
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(truck)],
            gesture: 2,
            goal: [400.0, 240.0],
            route: RoutePolicy::Shortest,
            direction: Default::default(),
            facing: None,
        },
    );
    let status = |b: &Battle| {
        b.observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(rifles))
            .unwrap()
            .service
    };
    let stock = |b: &Battle| b.unit(UnitId(truck)).unwrap().stock.unwrap();
    let (short_of, full_stock) = (living(&b, rifles), stock(&b));
    while status(&b) != ServiceStatus::Full {
        assert!(b.tick() < 4000, "the truck set up beside the house");
        step(&mut b);
    }
    let every = hold_ticks(rules.service.soldier_replacement_s, &rules);
    for _ in 0..3 * every {
        step(&mut b);
        assert_eq!(
            living(&b, rifles),
            short_of,
            "tick {}: reinforced into a full house",
            b.tick()
        );
        assert_eq!(
            stock(&b),
            full_stock,
            "tick {}: the truck spent stock",
            b.tick()
        );
    }

    // The tank shells the gunner's window and the AT team answers. Each seat
    // a casualty frees is a replacement for the rifles, at a window nobody
    // holds, each with the next soldier id.
    let window = plan[swapped[1]].position;
    c.ok(
        &mut b,
        Side::Red,
        Order::Attack {
            units: vec![UnitId(tank)],
            target: TargetRef::Ground {
                point: [window.x, window.y, window.z],
            },
        },
    );
    let next_id = |b: &Battle| {
        (0..4)
            .flat_map(|id| b.unit(UnitId(id)).unwrap().members.iter().map(|s| s.id))
            .max()
            .unwrap()
            + 1
    };
    let first_id = next_id(&b);
    let mut expected_id = first_id;
    let house_living = |b: &Battle| living(b, rifles) + living(b, at);
    while house_living(&b) == seats {
        assert!(b.tick() < 5000, "the tank killed someone in the house");
        step(&mut b);
    }
    c.ok(
        &mut b,
        Side::Red,
        Order::Stop {
            units: vec![UnitId(tank)],
        },
    );
    let squad = rules
        .catalog
        .get(b.unit(UnitId(rifles)).unwrap().kind)
        .squad_size();
    let mut before = b.unit(UnitId(rifles)).unwrap().members.len();
    while living(&b, rifles) < squad && house_living(&b) < seats {
        assert!(
            b.tick() < 8000,
            "the rifles reinforced into the freed seats"
        );
        step(&mut b);
        let now = window_holders(&b, &both);
        assert!(
            house_living(&b) <= seats,
            "tick {}: house overfilled",
            b.tick()
        );
        assert_eq!(
            now.values().collect::<BTreeSet<_>>().len(),
            now.len(),
            "tick {}: two soldiers at one window",
            b.tick()
        );
        let members = &b.unit(UnitId(rifles)).unwrap().members;
        for joined in &members[before..] {
            assert_eq!(
                joined.id, expected_id,
                "a soldier id was spent without a soldier"
            );
            expected_id += 1;
        }
        before = members.len();
    }
    assert!(expected_id > first_id, "no replacement joined the rifles");

    replays(&setup, &b, digests);

    // A carrier: an AT team gets into an APC, and a rifle squad short of
    // soldiers fills its last seats. A truck sets up beside it while the
    // rifles step in, slowly: they get no replacement, and board whole.
    let (apc, at, rifles) = (0u32, 1u32, 2u32);
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [], "buildings": [] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_apc", "position": [200.0, 200.0] },
        { "side": "blue", "kind": "test_at", "position": [195.0, 200.0] },
        { "side": "blue", "kind": "test_rifle", "position": [200.0, 225.0], "condition": { "casualties": 3 } },
        { "side": "blue", "kind": "test_supply", "position": [230.0, 200.0] },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    setup.rules.carriage.board_s = 10.0;
    setup.rules.service.soldier_replacement_s = 0.5;
    let mut v = Battle::new(&setup, 1);
    let rules = v.rules().clone();
    let carries = rules
        .catalog
        .get(v.unit(UnitId(apc)).unwrap().kind)
        .capabilities
        .carriage
        .unwrap()
        .seats as usize;
    let short_of = living(&v, rifles);
    assert!(
        living(&v, at) + short_of == carries
            && short_of
                < rules
                    .catalog
                    .get(v.unit(UnitId(rifles)).unwrap().kind)
                    .squad_size(),
        "the two squads fill the APC's {carries} seats and the rifles are short"
    );
    let mut digests = Vec::new();
    let mut c = common::Commander::new();
    c.ok(
        &mut v,
        Side::Blue,
        Order::Board {
            units: vec![UnitId(at), UnitId(rifles)],
            carrier: UnitId(apc),
            gesture: 1,
        },
    );
    let hold = |v: &Battle, id: u32| v.unit(UnitId(id)).unwrap().hold.as_ref().map(|h| h.phase);
    let status = |v: &Battle| {
        v.observe(Side::Blue)
            .own
            .iter()
            .find(|u| u.id == UnitId(rifles))
            .unwrap()
            .service
    };
    let mut ready_beside = 0;
    while hold(&v, rifles) != Some(Phase::Inside) {
        assert!(v.tick() < 1500, "the rifles aboard");
        v.step();
        digests.push(v.digest());
        assert_eq!(refused(&v, rifles), [], "the rifles are never refused");
        assert_eq!(
            living(&v, rifles),
            short_of,
            "tick {}: reinforced past the seats",
            v.tick()
        );
        if matches!(hold(&v, rifles), Some(Phase::Entering(_))) {
            ready_beside += (status(&v) == ServiceStatus::Full) as u64;
        }
    }
    assert!(
        ready_beside > hold_ticks(rules.service.soldier_replacement_s, &rules),
        "the truck stood ready while the rifles stepped in ({ready_beside} ticks)"
    );
    assert_eq!(hold(&v, at), Some(Phase::Inside), "the AT team aboard");
    replays(&setup, &v, digests);
}

/// The battle replays to `digests`, tick by tick.
fn replays(setup: &ScenarioDefinition, b: &Battle, digests: Vec<u64>) {
    let mut r = Battle::from_replay(setup, &b.replay()).unwrap();
    for d in digests {
        r.step();
        assert_eq!(r.digest(), d, "tick {}", r.tick());
    }
}

#[test]
fn a_squad_reinforced_during_entry_never_enters_only_partly_seated() {
    // Reinforced while stepping in only into the seats, it enters whole.
    let mut setup = crate::buildings::compound_setup(json!([]));
    setup.rules.garrison.enter_exit_s = 15.0;
    setup.rules.service.soldier_replacement_s = 0.1;
    setup.units = serde_json::from_value(json!([
        {"side":"blue","kind":"test_rifle","position":[380,300],"engagement":"return_fire_only","condition":{"casualties":2}},
        {"side":"blue","kind":"test_supply","position":[380,330],"engagement":"return_fire_only"}
    ])).unwrap();
    let size = setup.rules.catalog.by_id("test_rifle").squad_size();
    setup.rules.buildings.capacity_soldiers = size as u32 - 1;
    let mut b = Battle::new(&setup, 11);
    let n = |b: &Battle| b.observe(Side::Blue).own[0].members.len();
    let seats = sim::garrison::seats_for_state(b.world(), b.world().prop(0).unwrap(), b.rules());
    assert!(n(&b) < seats.len() && seats.len() < size);
    let squad = vec![UnitId(0)];
    let order = Order::OccupyBuilding {
        units: squad,
        building: 0,
        gesture: 0,
    };
    crate::common::order(&mut b, Side::Blue, 1, order);
    while !b.unit(UnitId(0)).unwrap().garrisoned() {
        assert!(b.tick() < 1400 && b.observe(Side::Blue).refusals.is_empty());
        b.step();
        assert!(n(&b) <= seats.len(), "reinforced past the seats");
    }
    assert_eq!(n(&b), seats.len(), "every seat filled");
}
