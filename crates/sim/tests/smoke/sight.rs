//! What smoke hides (D1, D9, D10, D12): sight into, out of and across a
//! cloud, the HIDDEN flag, guided missiles' sight lines, and which clouds a
//! side sees. Smoke never touches a launch path, a round or a mover.
use contract::command::TargetRef;
use contract::observation::{ContactSource, ObservationFrame};
use sim::flight::FlightEvent;

use super::*;

/// The test rules with `patches` (`(section, id, patch)`) applied.
pub(super) fn rules_with(patches: &[(&str, &str, Value)]) -> Value {
    let mut rules = common::game();
    for (section, id, patch) in patches {
        sim::fixtures::patch_catalog(&mut rules, section, id, patch.clone());
    }
    rules
}

pub(super) fn battle_on(rules: Value, map: Value, units: Value, seed: u64) -> Battle {
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map": map, "rules": rules, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, seed)
}

pub(super) fn open_map() -> Value {
    serde_json::from_str(&open_ground()).unwrap()
}

/// Tanks too tough to die in these scenes: a test of sight, not of killing.
pub(super) fn tough() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "units",
            "test_tank",
            json!({ "body": { "hull": { "hp": 1.0e6 } } }),
        ),
        (
            "units",
            TANK,
            json!({ "body": { "hull": { "hp": 1.0e6 } } }),
        ),
    ]
}

pub(super) fn facing(side: &str, kind: &str, at: [f64; 2], yaw: f64, engagement: &str) -> Value {
    json!({ "side": side, "kind": kind, "position": at, "yaw": yaw, "engagement": engagement })
}

fn identifies(o: &ObservationFrame, kind: &str) -> bool {
    let kind = common::unit_kind(kind);
    o.identified.iter().any(|u| u.kind == kind)
}

/// Ticks from a pop until the cloud is full and the enemy's last sight of
/// what it hides has lapsed (`sensors.acquisition_grace_s`), with a margin
/// of a sensing round.
pub(super) fn bloomed(kind: &str) -> u64 {
    seconds(smoke(kind).cloud.bloom_s + common::rules().sensors.acquisition_grace_s) + 4
}

/// The guided rounds (missiles) unit `id` has in flight.
fn missiles(b: &Battle, id: u32) -> Vec<sim::flight::ProjectileId> {
    b.rounds()
        .filter(|(_, r)| {
            r.unit == UnitId(id) && b.arsenal().weapons[r.weapon].profile.turn_rad_s.is_some()
        })
        .map(|(p, _)| p.id)
        .collect()
}

fn distance(a: [f64; 3], b: sim::math::V3) -> f64 {
    ((a[0] - b.x).powi(2) + (a[1] - b.y).powi(2)).sqrt()
}

#[test]
fn a_carrier_under_fire_vanishes_into_its_smoke_and_its_last_contact_draws_fire() {
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [200.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [450.0, 150.0],
                std::f64::consts::PI,
                "fire_at_will"
            ),
        ]),
        1,
    );
    run(&mut b, seconds(4.0));
    assert!(
        identifies(b.observe(Side::Red), TANK),
        "red sees the carrier in the open"
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    let at = b.unit(UnitId(0)).unwrap().position;
    let radius = smoke(TANK).cloud.radius_m;
    let o = b.observe(Side::Red);
    assert!(!identifies(o, TANK), "the cloud hides the carrier");
    assert!(
        o.contacts.iter().any(|c| distance(c.center, at) <= radius),
        "red's contact marks where it vanished"
    );
    // Red's gun keeps shelling that last contact, which is still the carrier.
    let mut near = 0;
    for _ in 0..seconds(15.0) {
        b.step();
        near += b
            .flight_events()
            .iter()
            .filter(|e| matches!(e, FlightEvent::Impact(i) if distance([i.point.x, i.point.y, 0.0], at) <= radius))
            .count();
    }
    assert!(near > 0, "rounds still land in the cloud");
}

#[test]
fn a_carrier_that_reverses_out_behind_its_cloud_is_not_seen_through_it() {
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [300.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [500.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    run(&mut b, seconds(2.0));
    assert!(identifies(b.observe(Side::Red), TANK));
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    let cloud = smoke(TANK).cloud;
    let behind = 300.0 - cloud.radius_m - 25.0;
    send(&mut b, 2, move_to(0, [behind, 150.0]), false);
    let mut seen = false;
    let mut out = false;
    let alive = seconds(cloud.life_s - cloud.fade_s) - bloomed(TANK) - 2;
    for _ in 0..alive {
        b.step();
        seen |= identifies(b.observe(Side::Red), TANK);
        out |= b.unit(UnitId(0)).unwrap().position.x < 300.0 - cloud.radius_m;
    }
    assert!(out, "it got clear of the cloud");
    assert!(!seen, "the full cloud stands between them all along");
    // It never fired: red's marker is where it was last seen, not where it went.
    let last = b
        .observe(Side::Red)
        .contacts
        .iter()
        .find(|c| c.source == ContactSource::LastSeen);
    let last = last.expect("a last-seen contact").center;
    assert!(
        (last[0] - 300.0).abs() <= cloud.radius_m,
        "where it vanished, not at {last:?}"
    );
}

#[test]
fn two_units_inside_one_cloud_still_see_each_other() {
    // Side by side, nearer than the smoke a full block takes (D1).
    let cloud = smoke(TANK).cloud;
    let block_m = common::rules().sensors.foliage_full_block / cloud.depth_per_m;
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [200.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [200.0, 150.0 + 0.8 * block_m],
                -std::f64::consts::FRAC_PI_2,
                "return_fire_only"
            ),
        ]),
        1,
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    assert!(
        identifies(b.observe(Side::Red), TANK),
        "close enough to see through the smoke"
    );
}

/// How far `observer` (a ground observer on open ground, `kind`'s sight
/// facing the target) identifies a target standing `inside` metres within
/// the full cloud's rim, by D1: its range shrunk by the target's
/// concealment in the cloud and by the smoke between them.
fn rim_reach(
    observer: &str,
    target: &str,
    cloud: &contract::smoke::SmokeCloudSpec,
    inside: f64,
) -> f64 {
    let rules = common::rules();
    let range = rules.catalog.by_id(observer).sensors.ground_m;
    let concealment = (rules.catalog.by_id(target).concealment * cloud.concealment).min(1.0);
    range * concealment * (-cloud.depth_per_m * inside).exp()
}

#[test]
fn from_outside_a_near_observer_sees_a_target_at_the_rim_and_a_far_one_does_not() {
    let cloud = smoke(SQUAD).cloud;
    // A plain tank stands just within the rim of a squad's cloud, toward red.
    let inside = 1.0;
    let rim = 200.0 + cloud.radius_m - inside;
    let reach = rim_reach("test_tank", "test_tank", &cloud, inside);
    let open = common::rules().catalog.by_id("test_tank").sensors.ground_m;
    let (near, far) = (rim + reach / 2.0, rim + (2.0 * reach).min(open * 0.9));
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", SQUAD, [200.0, 150.0], 0.0, "return_fire_only"),
            facing("blue", "test_tank", [rim, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [near, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
            facing(
                "red",
                "test_tank",
                [far, 140.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    run(&mut b, 4);
    let sightings = |b: &Battle, observer: u32| {
        let units: Vec<_> = (0..4).map(|i| b.unit(UnitId(i)).unwrap().clone()).collect();
        sim::sensing::evaluate(b.world(), &units, b.rules(), Side::Red, |u| {
            u.id == UnitId(observer)
        })
        .into_iter()
        .any(|s| s.target == UnitId(1))
    };
    assert!(
        sightings(&b, 2) && sightings(&b, 3),
        "both see the tank in the open"
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(SQUAD));
    assert!(sightings(&b, 2), "the near observer sees it at the rim");
    assert!(!sightings(&b, 3), "the far one does not");
}

#[test]
fn a_carrier_standing_in_its_cloud_is_hidden_until_the_cloud_is_gone() {
    let mut b = Battle::new(&setup(json!([unit("blue", SQUAD, [200.0, 150.0])])), 1);
    run(&mut b, 4);
    let hidden = |b: &Battle| b.observe(Side::Blue).own[0].concealed;
    assert!(!hidden(&b), "not hidden in the open");
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, seconds(smoke(SQUAD).cloud.bloom_s) + 2);
    assert!(hidden(&b), "HIDDEN while it stands in its smoke");
    run(&mut b, seconds(smoke(SQUAD).cloud.life_s));
    assert!(!hidden(&b), "and not once the cloud is gone");
}

#[test]
fn sight_returns_after_the_fade_and_nothing_of_the_cloud_remains() {
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [300.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [500.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    assert!(!identifies(b.observe(Side::Red), TANK));
    run(&mut b, seconds(smoke(TANK).cloud.life_s));
    assert_eq!(clouds(&b), 0);
    assert!(
        identifies(b.observe(Side::Red), TANK),
        "seen again once the smoke is gone"
    );
}

/// A launcher needing a clear launch path, on the test jeep, as the launch
/// clearance tests build it; and a smoke squad beside it.
fn launcher_rules() -> Value {
    let mut rules = rules_with(&[(
        "units",
        "test_jeep",
        json!({ "mounts": [{ "id": "Launcher", "name": "Launcher", "weapons": ["test_launcher"],
            "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }] }),
    )]);
    rules["weapons"]["test_launcher"] = json!({
        "extends": "artillery_mortar", "speed_mps": 80, "range_m": 650,
        "targets": { "ground": { "range_m": 650, "scatter_mrad": 26 } },
        "requires_clear_launch_path": true, "round_cost": 1,
    });
    rules
}

#[test]
fn a_launcher_needing_a_clear_path_still_launches_from_inside_smoke() {
    let mut b = battle_on(
        launcher_rules(),
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }),
        json!([
            facing("blue", "test_jeep", [200.0, 300.0], 0.0, "return_fire_only"),
            facing("blue", SQUAD, [200.0, 300.0], 0.0, "return_fire_only"),
        ]),
        3,
    );
    send(&mut b, 1, pop(&[1]), false);
    run(&mut b, seconds(smoke(SQUAD).cloud.bloom_s) + 1);
    send(
        &mut b,
        2,
        Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground {
                point: [800.0, 300.0, 0.0],
            },
        },
        false,
    );
    let mut launched = false;
    for _ in 0..seconds(15.0) {
        b.step();
        launched |= b.rounds().any(|(_, r)| r.unit == UnitId(0));
    }
    assert!(
        clouds(&b) > 0 && launched,
        "smoke at the muzzle is no crown overhead"
    );
}

#[test]
fn a_shoulder_sam_at_a_clouds_rim_still_launches_at_a_helicopter_it_sees() {
    let mut rules = common::game();
    rules["weapons"]["stinger"]["requires_clear_launch_path"] = json!(true);
    let radius = smoke(SQUAD).cloud.radius_m;
    let mut b = battle_on(
        rules,
        json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 }),
        json!([
            // The smoke squad's cloud reaches a metre past the AA team, which faces the helicopter.
            facing(
                "blue",
                SQUAD,
                [300.0 - radius + 1.0, 300.0],
                0.0,
                "return_fire_only"
            ),
            facing("blue", "test_aa", [300.0, 300.0], 0.0, "fire_at_will"),
            facing(
                "red",
                "test_heli",
                [600.0, 300.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        3,
    );
    send(&mut b, 1, pop(&[0]), false);
    let mut launched = false;
    for _ in 0..seconds(15.0) {
        b.step();
        launched |= b.rounds().any(|(_, r)| r.unit == UnitId(1));
    }
    assert!(
        clouds(&b) > 0 && launched,
        "the smoke behind it doesn't hold its launch"
    );
}

/// Red's team (unit 2, east) fires its missile at blue's `target` (unit 0,
/// west) across a blue smoke squad (unit 1) standing at `smoker` beside the
/// line, which pops as the missile leaves when `cut` is set. The team opens
/// fire once the target has settled (a helicopter at its cruise height).
/// Whether the target took the hit; it is too tough to die of it.
fn missile_duel(rules: Value, target: &str, team: &str, smoker: [f64; 2], cut: bool) -> bool {
    let mut b = battle_on(
        rules,
        open_map(),
        json!([
            facing("blue", target, [150.0, 150.0], 0.0, "return_fire_only"),
            facing("blue", SQUAD, smoker, 0.0, "return_fire_only"),
            facing(
                "red",
                team,
                [450.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        5,
    );
    run(&mut b, seconds(8.0));
    let ack = b.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        order: Order::SetEngagement {
            units: vec![UnitId(2)],
            policy: contract::command::Engagement::FireAtWill,
        },
        queued: false,
    });
    assert_eq!(ack.error, None);
    let tough = b.unit(UnitId(0)).unwrap().hp;
    // The team's first missile, from launch until it is gone.
    let mut first = None;
    for _ in 0..seconds(30.0) {
        b.step();
        let flying = missiles(&b, 2).first().copied();
        match (first, flying) {
            (None, Some(id)) => {
                first = Some(id);
                if cut {
                    send(&mut b, 1, pop(&[1]), false);
                }
            }
            (Some(id), _) if !missiles(&b, 2).contains(&id) => break,
            _ => {}
        }
    }
    assert!(first.is_some(), "the missile left");
    b.unit(UnitId(0)).unwrap().hp < tough
}

#[test]
fn a_wire_guided_missile_drops_for_good_when_smoke_cuts_its_line() {
    let rules = || {
        rules_with(&[(
            "units",
            "test_tank",
            json!({ "body": { "hull": { "hp": 1.0e6 } } }),
        )])
    };
    let duel = |cut| missile_duel(rules(), "test_tank", "test_at", [300.0, 158.0], cut);
    assert!(duel(false), "unhindered, the guided missile hits");
    assert!(!duel(true), "cut by smoke in flight, it drops");
}

#[test]
fn a_fire_and_forget_missile_flies_on_through_smoke() {
    // The team's missile given a seeker of its own: it no longer needs the
    // gunner once it is away.
    let mut rules = rules_with(&[(
        "units",
        "test_tank",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    )]);
    rules["weapons"]["atgm"]["seeker"] = json!("infrared");
    rules["weapons"]["atgm"]["seeker_fov_deg"] = json!(90);
    assert!(
        missile_duel(rules, "test_tank", "test_at", [300.0, 158.0], true),
        "cut by smoke in flight, it flies on and hits"
    );
}

/// Whether red's AT team (unit `at`) launches within 10 s, and whether red
/// identified the blue tank (unit 0) meanwhile.
fn at_team_fires(b: &mut Battle, at: u32) -> (bool, bool) {
    let (mut launched, mut identified) = (false, false);
    for _ in 0..seconds(10.0) {
        b.step();
        launched |= !missiles(b, at).is_empty();
        identified |= identifies(b.observe(Side::Red), "test_tank");
    }
    (launched, identified)
}

#[test]
fn a_wire_guided_missile_is_launched_neither_into_smoke_nor_out_of_it() {
    let tank = (
        "units",
        "test_tank",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    );
    // Into: a red scout beside the tank in its cloud spots it for the team outside.
    let mut into = battle_on(
        rules_with(std::slice::from_ref(&tank)),
        open_map(),
        json!([
            facing("blue", "test_tank", [300.0, 150.0], 0.0, "return_fire_only"),
            facing("blue", SQUAD, [300.0, 158.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_recon",
                [304.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
            facing(
                "red",
                "test_at",
                [450.0, 150.0],
                std::f64::consts::PI,
                "fire_at_will"
            ),
        ]),
        5,
    );
    send(&mut into, 1, pop(&[1]), false);
    run(&mut into, seconds(smoke(SQUAD).cloud.bloom_s));
    let (launched, identified) = at_team_fires(&mut into, 3);
    assert!(
        identified && !launched,
        "into the cloud: spotted, but no missile"
    );
    // Out of: the team stands in a red cloud; a scout in the open spots the tank.
    let mut out = battle_on(
        rules_with(&[tank]),
        open_map(),
        json!([
            facing("blue", "test_tank", [250.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                SQUAD,
                [400.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
            facing(
                "red",
                "test_recon",
                [330.0, 220.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
            facing(
                "red",
                "test_at",
                [400.0, 150.0],
                std::f64::consts::PI,
                "fire_at_will"
            ),
        ]),
        5,
    );
    let ack = out.accept(CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        order: pop(&[1]),
        queued: false,
    });
    assert_eq!(ack.error, None);
    run(&mut out, seconds(smoke(SQUAD).cloud.bloom_s));
    let (launched, identified) = at_team_fires(&mut out, 3);
    assert!(
        identified && !launched,
        "out of the cloud: spotted, but no missile"
    );
}

#[test]
fn a_carrier_in_smoke_fires_at_what_its_side_spots_and_each_shot_gives_it_away() {
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [200.0, 150.0], 0.0, "fire_at_will"),
            facing(
                "blue",
                "test_recon",
                [300.0, 260.0],
                0.0,
                "return_fire_only"
            ),
            facing(
                "red",
                "test_tank",
                [420.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    let mut evidence = Vec::new();
    let mut shots = 0;
    for _ in 0..seconds(15.0) {
        b.step();
        shots += b
            .rounds()
            .filter(|(p, r)| r.unit == UnitId(0) && p.age_s == 0.0)
            .count();
        let o = b.observe(Side::Red);
        assert!(!identifies(o, TANK), "red never sees it in its smoke");
        if let Some(c) = o
            .contacts
            .iter()
            .find(|c| c.source == ContactSource::Firing)
        {
            if evidence.last() != Some(&c.evidence_tick) {
                evidence.push(c.evidence_tick);
            }
        }
    }
    assert!(
        shots > 1,
        "it fires from its smoke at what its side spotted"
    );
    let red = b.unit(UnitId(2)).unwrap();
    assert!(
        red.hp < red.max_hp(b.rules()),
        "its rounds fly out of the cloud and hit"
    );
    assert!(
        evidence.len() > 1,
        "each shot refreshes red's firing contact: {evidence:?}"
    );
}

#[test]
fn a_side_sees_a_cloud_where_it_sees_its_ground_but_not_the_unit_in_it() {
    let mut b = battle_on(
        rules_with(&tough()),
        open_map(),
        json!([
            facing("blue", TANK, [200.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [380.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    let seen = |b: &Battle, side: Side| {
        sim::smoke::seen(b.world().smoke(), side, &b.observe(side).ground_visibility).count()
    };
    assert_eq!(seen(&b, Side::Blue), 1, "a side always sees its own cloud");
    assert_eq!(seen(&b, Side::Red), 1, "red sees the cloud from the open");
    assert!(
        !identifies(b.observe(Side::Red), TANK),
        "but not the tank inside it"
    );
}

#[test]
fn a_side_behind_a_wall_does_not_see_the_cloud() {
    let map = json!({ "size": [600, 300], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "props": [{ "kind": "wall", "center": [392, 150], "yaw": 0, "half_extents": [8, 150, 8] }] });
    let mut b = battle_on(
        rules_with(&tough()),
        map,
        json!([
            facing("blue", TANK, [200.0, 150.0], 0.0, "return_fire_only"),
            facing(
                "red",
                "test_tank",
                [410.0, 150.0],
                std::f64::consts::PI,
                "return_fire_only"
            ),
        ]),
        1,
    );
    send(&mut b, 1, pop(&[0]), false);
    run(&mut b, bloomed(TANK));
    let field = &b.observe(Side::Red).ground_visibility;
    assert_eq!(
        sim::smoke::seen(b.world().smoke(), Side::Red, field).count(),
        0
    );
}
