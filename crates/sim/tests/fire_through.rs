//! A gun holds fire only for what its round cannot break: when
//! the first thing on its arc is a destroyable body it can see past, and the
//! round does structural damage, it fires along that arc and wears the
//! blocker down until the line is clear. Driven through real battles.
use contract::command::{CommandEnvelope, Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ActionReason;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::{FlightEvent, Struck};
use sim::math::v2;

use crate::common;

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
    battle_with(common::village(), map, blue)
}

fn battle_with(rules: Value, map: String, blue: Value) -> Battle {
    let setup = serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(&map).unwrap(),
        "rules": rules,
        "units": [
            blue,
            { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// The village rules with the HMG doing no structural damage, so a tank's
/// cannon is the only mount that wears what stands on its line.
fn cannon_only() -> Value {
    let mut rules = common::village();
    rules["weapons"]["hmg"]["structural_damage"] = json!(0);
    rules
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
        b.world().props().any(|p| p.kind == common::kind("rubble")
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
                fired.push((p.id.0, b.arsenal().weapons[r.weapon].id.clone()));
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
fn a_gun_without_structural_damage_holds_fire_behind_sandbags() {
    // The rifle row with no structural damage: its rounds cannot break
    // the sandbags on the line, so the rifles hold. (Grenades lob over.)
    let mut rules = common::village();
    rules["weapons"]["rifle"]["structural_damage"] = json!(0);
    let setup = serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(&map(
            json!([house(), prop("sandbags", BLOCKER, [0.4, 4.0, 0.5])]),
            json!([]),
        ))
        .unwrap(),
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "rifle", "position": SHOOTER },
            { "side": "red", "kind": "rifle", "position": [1150, 550], "engagement": "return_fire_only" },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    shell_the_house(&mut b);
    let rifle = |b: &Battle| {
        b.rounds()
            .any(|(_, r)| r.unit == UnitId(0) && b.arsenal().weapons[r.weapon].id == "rifle")
    };
    assert!(!until(&mut b, 60, rifle), "a rifle fired");
    assert_eq!(reasons(&b)[0], ActionReason::BlockedTrajectory);
}

#[test]
fn a_gun_holds_fire_when_its_rounds_left_cannot_break_the_blocker() {
    // The same tank and sandbags with one HE round or two: one direct hit
    // cannot bring the sandbags down, two can.
    let rules = common::village();
    let per_round = rules["weapons"]["tank_he"]["structural_damage"]
        .as_f64()
        .unwrap();
    let hp = common::props().by_id("sandbags").body.hp.unwrap();
    assert!(per_round < hp && 2.0 * per_round >= hp);
    let shells = |he: u32| {
        let mut rules = cannon_only();
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
        // The cannon only: the tank's HMG may fire into the sandbags too.
        let fired = until(&mut b, 60, |b| {
            b.rounds()
                .any(|(_, r)| r.unit == UnitId(0) && b.arsenal().weapons[r.weapon].id == "tank_he")
        });
        (fired, reasons(&b)[0])
    };
    assert_eq!(shells(1), (false, ActionReason::BlockedTrajectory));
    assert!(shells(2).0, "two rounds can break the sandbags");
}

#[test]
fn a_rifle_squad_fires_through_a_fence_at_the_squad_beyond_it() {
    // A fence panel stops no rounds: the rifles fire through it and hit.
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
}

#[test]
fn an_he_round_through_a_fence_knocks_it_down_and_flies_on_to_the_house() {
    let mut b = battle_with(
        cannon_only(),
        map(
            json!([house(), prop("fence", BLOCKER, [0.1, 4.0, 0.6])]),
            json!([]),
        ),
        tank(),
    );
    shell_the_house(&mut b);
    // Rounds by weapon name, so the tank's HMG (which also fires) is told
    // apart from its HE.
    let mut weapon = std::collections::BTreeMap::new();
    let (mut struck_fence, mut he_through, mut he_on_house) = (false, false, false);
    until(&mut b, 60, |b| {
        for e in b.flight_events() {
            let name = |id| weapon.get(id).map(String::as_str);
            match e {
                FlightEvent::Impact(i) if i.struck == Struck::Prop(1) => struck_fence = true,
                FlightEvent::Impact(i) if i.struck == Struck::Prop(0) => {
                    he_on_house |= name(&i.projectile) == Some("tank_he")
                }
                FlightEvent::Pass(p) if p.prop == 1 => {
                    he_through |= name(&p.projectile) == Some("tank_he")
                }
                _ => {}
            }
        }
        for (p, r) in b.rounds() {
            weapon
                .entry(p.id)
                .or_insert_with(|| b.arsenal().weapons[r.weapon].id.clone());
        }
        he_on_house
    });
    assert!(he_through, "an HE round flew through the fence");
    assert!(he_on_house, "and on to the house: {:?}", reasons(&b));
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
    assert_eq!(house_hp(&b, 0), common::props().by_id("building").body.hp);
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

/// A blue unit of `kind` 30 m west of `props` ordered to fire at the
/// ground at `point`; seconds from its first round until prop 0 is gone, if
/// it goes within `limit_s`.
fn wears_down(kind: &str, props: Value, point: [f64; 2], limit_s: u64) -> Option<f64> {
    let shooter = json!({ "side": "blue", "kind": kind, "position": [270, 300] });
    let mut b = battle(map(props, json!([])), shooter);
    let ack = b.accept(CommandEnvelope {
        side: Side::Blue,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [point[0], point[1], 0.0],
            },
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let mut first = None;
    let gone = until(&mut b, limit_s, |b| {
        if first.is_none() && b.rounds().any(|(_, r)| r.unit == UnitId(0)) {
            first = Some(b.tick());
        }
        b.world().prop(0).is_none()
    });
    gone.then(|| (b.tick() - first.expect("it fired")) as f64 / 30.0)
}

#[test]
fn an_hmg_knocks_a_fence_panel_down_by_sustained_fire_through_it() {
    // The panel stands on the line to ground 10 m past it: every round
    // flies through it and wears it.
    let gone = wears_down(
        "jeep",
        json!([prop("fence", [300.0, 300.0], [0.1, 3.0, 0.6])]),
        [310.0, 300.0],
        30,
    );
    assert!(gone.is_some(), "the panel stands");
}

#[test]
fn an_hmg_fells_a_tree_in_about_ten_seconds_of_sustained_fire() {
    // The user's figure: one HMG fells a 100 hp trunk in about 10 s
    // of fire, within ±30%. The ground point lies in the trunk, so the
    // rounds are fired into it; at 30 m about half of them strike it.
    let secs = wears_down(
        "jeep",
        json!([prop("trunk", [300.0, 300.0], [0.35, 0.35, 6.0])]),
        [300.0, 300.0],
        60,
    )
    .expect("the tree falls");
    assert!((7.0..=13.0).contains(&secs), "felled in {secs:.1} s");
}

#[test]
fn a_rifle_squad_fells_a_tree_by_sustained_fire() {
    // Rifles chip wood (the user): a squad firing on one trunk fells
    // it under sustained fire.
    let secs = wears_down(
        "rifle",
        json!([prop("trunk", [300.0, 300.0], [0.35, 0.35, 6.0])]),
        [300.0, 300.0],
        300,
    )
    .expect("the tree falls");
    // Half the HMG's pace (the user): about 22 s, within ±30%.
    assert!((15.0..=29.0).contains(&secs), "felled in {secs:.1} s");
}

/// What a firefight across a row of trunks did.
#[derive(Debug, Default)]
struct Trunks {
    /// Blue rounds launched by a soldier firing from behind his cover body.
    from_cover: usize,
    /// Of those, rounds that struck the body he fired from behind.
    own_cover_struck: usize,
    /// Red rounds that struck a trunk.
    enemy_struck: usize,
    /// Whether any red soldier was hurt.
    red_hurt: bool,
    /// Blue rounds in the last 10 s that struck a trunk that isn't the
    /// shooter's own cover: chipping a stranger's tree.
    late_stranger_struck: usize,
}

/// Blue's squad at rest behind a row of trunks, red's squad in the open
/// 45 m east, both firing at will; soldiers too tough to fall.
fn firefight_from_trunks(seconds: u64) -> Trunks {
    let trunks: Vec<Value> = (0..7)
        .map(|k| prop("trunk", [64.0, 39.0 + 2.0 * k as f64], [0.35, 0.35, 6.0]))
        .collect();
    let mut rules = common::village();
    sim::fixtures::patch_catalog(&mut rules, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
    let setup = serde_json::from_value(json!({
        "map": { "size": [140, 90], "height_grid_m": 4, "slope_cutoff_deg": 35,
                 "props": trunks, "forests": [] },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": "rifle", "position": [61, 45] },
            { "side": "red", "kind": "rifle", "position": [110, 45] },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 1);
    let mut rounds = std::collections::BTreeMap::new();
    let mut out = Trunks::default();
    for _ in 0..seconds * 30 {
        b.step();
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            let (Struck::Prop(prop), Some(&(unit, cover))) = (i.struck, rounds.get(&i.projectile))
            else {
                continue;
            };
            if unit == UnitId(1) {
                out.enemy_struck += 1;
            } else if cover == Some(Struck::Prop(prop)) {
                out.own_cover_struck += 1;
            } else if b.tick() > (seconds - 10) * 30 {
                out.late_stranger_struck += 1;
            }
        }
        for (p, r) in b.rounds() {
            let cover = p.shooter.and_then(|s| s.cover);
            if rounds.insert(p.id, (r.unit, cover)).is_none() && cover.is_some() {
                out.from_cover += 1;
            }
        }
    }
    out.red_hurt = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .any(|s| s.hp < 1.0e6);
    out
}

#[test]
fn a_soldier_fires_back_past_the_trunk_he_takes_cover_behind() {
    // The hard-coded rule: his own rounds pass his cover body
    // untouched, and reach the enemy.
    let t = firefight_from_trunks(30);
    assert!(t.from_cover > 0, "someone fired from cover: {t:?}");
    assert_eq!(t.own_cover_struck, 0, "{t:?}");
    assert!(t.red_hurt, "blue's fire reached red: {t:?}");
}

#[test]
fn enemy_fire_still_strikes_and_wears_the_trunks() {
    let t = firefight_from_trunks(30);
    assert!(t.enemy_struck > 0, "{t:?}");
}

#[test]
fn nobody_ends_a_fight_behind_a_row_of_trunks_chipping_a_strangers_tree() {
    // A man whose line crosses a trunk that is not his cover moves
    // until he can engage (leaning, re-covering or stepping out), so by the
    // fight's last ten seconds no round of blue's strikes a stranger's trunk.
    let t = firefight_from_trunks(40);
    assert_eq!(t.late_stranger_struck, 0, "{t:?}");
    assert!(t.red_hurt, "{t:?}");
}
