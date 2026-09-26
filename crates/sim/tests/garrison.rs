//! Buildings as abstract fighting positions (slice 11), driven through real
//! scenarios and commands.
use std::collections::{BTreeMap, BTreeSet};

use contract::command::{CommandEnvelope, Engagement, Order, OrderError, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::{MoverClass, PropKind};
use contract::observation::{ActionReason, GarrisonPhase, OwnUnit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, ProjectileId, Struck};
use sim::math::{v2, v3, V2};
use sim::world::Prop;

mod common;

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

struct Commander {
    seq: [u64; 2],
}

impl Commander {
    fn new() -> Self {
        Commander { seq: [0, 0] }
    }

    fn send(
        &mut self,
        b: &mut Battle,
        side: Side,
        order: Order,
        queued: bool,
    ) -> Option<OrderError> {
        self.seq[side.index()] += 1;
        b.accept(CommandEnvelope {
            side,
            seq: self.seq[side.index()],
            order,
            queued,
        })
        .error
    }

    fn ok(&mut self, b: &mut Battle, side: Side, order: Order) {
        let e = self.send(b, side, order, false);
        assert_eq!(e, None);
    }
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

/// Both rifle squads garrisoned: 16 soldiers, a full building.
fn full_building(extra_units: &[Value], seed: u64) -> (Battle, Commander) {
    let mut units = west_squads(&["rifle", "rifle"]).as_array().unwrap().clone();
    units.extend(extra_units.iter().cloned());
    let mut b = battle(Value::Array(units), seed);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0, 1]));
    until(&mut b, 1200, "both squads inside", |b| {
        inside(b, 0) && inside(b, 1)
    });
    (b, c)
}

#[test]
fn whole_squads_enter_by_soldier_capacity_and_never_split() {
    let mut b = battle(west_squads(&["rifle", "rifle", "recon"]), 1);
    let mut c = Commander::new();
    // 8 + 8 + 4 soldiers do not fit 16: the whole order is refused, nobody moves.
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[0, 1, 2]), false),
        Some(OrderError::CapacityFull)
    );
    run(&mut b, 5);
    assert!((0..3).all(|u| own(&b, Side::Blue, u).unwrap().goal.is_none()));
    // Two squads share it exactly; the third is refused while they hold it.
    c.ok(&mut b, Side::Blue, garrison(&[0, 1]));
    b.step();
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[2]), false),
        Some(OrderError::CapacityFull),
        "squads on their way in count"
    );
    until(&mut b, 1200, "both inside", |b| {
        inside(b, 0) && inside(b, 1)
    });
    assert_eq!(
        c.send(&mut b, Side::Blue, garrison(&[2]), false),
        Some(OrderError::CapacityFull)
    );
    assert_eq!(phase(&b, Side::Blue, 2), None);
    // Every soldier of both squads holds a slot; none is split off outside.
    for id in [0, 1] {
        let u = b.unit(UnitId(id)).unwrap();
        let g = u.garrison.as_ref().unwrap();
        assert!(g.seats.iter().all(Option::is_some), "squad {id} whole");
    }
    // Capacity counts soldiers: once one squad leaves, the small squad fits.
    c.ok(&mut b, Side::Blue, exit(&[1]));
    until(&mut b, 200, "squad 1 outside", |b| {
        !inside(b, 1) && b.unit(UnitId(1)).unwrap().garrison.is_none()
    });
    c.ok(&mut b, Side::Blue, garrison(&[2]));
    until(&mut b, 1200, "recon inside", |b| inside(b, 2));
    // Orders in the same tick cannot both be counted at the command: whole
    // squads still enter only while they fit, and the late one waits.
    let mut r = battle(west_squads(&["rifle", "rifle", "rifle"]), 1);
    let mut cr = Commander::new();
    cr.ok(&mut r, Side::Blue, garrison(&[0, 1]));
    cr.ok(&mut r, Side::Blue, garrison(&[2]));
    until(&mut r, 1500, "one squad waiting for room", |r| {
        (0..3).any(|u| phase(r, Side::Blue, u) == Some(GarrisonPhase::WaitingForRoom))
            && (0..3).filter(|&u| inside(r, u)).count() == 2
    });
    let seated: usize = (0..3)
        .filter(|&u| inside(&r, u))
        .map(|u| {
            r.unit(UnitId(u))
                .unwrap()
                .garrison
                .as_ref()
                .unwrap()
                .seats
                .iter()
                .flatten()
                .count()
        })
        .sum();
    assert_eq!(seated, 16);
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
    assert!(
        outside_by(&p, v2(at.position[0], at.position[1])) <= num("garrison", "entry_distance_m")
    );
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
    assert_eq!(u.garrison.unwrap().phase, GarrisonPhase::Inside);
    assert!(u.goal.is_none() && u.queue.is_empty());
}

#[test]
fn occupants_stand_once_each_at_distinct_perimeter_slots() {
    let (b, _) = full_building(&[], 3);
    let p = building(&b);
    let standoff = num("garrison", "slot_standoff_m");
    let mut seen = BTreeSet::new();
    let mut per_facade = BTreeMap::<u8, usize>::new();
    for id in [0, 1] {
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
    assert_eq!(seen.len(), 16);
    // Capacity is reserved and filled evenly around the facades.
    assert!(per_facade.values().all(|&n| n == 4), "{per_facade:?}");
}

/// Rounds each unit has in the air, remembered across ticks.
fn remember_rounds(b: &Battle, owners: &mut BTreeMap<ProjectileId, (u32, String)>) {
    for (p, r) in b.rounds() {
        owners
            .entry(p.id)
            .or_insert_with(|| (r.unit.0, b.arsenal().weapons[r.weapon].name.clone()));
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
fn soldiers_wait_when_every_target_facing_slot_is_taken() {
    let (mut b, mut c) = full_building(&[], 5);
    let north = [CENTRE[0], CENTRE[1] + 150.0];
    c.ok(&mut b, Side::Blue, ground(0, north));
    let mut owners = BTreeMap::new();
    let mut largest = 0;
    let mut grenade_waits = false;
    for _ in 0..240 {
        b.step();
        largest = largest.max(volley(&b, &mut owners, 0, "rifle"));
        let u = own(&b, Side::Blue, 0).unwrap();
        grenade_waits |= u.mounts[1].reason == ActionReason::NoFacingSlot;
    }
    // Four north slots, two of them the other squad's: two rifles fire.
    assert_eq!(largest, 2, "only this squad's north slots fire");
    assert!(
        grenade_waits,
        "the grenadier's facade faces away and no north slot is free"
    );
    assert!(!owners.values().any(|o| o.0 == 0 && o.1 == "grenade"));
    // The other squad leaves: its north slots free up and this squad takes them.
    c.ok(&mut b, Side::Blue, exit(&[1]));
    until(&mut b, 200, "squad 1 out", |b| {
        b.unit(UnitId(1)).unwrap().garrison.is_none()
    });
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

/// Blue's squads garrisoned; red's tank to the east shells and machine-guns
/// the occupants it sees (its own eyes reach a garrison at 70 m).
fn under_fire(extra_props: Value, events: Value, seed: u64) -> (Battle, Commander) {
    let mut units = west_squads(&["rifle", "rifle"]).as_array().unwrap().clone();
    units.push(json!({ "side": "red", "kind": "tank", "position": [CENTRE[0] + HALF[0] + 45.0, CENTRE[1] + 4.0], "yaw": std::f64::consts::PI }));
    let mut b = battle_with(extra_props, Value::Array(units), events, seed);
    let mut c = Commander::new();
    c.ok(&mut b, Side::Blue, garrison(&[0, 1]));
    until(&mut b, 1200, "both inside", |b| {
        inside(b, 0) && inside(b, 1)
    });
    (b, c)
}

/// Blue's squads garrisoned and holding fire; a red spotter squad north sees
/// the north facade, and red's tank shells what it spots from 300 m, where
/// its spread puts most rounds into the walls.
fn shelled(extra_props: Value, events: Value, seed: u64) -> (Battle, Commander) {
    let mut units: Vec<Value> = west_squads(&["rifle", "rifle"])
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
    c.ok(&mut b, Side::Blue, garrison(&[0, 1]));
    until(&mut b, 1200, "both inside", |b| {
        inside(b, 0) && inside(b, 1)
    });
    (b, c)
}

#[test]
fn enemy_rounds_hit_occupants_or_the_shell_and_only_structural_weapons_wear_it() {
    let (mut b, _) = under_fire(json!([]), json!([]), 6);
    let tank = b.unit(UnitId(2)).unwrap().position;
    let p = building(&b);
    let hp0 = b.structures().hp(BUILDING).unwrap();
    assert_eq!(hp0, num("buildings", "hp"));
    let mut owners = BTreeMap::new();
    let (mut direct, mut shell_hmg, mut shell_he) = (0, 0, 0);
    let mut hp = hp0;
    for _ in 0..900 {
        b.step();
        remember_rounds(&b, &mut owners);
        let mut structural = 0.0;
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let Some((2, weapon)) = owners.get(&i.projectile).cloned() else {
                continue;
            };
            match i.struck {
                Struck::Body(_) => direct += 1,
                Struck::Prop(BUILDING) if weapon == "hmg" => shell_hmg += 1,
                Struck::Prop(BUILDING) => {
                    shell_he += 1;
                    structural += rules()["weapons"][&weapon]["structural_damage"]
                        .as_f64()
                        .unwrap();
                }
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
        let now = b.structures().hp(BUILDING).unwrap_or(0.0);
        assert!(
            (hp - now - structural).abs() < 1e-9,
            "only structural weapons wear it"
        );
        hp = now;
        if !b.structures().standing(BUILDING) {
            break;
        }
    }
    assert!(direct > 0, "occupants were hit directly");
    assert!(
        shell_hmg > 0,
        "machine-gun misses stopped in the wall without wearing it"
    );
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
            lifetime_s: common::profile("rifle").lifetime_s,
            suppression_radius_m: 0.0,
            shooter: None,
            guidance: None,
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
fn cover_is_the_strongest_source_applied_once() {
    let forest = json!({ "rect": [300, 200, 200, 200], "canopy_height_m": 12, "trunk_spacing_m": 24,
        "trunk_radius_m": 0.35, "trunk_height_m": 10, "trunk_clearance_m": 2 });
    let map: contract::map::MapDefinition = serde_json::from_value(json!({
        "size": [800, 600], "height_grid_m": 4, "slope_cutoff_deg": 35, "forests": [forest]
    }))
    .unwrap();
    let world = sim::world::WorldGeometry::new(&map);
    let r: contract::scenario::Rules = serde_json::from_value(rules()).unwrap();
    let deep = v3(400.0, 300.0, 0.0);
    let open = v3(100.0, 100.0, 0.0);
    let (bs, fs) = (
        num("cover", "building_spread_multiplier"),
        num("cover", "forest_spread_multiplier"),
    );
    let (bf, ff) = (
        num("cover", "building_fragment_probability_multiplier"),
        num("cover", "forest_fragment_probability_multiplier"),
    );
    let strength = num("buildings", "cover_strength");
    let ground = sim::ground::GroundLayer::new(world.width(), world.depth(), &r.ground);
    let spread = |p, s| sim::damage::cover_spread(&world, &ground, &r, p, s, true);
    let frag = |p, s| sim::damage::fragment_exposure(&world, &ground, &r, p, s);
    assert_eq!(spread(open, 0.0), 1.0);
    assert_eq!(spread(open, strength), 1.0 + (bs - 1.0) * strength);
    assert_eq!(spread(deep, 0.0), fs);
    // A garrison in a forest takes the stronger protection, not the product.
    assert_eq!(spread(deep, strength), bs.max(fs));
    assert_eq!(frag(open, strength), 1.0 + (bf - 1.0) * strength);
    assert_eq!(frag(deep, strength), bf.min(ff));
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
    let (mut b, _) = shelled(extra_props, events, seed);
    let snapshot = |b: &Battle| -> Vec<Occupants> {
        [0, 1]
            .iter()
            .map(|&id| {
                let u = b.unit(UnitId(id)).unwrap();
                let members = (0..u.members.len())
                    .map(|k| (u.members[k].alive(), u.member_position(k)))
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
        if !b.structures().standing(BUILDING) {
            return Collapse {
                b,
                before,
                corpses_before,
            };
        }
    }
    panic!("the building never collapsed");
}

#[test]
fn a_collapse_leaves_a_lower_ruin_and_accounts_for_every_occupant() {
    let Collapse {
        mut b,
        before,
        corpses_before,
    } = collapse(json!([]), json!([]), 7);
    // The building is gone for good; a lower ruin stands on its footprint.
    assert!(b.world().prop(BUILDING).is_none());
    let ruin = b
        .world()
        .props()
        .find(|p| p.kind == PropKind::Ruin)
        .cloned()
        .expect("a ruin");
    assert_eq!(b.structures().replaced_by(ruin.id), Some(BUILDING));
    assert_eq!(ruin.center, v2(CENTRE[0], CENTRE[1]));
    assert_eq!([ruin.half.x, ruin.half.y], [HALF[0], HALF[1]]);
    assert_eq!(2.0 * ruin.half.z, num("buildings", "ruin_height_m"));
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
    let alive_now = living(&b, 0) + living(&b, 1);
    assert_eq!(alive_before, alive_now + corpses_now - corpses_before);
    let radius = num("garrison", "exit_search_radius_m");
    let survivors: usize = [0, 1].iter().map(|&id| living(&b, id)).sum();
    for (id, members, _) in &before {
        let u = b.unit(UnitId(*id)).unwrap();
        assert!(u.garrison.is_none());
        if !u.alive() {
            continue;
        }
        // Survivors come out heavily suppressed.
        assert!(
            u.suppression >= num("suppression", "collapse_level"),
            "{}",
            u.suppression
        );
        for (k, (_, was)) in members.iter().enumerate() {
            if !u.members[k].alive() {
                continue;
            }
            let q = u.member_position(k).xy();
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
    assert!(survivors > 0, "seed 7 leaves survivors to check");
    // The ruin is permanent.
    run(&mut b, 300);
    assert!(b
        .world()
        .props()
        .any(|p| p.id == ruin.id && p.kind == PropKind::Ruin));
}

#[test]
fn a_survivor_with_no_legal_way_out_dies_rather_than_teleporting() {
    // Low walls, knee-high so the tank still fires over them, fill the ground
    // around the building beyond the local search: nowhere to escape to.
    let band = num("garrison", "exit_search_radius_m") + 5.0;
    let (cx, cy, hx, hy) = (CENTRE[0], CENTRE[1], HALF[0], HALF[1]);
    let wall = |x: f64, y: f64, w: f64, h: f64| json!({ "tick": 900, "add_prop": { "kind": "wall", "center": [x, y], "yaw": 0, "half_extents": [w, h, 0.25] } });
    let events = json!([
        wall(cx + hx + band / 2.0, cy, band / 2.0, hy + band),
        wall(cx - hx - band / 2.0, cy, band / 2.0, hy + band),
        wall(cx, cy + hy + band / 2.0, hx, band / 2.0),
        wall(cx, cy - hy - band / 2.0, hx, band / 2.0),
    ]);
    let Collapse { b, before, .. } = collapse(json!([]), events, 8);
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
fn the_ruin_blocks_ground_movement_while_sight_and_fire_pass_over_it() {
    let Collapse { mut b, .. } = collapse(json!([]), json!([]), 7);
    let ruin = b
        .world()
        .props()
        .find(|p| p.kind == PropKind::Ruin)
        .cloned()
        .unwrap();
    assert!(MoverClass::ALL.iter().all(|&c| PropKind::Ruin.blocks(c)));
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
    let squad = [0u32, 1]
        .into_iter()
        .find(|&id| living(&b, id) > 0)
        .unwrap();
    let mut c = Commander { seq: [1, 0] };
    let from = b.unit(UnitId(squad)).unwrap().position.xy();
    let goal = v2(2.0 * CENTRE[0] - from.x, 2.0 * CENTRE[1] - from.y);
    c.ok(
        &mut b,
        Side::Blue,
        Order::Move {
            units: vec![UnitId(squad)],
            gesture: 1,
            goal: [goal.x, goal.y],
            route: RoutePolicy::Shortest,
        },
    );
    for _ in 0..600 {
        b.step();
        let Some(u) = b.unit(UnitId(squad)).filter(|u| u.alive()) else {
            break;
        };
        assert!(
            outside_by(&ruin, u.position.xy()) > 0.0,
            "walked into the ruin"
        );
    }
}

#[test]
fn occupants_see_only_from_occupied_slots_and_are_seen_only_there() {
    // An AT squad of three fills the east, north and west facades; the south
    // facade is empty. Red squads stand in the open south and north.
    let mut b = battle(
        json!([
            { "side": "blue", "kind": "at", "position": [350, 300] },
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
    assert_eq!(
        facades,
        BTreeSet::from([0, 1, 2]),
        "no one on the south facade"
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
        },
    );
    assert_eq!(c.send(&mut b, Side::Blue, garrison(&[0]), true), None);
    until(&mut b, 1500, "inside", |b| inside(b, 0));
    // It went by way of the first destination.
    assert!(b.unit(UnitId(0)).unwrap().orders.is_empty());
}

#[test]
fn garrison_state_enters_the_digest_and_replays_exactly() {
    let units = west_squads(&["rifle", "rifle"]);
    let setup = common::scenario_with(&map(json!([])), units, json!([]), json!([]));
    let mut b = Battle::new(&setup, 13);
    let mut c = Commander::new();
    let mut digests = Vec::new();
    c.ok(&mut b, Side::Blue, garrison(&[0, 1]));
    for t in 0..1200 {
        if t == 700 {
            c.ok(
                &mut b,
                Side::Blue,
                ground(0, [CENTRE[0], CENTRE[1] + 150.0]),
            );
        }
        if t == 900 {
            c.ok(&mut b, Side::Blue, exit(&[1]));
        }
        let before = b.digest();
        b.step();
        digests.push(b.digest());
        // Entering and leaving change the state, so they change the digest.
        assert_ne!(before, b.digest());
    }
    assert!(inside(&b, 0) && !inside(&b, 1));
    let mut r = Battle::from_replay(&setup, &b.replay()).unwrap();
    for d in digests {
        r.step();
        assert_eq!(r.digest(), d, "tick {}", r.tick());
    }
}
