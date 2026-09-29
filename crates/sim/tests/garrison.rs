//! Buildings as abstract fighting positions (slice 11), driven through real
//! scenarios and commands.
use std::collections::{BTreeMap, BTreeSet};

use contract::command::{CommandEnvelope, Engagement, Order, OrderError, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::{ActionReason, GarrisonPhase, OwnUnit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, ProjectileId, Struck};
use sim::math::{v2, v3, V2};
use sim::world::Prop;

use crate::common::{self, Commander};

/// The building every test garrisons: prop 0, 24 × 18 m, 8 m tall.
const CENTRE: [f64; 2] = [400.0, 300.0];
const HALF: [f64; 3] = [12.0, 9.0, 4.0];
const BUILDING: u32 = 0;

fn rules() -> Value {
    common::village()
}

fn num(section: &str, key: &str) -> f64 {
    rules()[section][key].as_f64().unwrap()
}

fn ticks(seconds: f64) -> u64 {
    (seconds * rules()["tick_hz"].as_f64().unwrap()).round() as u64
}

fn map(extra_props: Value) -> String {
    let mut props =
        vec![json!({ "kind": "building", "center": CENTRE, "yaw": 0, "half_extents": HALF })];
    props.extend(extra_props.as_array().unwrap().iter().cloned());
    json!({ "size": [800, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

fn battle_with(extra_props: Value, units: Value, events: Value, seed: u64) -> Battle {
    Battle::new(
        &common::scenario_with(&map(extra_props), units, events, json!([])),
        seed,
    )
}

fn battle(units: Value, seed: u64) -> Battle {
    battle_with(json!([]), units, json!([]), seed)
}

fn garrison(units: &[u32]) -> Order {
    Order::Garrison {
        units: units.iter().map(|&u| UnitId(u)).collect(),
        building: BUILDING,
    }
}

fn exit(units: &[u32]) -> Order {
    Order::ExitBuilding {
        units: units.iter().map(|&u| UnitId(u)).collect(),
    }
}

fn ground(unit: u32, p: [f64; 2]) -> Order {
    Order::Attack {
        units: vec![UnitId(unit)],
        target: TargetRef::Ground {
            point: [p[0], p[1], 0.0],
        },
    }
}

fn own(b: &Battle, side: Side, id: u32) -> Option<OwnUnit> {
    b.observe(side)
        .own
        .iter()
        .find(|u| u.id == UnitId(id))
        .cloned()
}

fn phase(b: &Battle, side: Side, id: u32) -> Option<GarrisonPhase> {
    own(b, side, id)?.garrison.map(|g| g.phase)
}

fn inside(b: &Battle, id: u32) -> bool {
    b.unit(UnitId(id)).is_some_and(|u| u.garrisoned())
}

fn run(b: &mut Battle, n: u64) {
    for _ in 0..n {
        b.step();
    }
}

/// Step until `test` holds; panics after `limit` ticks.
fn until(b: &mut Battle, limit: u64, what: &str, mut test: impl FnMut(&Battle) -> bool) -> u64 {
    for t in 0..limit {
        if test(b) {
            return t;
        }
        b.step();
    }
    panic!("{what} within {limit} ticks");
}

fn building(b: &Battle) -> Prop {
    b.world().prop(BUILDING).cloned().expect("building stands")
}

/// Metres a point lies outside the building's footprint (negative inside).
fn outside_by(p: &Prop, q: V2) -> f64 {
    let d = (q - p.center).rotated(-p.yaw);
    let (ex, ey) = (d.x.abs() - p.half.x, d.y.abs() - p.half.y);
    if ex > 0.0 || ey > 0.0 {
        ex.max(0.0).hypot(ey.max(0.0))
    } else {
        ex.max(ey)
    }
}

fn living(b: &Battle, id: u32) -> usize {
    b.unit(UnitId(id))
        .map_or(0, |u| u.members.iter().filter(|s| s.alive()).count())
}

/// Blue squads west of the building (ids 0.. in order).
fn west_squads(kinds: &[&str]) -> Value {
    Value::Array(
        kinds
            .iter()
            .enumerate()
            .map(|(k, kind)| {
                json!({ "side": "blue", "kind": kind, "position": [350.0, 285.0 + 15.0 * k as f64] })
            })
            .collect(),
    )
}

/// A battle on the test map whose buildings take `capacity` soldiers.
fn battle_with_capacity(units: Value, seed: u64, capacity: u32) -> Battle {
    let mut setup = common::scenario_with(&map(json!([])), units, json!([]), json!([]));
    setup.rules.buildings.capacity_soldiers = capacity;
    Battle::new(&setup, seed)
}

/// One rifle squad garrisoned in a building that takes exactly its eight
/// soldiers: every slot held.
fn full_building(seed: u64) -> (Battle, Commander) {
    let mut b = battle_with_capacity(west_squads(&["rifle"]), seed, 8);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "the squad inside", |b| inside(b, 0));
    (b, c)
}

#[test]
fn a_building_takes_one_squad_and_a_second_is_refused_until_it_leaves() {
    let mut b = battle(west_squads(&["rifle", "recon"]), 1);
    let mut c = Commander::new();
    // An order naming two squads is refused whole: nobody moves.
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[0, 1]), false),
        Some(OrderError::OneSquadPerBuilding)
    );
    run(&mut b, 5);
    assert!((0..2).all(|u| own(&b, Side::Blue, u).unwrap().goal.is_none()));
    // A second squad ordered in the same tick is refused, as is one ordered
    // while the first walks in, once it is inside, queued or not.
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[1]), false),
        Some(OrderError::BuildingOccupied),
        "ordered in the same tick"
    );
    b.step();
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[1]), false),
        Some(OrderError::BuildingOccupied),
        "the first squad is on its way in"
    );
    until(&mut b, 1200, "the first squad inside", |b| inside(b, 0));
    for queued in [false, true] {
        assert_eq!(
            c.send(&mut b, Side::Blue, garrison(&[1]), queued),
            Some(OrderError::BuildingOccupied)
        );
    }
    assert_eq!(phase(&b, Side::Blue, 1), None);
    // Leaving frees the building, once the squad is out.
    c.ok(&mut b, Side::Blue, exit(&[0]));
    run(&mut b, 2);
    assert_eq!(phase(&b, Side::Blue, 0), Some(GarrisonPhase::Exiting));
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[1]), false),
        Some(OrderError::BuildingOccupied),
        "still leaving"
    );
    until(&mut b, 200, "the first squad out", |b| {
        b.unit(UnitId(0)).unwrap().garrison.is_none()
    });
    c.ok(&mut b, Side::Blue, garrison(&[1]));
    until(&mut b, 1200, "the second squad inside", |b| inside(b, 1));
    // Non-infantry and non-buildings are refused.
    let mut v = battle(
        json!([{ "side": "blue", "kind": "tank", "position": [350, 300] }]),
        1,
    );
    let mut cv = Commander::new();
    assert_eq!(
        cv.send(&mut v, Side::Blue, garrison(&[0]), false),
        Some(OrderError::NotInfantry { unit: UnitId(0) })
    );
    let mut w = battle_with(
        json!([{ "kind": "wall", "center": [300, 100], "yaw": 0, "half_extents": [5, 1, 2] }]),
        west_squads(&["rifle"]),
        json!([]),
        1,
    );
    let mut cw = Commander::new();
    assert_eq!(
        cw.send(
            &mut w,
            Side::Blue,
            Order::Garrison {
                units: vec![UnitId(0)],
                building: 1
            },
            false
        ),
        Some(OrderError::NotABuilding)
    );
}

#[test]
fn a_squad_larger_than_the_building_is_refused_and_a_smaller_one_enters_whole() {
    // Buildings that take six: the rifle squad's eight are refused, the
    // scouts' four enter, every one of them at a slot.
    let mut b = battle_with_capacity(west_squads(&["rifle", "recon"]), 1, 6);
    let mut c = Commander::new();
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[0]), false),
        Some(OrderError::CapacityFull)
    );
    run(&mut b, 5);
    assert!(own(&b, Side::Blue, 0).unwrap().goal.is_none());
    c.ok(&mut b, Side::Blue, garrison(&[1]));
    until(&mut b, 1200, "the scouts inside", |b| inside(b, 1));
    let g = b.unit(UnitId(1)).unwrap().garrison.clone().unwrap();
    assert_eq!(g.seats.iter().flatten().count(), 4);
}

#[test]
fn a_squad_that_finds_an_enemy_squad_inside_gives_up_its_order() {
    // Neither side knows the other's orders at the command: blue's order is
    // accepted, and at the end of its entry timer blue finds red inside. The
    // order lapses and blue stands where it is; it never waits for room.
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "rifle", "position": [350.0, 300.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [450.0, 300.0], "engagement": "return_fire_only" },
        ]),
        1,
    );
    let mut c = Commander::new();
    c.ok(&mut b, Side::Red, garrison(&[1]));
    until(&mut b, 1200, "red inside", |b| inside(b, 1));
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1500, "blue entering", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    until(&mut b, 600, "blue's order lapsed", |b| {
        phase(b, Side::Blue, 0).is_none()
    });
    let u = own(&b, Side::Blue, 0).unwrap();
    assert!(u.goal.is_none() && u.queue.is_empty());
    assert!(inside(&b, 1));
    // Red leaving later does not bring blue in.
    c.ok(&mut b, Side::Red, exit(&[1]));
    run(&mut b, 600);
    assert!(!inside(&b, 0) && phase(&b, Side::Blue, 0).is_none());
}

#[test]
fn a_squad_enters_after_arriving_and_a_stationary_timer() {
    let mut b = battle(west_squads(&["rifle"]), 2);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    let walk = until(&mut b, 1200, "entering", |b| {
        phase(b, Side::Blue, 0) == Some(GarrisonPhase::Entering)
    });
    assert!(walk > 30, "it walked to the building first");
    let p = building(&b);
    let at = own(&b, Side::Blue, 0).unwrap();
    // Within the entry distance as the rule measures it: the footprint grown
    // by it on every side (square corners).
    assert!(p.footprint().contains(
        v2(at.position[0], at.position[1]),
        num("garrison", "entry_distance_m")
    ));
    // Stationary for the whole timer, then inside on the tick it ends.
    let start = [at.position[0], at.position[1]];
    let timer = ticks(num("garrison", "enter_exit_s"));
    for _ in 1..timer {
        b.step();
        let u = own(&b, Side::Blue, 0).unwrap();
        assert_eq!(u.garrison.unwrap().phase, GarrisonPhase::Entering);
        assert_eq!(
            [u.position[0], u.position[1]],
            start,
            "stationary while entering"
        );
        assert!(u
            .mounts
            .iter()
            .all(|m| m.reason == ActionReason::ChangingPosition));
    }
    b.step();
    let u = own(&b, Side::Blue, 0).unwrap();
    let g = u.garrison.unwrap();
    assert_eq!(g.phase, GarrisonPhase::Inside);
    assert_eq!(g.progress, 1.0);
    assert_eq!((g.center, g.half), (CENTRE, [HALF[0], HALF[1]]));
    assert!(u.goal.is_none() && u.queue.is_empty());
}

#[test]
fn occupants_stand_once_each_at_distinct_perimeter_slots() {
    let (b, _) = full_building(3);
    let p = building(&b);
    let standoff = num("garrison", "slot_standoff_m");
    let mut seen = BTreeSet::new();
    let mut per_facade = BTreeMap::<u8, usize>::new();
    {
        let id = 0;
        let u = b.unit(UnitId(id)).unwrap();
        let g = u.garrison.as_ref().unwrap();
        let positions: Vec<_> = u.member_positions().collect();
        assert_eq!(positions.len(), u.members.len(), "one body per soldier");
        for (k, q) in positions.iter().enumerate() {
            let slot = g.seats[k].unwrap();
            assert!(seen.insert(slot), "slot {slot} held twice");
            *per_facade.entry(g.slots[slot].slot.facade).or_default() += 1;
            // Just outside the facade, never inside the shell.
            assert!((outside_by(&p, q.xy()) - standoff).abs() < 1e-9, "{q:?}");
        }
        // The owner sees the same places.
        let o = own(&b, Side::Blue, id).unwrap();
        for (a, q) in o.members.iter().zip(&positions) {
            assert_eq!(*a, [q.x, q.y, q.z]);
        }
    }
    assert_eq!(seen.len(), 8);
    // Capacity is reserved and filled evenly around the facades.
    assert!(per_facade.values().all(|&n| n == 2), "{per_facade:?}");
}

/// Rounds each unit has in the air, remembered across ticks.
fn remember_rounds(b: &Battle, owners: &mut BTreeMap<ProjectileId, (u32, String)>) {
    for (p, r) in b.rounds() {
        owners
            .entry(p.id)
            .or_insert_with(|| (r.unit.0, b.arsenal().weapons[r.weapon].id.clone()));
    }
}

/// The direction each unit-0 round was launched toward, by projectile.
fn remember_heading(b: &Battle, headings: &mut BTreeMap<ProjectileId, f64>, deg: f64) {
    for (p, r) in b.rounds() {
        if r.unit == UnitId(0) {
            headings.entry(p.id).or_insert(deg);
        }
    }
}

#[test]
fn outgoing_fire_clears_its_own_walls_toward_every_facade_and_corner() {
    let mut b = battle(west_squads(&["rifle"]), 4);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    let mut headings = BTreeMap::new();
    for deg in [0, 45, 90, 135, 180, 225, 270, 315] {
        let a = (deg as f64).to_radians();
        let target = [CENTRE[0] + 120.0 * a.cos(), CENTRE[1] + 120.0 * a.sin()];
        c.ok(&mut b, Side::Blue, ground(0, target));
        let mut fired = 0;
        for _ in 0..150 {
            b.step();
            let before = headings.len();
            remember_heading(&b, &mut headings, deg as f64);
            fired += headings.len() - before;
            for e in b.flight_events() {
                let FlightEvent::Impact(i) = e else { continue };
                let Some(&toward) = headings.get(&i.projectile) else {
                    continue;
                };
                assert_ne!(
                    i.struck,
                    Struck::Prop(BUILDING),
                    "own wall hit toward {toward}°"
                );
                let t = toward.to_radians();
                let dir = (i.point.xy() - v2(CENTRE[0], CENTRE[1])).normalized();
                assert!(
                    dir.dot(v2(t.cos(), t.sin())) > 0.8,
                    "a round meant for {toward}° went astray"
                );
            }
        }
        assert!(fired >= 8, "{fired} rounds toward {deg}°");
    }
}

/// Rounds launched this tick by `unit`'s mount `weapon`.
fn volley(
    b: &Battle,
    owners: &mut BTreeMap<ProjectileId, (u32, String)>,
    unit: u32,
    weapon: &str,
) -> usize {
    let before: BTreeSet<_> = owners.keys().copied().collect();
    remember_rounds(b, owners);
    owners
        .iter()
        .filter(|(id, o)| !before.contains(id) && o.0 == unit && o.1 == weapon)
        .count()
}

#[test]
fn only_soldiers_at_facing_windows_fire_and_free_facing_slots_fill() {
    // Every window held, firing north: the two at the north windows fire
    // their rifles, the rest wait. (Who trades into a facing window is
    // `an_atgm_gunner_trades_windows_with_a_rifleman_to_face_armour`'s.)
    let (mut b, mut c) = full_building(5);
    let north = [CENTRE[0], CENTRE[1] + 150.0];
    c.ok(&mut b, Side::Blue, ground(0, north));
    let mut owners = BTreeMap::new();
    let mut largest = 0;
    for _ in 0..240 {
        b.step();
        largest = largest.max(volley(&b, &mut owners, 0, "rifle"));
    }
    assert_eq!(largest, 2, "only the two north windows fire");
    // In a building with room, the free north slots fill from the squad.
    let mut b = battle(west_squads(&["rifle"]), 5);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    c.ok(&mut b, Side::Blue, ground(0, north));
    let mut owners = BTreeMap::new();
    let mut largest = 0;
    for _ in 0..240 {
        b.step();
        largest = largest.max(volley(&b, &mut owners, 0, "rifle"));
    }
    assert_eq!(largest, 4, "all four north slots fire");
    assert!(
        owners.values().any(|o| o.0 == 0 && o.1 == "grenade"),
        "the grenadier moved north"
    );
}

/// Blue's squad garrisoned; red's tank to the east shells and machine-guns
/// the occupants it sees (its own eyes reach a garrison at 70 m).
fn under_fire(extra_props: Value, events: Value, seed: u64) -> (Battle, Commander) {
    let mut units = west_squads(&["rifle"]).as_array().unwrap().clone();
    units.push(json!({ "side": "red", "kind": "tank", "position": [CENTRE[0] + HALF[0] + 45.0, CENTRE[1] + 4.0], "yaw": std::f64::consts::PI }));
    let mut b = battle_with(extra_props, Value::Array(units), events, seed);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    (b, c)
}

/// Blue's squad garrisoned and holding fire; a red spotter squad north sees
/// the north facade, and red's tank shells what it spots from 300 m, where
/// its spread puts most rounds into the walls.
fn shelled(extra_props: Value, events: Value, seed: u64) -> (Battle, Commander) {
    let mut units: Vec<Value> = west_squads(&["rifle"])
        .as_array()
        .unwrap()
        .iter()
        .map(|u| {
            let mut u = u.clone();
            u["engagement"] = json!("return_fire_only");
            u
        })
        .collect();
    units.push(json!({ "side": "red", "kind": "tank", "position": [620.0, 520.0] }));
    units.push(json!({ "side": "red", "kind": "rifle", "position": [CENTRE[0], CENTRE[1] + 100.0], "engagement": "return_fire_only" }));
    let mut b = battle_with(extra_props, Value::Array(units), events, seed);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    (b, c)
}

#[test]
fn enemy_rounds_hit_occupants_or_the_shell_and_only_structural_weapons_wear_it() {
    let (mut b, _) = under_fire(json!([]), json!([]), 6);
    let tank = b.unit(UnitId(1)).unwrap().position;
    let p = building(&b);
    let hp0 = b.structures().hp(b.world(), BUILDING).unwrap();
    assert_eq!(Some(hp0), common::props().by_id("building").body.hp);
    let mut owners = BTreeMap::new();
    let (mut direct, mut shell_hmg, mut shell_he) = (0, 0, 0);
    let mut hp = hp0;
    for _ in 0..900 {
        b.step();
        remember_rounds(&b, &mut owners);
        let mut structural = 0.0;
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let Some((owner, weapon)) = owners.get(&i.projectile).cloned() else {
                continue;
            };
            // A burst off the building wears it by its distance (Q17), and a
            // direct hit by the row's structural damage (the HMG's and the
            // rifles' too), whoever fired.
            let w = &rules()["weapons"][&weapon];
            let (sd, radius) = (
                w["structural_damage"].as_f64().unwrap_or(0.0),
                w["blast_radius_m"].as_f64().unwrap_or(0.0),
            );
            let r = p.footprint().distance((i.point + i.normal * 0.05).xy());
            if i.detonated && !matches!(i.struck, Struck::Prop(BUILDING)) && r < radius {
                structural += sd * (1.0 - r / radius);
            }
            if i.struck == Struck::Prop(BUILDING) {
                structural += sd;
            }
            if owner != 1 {
                continue;
            }
            match i.struck {
                Struck::Body(_) => direct += 1,
                Struck::Prop(BUILDING) if weapon == "hmg" => shell_hmg += 1,
                Struck::Prop(BUILDING) => shell_he += 1,
                _ => {
                    // A round that missed everything never went through the shell.
                    let muzzle = tank + v3(0.0, 0.0, 2.0);
                    let d = i.point - muzzle;
                    let len = d.length();
                    assert!(
                        p.raycast(muzzle, d * (1.0 / len), len - 0.05).is_none(),
                        "a round passed through the building to {:?}",
                        i.point
                    );
                }
            }
        }
        let now = b.structures().hp(b.world(), BUILDING).unwrap_or(0.0);
        // The last hit may take more than is left (the building falls).
        assert!(
            (hp - now - structural.min(hp)).abs() < 1e-9,
            "only structural weapons wear it, by direct hits and nearby bursts"
        );
        hp = now;
        if b.world().prop(BUILDING).is_none() {
            break;
        }
    }
    assert!(direct > 0, "occupants were hit directly");
    assert!(shell_hmg > 0, "machine-gun misses stopped in the wall");
    assert!(
        shell_he > 0 && hp < hp0,
        "HE on the wall wore the building down"
    );
}

#[test]
fn every_round_meets_the_same_capsules_and_shell_whatever_it_was_aimed_at() {
    // A world with the building, an occupant capsule at an east slot and a
    // soldier beyond the building to the west: rounds from the east.
    let world = common::flat(
        [800.0, 600.0],
        &format!(
            r#","props":[{{"kind":"building","center":{CENTRE:?},"yaw":0,"half_extents":{HALF:?}}}]"#
        ),
    );
    let prop = world.prop(BUILDING).unwrap().clone();
    let r: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let slot = sim::garrison::slots(&world, &prop, &r)[1]; // east facade
    let beyond = v3(CENTRE[0] - 60.0, slot.position.y, 0.0);
    let body =
        |id: u32, unit: u32, at| common::Mover::standing(id, unit, common::soldier_shape(), at);
    let bodies = [body(1, 1, slot.position), body(2, 2, beyond)];
    let fire = |aim: sim::math::V3| {
        let mut store = sim::flight::Projectiles::new(common::config());
        let origin = v3(CENTRE[0] + 80.0, slot.position.y, 1.0);
        let v = (aim - origin).normalized() * common::profile("rifle").speed_mps;
        store.launch(sim::flight::Launch {
            origin,
            velocity: v,
            gravity_scale: common::profile("rifle").gravity_scale,
            lifetime_s: common::profile("rifle").lifetime_s,
            suppression_radius_m: 0.0,
            shooter: None,
            guidance: None,
            motor: None,
        });
        let dt = 1.0 / common::tick_hz() as f64;
        let events = common::fly(&mut store, &world, 40, |k| {
            bodies.iter().map(|m| m.body(k, dt)).collect()
        });
        common::impacts(&events)[0].1.struck
    };
    // Aimed at the occupant: it is hit where it stands at the facade.
    assert_eq!(
        fire(slot.position + v3(0.0, 0.0, 1.0)),
        Struck::Body(sim::flight::BodyId(1))
    );
    // Just wide of it: the wall behind stops the round.
    assert_eq!(
        fire(slot.position + v3(0.0, 0.8, 1.0)),
        Struck::Prop(BUILDING)
    );
    // Aimed at the soldier beyond: the shell stops it (it would have missed
    // the occupant only by being aimed past it).
    assert_eq!(fire(beyond + v3(0.0, 2.0, 1.0)), Struck::Prop(BUILDING));
}

#[test]
fn a_garrison_is_sheltered_by_its_building_and_only_a_garrison() {
    let r: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let mut b = battle(west_squads(&["rifle"]), 1);
    let shelter = |b: &Battle| sim::garrison::shelter(b.unit(UnitId(0)).unwrap(), b.rules());
    assert_eq!(shelter(&b), 0.0, "an outside squad has no building shelter");
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 600, "inside", |b| inside(b, 0));
    let strength = shelter(&b);
    assert_eq!(strength, num("buildings", "cover_strength"));
    let (bs, bf) = (
        num("cover", "building_spread_multiplier"),
        num("cover", "building_fragment_probability_multiplier"),
    );
    assert_eq!(sim::damage::shelter_spread(&r, 0.0), 1.0);
    assert_eq!(sim::damage::fragment_exposure(&r, 0.0), 1.0);
    assert_eq!(
        sim::damage::shelter_spread(&r, strength),
        1.0 + (bs - 1.0) * strength
    );
    assert_eq!(
        sim::damage::fragment_exposure(&r, strength),
        1.0 + (bf - 1.0) * strength
    );
    c.ok(&mut b, Side::Blue, exit(&[0]));
    until(&mut b, 600, "outside", |b| {
        !b.unit(UnitId(0)).unwrap().garrisoned()
    });
    assert_eq!(shelter(&b), 0.0, "leaving loses building shelter");
}

/// A squad's members (alive, where) and its suppression, a tick before.
type Occupants = (u32, Vec<(bool, sim::math::V3)>, f64);

/// Run `shelled` until the building collapses; the tick before and after.
struct Collapse {
    b: Battle,
    before: Vec<Occupants>,
    corpses_before: usize,
}

fn collapse(extra_props: Value, events: Value, seed: u64) -> Collapse {
    try_collapse(extra_props, events, seed).expect("the building never collapsed")
}

/// Like [`collapse`], but `None` when the shelling never brings it down
/// (the tank may run out of targets first).
fn try_collapse(extra_props: Value, events: Value, seed: u64) -> Option<Collapse> {
    let (mut b, _) = shelled(extra_props, events, seed);
    let snapshot = |b: &Battle| -> Vec<Occupants> {
        [0].iter()
            .map(|&id| {
                let u = b.unit(UnitId(id)).unwrap();
                let members = (0..u.members.len())
                    .map(|k| (u.members[k].alive(), u.members[k].position))
                    .collect();
                (id, members, u.suppression)
            })
            .collect()
    };
    let corpses = |b: &Battle| {
        b.observe(Side::Blue)
            .corpses
            .iter()
            .filter(|c| c.own)
            .count()
    };
    for _ in 0..9000 {
        let before = snapshot(&b);
        let corpses_before = corpses(&b);
        b.step();
        if b.world().prop(BUILDING).is_none() {
            return Some(Collapse {
                b,
                before,
                corpses_before,
            });
        }
    }
    None
}

/// The first collapse, from seed 1 on, that some occupant survives: which
/// seed that is depends on every random draw before it, so it is searched for,
/// not pinned.
fn collapse_with_survivors() -> Collapse {
    (1..=20)
        .map(|seed| collapse(json!([]), json!([]), seed))
        .find(|c| living(&c.b, 0) > 0)
        .expect("some seed leaves survivors")
}

#[test]
fn a_collapse_leaves_a_lower_ruin_and_accounts_for_every_occupant() {
    let Collapse {
        mut b,
        before,
        corpses_before,
    } = collapse_with_survivors();
    // The building is gone for good; a lower ruin stands on its footprint.
    assert!(b.world().prop(BUILDING).is_none());
    let ruin = b
        .world()
        .props()
        .find(|p| p.kind == common::kind("ruin"))
        .cloned()
        .expect("a ruin");
    assert_eq!(b.structures().replaced_by(ruin.id), Some(BUILDING));
    assert_eq!(ruin.center, v2(CENTRE[0], CENTRE[1]));
    assert_eq!([ruin.half.x, ruin.half.y], [HALF[0], HALF[1]]);
    let Some(contract::catalog::Destroyed::Into { height_m, .. }) =
        &common::props().by_id("building").destroyed
    else {
        panic!("a building leaves remains");
    };
    assert_eq!(2.0 * ruin.half.z, *height_m);
    // Every occupant alive a tick earlier is now a survivor or a corpse.
    let corpses_now = b
        .observe(Side::Blue)
        .corpses
        .iter()
        .filter(|c| c.own)
        .count();
    let alive_before: usize = before
        .iter()
        .map(|(_, m, _)| m.iter().filter(|x| x.0).count())
        .sum();
    let alive_now = living(&b, 0);
    assert_eq!(alive_before, alive_now + corpses_now - corpses_before);
    let radius = num("garrison", "exit_search_radius_m");
    let survivors = living(&b, 0);
    for (id, members, _) in &before {
        let u = b.unit(UnitId(*id)).unwrap();
        assert!(u.garrison.is_none());
        if !u.alive() {
            continue;
        }
        // Survivors come out pinned, as their side sees them.
        let seen = b
            .observe(Side::Blue)
            .own
            .iter()
            .find(|o| o.id == u.id)
            .unwrap();
        assert_eq!(
            seen.suppression,
            contract::observation::SuppressionTier::Pinned,
            "{}",
            u.suppression
        );
        for (k, (_, was)) in members.iter().enumerate() {
            if !u.members[k].alive() {
                continue;
            }
            let q = u.members[k].position.xy();
            // On legal ground outside the ruin, within the local search of
            // where it stood: no teleport.
            assert!(
                outside_by(&ruin, q) > 0.0,
                "survivor inside the ruin at {q:?}"
            );
            let s = b.world().surface_at(q.x, q.y).unwrap();
            assert!(s.traversable);
            assert!((q - was.xy()).length() <= radius + 1e-9);
        }
    }
    assert!(survivors > 0);
    // The ruin is permanent.
    run(&mut b, 300);
    assert!(b
        .world()
        .props()
        .any(|p| p.id == ruin.id && p.kind == common::kind("ruin")));
}

#[test]
fn a_survivor_with_no_legal_way_out_dies_rather_than_teleporting() {
    // Low ruins (walls fire can't destroy, Q17), knee-high so the tank still
    // fires over them, fill the ground around the building beyond the local
    // search: nowhere to escape to.
    let band = num("garrison", "exit_search_radius_m") + 5.0;
    let (cx, cy, hx, hy) = (CENTRE[0], CENTRE[1], HALF[0], HALF[1]);
    let wall = |x: f64, y: f64, w: f64, h: f64| json!({ "tick": 900, "add_prop": { "kind": "ruin", "center": [x, y], "yaw": 0, "half_extents": [w, h, 0.25] } });
    let events = json!([
        wall(cx + hx + band / 2.0, cy, band / 2.0, hy + band),
        wall(cx - hx - band / 2.0, cy, band / 2.0, hy + band),
        wall(cx, cy + hy + band / 2.0, hx, band / 2.0),
        wall(cx, cy - hy - band / 2.0, hx, band / 2.0),
    ]);
    // Which seed brings the building down with someone inside depends on
    // every draw before it, so it is searched for, not pinned.
    let Collapse { b, before, .. } = (1..=20)
        .filter_map(|seed| try_collapse(json!([]), events.clone(), seed))
        .find(|c| {
            c.before
                .iter()
                .any(|(_, m, _)| m.iter().any(|(alive, _)| *alive))
        })
        .expect("some seed collapses the building on its occupants");
    for (id, members, _) in &before {
        let u = b.unit(UnitId(*id)).unwrap();
        assert!(!u.alive(), "squad {id} had nowhere to go");
        // Everyone alive before lies where they stood.
        for (k, (alive, at)) in members.iter().enumerate() {
            if *alive {
                assert_eq!(u.members[k].corpse.map(|f| f.at), Some(*at));
            }
        }
    }
}

#[test]
fn a_survivor_squeezes_out_where_a_soldier_fits() {
    // Low ruins ring the building 0.8 m off its walls: room for a soldier's
    // body (a 0.6 m disc), though not for a squad's path clearance. A
    // survivor escapes into that gap and stands there.
    let gap = 0.8;
    let band = num("garrison", "exit_search_radius_m") + 5.0;
    let (cx, cy, hx, hy) = (CENTRE[0], CENTRE[1], HALF[0], HALF[1]);
    let wall = |x: f64, y: f64, w: f64, h: f64| json!({ "tick": 900, "add_prop": { "kind": "ruin", "center": [x, y], "yaw": 0, "half_extents": [w, h, 0.25] } });
    let (ox, oy) = (hx + gap + band / 2.0, hy + gap + band / 2.0);
    let events = json!([
        wall(cx + ox, cy, band / 2.0, hy + gap + band),
        wall(cx - ox, cy, band / 2.0, hy + gap + band),
        wall(cx, cy + oy, hx + gap, band / 2.0),
        wall(cx, cy - oy, hx + gap, band / 2.0),
    ]);
    // Each survives by a draw: over the first collapses with occupants
    // inside, some survive (each a coin toss, so all dying is 2^-n).
    let collapses: Vec<Collapse> = (1..=20)
        .filter_map(|seed| try_collapse(json!([]), events.clone(), seed))
        .filter(|c| {
            let inside: usize = c
                .before
                .iter()
                .map(|(_, m, _)| m.iter().filter(|x| x.0).count())
                .sum();
            inside >= 4
        })
        .take(2)
        .collect();
    assert!(
        !collapses.is_empty(),
        "some seed collapses it on its occupants"
    );
    let mut survivors = 0;
    for c in &collapses {
        if let Some(u) = c.b.unit(UnitId(0)) {
            for s in u.members.iter().filter(|s| s.alive()) {
                let q = s.position.xy();
                let (dx, dy) = ((q.x - cx).abs() - hx, (q.y - cy).abs() - hy);
                assert!(dx.max(dy) <= gap, "in the gap: {q:?}");
                survivors += 1;
            }
        }
    }
    assert!(survivors > 0, "someone squeezed out");
}

#[test]
fn the_ruin_blocks_ground_movement_while_sight_and_fire_pass_over_it() {
    let Collapse { mut b, .. } = collapse_with_survivors();
    let ruin = b
        .world()
        .props()
        .find(|p| p.kind == common::kind("ruin"))
        .cloned()
        .unwrap();
    assert!(MoverClass::ALL.iter().all(|&c| ruin.blocks(c)));
    // It closes the authored building's footprint as the building did, so
    // every side plans with it, whether or not it saw the collapse.
    assert!(ruin.known_to_all);
    let top = ruin.top_z();
    // A low line meets the ruin; a line above its top passes over.
    let (a, z) = (v2(CENTRE[0] - 40.0, CENTRE[1]), ruin.base_z);
    let far = v2(CENTRE[0] + 40.0, CENTRE[1]);
    assert!(z < top);
    assert!(!b
        .world()
        .segment_clear(a.with_z(top - 1.0), far.with_z(top - 1.0)));
    assert!(b
        .world()
        .segment_clear(a.with_z(top + 1.0), far.with_z(top + 1.0)));
    // A survivor squad sent straight across walks round the ruin.
    let squad = 0;
    let from = b.unit(UnitId(squad)).unwrap().position.xy();
    let goal = v2(2.0 * CENTRE[0] - from.x, 2.0 * CENTRE[1] - from.y);
    common::order(
        &mut b,
        Side::Blue,
        2,
        Order::Move {
            units: vec![UnitId(squad)],
            gesture: 1,
            goal: [goal.x, goal.y],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    for _ in 0..600 {
        b.step();
        let Some(u) = b.unit(UnitId(squad)).filter(|u| u.alive()) else {
            break;
        };
        // No soldier's body enters it (the squad's middle may lie over it
        // while survivors walk round both sides).
        let radius = num("physics", "soldier_radius_m");
        for p in u.member_positions() {
            assert!(
                outside_by(&ruin, p.xy()) >= radius - 1e-6,
                "walked into the ruin"
            );
        }
    }
}

#[test]
fn occupants_see_only_from_occupied_slots_and_are_seen_only_there() {
    // An AT squad of three walks in from the north, the building hiding the
    // south; red squads stand in the open south and north. It turns to the
    // north squad it knows of, and nobody holds the south facade.
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "at", "position": [400, 335] },
            { "side": "red", "kind": "rifle", "position": [CENTRE[0], CENTRE[1] - HALF[1] - 30.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [CENTRE[0], CENTRE[1] + HALF[1] + 60.0], "engagement": "return_fire_only" },
        ]),
        9,
    );
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::SetEngagement {
            units: vec![UnitId(0)],
            policy: Engagement::ReturnFireOnly,
        },
    );
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    run(&mut b, 3);
    let u = b.unit(UnitId(0)).unwrap();
    let g = u.garrison.as_ref().unwrap();
    let facades: BTreeSet<u8> = g
        .seats
        .iter()
        .flatten()
        .map(|&s| g.slots[s].slot.facade)
        .collect();
    assert!(
        !facades.contains(&3),
        "no one on the south facade: {facades:?}"
    );
    let identified = &b.observe(Side::Blue).identified;
    assert_eq!(
        identified.len(),
        1,
        "only the north squad is seen: {identified:?}"
    );
    assert!(identified[0].position[1] > CENTRE[1]);
    // The south squad cannot see the occupants past the building either; the
    // north squad sees them only at their slots (within the garrison's
    // concealment reach).
    let red = &b.observe(Side::Red).identified;
    assert!(red.len() <= 1);
    for e in red {
        for m in &e.members {
            assert!(
                m[1] > CENTRE[1] - HALF[1],
                "seen from the north only: {m:?}"
            );
        }
    }
}

#[test]
fn another_building_still_blocks_a_garrison_normally() {
    // A second building east blocks the squad's line to ground beyond it.
    let mut b = battle_with(
        json!([{ "kind": "building", "center": [CENTRE[0] + 60.0, CENTRE[1]], "yaw": 0, "half_extents": [10, 10, 4] }]),
        west_squads(&["rifle"]),
        json!([]),
        10,
    );
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    c.ok(
        &mut b,
        Side::Blue,
        ground(0, [CENTRE[0] + 110.0, CENTRE[1]]),
    );
    run(&mut b, 60);
    let u = own(&b, Side::Blue, 0).unwrap();
    assert_eq!(u.mounts[0].reason, ActionReason::BlockedTrajectory);
    assert!(!b.rounds().any(|(_, r)| r.unit == UnitId(0)));
}

#[test]
fn leaving_takes_the_timer_and_steps_out_to_free_ground() {
    let mut b = battle(west_squads(&["rifle"]), 11);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    let timer = ticks(num("garrison", "enter_exit_s"));
    // Stop while leaving keeps the squad inside.
    c.ok(&mut b, Side::Blue, exit(&[0]));
    run(&mut b, timer / 2);
    assert_eq!(phase(&b, Side::Blue, 0), Some(GarrisonPhase::Exiting));
    c.ok(
        &mut b,
        Side::Blue,
        Order::Stop {
            units: vec![UnitId(0)],
        },
    );
    run(&mut b, 2);
    assert_eq!(phase(&b, Side::Blue, 0), Some(GarrisonPhase::Inside));
    // A move order leaves first, after the whole timer, then walks on.
    let goal = [200.0, 300.0];
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 7,
            goal,
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    for _ in 0..timer {
        b.step();
        assert!(inside(&b, 0));
    }
    b.step();
    let u = b.unit(UnitId(0)).unwrap();
    assert!(u.garrison.is_none());
    let p = building(&b);
    // Outside on the side it is heading to, every soldier clear of the shell.
    assert!(u.position.x < CENTRE[0] - HALF[0]);
    for q in u.member_positions() {
        assert!(outside_by(&p, q.xy()) > 0.5, "{q:?}");
    }
    until(&mut b, 3000, "arrived", |b| {
        own(b, Side::Blue, 0).unwrap().goal.is_none()
    });
}

#[test]
fn a_queued_garrison_follows_the_move_before_it() {
    let mut b = battle(west_squads(&["rifle"]), 12);
    let mut c = Commander::new();
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(0)],
            gesture: 1,
            goal: [380.0, 240.0],
            route: RoutePolicy::Shortest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        },
    );
    assert_eq!(c.send(&mut b, Side::Blue, garrison(&[0]), true), None);
    let mut visited = false;
    until(&mut b, 1500, "inside", |b| {
        visited |= (b.unit(UnitId(0)).unwrap().position.xy() - v2(380.0, 240.0)).length() < 2.0;
        if inside(b, 0) {
            assert!(visited, "the preceding move reached its waypoint");
        }
        inside(b, 0)
    });
    assert!(b.unit(UnitId(0)).unwrap().orders.is_empty());
}

#[test]
fn garrison_orders_replay_every_tick_exactly() {
    let units = west_squads(&["rifle", "rifle"]);
    let setup = common::scenario_with(&map(json!([])), units, json!([]), json!([]));
    let mut b = Battle::new(&setup, 13);
    let mut c = Commander::new();
    let mut digests = Vec::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    for t in 0..1800 {
        if t == 600 {
            c.ok(
                &mut b,
                Side::Blue,
                ground(0, [CENTRE[0], CENTRE[1] + 150.0]),
            );
        }
        if t == 700 {
            c.ok(&mut b, Side::Blue, exit(&[0]));
        }
        // The building is free once squad 0 is out: squad 1 takes it.
        if t == 900 {
            c.ok(&mut b, Side::Blue, garrison(&[1]));
        }
        b.step();
        digests.push(b.digest());
    }
    assert!(!inside(&b, 0) && inside(&b, 1));
    let mut r = Battle::from_replay(&setup, &b.replay()).unwrap();
    for d in digests {
        r.step();
        assert_eq!(r.digest(), d, "tick {}", r.tick());
    }
}

/// The facade (0 east, 1 north, 2 west, 3 south) member `k` of squad `id`
/// holds a window on, while inside.
fn facade_of(b: &Battle, id: u32, k: usize) -> Option<u8> {
    let g = b.unit(UnitId(id))?.garrison.clone()?;
    g.seats[k].map(|s| g.slots[s].slot.facade)
}

/// Ticks from the fixture's `garrison` section.
fn garrison_ticks(key: &str) -> u64 {
    ticks(num("garrison", key))
}

#[test]
fn an_atgm_gunner_trades_windows_with_a_rifleman_to_face_armour() {
    // A house with one window per facade. The AT team is seated gunner east,
    // riflemen north and west (each soldier on the next facade in turn); a
    // tank it saw on the way waits 250 m north, holding fire.
    let mut b = battle_with_capacity(
        json!([
            { "side": "blue", "kind": "at", "position": [350.0, 300.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [CENTRE[0], CENTRE[1] + 250.0], "engagement": "return_fire_only" },
        ]),
        1,
        4,
    );
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    // On the tick it is seated, the gunner trades places with the rifleman
    // at the north window.
    let (gunner, rifleman) = (0, 1);
    assert_eq!(
        facade_of(&b, 0, gunner),
        Some(1),
        "the gunner at the north window"
    );
    assert_eq!(
        facade_of(&b, 0, rifleman),
        Some(0),
        "the rifleman at the gunner's"
    );
    let g = b.unit(UnitId(0)).unwrap().garrison.clone().unwrap();
    let moved = g.slots[g.seats[gunner].unwrap()]
        .changed
        .expect("he changed windows");
    assert_eq!(moved, b.tick());
    // Neither may fire while they change windows; then the gunner launches.
    let r: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let swap = garrison_ticks("window_swap_s");
    let changing = |b: &Battle, k: usize, tick: u64| {
        sim::garrison::changing_window(b.unit(UnitId(0)).unwrap(), k, &r, tick)
    };
    for k in [gunner, rifleman] {
        assert!(changing(&b, k, moved + swap - 1) && !changing(&b, k, moved + swap));
    }
    assert!(!changing(&b, 2, moved), "the other rifleman stayed put");
    c.ok(
        &mut b,
        Side::Blue,
        Order::SetEngagement {
            units: vec![UnitId(0)],
            policy: Engagement::FireAtWill,
        },
    );
    let mut owners = BTreeMap::new();
    until(&mut b, 600, "the gunner launched at the tank", |b| {
        remember_rounds(b, &mut owners);
        owners.values().any(|o| o.0 == 0 && o.1 == "atgm")
    });
}

#[test]
fn a_squad_faces_armour_with_its_launcher_and_infantry_with_its_rifles() {
    // Two windows a facade; a tank north and a rifle squad south, everyone
    // holding fire: the gunner watches the tank, the riflemen the infantry.
    let mut b = battle_with_capacity(
        json!([
            { "side": "blue", "kind": "at", "position": [350.0, 300.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "tank", "position": [CENTRE[0], CENTRE[1] + 250.0], "engagement": "return_fire_only" },
            { "side": "red", "kind": "rifle", "position": [CENTRE[0], CENTRE[1] - 150.0], "engagement": "return_fire_only" },
        ]),
        1,
        8,
    );
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0]));
    until(&mut b, 1200, "inside", |b| inside(b, 0));
    until(&mut b, 300, "gunner north, riflemen south", |b| {
        facade_of(b, 0, 0) == Some(1) && [1, 2].iter().all(|&k| facade_of(b, 0, k) == Some(3))
    });
}

#[test]
fn alternating_threats_do_not_shuffle_the_squad_faster_than_its_hold() {
    // A full house (every window held); the squad is ordered to fire north
    // and south in turn, far faster than a soldier may change windows again.
    let (mut b, mut c) = full_building(7);
    let hold = garrison_ticks("window_hold_s");
    let span = 900;
    let mut last: Vec<Option<usize>> = b.unit(UnitId(0)).unwrap().garrison.clone().unwrap().seats;
    let mut changes = vec![0u64; last.len()];
    for t in 0..span {
        if t % 20 == 0 {
            let y = if (t / 20) % 2 == 0 { 150.0 } else { -150.0 };
            c.ok(&mut b, Side::Blue, ground(0, [CENTRE[0], CENTRE[1] + y]));
        }
        b.step();
        let seats = b.unit(UnitId(0)).unwrap().garrison.clone().unwrap().seats;
        for (k, (now, was)) in seats.iter().zip(&last).enumerate() {
            if now != was {
                changes[k] += 1;
            }
        }
        last = seats;
    }
    assert!(changes.iter().any(|&n| n > 0), "someone changed windows");
    for (k, &n) in changes.iter().enumerate() {
        assert!(
            n <= span / hold + 1,
            "soldier {k} changed windows {n} times in {span} ticks"
        );
    }
}

#[test]
fn an_unseen_collapse_cannot_change_a_garrison_order_until_discovered() {
    let make = |destroy| {
        let mut map: Value = serde_json::from_str(&map(json!([]))).unwrap();
        map["size"] = json!([1600, 600]);
        let events: Vec<_> = (1..=20)
            .filter(|_| destroy)
            .map(|tick| json!({ "tick": tick, "burst": { "point": CENTRE, "weapon": "tank_he" } }))
            .collect();
        let setup = common::scenario_with(
            &map.to_string(),
            json!([
                { "side": "blue", "kind": "rifle", "position": [1500, 40], "engagement": "return_fire_only" }
            ]),
            json!(events),
            json!([]),
        );
        let mut b = Battle::new(&setup, 1);
        for _ in 0..30 {
            b.step();
        }
        b
    };
    let (mut hit, mut calm) = (make(true), make(false));
    assert!(hit.world().prop(BUILDING).is_none());
    assert!(calm.world().prop(BUILDING).is_some());
    assert_eq!(hit.observe(Side::Blue), calm.observe(Side::Blue));
    let command = CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        queued: false,
        order: Order::Garrison {
            units: vec![UnitId(0)],
            building: BUILDING,
        },
    };
    assert_eq!(calm.accept(command.clone()).error, None);
    assert_eq!(hit.accept(command).error, None);
    for _ in 0..60 {
        hit.step();
        calm.step();
        assert_eq!(
            hit.observe(Side::Blue).own,
            calm.observe(Side::Blue).own,
            "hidden destruction cannot cancel or redirect the approach"
        );
    }
    for _ in 0..ticks(600.0) {
        hit.step();
        if hit
            .observe(Side::Blue)
            .known_props
            .iter()
            .any(|p| p.replaces == Some(BUILDING))
        {
            break;
        }
    }
    assert!(
        !hit.observe(Side::Blue).known_props.is_empty(),
        "the squad discovers the ruin"
    );
    hit.step();
    assert!(
        hit.unit(UnitId(0)).unwrap().orders.is_empty(),
        "discovery ends the approach"
    );
    assert!(hit.unit(UnitId(0)).unwrap().garrison.is_none());
}
