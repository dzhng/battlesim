//! Shared hosts: several squads share a building or a carrier while seats
//! remain, driven through commands, stepping and the side's observation.
use contract::command::{CommandEnvelope, Order, OrderError, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::observation::{HoldPhase, HostRef};
use serde_json::json;
use sim::battle::Battle;
use sim::garrison::seats_for_state;

use crate::common;

mod engaged;
mod occupancy;

const TEMPLATE: &str = "china-apartment-point-20x20-7f";
const BUILDING: u32 = 0;

/// Two blue rifle squads: `A` beside the building, `B` well to the west.
const A: u32 = 0;
const B: u32 = 1;

/// A battle whose building seats one of the two squads but not both: the
/// test sets the cap itself from the squads' own size.
fn relief_battle() -> Battle {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": [370.0, 300.0] },
        { "side": "blue", "kind": "test_rifle", "position": [220.0, 300.0] },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    let squad = Battle::new(&setup, 1)
        .unit(UnitId(A))
        .unwrap()
        .members
        .len() as u32;
    setup.rules.buildings.capacity_soldiers = 2 * squad - 1;
    Battle::new(&setup, 1)
}

fn occupy(unit: u32) -> Order {
    serde_json::from_value(json!({
        "kind": "occupy_building", "units": [unit], "building": BUILDING, "gesture": unit + 1
    }))
    .unwrap()
}

/// Send a blue order; it must be accepted with no unit refused.
fn accepted(b: &mut Battle, seq: u64, order: Order) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    });
    assert!(ack.error.is_none() && ack.refused.is_empty(), "{ack:?}");
}

fn living(b: &Battle, id: u32) -> usize {
    b.unit(UnitId(id))
        .map_or(0, |u| u.members.iter().filter(|s| s.alive()).count())
}

/// The unit's phase in the building, as its side sees it.
fn phase(b: &Battle, id: u32) -> Option<HoldPhase> {
    let unit = b
        .observe(Side::Blue)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))?;
    let hold = unit.hold?;
    matches!(hold.host, HostRef::Building { id, .. } if id == BUILDING).then_some(hold.phase)
}

/// Every own squad holding the building publishes the seat total admission
/// counts against, stepping in, inside and stepping out alike.
fn assert_published_seats(b: &Battle, seats: usize) {
    for unit in &b.observe(Side::Blue).own {
        if let Some(HostRef::Building {
            id, seats: shown, ..
        }) = unit.hold.map(|h| h.host)
        {
            assert_eq!(
                (id, shown as usize),
                (BUILDING, seats),
                "tick {}: unit {} publishes the house's seats",
                b.tick(),
                unit.id.0
            );
        }
    }
}

#[test]
fn a_squad_ordered_into_a_held_house_walks_in_when_its_holder_leaves() {
    let mut b = relief_battle();
    let building = b.world().prop(BUILDING).cloned().unwrap();
    let seats = seats_for_state(b.world(), &building, b.rules()).len();
    let (a, squad_b) = (living(&b, A), living(&b, B));
    assert!(
        a <= seats && squad_b <= seats && a + squad_b > seats,
        "the building seats either squad but not both: {a} + {squad_b} into {seats}"
    );
    accepted(&mut b, 1, occupy(A));
    let limit = 3000;
    let mut t = 0;
    while phase(&b, A) != Some(HoldPhase::Inside) {
        assert!(t < limit, "A inside within {limit} ticks");
        b.step();
        assert_published_seats(&b, seats);
        t += 1;
    }

    // B is ordered into the full house, then A is told to leave: the player's
    // relief manoeuvre. Both commands are accepted.
    accepted(&mut b, 2, occupy(B));
    accepted(
        &mut b,
        3,
        Order::ExitBuilding {
            units: vec![UnitId(A)],
        },
    );

    let mut refusals: Vec<OrderError> = Vec::new();
    let (mut a_left, mut b_stepped_in) = (false, false);
    for t in 0..limit {
        b.step();
        let frame = b.observe(Side::Blue);
        refusals.extend(
            frame
                .refusals
                .iter()
                .filter(|r| r.unit == UnitId(B))
                .map(|r| r.reason.clone()),
        );
        assert_published_seats(&b, seats);
        let (pa, pb) = (phase(&b, A), phase(&b, B));
        a_left |= pa == Some(HoldPhase::Exiting);
        if pb.is_some() {
            b_stepped_in = true;
            assert!(
                a_left,
                "tick {t}: B stepped in ({pb:?}) before A began stepping out"
            );
        }
        // A seat is taken from the moment a squad starts stepping in; a squad
        // stepping out still stands at its windows.
        let taken = |count: &[HoldPhase]| -> usize {
            [(A, pa), (B, pb)]
                .iter()
                .filter(|(_, p)| p.is_some_and(|p| count.contains(&p)))
                .map(|&(id, _)| living(&b, id))
                .sum()
        };
        assert!(
            taken(&[HoldPhase::Entering, HoldPhase::Inside]) <= seats,
            "tick {t}: seats claimed past the {seats} the building has"
        );
        assert!(
            taken(&[HoldPhase::Inside, HoldPhase::Exiting]) <= seats,
            "tick {t}: more soldiers at windows than the {seats} the building has"
        );
        if pa.is_none() && pb == Some(HoldPhase::Inside) {
            break;
        }
    }
    assert!(b_stepped_in, "B never started stepping in");
    assert_eq!(refusals, [], "no refusal is published for B");
    assert_eq!(phase(&b, A), None, "A ends outside");
    assert_eq!(phase(&b, B), Some(HoldPhase::Inside), "B ends inside");
    assert_eq!(living(&b, B), squad_b, "B is inside whole");
}

/// One building order names three squads, two of them side by side west of
/// the house: each gets its own approach, the squads' grounds apart, and walks
/// to the house to step in; a double right-click hurries them all.
#[test]
fn every_squad_a_building_order_names_walks_to_the_house_and_a_double_click_hurries_them() {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let starts = [[300.0, 300.0], [300.0, 305.0], [520.0, 320.0]];
    let units = json!([
        { "side": "blue", "kind": "test_rifle", "position": starts[0] },
        { "side": "blue", "kind": "test_recon", "position": starts[1] },
        { "side": "blue", "kind": "test_recon", "position": starts[2] },
    ]);
    let mut b = Battle::new(&common::scenario_with(&map, units, json!([]), json!([])), 1);
    let squads = [0, 1, 2];
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        order: serde_json::from_value(json!({
            "kind": "occupy_building", "units": squads, "building": BUILDING, "gesture": 1
        }))
        .unwrap(),
        queued: false,
    });
    assert!(ack.error.is_none() && ack.refused.is_empty(), "{ack:?}");
    let entries = ack.building.expect("a building plan").entries;
    assert_eq!(
        entries.iter().map(|e| e.unit.0).collect::<Vec<_>>(),
        squads,
        "one entry per squad"
    );
    // Each approach lies on the side of the house its squad comes from.
    for (e, from) in entries.iter().zip(starts) {
        let (to_squad, to_approach) = (
            [from[0] - 400.0, from[1] - 300.0],
            [e.approach[0] - 400.0, e.approach[1] - 300.0],
        );
        assert!(
            to_squad[0] * to_approach[0] + to_squad[1] * to_approach[1] > 0.0,
            "{e:?} faces away from its squad at {from:?}"
        );
    }
    // No two squads' grounds overlap where they stand to step in.
    let ground =
        |id: u32| sim::arrangement::spread(&b.rules().infantry_movement, living(&b, id)) / 2.0;
    for (i, e) in entries.iter().enumerate() {
        for f in &entries[i + 1..] {
            let apart = (e.approach[0] - f.approach[0]).hypot(e.approach[1] - f.approach[1]);
            assert!(
                apart >= ground(e.unit.0) + ground(f.unit.0),
                "{e:?} and {f:?} stand {apart:.1} m apart, their grounds overlap"
            );
        }
    }
    // A double right-click hurries the walk its first click started, and
    // only that one: every squad walks in by the fastest route.
    let walks = |b: &Battle| -> Vec<_> {
        let own = &b.observe(Side::Blue).own;
        (squads.iter())
            .map(|&id| own.iter().find(|u| u.id == UnitId(id)).unwrap().policy)
            .collect()
    };
    let upgrade = |gesture| Order::UpgradeMove {
        gesture,
        route: RoutePolicy::Fastest,
    };
    b.step();
    accepted(&mut b, 2, upgrade(2));
    b.step();
    assert_eq!(
        walks(&b),
        [Some(RoutePolicy::Shortest); 3],
        "another gesture's upgrade"
    );
    accepted(&mut b, 3, upgrade(1));
    b.step();
    assert_eq!(
        walks(&b),
        [Some(RoutePolicy::Fastest); 3],
        "the hurried walk"
    );
    let mut stepped_in = [false; 3];
    for _ in 0..3000 {
        b.step();
        for (k, &id) in squads.iter().enumerate() {
            stepped_in[k] |= phase(&b, id).is_some();
        }
        if stepped_in.iter().all(|&s| s) {
            break;
        }
    }
    assert_eq!(stepped_in, [true; 3], "every squad reached the house");
}

/// Blue's refusals this tick for `id`.
fn refused(b: &Battle, id: u32) -> Vec<OrderError> {
    let frame = b.observe(Side::Blue);
    (frame.refusals.iter())
        .filter(|r| r.unit == UnitId(id))
        .map(|r| r.reason.clone())
        .collect()
}

/// Step at least once and until `test` holds, collecting `watched`'s
/// refusals on the way; panics after `limit` ticks.
fn until(
    b: &mut Battle,
    limit: u32,
    watched: u32,
    what: &str,
    mut test: impl FnMut(&Battle) -> bool,
) -> Vec<OrderError> {
    let mut seen = Vec::new();
    for _ in 0..limit {
        b.step();
        seen.extend(refused(b, watched));
        if test(b) {
            return seen;
        }
    }
    panic!("{what} within {limit} ticks");
}

/// Whether a squad holds no host and has no orders left.
fn idle(b: &Battle, id: u32) -> bool {
    let unit = b.unit(UnitId(id)).unwrap();
    unit.hold.is_none() && unit.orders.is_empty()
}

/// Where each living soldier of the squads inside `ids` stands.
fn at_windows(b: &Battle, ids: &[u32]) -> Vec<(u32, [i64; 3])> {
    let mut held = Vec::new();
    for &id in ids {
        let unit = b.unit(UnitId(id)).unwrap();
        if !unit.garrisoned() {
            continue;
        }
        for s in unit.members.iter().filter(|s| s.alive()) {
            let p = s.position;
            let mm = |v: f64| (v * 1000.0).round() as i64;
            held.push((id, [mm(p.x), mm(p.y), mm(p.z)]));
        }
    }
    held
}

/// Send `order` for blue; returns the units refused one by one.
fn send(b: &mut Battle, seq: &mut u64, order: Order) -> Vec<(UnitId, OrderError)> {
    *seq += 1;
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: *seq,
        order,
        queued: false,
    });
    assert!(ack.error.is_none(), "{ack:?}");
    ack.refused
        .into_iter()
        .map(|r| (r.unit, r.reason))
        .collect()
}

#[test]
fn squads_share_a_host_while_seats_remain_and_a_latecomer_finds_no_room() {
    // A building: three AT squads, any two of which fill it, and a rifle
    // squad bigger than the whole building. The test sets the seats itself.
    let (a, b_, c, big) = (0, 1, 2, 3);
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [],
        "buildings": [common::building(BUILDING, TEMPLATE, [400.0, 300.0], 0.0)] })
    .to_string();
    let units = json!([
        { "side": "blue", "kind": "test_at", "position": [360.0, 290.0] },
        { "side": "blue", "kind": "test_at", "position": [360.0, 310.0] },
        { "side": "blue", "kind": "test_at", "position": [330.0, 300.0] },
        { "side": "blue", "kind": "test_rifle", "position": [360.0, 270.0] },
    ]);
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    let size = |id: u32| {
        Battle::new(&setup, 1)
            .unit(UnitId(id))
            .unwrap()
            .members
            .len() as u32
    };
    setup.rules.buildings.capacity_soldiers = size(a) + size(b_);
    let mut w = Battle::new(&setup, 1);
    let house = w.world().prop(BUILDING).cloned().unwrap();
    let seats = seats_for_state(w.world(), &house, w.rules()).len();
    assert!(
        living(&w, a) + living(&w, b_) <= seats
            && living(&w, a) + living(&w, b_) + living(&w, c) > seats
            && living(&w, big) > seats,
        "two AT squads fill the {seats} seats and the rifles alone overflow them"
    );
    let mut seq = 0;
    let occupy = |units: &[u32]| -> Order {
        serde_json::from_value(json!({
            "kind": "occupy_building", "units": units, "building": BUILDING, "gesture": 1
        }))
        .unwrap()
    };
    let exit = |id: u32| Order::ExitBuilding {
        units: vec![UnitId(id)],
    };

    // Only a squad bigger than the whole building is refused at the command.
    assert_eq!(
        send(&mut w, &mut seq, occupy(&[big])),
        [(UnitId(big), OrderError::CapacityFull)]
    );
    assert!(idle(&w, big), "the refused squad was given nothing");

    // Two squads share it, each at its own windows.
    assert_eq!(send(&mut w, &mut seq, occupy(&[a, b_])), []);
    until(&mut w, 3000, a, "A and B inside", |w| {
        phase(w, a) == Some(HoldPhase::Inside) && phase(w, b_) == Some(HoldPhase::Inside)
    });
    let windows = |w: &Battle| at_windows(w, &[a, b_, c]);
    let distinct = |held: &[(u32, [i64; 3])]| {
        let spots: std::collections::BTreeSet<_> = held.iter().map(|(_, p)| *p).collect();
        spots.len() == held.len()
    };
    assert!(distinct(&windows(&w)), "two soldiers at one window");

    // A third walks over, finds no room and hears why; it doesn't wait.
    assert_eq!(send(&mut w, &mut seq, occupy(&[c])), []);
    let heard = until(&mut w, 3000, c, "C's order ends", |w| idle(w, c));
    assert_eq!(heard, [OrderError::CapacityFull], "C hears no room once");
    assert!(phase(&w, a).is_some() && phase(&w, b_).is_some());

    // A leaves and its seats free up; C stays where it gave up.
    assert_eq!(send(&mut w, &mut seq, exit(a)), []);
    until(&mut w, 600, c, "A out", |w| phase(w, a).is_none());
    for _ in 0..60 {
        w.step();
        assert!(idle(&w, c), "C came back on its own");
    }

    // A goes back in and leaves again; C, sent in while A steps out, is
    // admitted: a leaver's seats don't count when stepping in starts.
    assert_eq!(send(&mut w, &mut seq, occupy(&[a])), []);
    until(&mut w, 600, a, "A back inside", |w| {
        phase(w, a) == Some(HoldPhase::Inside)
    });
    assert_eq!(send(&mut w, &mut seq, exit(a)), []);
    until(&mut w, 60, a, "A stepping out", |w| {
        phase(w, a) == Some(HoldPhase::Exiting)
    });
    let staying = windows(&w);
    assert_eq!(send(&mut w, &mut seq, occupy(&[c])), []);
    until(&mut w, 60, c, "C stepping in", |w| {
        phase(w, c) == Some(HoldPhase::Entering)
    });
    assert_eq!(
        phase(&w, a),
        Some(HoldPhase::Exiting),
        "C admitted beside a leaver"
    );
    let heard = until(&mut w, 600, c, "C inside", |w| {
        phase(w, c) == Some(HoldPhase::Inside)
    });
    assert_eq!(heard, []);
    let now = windows(&w);
    assert!(distinct(&now), "C took a window someone holds");
    for held in staying.iter().filter(|(id, _)| *id == b_) {
        assert!(now.contains(held), "B's soldier moved when C came in");
    }

    // B starts out, A is sent in, then B turns back: A is admitted past the
    // leaver, and at the end of its timer finds everyone and no room.
    assert_eq!(send(&mut w, &mut seq, exit(b_)), []);
    until(&mut w, 60, b_, "B stepping out", |w| {
        phase(w, b_) == Some(HoldPhase::Exiting)
    });
    assert_eq!(send(&mut w, &mut seq, occupy(&[a])), []);
    until(&mut w, 60, a, "A stepping in", |w| {
        phase(w, a) == Some(HoldPhase::Entering)
    });
    assert_eq!(
        send(
            &mut w,
            &mut seq,
            Order::Stop {
                units: vec![UnitId(b_)]
            }
        ),
        []
    );
    let heard = until(&mut w, 600, a, "A's order ends", |w| idle(w, a));
    assert_eq!(
        heard,
        [OrderError::CapacityFull],
        "A hears no room at its end check"
    );
    assert_eq!(phase(&w, b_), Some(HoldPhase::Inside), "B turned back");
    assert_eq!(phase(&w, c), Some(HoldPhase::Inside));

    // A carrier: two scout squads beside an APC that seats both, a third a
    // short walk off, and a scout car too small for any of them. Getting in
    // takes long enough that the third arrives while the two still are.
    let units = json!([
        { "side": "blue", "kind": "test_apc", "position": [200.0, 200.0] },
        { "side": "blue", "kind": "test_scout_car", "position": [200.0, 260.0] },
        { "side": "blue", "kind": "test_recon", "position": [195.0, 200.0] },
        { "side": "blue", "kind": "test_recon", "position": [196.0, 203.0] },
        { "side": "blue", "kind": "test_recon", "position": [200.0, 210.0] },
    ]);
    let (apc, car, p, q, s) = (0, 1, 2, 3, 4);
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35, "props": [], "buildings": [] })
    .to_string();
    let mut setup = common::scenario_with(&map, units, json!([]), json!([]));
    setup.rules.carriage.board_s = 30.0;
    let mut v = Battle::new(&setup, 1);
    let carries = |v: &Battle, id: u32| {
        let kind = v.unit(UnitId(id)).unwrap().kind;
        v.rules()
            .catalog
            .get(kind)
            .capabilities
            .carriage
            .unwrap()
            .seats as usize
    };
    assert!(
        living(&v, p) + living(&v, q) <= carries(&v, apc)
            && living(&v, p) + living(&v, q) + living(&v, s) > carries(&v, apc)
            && living(&v, s) > carries(&v, car),
        "the APC seats two squads, not three; the scout car none"
    );
    let mut seq = 0;
    // Bigger than the carrier, or than every carrier a Collect lists.
    assert_eq!(
        send(
            &mut v,
            &mut seq,
            Order::Board {
                units: vec![UnitId(s)],
                carrier: UnitId(car),
                gesture: 1
            }
        ),
        [(UnitId(s), OrderError::CapacityFull)]
    );
    assert_eq!(
        send(
            &mut v,
            &mut seq,
            Order::Collect {
                units: vec![UnitId(car)],
                squads: vec![UnitId(s)],
                gesture: 2
            }
        ),
        [(UnitId(s), OrderError::CapacityFull)]
    );
    // The side hears those two refusals as the orders apply.
    v.step();
    // An overflowing Collect: the squad that gets there last finds no room,
    // though the two before it are still getting in; they finish.
    assert_eq!(
        send(
            &mut v,
            &mut seq,
            Order::Collect {
                units: vec![UnitId(apc)],
                squads: vec![UnitId(p), UnitId(q), UnitId(s)],
                gesture: 3
            }
        ),
        []
    );
    let heard = until(&mut v, 1200, s, "S's order ends", |v| idle(v, s));
    assert_eq!(heard, [OrderError::CapacityFull], "S hears no room once");
    let getting_in = |v: &Battle, id: u32| {
        (v.unit(UnitId(id)).and_then(|u| u.hold.as_ref()))
            .is_some_and(|h| matches!(h.phase, sim::hold::Phase::Entering(_)))
    };
    assert!(
        getting_in(&v, p) && getting_in(&v, q),
        "S came too late to matter"
    );
    let heard = until(&mut v, 1200, p, "P and Q aboard", |v| {
        [p, q].iter().all(|&id| {
            v.unit(UnitId(id))
                .and_then(|u| u.hold.as_ref())
                .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
        })
    });
    assert_eq!(heard, [], "P, first to finish, is never refused");
}
