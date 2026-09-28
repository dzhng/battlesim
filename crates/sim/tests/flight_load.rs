//! 200-unit-equivalent emitter load on the village map: 100 eight-soldier rifle
//! squads and 100 tanks, every one firing at its weapons' cycle rate through
//! the same launch path weapons use, with no projectile cap, and hulls judged
//! by the battle's armour policy (so rounds glance off them). Prints measured
//! per-tick cost; wall time is reported, never asserted. Run with
//! `cargo test -p sim --release --test sim flight_load:: -- --nocapture` for the numbers.
use crate::common;

use std::time::Instant;

use crate::common::*;
use contract::ballistics::WeaponBallistics;
use contract::map::MapDefinition;
use sim::flight::{
    advance_projectiles, prepare_launch, Aim, Body, FlightEvent, NoSolution, Projectiles, Shape,
    Shooter,
};
use sim::math::v3;
use sim::rng::Rng;
use sim::world::WorldGeometry;

struct Emitter {
    body: Mover,
    muzzle_z: f64,
    /// (row name, weapon, cycle seconds, next shot at).
    weapons: Vec<(&'static str, WeaponBallistics, f64, f64)>,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// `rate_scale` multiplies every weapon's fire rate (1 = authored cycles).
fn run(rate_scale: f64, seconds: f64) {
    let village = village();
    let map: MapDefinition = serde_json::from_value(village["map"].clone()).unwrap();
    let world = WorldGeometry::new(&map, &common::rules());
    let config = config();
    let dt = config.tick_s();
    let cycle = |name: &str| village["weapons"][name]["reload_s"].as_f64().unwrap() / rate_scale;
    let mut rng = Rng::new(20260925);
    let mut emitters = Vec::new();
    let mut id = 0;
    for side in 0..2 {
        let (x0, heading) = if side == 0 {
            (320.0, 1.0)
        } else {
            (980.0, -1.0)
        };
        for unit in 0..100u32 {
            let lane = 200.0 + (unit / 2) as f64 * 24.0;
            let base_x = x0 + (unit % 2) as f64 * 60.0;
            let unit_id = side * 100 + unit;
            let march = v3(heading * 1.2, 0.0, 0.0);
            if unit % 2 == 0 {
                for s in 0..8 {
                    let base = v3(
                        base_x + (s % 4) as f64 * 3.0,
                        lane + (s / 4) as f64 * 3.0,
                        0.0,
                    );
                    let body = Mover::standing(id, unit_id, soldier_shape(), base).moving(march);
                    id += 1;
                    emitters.push(Emitter {
                        body,
                        muzzle_z: physics("infantry_muzzle_m"),
                        weapons: vec![("rifle", weapon("rifle"), cycle("rifle"), rng.unit() * 0.5)],
                    });
                }
            } else {
                let body = Mover {
                    turn_rate: 0.15,
                    ..Mover::standing(id, unit_id, tank_shape(), v3(base_x, lane, 0.0))
                        .moving(march * 2.0)
                };
                id += 1;
                emitters.push(Emitter {
                    body,
                    muzzle_z: 2.0,
                    weapons: vec![
                        ("hmg", weapon("hmg"), cycle("hmg"), rng.unit() * 0.5),
                        (
                            "tank_he",
                            weapon("tank_he"),
                            cycle("tank_he"),
                            rng.unit() * 6.0,
                        ),
                    ],
                });
            }
        }
    }
    let (blue, red): (Vec<usize>, Vec<usize>) =
        (0..emitters.len()).partition(|&i| emitters[i].body.unit < 100);
    // Rounds launch at the start of the tick they first fly.
    let place = |b: &Body, z: f64| b.from.base + v3(0.0, 0.0, z);
    let mut store = Projectiles::new(config.clone());
    let tanks = emitters
        .iter()
        .filter(|e| matches!(e.body.shape, Shape::Box { .. }))
        .map(|e| e.body.id);
    let mut hulls = TankHulls::new(tanks, 20260925);
    let mut events = Vec::new();
    let (mut launched, mut refused, mut impacts, mut near, mut expired, mut glanced) =
        (0, 0, 0, 0, 0, 0);
    let (mut advance_ms, mut launch_ms) = (Vec::new(), Vec::new());
    let mut peak = 0;
    let ticks = (seconds / dt).round() as u64;
    for tick in 1..=ticks {
        let now = tick as f64 * dt;
        let bodies: Vec<Body> = emitters.iter().map(|e| e.body.body(tick, dt)).collect();
        let started = Instant::now();
        for (i, e) in emitters.iter_mut().enumerate() {
            // Sides are built in the same lane order: fire across at a nearby lane.
            let (foes, rank) = if e.body.unit < 100 {
                (&red, i)
            } else {
                (&blue, i - blue.len())
            };
            for (name, w, cycle_s, next) in &mut e.weapons {
                if *next > now {
                    continue;
                }
                *next += *cycle_s;
                let spread = (rng.next_u64() % 81) as usize;
                let foe = &bodies[foes[(rank + spread).saturating_sub(40).min(foes.len() - 1)]];
                let profile = config.profile(w).unwrap();
                let aim = Aim {
                    origin: place(&bodies[i], e.muzzle_z),
                    target: place(foe, 0.9),
                    target_velocity: (foe.to.base - foe.from.base) * (1.0 / dt),
                };
                let shooter = Some(Shooter {
                    unit: bodies[i].unit,
                    body: bodies[i].id,
                    cover: None,
                });
                match prepare_launch(
                    &world,
                    &config,
                    &profile,
                    &aim,
                    profile.scatter_mrad,
                    &mut rng,
                    shooter,
                ) {
                    Ok((launch, _)) => {
                        hulls.register(store.launch(launch), name);
                        launched += 1;
                    }
                    // Blocked by the ridge or trunks, or beyond reach.
                    Err(NoSolution::Blocked { .. } | NoSolution::OutOfReach) => refused += 1,
                }
            }
        }
        launch_ms.push(started.elapsed().as_secs_f64() * 1e3);
        peak = peak.max(store.active().len());
        events.clear();
        let started = Instant::now();
        advance_projectiles(&mut store, &world, &bodies, &mut events, &mut hulls);
        advance_ms.push(started.elapsed().as_secs_f64() * 1e3);
        for e in &events {
            match e {
                FlightEvent::Impact(_) => impacts += 1,
                FlightEvent::Ricochet(_) => glanced += 1,
                FlightEvent::NearMiss(_) => near += 1,
                FlightEvent::Pass(_) => {}
                FlightEvent::Expired(_) => expired += 1,
            }
        }
    }
    // No round is dropped, and a ricochet is not an ending: every launch is
    // still flying or ended in exactly one event.
    assert_eq!(launched, impacts + expired + store.active().len());
    let sorted = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v
    };
    let (adv, lau) = (sorted(advance_ms), sorted(launch_ms));
    eprintln!(
        "flight load x{rate_scale}: {} bodies, {:.0} rounds/s launched ({launched} in {seconds} s, \
         {refused} refused), peak {peak} in flight; advance ms p50 {:.2} p95 {:.2} p99 {:.2} max \
         {:.2}; launch solving ms/tick p50 {:.2} p95 {:.2}; {impacts} impacts, {near} near \
         misses, {expired} expired, {glanced} ricochets",
        emitters.len(),
        launched as f64 / seconds,
        percentile(&adv, 0.5),
        percentile(&adv, 0.95),
        percentile(&adv, 0.99),
        adv.last().unwrap(),
        percentile(&lau, 0.5),
        percentile(&lau, 0.95),
    );
}

#[test]
fn report_200_unit_emitter_load() {
    // Authored cycle rates, then bursts toward the 6k–8k rounds/s stress level.
    run(1.0, 6.0);
    run(2.0, 3.0);
    run(3.0, 2.0);
}
