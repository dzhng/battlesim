//! A gun holds fire only for what its round cannot break (slice 27c): when
//! the first thing on its arc is a destroyable body it can see past, and the
//! round does structural damage, it fires along that arc and wears the
//! blocker down until the line is clear. Driven through real battles.
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::map::PropKind;
use contract::observation::ActionReason;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, Struck};
use sim::math::v2;

mod common;

/// The house every test shells, and where its tank or squad stands.
const HOUSE: [f64; 2] = [400.0, 300.0];
const SHOOTER: [f64; 2] = [100.0, 300.0];

fn prop(kind: &str, center: [f64; 2], half: [f64; 3]) -> Value {
    json!({ "kind": kind, "center": center, "yaw": 0, "half_extents": half })
}

fn house() -> Value {
    prop("building", HOUSE, [10.0, 10.0, 4.0])
}

/// Flat ground, or a ridge between the shooter and the house.
fn map(props: Value, relief: Value) -> String {
    json!({ "size": [1200, 600], "height_grid_m": 4, "slope_cutoff_deg": 35,
            "props": props, "forests": [], "relief": relief })
    .to_string()
}

fn battle(map: String, blue: Value) -> Battle {
    let units = json!([
        blue,
        { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
    ]);
    Battle::new(&common::scenario(&map, units, json!([])), 1)
}

fn tank() -> Value {
    json!({ "side": "blue", "kind": "tank", "position": SHOOTER })
}

/// Blue's unit 0 attacks the ground at the house's centre, as the flank
/// script's bombardment does.
fn shell_the_house(b: &mut Battle) {
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [HOUSE[0], HOUSE[1], 0.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
}

fn house_hp(b: &Battle, id: u32) -> Option<f64> {
    b.structures().hp(b.world(), id)
}

fn reasons(b: &Battle) -> Vec<ActionReason> {
    b.unit(UnitId(0))
        .unwrap()
        .mounts
        .iter()
        .map(|m| m.reason)
        .collect()
}

/// Step until `done`, at most `seconds`; whether it happened.
fn until(b: &mut Battle, seconds: u64, mut done: impl FnMut(&Battle) -> bool) -> bool {
    for _ in 0..seconds * 30 {
        b.step();
        if done(b) {
            return true;
        }
    }
    false
}

#[test]
fn a_tank_shells_a_house_through_the_sandbags_in_front_of_it() {
    // Sandbags 30 m short of the house (beyond HE's blast reach of it) cross
    // the tank's line to the house's centre. Spread may carry a round over
    // them, so the claim is only that the tank fires, the sandbags fall, and
    // the house is still shelled once they have.
    let blast = common::village()["weapons"]["tank_he"]["blast_radius_m"]
        .as_f64()
        .unwrap();
    let sandbags = [HOUSE[0] - 40.0, HOUSE[1]];
    assert!(HOUSE[0] - 10.0 - (sandbags[0] + 0.4) > blast);
    let mut b = battle(
        map(
            json!([house(), prop("sandbags", sandbags, [0.4, 4.0, 0.5])]),
            json!([]),
        ),
        tank(),
    );
    shell_the_house(&mut b);
    let fell = until(&mut b, 120, |b| b.world().prop(1).is_none());
    assert!(fell, "the sandbags stand: {:?}", reasons(&b));
    assert!(
        b.world().props().any(|p| p.kind == PropKind::Rubble
            && p.footprint().contains(v2(sandbags[0], sandbags[1]), 0.1)),
        "the sandbags left rubble"
    );
    let hp = house_hp(&b, 0).expect("the house stands");
    assert!(
        until(&mut b, 60, |b| house_hp(b, 0).is_none_or(|now| now < hp)),
        "the house is shelled once the line is clear: {:?}",
        reasons(&b)
    );
}

/// Blue's unit 0 is ordered to shell the house past the given props and
/// relief; after a minute, the reasons its mounts give and the weapons of
/// every round it launched.
fn shell_past(props: Value, relief: Value, blue: Value) -> (Vec<ActionReason>, Vec<String>) {
    let mut b = battle(map(props, relief), blue);
    shell_the_house(&mut b);
    let mut fired: Vec<(u64, String)> = Vec::new();
    for _ in 0..30 * 60 {
        b.step();
        for (p, r) in b.rounds() {
            if r.unit == UnitId(0) && !fired.iter().any(|(id, _)| *id == p.id.0) {
                fired.push((p.id.0, b.arsenal().weapons[r.weapon].name.clone()));
            }
        }
    }
    (reasons(&b), fired.into_iter().map(|(_, n)| n).collect())
}

/// Where the sandbags stood in the first test: on the line, 30 m short.
const BLOCKER: [f64; 2] = [HOUSE[0] - 40.0, HOUSE[1]];

#[test]
fn a_tank_holds_fire_for_what_its_rounds_cannot_break() {
    // A dragon's tooth has no integrity; a wall has, but hides what is past
    // it; a ridge is terrain.
    let cases = [
        (
            "tooth",
            json!([house(), prop("tooth", BLOCKER, [0.6, 0.6, 0.6])]),
            json!([]),
        ),
        (
            "wall",
            json!([house(), prop("wall", BLOCKER, [0.4, 4.0, 2.0])]),
            json!([]),
        ),
        (
            "ridge",
            json!([house()]),
            json!([{ "kind": "ridge", "center": [250, 300], "peak_m": 8, "radius_m": 60 }]),
        ),
    ];
    for (name, props, relief) in cases {
        let (reasons, fired) = shell_past(props, relief, tank());
        assert_eq!(reasons[0], ActionReason::BlockedTrajectory, "{name}");
        assert!(fired.is_empty(), "{name}: fired {fired:?}");
    }
}

#[test]
fn a_rifle_squad_holds_its_rifles_behind_sandbags() {
    // Rifles do no structural damage, so their rounds cannot break the
    // sandbags on the line. (The squad's grenades lob over them.)
    let squad = json!({ "side": "blue", "kind": "rifle", "position": SHOOTER });
    let (reasons, fired) = shell_past(
        json!([house(), prop("sandbags", BLOCKER, [0.4, 4.0, 0.5])]),
        json!([]),
        squad,
    );
    assert_eq!(reasons[0], ActionReason::BlockedTrajectory);
    assert!(!fired.iter().any(|w| w == "rifle"), "fired {fired:?}");
}

#[test]
fn a_gun_holds_fire_when_its_rounds_left_cannot_break_the_blocker() {
    // The same tank and sandbags with one HE round or two: one direct hit
    // cannot bring the sandbags down, two can.
    let rules = common::village();
    let per_round = rules["weapons"]["tank_he"]["structural_damage"]
        .as_f64()
        .unwrap();
    let hp = rules["props"]["sandbags"]["hp"].as_f64().unwrap();
    assert!(per_round < hp && 2.0 * per_round >= hp);
    let shells = |he: u32| {
        let mut rules = common::village();
        rules["weapons"]["tank_he"]["ammo"] = json!(he);
        let setup = serde_json::from_value(json!({
            "map": serde_json::from_str::<Value>(&map(
                json!([house(), prop("sandbags", BLOCKER, [0.4, 4.0, 0.5])]),
                json!([]),
            ))
            .unwrap(),
            "rules": rules,
            "units": [
                tank(),
                { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
            ],
            "events": [],
            "scripts": [],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        shell_the_house(&mut b);
        let fired = until(&mut b, 60, |b| b.rounds().any(|(_, r)| r.unit == UnitId(0)));
        (fired, reasons(&b)[0])
    };
    assert_eq!(shells(1), (false, ActionReason::BlockedTrajectory));
    assert!(shells(2).0, "two rounds can break the sandbags");
}

#[test]
fn a_rifle_squad_fires_through_a_fence_at_the_squad_beyond_it() {
    // A fence panel stops no rounds: the rifles fire through it and hit,
    // and do it no harm (no structural damage).
    let map = map(
        json!([prop("fence", [300.0, 300.0], [0.1, 8.0, 0.6])]),
        json!([]),
    );
    let units = json!([
        { "side": "blue", "kind": "rifle", "position": [240, 300] },
        { "side": "red", "kind": "rifle", "position": [303, 300], "engagement": "return_fire_only" },
    ]);
    let mut b = Battle::new(&common::scenario(&map, units, json!([])), 1);
    let full = b.unit(UnitId(1)).unwrap().members[0].hp;
    let hurt = until(&mut b, 30, |b| {
        b.unit(UnitId(1))
            .unwrap()
            .members
            .iter()
            .any(|s| s.hp < full)
    });
    assert!(hurt, "red is hit through the fence: {:?}", reasons(&b));
    let hp = common::village()["props"]["fence"]["hp"].as_f64();
    assert_eq!(b.structures().hp(b.world(), 0), hp, "rifles never wear it");
}

#[test]
fn an_he_round_through_a_fence_knocks_it_down_and_flies_on_to_the_house() {
    let mut b = battle(
        map(
            json!([house(), prop("fence", BLOCKER, [0.1, 4.0, 0.6])]),
            json!([]),
        ),
        tank(),
    );
    let full = house_hp(&b, 0).unwrap();
    shell_the_house(&mut b);
    let mut struck_fence = false;
    let hit = until(&mut b, 60, |b| {
        struck_fence |= b
            .flight_events()
            .iter()
            .any(|e| matches!(e, FlightEvent::Impact(i) if i.struck == Struck::Prop(1)));
        house_hp(b, 0).is_none_or(|hp| hp < full)
    });
    assert!(hit, "the house is shelled: {:?}", reasons(&b));
    assert!(!struck_fence, "no round stopped at the fence");
    assert!(b.world().prop(1).is_none(), "the fence panel is down");
}

#[test]
fn a_tank_never_fires_through_a_house_at_ground_beyond_it() {
    // The house is not the target here, and it hides what is past it.
    let mut b = battle(map(json!([house()]), json!([])), tank());
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [HOUSE[0] + 60.0, HOUSE[1], 0.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let fired = until(&mut b, 60, |b| b.rounds().any(|(_, r)| r.unit == UnitId(0)));
    assert!(!fired);
    assert_eq!(reasons(&b)[0], ActionReason::BlockedTrajectory);
    assert_eq!(
        house_hp(&b, 0),
        common::village()["props"]["building"]["hp"].as_f64()
    );
}

#[test]
fn a_tank_firing_at_will_shoots_through_sandbags_at_the_squad_behind_them() {
    // Automatic engagement follows the same rule: a chest-high run of
    // sandbags stands 3 m in front of a red squad, on the tank's line to it.
    let map = map(
        json!([prop("sandbags", [357.0, 300.0], [0.4, 8.0, 0.75])]),
        json!([]),
    );
    let units = json!([
        tank(),
        { "side": "red", "kind": "rifle", "position": [360, 300], "engagement": "return_fire_only" },
    ]);
    let mut b = Battle::new(&common::scenario(&map, units, json!([])), 1);
    assert!(
        until(&mut b, 90, |b| b.world().prop(0).is_none()),
        "the sandbags stand: {:?}",
        reasons(&b)
    );
}
