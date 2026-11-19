//! Who works a launcher: a survivor takes up a fallen gunner's launcher and
//! keeps guiding, the round leaves from the soldier carrying it in whatever
//! slot he stands, and a side sees an enemy operator only when it sees him.
use contract::command::Order;
use contract::ids::{Side, UnitId};
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common::{self as c, Commander};
use crate::guidance::{missile, own, quick, until_launch};
use crate::top_attack::{apex, top_attack};

#[test]
fn a_survivor_keeps_guiding_when_the_original_gunner_falls() {
    let takeovers = survivor_takeovers(|_| {}).count();
    assert!(takeovers > 0, "no seed saw the gunner fall mid-flight");
}

#[test]
fn a_survivor_keeps_a_top_attack_missile_climbing_and_diving_onto_the_tank() {
    // A handed-off missile keeps its loft and still strikes the tank.
    let mut struck = 0;
    for (mut b, mut path) in survivor_takeovers(top_attack) {
        while let Some(m) = missile(&b) {
            path.push(m.position);
            b.step();
        }
        assert!(apex(&path) > 30.0, "it climbed: apex {:.1} m", apex(&path));
        if own(&b, Side::Red, 1).map_or(0.0, |u| u.hp) < c::hull("test_tank").hp {
            struck += 1;
        }
    }
    assert!(struck > 0, "no handed-off missile struck the tank");
}

/// A fragile gunner guides at a far tank; once he launches, a red scout team
/// opens fire on his team, whose riflemen do not fall. Whenever he falls with
/// a missile in flight, a survivor takes up the same launcher and keeps
/// guiding that missile. Each takeover's battle, the tick after the handoff,
/// with the missile's path as blue saw it until then.
fn survivor_takeovers(rules: fn(&mut Value)) -> impl Iterator<Item = (Battle, Vec<[f64; 3]>)> {
    (1..=6).filter_map(move |seed| survivor_takeover(seed, rules))
}

fn survivor_takeover(seed: u64, rules: fn(&mut Value)) -> Option<(Battle, Vec<[f64; 3]>)> {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [630, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 330], "engagement": "return_fire_only" },
        ]),
        seed,
        |r| {
            let hp = |hp| json!({ "hp": hp });
            sim::fixtures::patch_catalog(r, "soldiers", "test_at_rifleman", hp(100_000));
            sim::fixtures::patch_catalog(r, "soldiers", "test_atgm_gunner", hp(1));
            rules(r);
        },
    );
    until_launch(&mut b);
    let open_fire = Order::SetEngagement {
        units: vec![UnitId(2)],
        policy: contract::command::Engagement::FireAtWill,
    };
    Commander::new().ok(&mut b, Side::Red, open_fire);
    let mut path = Vec::new();
    for _ in 0..600 {
        let flying = missile(&b).filter(|m| m.supported);
        path.extend(flying.as_ref().map(|m| m.position));
        b.step();
        let gunner = own(&b, Side::Blue, 0).is_some_and(|u| u.member_slots.contains(&0));
        // His missile, if it is still in flight the tick he fell: one that
        // struck that same tick had nothing left to release.
        let still = |b: &Battle| {
            let id = flying.as_ref()?.id;
            b.observe(Side::Blue)
                .guided
                .iter()
                .find(|m| m.id == id)
                .cloned()
        };
        if let (Some(before), false, true) = (&flying, gunner, still(&b).is_some()) {
            // Guidance runs before damage: the next tick assigns a
            // survivor before steering the existing missile.
            b.step();
            let Some(m) = still(&b) else {
                // It may hit during the handoff tick: there is no continuing
                // guidance to inspect, but disappearance must be a real impact.
                assert!(b.flight_events().iter().any(|event| matches!(
                    event, sim::flight::FlightEvent::Impact(i)
                        if i.projectile.0 == before.id && i.detonated
                )));
                return None;
            };
            assert!(m.supported, "seed {seed}: a survivor keeps guiding");
            let team = b.unit(UnitId(0)).unwrap();
            let launcher = team.mounts.iter().find(|m| m.support.is_some()).unwrap();
            let operator = launcher.operator.unwrap();
            let survivor = team.members.iter().find(|s| s.id == operator).unwrap();
            assert_eq!(
                survivor.active_mount,
                Some(launcher.spec),
                "guidance reserves the survivor on the handoff tick"
            );
            assert!(
                !b.rounds().any(|(p, r)| {
                    p.age_s == 0.0
                        && r.unit == UnitId(0)
                        && b.arsenal().weapons[r.weapon].id == "rifle"
                        && p.shooter.unwrap().body.0 == operator
                }),
                "the surviving guide cannot fire his rifle on that tick"
            );
            assert_eq!(m.id, before.id, "the same missile survives the handoff");
            assert!(
                m.point[0] > 600.0 && m.point[2] > 0.0,
                "still aiming at the tank"
            );
            path.push(m.position);
            return Some((b, path));
        }
        if !gunner {
            return None;
        }
    }
    None
}

/// A living soldier: his slot and where he stands.
type Member = (u8, [f64; 3]);

/// Steps until blue unit 0 launches an ATGM: where it left (its muzzle) and
/// the launcher's living soldiers then, as (slot, position).
fn atgm_launch(b: &mut Battle, ticks: u64) -> Option<([f64; 3], Vec<Member>)> {
    let mut seen: std::collections::BTreeSet<_> = b.rounds().map(|(p, _)| p.id).collect();
    for _ in 0..ticks {
        b.step();
        let launched = b.rounds().find_map(|(p, r)| {
            let atgm = b.arsenal().weapons[r.weapon].id == "atgm";
            (seen.insert(p.id) && atgm && r.unit == UnitId(0))
                .then_some(p.position - p.velocity * p.age_s)
        });
        if let Some(at) = launched {
            let u = own(b, Side::Blue, 0).unwrap();
            let members = u.member_slots.iter().copied().zip(u.members).collect();
            return Some(([at.x, at.y, at.z], members));
        }
    }
    None
}

pub(crate) fn horizontal(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// A round left from this soldier's own muzzle where he stands.
fn standing_at(origin: [f64; 3], soldier: [f64; 3]) -> bool {
    let muzzle = c::game()["physics"]["infantry_muzzle_m"].as_f64().unwrap();
    horizontal(origin, soldier) < 0.05 && (origin[2] - soldier[2] - muzzle).abs() < 0.05
}

#[test]
fn a_special_weapon_leaves_from_its_carrier_in_whatever_slot_he_stands() {
    // The gunner stands in the team's middle slot: the missile leaves from
    // his own muzzle, not the first soldier's and not the team's middle.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
        ]),
        1,
        |r| {
            let slots = ["test_at_rifleman", "test_atgm_gunner", "test_at_rifleman"];
            let at = json!({ "body": { "squad": { "slots": slots } } });
            sim::fixtures::patch_catalog(r, "units", "test_at", at);
        },
    );
    let (origin, members) = atgm_launch(&mut b, 600).expect("the team launched");
    let gunner = members
        .iter()
        .find(|m| m.0 == 1)
        .expect("the gunner lives")
        .1;
    assert!(
        standing_at(origin, gunner),
        "from the gunner at {gunner:?}, not {origin:?}"
    );
}

#[test]
fn a_fallen_gunners_launcher_fires_from_the_soldier_who_took_it_up() {
    // A scout kills the gunner; his survivor fires at a nonlethal target.
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_tank", "position": [600, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_recon", "position": [100, 330] },
        ]),
        2,
        |r| {
            r["weapons"]["atgm"]["damage"] = json!(0);
            let hp = |hp| json!({ "hp": hp });
            sim::fixtures::patch_catalog(r, "soldiers", "test_at_rifleman", hp(100_000));
            sim::fixtures::patch_catalog(r, "soldiers", "test_atgm_gunner", hp(1));
        },
    );
    let mut fallen = false;
    for _ in 0..900 {
        b.step();
        let Some(u) = own(&b, Side::Blue, 0) else {
            break;
        };
        if !u.member_slots.contains(&0) {
            fallen = true;
            break;
        }
    }
    assert!(fallen, "the gunner fell with his team alive");
    let (origin, members) = atgm_launch(&mut b, 900).unwrap_or_else(|| {
        panic!(
            "the team launched again: {:?} tank {:?}",
            own(&b, Side::Blue, 0).map(|u| (u.member_slots, u.mounts)),
            own(&b, Side::Red, 1).map(|u| u.hp)
        )
    });
    let first = members.iter().min_by_key(|m| m.0).unwrap();
    assert!(
        standing_at(origin, first.1),
        "from slot {} at {:?}, not {origin:?}",
        first.0,
        first.1
    );
}

#[test]
fn the_published_launcher_operator_follows_the_survivor_who_takes_it_up() {
    let mut b = quick(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_at", "position": [40, 300] },
            { "side": "red", "kind": "test_recon", "position": [100, 330], "engagement": "return_fire_only" }
        ]),
        2,
        |r| {
            sim::fixtures::patch_catalog(
                r,
                "soldiers",
                "test_at_rifleman",
                json!({ "hp": 100_000 }),
            );
            sim::fixtures::patch_catalog(r, "soldiers", "test_atgm_gunner", json!({ "hp": 1 }));
        },
    );
    let team = own(&b, Side::Blue, 0).unwrap();
    let operator = |team: &contract::observation::OwnUnit| {
        serde_json::to_value(team.weapon_poses[1]).unwrap()["operator"].clone()
    };
    assert_eq!(
        operator(&team),
        json!(team.member_ids[0]),
        "the launcher has one published operator"
    );
    Commander::new().ok(
        &mut b,
        Side::Red,
        Order::SetEngagement {
            units: vec![UnitId(1)],
            policy: contract::command::Engagement::FireAtWill,
        },
    );
    let mut survivor = None;
    for _ in 0..900 {
        b.step();
        let team = own(&b, Side::Blue, 0).unwrap();
        if !team.member_slots.contains(&0) {
            b.step();
            survivor = own(&b, Side::Blue, 0);
            break;
        }
    }
    let survivor = survivor.expect("the gunner fell and his riflemen survived");
    assert_eq!(
        operator(&survivor),
        json!(survivor.member_ids[0]),
        "the same launcher now names its surviving operator"
    );
}

#[test]
fn a_seen_team_does_not_publish_its_unseen_launcher_operator() {
    let units = json!([
        { "side": "blue", "kind": "test_tank", "position": [100, 300], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_at", "position": [200, 300], "engagement": "return_fire_only" }
    ]);
    let mut clear = quick(json!([]), units.clone(), 7, |_| {});
    for _ in 0..5 {
        clear.step();
    }
    let seen = clear
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == c::unit_kind("test_at"))
        .unwrap();
    let gunner = seen.weapon_poses[1]
        .operator
        .expect("the unobstructed operator is seen");
    let k = seen.member_ids.iter().position(|id| *id == gunner).unwrap();
    let at = seen.members[k];
    let screen = json!([{ "kind": "wall", "center": [(100.0 + at[0]) / 2.0, (300.0 + at[1]) / 2.0], "yaw": 0, "half_extents": [0.1, 0.1, 8] }]);
    let mut screened = quick(screen, units, 7, |_| {});
    for _ in 0..5 {
        screened.step();
    }
    let seen = screened
        .observe(Side::Blue)
        .identified
        .iter()
        .find(|e| e.kind == c::unit_kind("test_at"))
        .expect("the guards still identify the team");
    assert!(
        !seen.member_ids.contains(&gunner),
        "only the gunner is screened"
    );
    assert_eq!(
        seen.weapon_poses[1].operator, None,
        "an unseen carrier's identity never crosses the fog boundary"
    );
    let team = screened.unit(UnitId(1)).unwrap();
    let visible_activity: Vec<_> = seen
        .member_ids
        .iter()
        .map(|id| {
            team.members
                .iter()
                .find(|s| s.id == *id)
                .unwrap()
                .active_mount
                .map(|m| m as u8)
        })
        .collect();
    assert_eq!(
        seen.member_active_mounts, visible_activity,
        "activity is aligned only with the visible member identities"
    );
}
