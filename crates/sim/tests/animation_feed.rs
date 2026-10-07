//! The animation feed: what each side is published to pose its own
//! and identified enemy models and to place effects, driven through real
//! battles. The simulation never names an animation (Q7).
use std::collections::{BTreeMap, BTreeSet};

use contract::ids::{Side, UnitId};
use contract::observation::{ObservationFrame, OwnUnit, SegmentHit};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::flight::ProjectileId;

use crate::common;

/// A flat 1200 × 600 map plus extra props.
fn map(props: Value) -> String {
    json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": props })
        .to_string()
}

fn battle(props: Value, units: Value, scripts: Value, seed: u64) -> Battle {
    Battle::new(
        &common::scenario_with(&map(props), units, json!([]), scripts),
        seed,
    )
}

fn own(frame: &ObservationFrame, id: u32) -> Option<&OwnUnit> {
    frame.own.iter().find(|u| u.id == UnitId(id))
}

fn weapon_index(b: &Battle, name: &str) -> usize {
    b.arsenal()
        .weapons
        .iter()
        .position(|w| w.id == name)
        .unwrap()
}

#[test]
fn shot_counters_and_elevation_move_only_when_a_round_leaves() {
    // Blue's tank (cannon and HMG) and rifle squad (rifles and grenades)
    // against a red tank and squad that only answer.
    let mut setup = common::scenario_with(
        &map(json!([])),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 250] },
            { "side": "blue", "kind": "test_rifle", "position": [100, 350] },
            { "side": "red", "kind": "test_tank", "position": [380, 150], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [400, 350], "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    // Pose publication is independent of the live balance range.
    setup
        .rules
        .weapons
        .get_mut("rifle")
        .unwrap()
        .ballistics
        .range_m = 450.0;
    let grenade = &mut setup.rules.weapons.get_mut("grenade").unwrap().ballistics;
    grenade.range_m = 450.0;
    grenade.speed_mps = 100.0;
    grenade.gravity_scale = 0.09;
    grenade.scatter_mrad = 30.0;
    let mut b = Battle::new(&setup, 4);
    let mut seen: BTreeSet<ProjectileId> = BTreeSet::new();
    let mut before: BTreeMap<(u32, usize), (f64, f64, u32)> = BTreeMap::new();
    let mut fired_mounts = BTreeSet::new();
    let mut turned_without_firing = 0;
    for _ in 0..900 {
        b.step();
        // This tick's launches per (unit, mount): rounds fire after flight,
        // so a new round still holds its launch velocity.
        let mut launched: BTreeMap<(u32, usize), Vec<(ProjectileId, f64)>> = BTreeMap::new();
        for (p, r) in b.rounds() {
            if !seen.insert(p.id) || r.side != Side::Blue {
                continue;
            }
            let unit = b.unit(r.unit).unwrap();
            let mount = b
                .arsenal()
                .specs(unit.kind)
                .iter()
                .position(|s| s.kinds.contains(&r.weapon))
                .unwrap();
            let v = p.velocity;
            launched
                .entry((r.unit.0, mount))
                .or_default()
                // The simulation's own portable functions: the platform's differ by a bit.
                .push((p.id, libm::atan2(v.z, libm::hypot(v.x, v.y))));
        }
        let frame = b.observe(Side::Blue);
        for u in &frame.own {
            assert_eq!(u.weapon_poses.len(), u.mounts.len());
            for (m, pose) in u.weapon_poses.iter().enumerate() {
                assert_eq!(pose.mount as usize, m);
                let key = (u.id.0, m);
                let (bearing, elevation, shots) =
                    before.get(&key).copied().unwrap_or((pose.bearing, 0.0, 0));
                let new = launched.get(&key).map_or(&[][..], Vec::as_slice);
                assert_eq!(
                    pose.shots,
                    shots + new.len() as u32,
                    "unit {} mount {m}: the counter rises by the rounds launched",
                    u.id.0
                );
                match new.iter().max_by_key(|(id, _)| *id) {
                    Some(&(_, pitch)) => {
                        assert_eq!(pose.elevation, pitch, "elevation of the last round");
                        fired_mounts.insert(key);
                    }
                    None => {
                        assert_eq!(pose.elevation, elevation, "no launch, no change");
                        turned_without_firing += (pose.bearing != bearing) as u32;
                    }
                }
                before.insert(key, (pose.bearing, pose.elevation, pose.shots));
            }
        }
    }
    assert_eq!(
        fired_mounts,
        BTreeSet::from([(0, 0), (0, 1), (1, 0), (1, 1)]),
        "every blue mount fired"
    );
    assert!(turned_without_firing > 0, "weapons turned between shots");
}

/// Blue's rifle squad holds fire at [100, 300]; red's tank stands at `red`.
/// With `order`, red is told at tick 1 to attack ground 200 m north of it.
fn watched_turret(red: [f64; 2], order: bool) -> Battle {
    let scripts = if order {
        json!([{ "tick": 1, "side": "red", "order": { "kind": "attack", "units": [1],
            "target": { "kind": "ground", "point": [red[0], red[1] + 200.0, 0.0] } } }])
    } else {
        json!([])
    };
    battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": red, "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
        ]),
        scripts,
        2,
    )
}

/// Steps both battles until red first launches in `turned`; returns whether
/// blue's frames ever differed and whether red's turret bearings did.
fn compare_until_red_fires(mut quiet: Battle, mut turned: Battle) -> (bool, bool, bool) {
    let (mut frames_differ, mut bearings_differ, mut identified) = (false, false, false);
    for _ in 0..600 {
        quiet.step();
        turned.step();
        if turned.load().rounds_launched > 0 {
            return (frames_differ, bearings_differ, identified);
        }
        let (a, b) = (quiet.observe(Side::Blue), turned.observe(Side::Blue));
        frames_differ |= a != b;
        identified |= !b.identified.is_empty();
        let bearing = |x: &Battle| x.unit(UnitId(1)).unwrap().mounts[0].bearing;
        bearings_differ |= bearing(&quiet) != bearing(&turned);
    }
    panic!("red never fired");
}

#[test]
fn an_enemy_turret_is_published_only_while_identified() {
    // Metamorphic: red's hidden turret turns in one battle and not the other.
    // Out of sight (900 m, past the squad's 600 m), blue's frames are equal.
    let (differ, turned, identified) = compare_until_red_fires(
        watched_turret([1000.0, 300.0], false),
        watched_turret([1000.0, 300.0], true),
    );
    assert!(turned, "red's turret turned in one battle");
    assert!(!identified, "red stayed hidden");
    assert!(!differ, "a hidden turret changes nothing blue is told");
    // Control: identified at 400 m, the same turn reaches blue's frame.
    let (differ, turned, identified) = compare_until_red_fires(
        watched_turret([500.0, 300.0], false),
        watched_turret([500.0, 300.0], true),
    );
    assert!(turned && identified);
    assert!(differ, "an identified turret's pose is published");
}

#[test]
fn identified_enemies_carry_their_seen_soldiers_ids_through_reacquisition() {
    // Red's squad walks behind a wall and out again: blue's tank loses it past
    // the grace and reidentifies it under a new handle.
    let wall =
        json!([{ "kind": "wall", "center": [250, 300], "yaw": 0, "half_extents": [1, 10, 5] }]);
    let scripts = json!([{ "tick": 1, "side": "red", "order":
        { "kind": "move", "units": [1], "gesture": 1, "goal": [400, 370], "route": "shortest" } }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [400, 250], "engagement": "return_fire_only" },
        ]),
        scripts,
        3,
    );
    let mut handles = BTreeSet::new();
    let mut checked = 0;
    for _ in 0..900 {
        b.step();
        let red = own(b.observe(Side::Red), 1).unwrap().clone();
        let truth: BTreeMap<u32, [f64; 3]> = red
            .member_ids
            .iter()
            .copied()
            .zip(red.members.iter().copied())
            .collect();
        for e in &b.observe(Side::Blue).identified {
            handles.insert(e.id);
            assert_eq!(e.member_ids.len(), e.members.len());
            for (id, p) in e.member_ids.iter().zip(&e.members) {
                assert_eq!(
                    truth.get(id),
                    Some(p),
                    "id {id} names the soldier standing there"
                );
                checked += 1;
            }
        }
    }
    assert!(
        handles.len() >= 2,
        "reacquired under a new handle: {handles:?}"
    );
    assert!(checked > 0);
}

#[test]
fn own_soldier_ids_survive_casualties_and_reinforcement_and_corpses_name_the_fallen() {
    // A squad that starts two down beside a supply truck, which refills it.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_supply", "position": [200, 250], "stock": 55 },
            { "side": "blue", "kind": "test_rifle", "position": [220, 250], "yaw": 0.7,
              "engagement": "return_fire_only", "condition": { "casualties": 2 } },
        ]),
        json!([]),
        5,
    );
    b.step();
    let frame = b.observe(Side::Blue).clone();
    let squad = own(&frame, 1).unwrap();
    let first: Vec<u32> = squad.member_ids.clone();
    assert_eq!(first.len(), 6);
    assert_eq!(squad.members.len(), 6);
    let fallen: Vec<u32> = frame.corpses.iter().map(|c| c.soldier).collect();
    assert_eq!(fallen.len(), 2);
    for c in &frame.corpses {
        assert!(
            c.own && c.kind == common::unit_kind("test_rifle") && c.yaw == 0.7,
            "{c:?}"
        );
        assert!(
            !first.contains(&c.soldier),
            "the fallen are not among the living"
        );
    }
    let issued: BTreeSet<u32> = first.iter().chain(&fallen).copied().collect();
    for _ in 0..900 {
        b.step();
    }
    let later = own(b.observe(Side::Blue), 1).unwrap().member_ids.clone();
    assert_eq!(later.len(), 8, "refilled: {later:?}");
    assert_eq!(later[..6], first[..], "the living keep their ids and order");
    for id in &later[6..] {
        assert!(!issued.contains(id), "a replacement gets a fresh id");
    }
}

#[test]
fn ids_follow_their_soldiers_as_the_squad_takes_losses() {
    // Two squads trade fire in the open until blue has lost soldiers.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [360, 300] },
        ]),
        json!([]),
        6,
    );
    let mut last: BTreeMap<u32, [f64; 3]> = BTreeMap::new();
    let mut losses = 0;
    for _ in 0..1800 {
        b.step();
        let frame = b.observe(Side::Blue);
        let Some(squad) = own(frame, 0) else { break };
        let now: BTreeMap<u32, [f64; 3]> = squad
            .member_ids
            .iter()
            .copied()
            .zip(squad.members.iter().copied())
            .collect();
        for (id, p) in &now {
            if let Some(q) = last.get(id) {
                let step = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
                assert!(step < 0.5, "soldier {id} jumped {step} m");
            }
        }
        for (id, q) in &last {
            if !now.contains_key(id) {
                losses += 1;
                let c = frame
                    .corpses
                    .iter()
                    .find(|c| c.soldier == *id)
                    .expect("a corpse");
                let d = ((c.position[0] - q[0]).powi(2) + (c.position[1] - q[1]).powi(2)).sqrt();
                assert!(d < 0.5, "soldier {id} fell where he stood ({d} m)");
            }
        }
        last = now;
        if losses >= 3 {
            break;
        }
    }
    assert!(losses >= 3, "blue lost soldiers: {losses}");
}

#[test]
fn segments_and_blasts_carry_kind_shooter_and_what_was_struck() {
    // Blue's tank shells red's squad in the open; the squad shoots back.
    let mut b = battle(
        json!([]),
        json!([
            { "side": "blue", "kind": "test_tank", "position": [100, 300] },
            { "side": "red", "kind": "test_rifle", "position": [330, 300] },
        ]),
        json!([]),
        7,
    );
    let red_soldiers: BTreeSet<u32> = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .map(|s| s.id)
        .collect();
    let rifle = weapon_index(&b, "rifle");
    let tank_rounds = [
        weapon_index(&b, "tank_ap"),
        weapon_index(&b, "tank_he"),
        weapon_index(&b, "hmg"),
    ];
    let he = weapon_index(&b, "tank_he");
    let he_radius = b.arsenal().weapons[he].def.blast_radius_m;
    let (mut enemy_rifle, mut hits, mut blasts) = (0, BTreeSet::new(), 0);
    for _ in 0..900 {
        b.step();
        let f = b.observe(Side::Blue);
        for s in &f.projectiles {
            match s.own {
                true => {
                    assert!(tank_rounds.contains(&s.kind));
                    assert_eq!(s.shooter_member, None, "a vehicle's gun has no soldier");
                }
                false if s.kind == rifle => {
                    enemy_rifle += 1;
                    assert!(red_soldiers.contains(&s.shooter_member.unwrap()));
                }
                false => {}
            }
            match (s.hit, s.impact_normal) {
                (SegmentHit::None, None) => {}
                (hit, Some(n)) if hit != SegmentHit::None => {
                    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    assert!((len - 1.0).abs() < 1e-9, "a unit normal");
                    if !s.own {
                        let end = s.path.last().unwrap();
                        assert!(f.ground_visibility.visible(end[0], end[1]));
                    }
                    hits.insert(format!("{hit:?}"));
                }
                other => panic!("a hit and its normal go together: {other:?}"),
            }
        }
        for blast in &f.blasts {
            if blast.kind == he {
                blasts += 1;
                assert_eq!(blast.radius, he_radius);
            }
        }
    }
    assert!(enemy_rifle > 0, "red's rifle tracers reached blue");
    assert!(blasts > 0, "blue saw its HE burst");
    assert!(
        hits.contains("Soldier") && hits.contains("Ground"),
        "{hits:?}"
    );
}

#[test]
fn a_blast_on_unseen_ground_is_not_published_to_the_enemy() {
    // Red's tank shells ground behind a long wall from where blue cannot see.
    let wall =
        json!([{ "kind": "wall", "center": [300, 300], "yaw": 0, "half_extents": [1, 200, 8] }]);
    let scripts = json!([{ "tick": 1, "side": "red", "order": { "kind": "attack", "units": [1],
        "target": { "kind": "ground", "point": [450, 300, 0] } } }]);
    let mut b = battle(
        wall,
        json!([
            { "side": "blue", "kind": "test_rifle", "position": [100, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [700, 300], "yaw": std::f64::consts::PI },
        ]),
        scripts,
        8,
    );
    let mut hidden_blasts = 0;
    for _ in 0..900 {
        b.step();
        let blue = b.observe(Side::Blue);
        let fog = &blue.ground_visibility;
        hidden_blasts += b
            .observe(Side::Red)
            .blasts
            .iter()
            .filter(|blast| !fog.visible(blast.point[0], blast.point[1]))
            .count();
        // A scattered shell may hit visible ground or the wall. Only those
        // visible impacts may reach blue, regardless of the intended target.
        for blast in &blue.blasts {
            assert!(fog.visible(blast.point[0], blast.point[1]));
        }
    }
    assert!(
        hidden_blasts > 0,
        "red sees blasts on ground blue cannot see"
    );
}
