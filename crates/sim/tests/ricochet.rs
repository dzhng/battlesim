//! Ricochets: a kinetic round that fails to pierce a hull may
//! glance off by its face's chance, flying on in the same tick from the hit,
//! slower and weaker, at most `max_bounces` times; HE always bursts.

use std::collections::BTreeMap;

use crate::common::*;
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::random::Rng;
use contract::scenario::FaceChances;
use serde_json::json;
use sim::battle::Battle;
use sim::damage::RoundPower;
use sim::flight::{BodyId, FlightEvent, Launch, ProjectileId, Projectiles, Ricochet, Struck};
use sim::math::{v2, v3, V3};

fn launch(origin: V3, velocity: V3, suppression_radius_m: f64) -> Launch {
    Launch {
        origin,
        velocity,
        gravity_scale: profile("rifle").gravity_scale,
        lifetime_s: profile("rifle").lifetime_s,
        suppression_radius_m,
        shooter: None,
        guidance: None,
        motor: None,
        fall: None,
    }
}

fn tank(id: u32, x: f64, y: f64, yaw: f64) -> Mover {
    Mover {
        yaw,
        ..Mover::standing(id, id, tank_shape(), v3(x, y, 0.0))
    }
}

const ALWAYS: FaceChances = FaceChances {
    front: 1.0,
    side: 1.0,
    rear: 1.0,
    roof: 1.0,
};

/// Hulls that glance every failed penetration straight along the mirror.
fn always_glancing(hulls: impl IntoIterator<Item = u32>) -> TankHulls {
    let mut r = TankHulls::new(hulls, 7);
    r.armor.ricochet = ALWAYS;
    r.rules.scatter_deg = 0.0;
    r
}

fn ricochets(events: &[(u64, FlightEvent)]) -> Vec<Ricochet> {
    events
        .iter()
        .filter_map(|(_, e)| match e {
            FlightEvent::Ricochet(r) => Some(*r),
            _ => None,
        })
        .collect()
}

fn speed(v: V3) -> f64 {
    v.length()
}

/// Two hulls side by side with a 1 m lane between them (A's +y side at
/// 101.8, B's -y side at 102.8), both spanning x 46.5–53.5.
fn lane() -> [Mover; 2] {
    [tank(1, 50.0, 100.0, 0.0), tank(2, 50.0, 104.6, 0.0)]
}

#[test]
fn a_round_glances_between_two_hulls_at_most_twice_then_stops() {
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let hulls_movers = lane();
    let mut hulls = always_glancing([1, 2]);
    let rules = hulls.rules.clone();
    assert_eq!(rules.max_bounces, 2, "the fixture's bound");
    let mut store = Projectiles::new(config());
    // Into the lane at 30°: B's side at x ≈ 47.9, A's at ≈ 49.6, B's again
    // at ≈ 51.3, all inside one tick.
    let rifle = 850.0; // fast enough to cross the lane in one tick
    let (s, c) = 30f64.to_radians().sin_cos();
    let id = store.launch(launch(
        v3(47.0, 102.3, 1.2),
        v3(rifle * c, rifle * s, 0.0),
        0.0,
    ));
    hulls.register(id, "rifle");
    let events = fly_with(
        &mut store,
        &world,
        5,
        |k| hulls_movers.iter().map(|m| m.body(k, dt)).collect(),
        &mut hulls,
    );
    let glances = ricochets(&events);
    assert_eq!(
        glances.iter().map(|r| r.body).collect::<Vec<_>>(),
        [BodyId(2), BodyId(1)],
        "B, then the other collider, A: {events:?}"
    );
    assert!(glances.iter().all(|r| r.projectile == id));
    let [(1, hit)] = impacts(&events)[..] else {
        panic!("one ending, in the first tick: {events:?}")
    };
    assert_eq!(hit.projectile, id, "a ricochet keeps the round's id");
    assert_eq!(hit.struck, Struck::Body(BodyId(2)));
    assert_eq!(hit.bounces, 2, "the third failed penetration stops it");
    assert!(!hit.detonated);
    // Each glance keeps the fixture's share of the speed (gravity adds
    // micrometres per second over a metre of flight).
    for r in &glances {
        let kept = speed(r.deflected) / speed(r.velocity);
        assert!((kept - rules.speed_kept).abs() < 1e-9, "{kept}");
    }
    let arrived = speed(hit.velocity) / rifle;
    assert!(
        (arrived - rules.speed_kept.powi(2)).abs() < 1e-3,
        "{arrived}"
    );
    // The glance points lie on the struck sides, in time order.
    assert!((glances[0].point.y - 102.8).abs() < 1e-6);
    assert!((glances[1].point.y - 101.8).abs() < 1e-6);
    assert!(glances[0].time < glances[1].time && glances[1].time < hit.time);
    assert!(store.active().is_empty());
}

#[test]
fn a_glanced_round_is_weaker_against_the_next_hull() {
    // A round of 120 fails A's front (140) and glances onto B's side (100),
    // which it would pierce fresh but not at half penetration.
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let movers = [tank(1, 50.0, 100.0, 0.0), tank(2, 60.0, 106.0, 0.0)];
    let bodies = |k| movers.iter().map(|m| m.body(k, dt)).collect();
    let power = RoundPower {
        penetration: 120.0,
        bursts: false,
    };
    let rifle = weapon("rifle").speed_mps;
    // Onto A's front at (53.5, 100), heading (-6.5, 3): it mirrors to (6.5, 3)
    // and meets B's -y side near x 62.6.
    let dir = v3(-6.5, 3.0, 0.0).normalized();
    let mut hulls = always_glancing([1, 2]);
    let mut store = Projectiles::new(config());
    let glanced = store.launch(launch(v3(60.0, 97.0, 1.2), dir * rifle, 0.0));
    hulls.rounds.insert(glanced, power);
    let events = fly_with(&mut store, &world, 5, bodies, &mut hulls);
    let glances = ricochets(&events);
    assert_eq!(
        glances.iter().map(|r| r.body).collect::<Vec<_>>(),
        [BodyId(1), BodyId(2)],
        "B's side turns the weakened round away: {events:?}"
    );
    // The same round fresh, along the glanced path, pierces B's side.
    let mut store = Projectiles::new(config());
    let clear = glances[0].point + glances[0].normal * 0.01;
    let fresh = store.launch(launch(clear, glances[0].deflected, 0.0));
    hulls.rounds.insert(fresh, power);
    let events = fly_with(&mut store, &world, 5, bodies, &mut hulls);
    assert!(ricochets(&events).is_empty(), "{events:?}");
    let [(_, hit)] = impacts(&events)[..] else {
        panic!("{events:?}")
    };
    assert_eq!((hit.struck, hit.bounces), (Struck::Body(BodyId(2)), 0));
}

/// Directions onto each face of a hull at `yaw`, in its frame: an origin
/// offset from the centre and a heading into the face.
fn onto(face: &str) -> (V3, V3) {
    match face {
        "front" => (v3(20.0, 0.0, 1.2), v3(-1.0, 0.0, 0.0)),
        "rear" => (v3(-20.0, 0.0, 1.2), v3(1.0, 0.0, 0.0)),
        "side" => (v3(0.0, 20.0, 1.2), v3(0.0, -1.0, 0.0)),
        "roof" => (v3(0.0, 0.0, 20.0), v3(0.0, 0.0, -1.0)),
        _ => unreachable!(),
    }
}

fn to_world(local: V3, centre: V3, yaw: f64) -> V3 {
    let r = local.xy().rotated(yaw);
    v3(centre.x + r.x, centre.y + r.y, centre.z + local.z)
}

/// Fire `n` rounds of `row` squarely onto `face` of a hull turned 0.3 rad,
/// spread over the face; returns every event.
fn volley(face: &str, row: &str, n: usize, hulls: &mut TankHulls) -> Vec<(u64, FlightEvent)> {
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let yaw = 0.3;
    let hull = tank(1, 100.0, 100.0, yaw);
    let speed = weapon(row).speed_mps;
    let (origin, heading) = onto(face);
    let mut spread = Rng::new(3);
    let mut store = Projectiles::new(config());
    for _ in 0..n {
        // Across the face by up to ±1 m (half a metre on the side's height).
        let a = spread.unit() * 2.0 - 1.0;
        let b = spread.unit() * 2.0 - 1.0;
        let offset = match face {
            "front" | "rear" => v3(0.0, a, b * 0.5),
            "side" => v3(a, 0.0, b * 0.5),
            _ => v3(a, b, 0.0),
        };
        let o = to_world(origin + offset, hull.base, yaw);
        let d = to_world(heading, V3::default(), yaw);
        let id = store.launch(launch(o, d * speed, 0.0));
        hulls.register(id, row);
    }
    fly_with(&mut store, &world, 40, |k| vec![hull.body(k, dt)], hulls)
}

/// Each round's first event at the hull: its ricochet or its ending.
fn first_meeting(events: &[(u64, FlightEvent)]) -> BTreeMap<ProjectileId, FlightEvent> {
    let mut first = BTreeMap::new();
    for (_, e) in events {
        if matches!(e, FlightEvent::Ricochet(_) | FlightEvent::Impact(_)) {
            first.entry(e.projectile()).or_insert(*e);
        }
    }
    first
}

#[test]
fn each_face_glances_at_its_fixture_chance_within_the_scatter_cone() {
    let chances = hull("test_tank").armor.ricochet;
    let n = 3000;
    for (face, chance) in [
        ("front", chances.front),
        ("side", chances.side),
        ("rear", chances.rear),
        ("roof", chances.roof),
    ] {
        let mut hulls = TankHulls::new([1], 11);
        let rules = hulls.rules.clone();
        let events = volley(face, "hmg", n, &mut hulls);
        let first = first_meeting(&events);
        assert_eq!(first.len(), n, "{face}: every round meets the hull");
        let glanced: Vec<Ricochet> = first
            .values()
            .filter_map(|e| match e {
                FlightEvent::Ricochet(r) => Some(*r),
                _ => None,
            })
            .collect();
        for e in first.values() {
            if let FlightEvent::Impact(i) = e {
                assert_eq!(i.struck, Struck::Body(BodyId(1)), "{face}");
            }
        }
        // 3000 draws: one standard deviation is at most 0.009.
        let rate = glanced.len() as f64 / n as f64;
        assert!(
            (rate - chance).abs() < 0.04,
            "{face}: glanced {rate}, fixture {chance}"
        );
        let cone = rules.scatter_deg.to_radians();
        let mut widest: f64 = 0.0;
        for r in &glanced {
            let v = r.velocity;
            let mirror = (v - r.normal * (2.0 * v.dot(r.normal))).normalized();
            let off = r.deflected.normalized().dot(mirror).clamp(-1.0, 1.0).acos();
            assert!(off <= cone + 1e-9, "{face}: {off} beyond the cone");
            widest = widest.max(off);
            let kept = speed(r.deflected) / speed(r.velocity);
            assert!((kept - rules.speed_kept).abs() < 1e-9);
        }
        assert!(widest > cone * 0.9, "{face}: scatter fills the cone");
    }
}

#[test]
fn he_bursts_on_every_face_and_never_glances() {
    for face in ["front", "side", "rear", "roof"] {
        for row in ["tank_he", "grenade"] {
            let mut hulls = always_glancing([1]);
            let events = volley(face, row, 50, &mut hulls);
            assert!(ricochets(&events).is_empty(), "{face} {row}");
            let hits = impacts(&events);
            assert_eq!(hits.len(), 50, "{face} {row}");
            assert!(hits.iter().all(|(_, i)| i.detonated), "{face} {row}");
        }
        // The same hulls turn every kinetic round away.
        let mut hulls = always_glancing([1]);
        let first = first_meeting(&volley(face, "hmg", 50, &mut hulls));
        assert!(first
            .values()
            .all(|e| matches!(e, FlightEvent::Ricochet(_))));
    }
}

#[test]
fn a_unit_passed_before_and_after_a_glance_hears_one_near_miss() {
    // A round glances off A's front at (53.5, 100, 1.2); a soldier stands 2 m
    // out from the hit, 0.84 m off both the incoming and the outgoing path.
    let world = flat([200.0, 200.0], "");
    let dt = config().tick_s();
    let hull = tank(1, 50.0, 100.0, 0.0);
    let soldier = Mover::standing(20, 9, soldier_shape(), v3(55.5, 100.0, 0.0));
    let mut hulls = always_glancing([1]);
    let rifle = weapon("rifle");
    let mut store = Projectiles::new(config());
    let dir = v3(-6.5, 3.0, 0.0).normalized();
    let id = store.launch(launch(
        v3(60.0, 97.0, 1.2),
        dir * rifle.speed_mps,
        rifle.suppression_radius_m,
    ));
    hulls.register(id, "rifle");
    let events = fly_with(
        &mut store,
        &world,
        1,
        |k| vec![hull.body(k, dt), soldier.body(k, dt)],
        &mut hulls,
    );
    let [glance] = ricochets(&events)[..] else {
        panic!("{events:?}")
    };
    let misses: Vec<_> = events
        .iter()
        .filter_map(|(_, e)| match e {
            FlightEvent::NearMiss(m) => Some(*m),
            _ => None,
        })
        .collect();
    assert_eq!(misses.len(), 1, "one per unit per tick: {misses:?}");
    assert_eq!(misses[0].unit, UnitId(9), "never the hull it glanced off");
    // Both legs pass the soldier's axis at the same distance.
    let axis = v2(55.5, 100.0);
    let off = |from: V3, d: V3| {
        let (d, w) = (d.xy().normalized(), axis - from.xy());
        (w.x * d.y - w.y * d.x).abs()
    };
    let before = off(glance.point, dir);
    let after = off(glance.point, glance.deflected);
    assert!((before - after).abs() < 1e-9 && before < rifle.suppression_radius_m);
    let r = physics("soldier_radius_m");
    assert!((misses[0].distance - (before - r)).abs() < 1e-6);
}

#[test]
fn rifle_fire_through_a_tank_glances_off_it_and_replays() {
    // Blue scouts fire at ground beyond a red tank standing in the line,
    // side on and turned: their rounds fail its armour and some glance off.
    let map =
        json!({ "size": [600, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 })
            .to_string();
    let setup = scenario_with(
        &map,
        json!([
            { "side": "blue", "kind": "test_recon", "position": [180, 300], "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_tank", "position": [240, 300], "yaw": 2.0, "engagement": "return_fire_only" },
        ]),
        json!([]),
        json!([]),
    );
    let mut b = Battle::new(&setup, 3);
    order(
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
    let hp = b.unit(UnitId(1)).unwrap().hp;
    let mut digests = Vec::new();
    let (mut glanced, mut drawn, mut seen_by_red) = (0, 0, 0);
    let mut endings: BTreeMap<ProjectileId, u32> = BTreeMap::new();
    let mut bounced = Vec::new();
    for _ in 0..150 {
        b.step();
        digests.push(b.digest());
        for e in b.flight_events() {
            match e {
                FlightEvent::Ricochet(r) => {
                    glanced += 1;
                    bounced.push(r.projectile);
                    assert_eq!(r.body, BodyId(sim::weapons::VEHICLE_BODY_BASE + 1));
                }
                FlightEvent::Impact(i) => *endings.entry(i.projectile).or_default() += 1,
                FlightEvent::Expired(x) => *endings.entry(x.projectile).or_default() += 1,
                FlightEvent::NearMiss(_) | FlightEvent::Pass(_) => {}
            }
        }
        // Blue draws its own rounds whole: each ricochet is a corner of the
        // path, on the red tank's hull.
        let tank = b.unit(UnitId(1)).unwrap();
        let (half, base, yaw) = (tank.hull.unwrap(), tank.position, tank.yaw);
        for s in b.observe(Side::Blue).projectiles.iter().filter(|s| s.own) {
            for r in &s.ricochets {
                drawn += 1;
                assert!(r.point > 0 && r.point + 1 < s.path.len(), "an inner corner");
                let p = s.path[r.point];
                let local = (v2(p[0], p[1]) - base.xy()).rotated(-yaw);
                let up = (p[2] - base.z - half.z).abs();
                let q = [local.x.abs() - half.x, local.y.abs() - half.y, up - half.z];
                assert!(q.iter().all(|&e| e < 1e-6) && q.iter().any(|&e| e > -1e-6));
            }
        }
        // Red sees blue's rounds only over ground it sees, clipped leg by leg
        // with the glances kept as corners.
        let red = b.observe(Side::Red);
        for s in red.projectiles.iter().filter(|s| !s.own) {
            assert!(s.path.len() >= 2);
            assert!(s
                .path
                .iter()
                .all(|p| red.ground_visibility.visible(p[0], p[1])));
            assert!(s.ricochets.iter().all(|r| r.point < s.path.len()));
            seen_by_red += s.ricochets.len();
        }
    }
    assert!(seen_by_red > 0, "red watched rounds glance off its tank");
    assert!(
        glanced > 0 && drawn == glanced,
        "glanced {glanced}, drawn {drawn}"
    );
    // A ricochet is no ending: each glanced round ends exactly once later.
    for id in bounced.iter().filter(|id| endings.contains_key(id)) {
        assert_eq!(endings[id], 1);
    }
    assert!(endings.values().all(|&n| n == 1));
    assert_eq!(
        b.unit(UnitId(1)).unwrap().hp,
        hp,
        "rifle rounds never hurt it"
    );
    let mut replay = Battle::from_replay(&setup, &b.replay()).unwrap();
    for (t, expected) in digests.iter().enumerate() {
        replay.step();
        assert_eq!(
            replay.digest(),
            *expected,
            "first mismatch at tick {}",
            t + 1
        );
    }
}

/// A real hull bounce whose azimuth cosine differed on Native and Wasm;
/// the persistent battle and every impact input agreed before deflection.
#[test]
fn hull_scatter_retains_portable_velocity_bits_and_random_draws() {
    use contract::scenario::{Armor, RicochetRules};
    use sim::damage::{decide, StruckHull};
    use sim::flight::{ImpactContext, ImpactDecision, Pose};
    let armor = Armor {
        front: 140.0,
        side: 100.0,
        rear: 60.0,
        roof: 40.0,
        ricochet: FaceChances {
            front: 0.5,
            side: 0.35,
            rear: 0.2,
            roof: 0.6,
        },
    };
    let rules = RicochetRules {
        speed_kept: 0.6,
        scatter_deg: 12.0,
        penetration_kept: 0.5,
        max_bounces: 2,
    };
    let pose = Pose {
        base: v3(5079.0, 4999.0, 0.0),
        yaw: std::f64::consts::PI,
    };
    let hit = ImpactContext {
        projectile: ProjectileId(1108),
        bounces: 0,
        struck: Struck::Body(BodyId(16777366)),
        point: v3(5075.5, 4999.216335123485, 1.8114914644760762),
        normal: v3(-1.0, 1.2246467991473532e-16, 0.0),
        velocity: v3(794.2920370002005, -95.36647169291987, -2.629057549916237),
        pose: Some(pose),
    };
    let mut rng = Rng::new(0xe061_42c4_42ca_b136);
    let ImpactDecision::Bounce { velocity } = decide(
        RoundPower {
            penetration: 20.0,
            bursts: false,
        },
        Some(StruckHull {
            armor: &armor,
            half: v3(3.5, 1.8, 1.2),
            pose,
        }),
        &hit,
        &rules,
        &mut rng,
    ) else {
        panic!("the failed penetration must ricochet");
    };
    assert_eq!(
        [
            velocity.x.to_bits(),
            velocity.y.to_bits(),
            velocity.z.to_bits()
        ],
        [
            0xc07d_d117_8c14_5b0e,
            0x4010_5104_272a_ab02,
            0xc04a_68c4_2876_afa3
        ]
    );
    assert_eq!(
        rng.state(),
        0xbb07_aff0_c0aa_2575,
        "eligibility and two scatter draws"
    );
}
