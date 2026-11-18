//! Landing and packing read the published pose, including a first sighting on land.
use presentation::units::driver::PoseDriver;
use presentation::units::pose::VehiclePose;

mod common;
use common::*;

#[test]
fn a_landing_rotor_slows_from_touchdown_and_stands_still_once_set_up() {
    let units = landing_units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time, grounded, progress| -> VehiclePose {
        let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
        heli.kind = "test_supply_heli".into();
        heli.grounded = grounded;
        heli.deployment = Some(progress);
        heli.deployment_target = Some("deployed".into());
        driver
            .update(&frame(time, vec![heli], &none), &units, None)
            .vehicles[0]
            .clone()
    };
    let flying = at(0.0, false, 0.1);
    let touchdown = at(1.0, true, 0.2);
    let half = at(2.0, true, 0.6);
    let ready = at(3.0, true, 1.0);
    let still = at(4.0, true, 1.0);
    assert!(flying.airborne && !touchdown.airborne);
    assert_eq!(
        touchdown.articulation.rotor_speed,
        flying.articulation.rotor_speed
    );
    assert!(half.articulation.rotor_speed > ready.articulation.rotor_speed);
    assert!(half.articulation.rotor_speed < touchdown.articulation.rotor_speed);
    let fast_travel = touchdown.articulation.rotor - flying.articulation.rotor;
    let slow_travel = half.articulation.rotor - touchdown.articulation.rotor;
    assert!(slow_travel > 0.0 && slow_travel < fast_travel);
    assert_eq!(half.articulation.rotor, ready.articulation.rotor);
    assert_eq!(ready.articulation.rotor, still.articulation.rotor);
    assert_eq!(still.articulation.rotor_speed, 0.0);
}

fn landing_units() -> presentation::world::UnitsSection {
    units_with(|v| {
        let mut heli = v["types"]["capture_us_tank"].clone();
        heli["flies"] = true.into();
        heli["flight"] = "rotor".into();
        heli["deploy"] = serde_json::json!({"seconds": 10, "drop_seconds": 2, "pack_seconds": 5});
        v["types"]["test_supply_heli"] = heli;
    })
}

#[test]
fn a_packing_rotor_winds_up_before_it_climbs() {
    packing(false);
}

#[test]
fn a_heli_first_seen_on_the_ground_spins_up_when_it_packs() {
    packing(true);
}

fn packing(first_seen_grounded: bool) {
    let units = landing_units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    heli.kind = "test_supply_heli".into();
    let mut time = 0.0;
    if !first_seen_grounded {
        driver.update(&frame(time, vec![heli.clone()], &none), &units, None);
        time += 1.0;
        // Build a bank which must not survive the landing.
        heli.position[0] = 10.0;
        let flying = driver.update(&frame(time, vec![heli.clone()], &none), &units, None);
        assert!(flying.vehicles[0].motion.is_some());
        time += 1.0;
    }
    heli.grounded = true;
    heli.deployment_target = Some("packed".into());
    let mut poses = Vec::new();
    for progress in [1.0, 0.75, 0.5, 0.25] {
        heli.deployment = Some(progress);
        poses.push(
            driver
                .update(&frame(time, vec![heli.clone()], &none), &units, None)
                .vehicles[0]
                .clone(),
        );
        time += 1.0;
    }
    assert!(poses.iter().all(|p| !p.airborne && p.motion.is_none()));
    for pair in poses.windows(2) {
        assert!(pair[1].articulation.rotor_speed > pair[0].articulation.rotor_speed);
        assert!(pair[1].articulation.rotor > pair[0].articulation.rotor);
    }
    heli.grounded = false;
    heli.deployment = Some(0.0);
    heli.position[2] = 1.0;
    let lifted = driver
        .update(&frame(time, vec![heli], &none), &units, None)
        .vehicles[0]
        .clone();
    assert!(
        lifted.airborne && lifted.motion.is_none(),
        "takeoff starts with fresh flight attitude"
    );
    assert!(lifted.articulation.rotor_speed > poses.last().unwrap().articulation.rotor_speed);
    assert_eq!(lifted.articulation.gear_up, 0.0);
}

#[test]
fn an_enemy_landed_heli_s_rotor_follows_its_seen_pose() {
    use contract::ids::Side;
    use presentation::units::{feed::ObservationFeed, interpolate::TickInterpolator};
    use sim::publication::{self, reader::Reader, Publisher};
    use std::{path::PathBuf, sync::Arc};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = presentation::static_world::files::rules(&root, "supply").unwrap();
    let scenario = sim::maps::scenario("supply", "heli-fire", rules).unwrap();
    let mut battle = sim::battle::Battle::new(&scenario, 13);
    let kinds = presentation::fielded::encounter_kinds(&root, "supply", "heli-fire").unwrap();
    let f = presentation::fielded::fielded(&root, battle.rules(), &kinds).unwrap();
    let mut driver = PoseDriver::new(&f.units).unwrap();
    let mut feed = ObservationFeed::new("red");
    let mut interpolation = TickInterpolator::new(battle.rules().tick_hz as f64);
    let mut publisher = Publisher::new();
    let mut reader = Reader::new(&publication::layout_json(&battle)).unwrap();
    let mut slowing = Vec::new();
    for _ in 0..900 {
        battle.step();
        let words = publisher
            .publish(&battle, Side::Red)
            .unwrap()
            .iter()
            .map(|f| f.to_bits())
            .collect::<Vec<_>>();
        let observation = Arc::new(reader.decode(&words).unwrap().unwrap());
        let time = observation.tick / battle.rules().tick_hz as f64;
        interpolation.push(observation.clone(), time);
        let sample = interpolation.frame(time).unwrap();
        let frame = feed.frame(sample, &f.units, &f.effects);
        let poses = driver.update(frame, &f.units, None);
        let Some(known) = observation
            .identified
            .iter()
            .find(|u| &*u.kind == "test_supply_heli")
        else {
            continue;
        };
        let drawn = poses
            .vehicles
            .iter()
            .find(|p| p.unit == known.id && &*p.side == "blue")
            .unwrap();
        assert_eq!(drawn.airborne, !known.grounded);
        if known.grounded {
            slowing.push(drawn.articulation.rotor_speed);
            if known.deployment.as_ref().unwrap().progress == 1.0 {
                assert_eq!(drawn.articulation.rotor_speed, 0.0);
                break;
            }
        }
    }
    assert!(slowing.len() > 2, "the enemy watches the wind-down");
    assert!(slowing.windows(2).all(|p| p[1] <= p[0]));
    assert!(slowing[0] > *slowing.last().unwrap());
}

#[test]
fn an_ungrounded_rotor_is_full_while_its_last_packing_progress_blends_out() {
    let units = landing_units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    heli.kind = "test_supply_heli".into();
    heli.deployment_target = Some("packed".into());
    heli.deployment = Some(0.0);
    let full = driver
        .update(&frame(0.0, vec![heli.clone()], &none), &units, None)
        .vehicles[0]
        .articulation
        .rotor_speed;
    heli.grounded = true;
    heli.deployment = Some(0.25);
    let packing = driver
        .update(&frame(1.0, vec![heli.clone()], &none), &units, None)
        .vehicles[0]
        .articulation
        .rotor_speed;
    assert!(packing < full);
    // The latest landing latch has cleared; progress still blends from the prior tick.
    heli.grounded = false;
    heli.deployment = Some(0.1);
    let lifted = driver
        .update(&frame(2.0, vec![heli], &none), &units, None)
        .vehicles[0]
        .clone();
    assert!(lifted.airborne);
    assert_eq!(lifted.articulation.rotor_speed, full);
}

#[test]
fn reversing_a_grounded_rotor_preserves_its_speed_at_every_progress() {
    let units = landing_units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    heli.kind = "test_supply_heli".into();
    heli.grounded = true;
    for step in 0..=1000 {
        heli.deployment = Some(f64::from(step) / 1000.0);
        heli.deployment_target = Some("deployed".into());
        let before = driver
            .update(
                &frame(f64::from(step), vec![heli.clone()], &none),
                &units,
                None,
            )
            .vehicles[0]
            .articulation
            .rotor_speed;
        heli.deployment_target = Some("packed".into());
        let after = driver
            .update(
                &frame(f64::from(step), vec![heli.clone()], &none),
                &units,
                None,
            )
            .vehicles[0]
            .articulation
            .rotor_speed;
        assert_eq!(after, before, "reversal at progress {:?}", heli.deployment);
    }
}
