//! What a player sees the pose driver do with vehicles: a tank's turret,
//! guns and tracks follow what carries them, an aircraft leans into its
//! flight and its rotors turn, and a shot rocks the hull that fires it on
//! its springs, the drawn muzzle riding it. Driven by synthetic feed frames
//! over the fixture's real appearances and pose feel.
use std::f64::consts::FRAC_PI_2;
use std::sync::Arc;

use presentation::effects::MuzzleSource;
use presentation::seed::side_key;
use presentation::units::drawn::{ClipTables, DrawnModels};
use presentation::units::driver::PoseDriver;
use presentation::units::skin::Mat4;
use presentation::world::{UnitsSection, Vec3};
use serde_json::json;

mod common;
use common::*;

#[test]
fn a_diving_jet_pitches_nose_down() {
    let units = units_with(|v| {
        let mut jet = v["types"]["capture_us_tank"].clone();
        jet["airborne"] = true.into();
        jet["flight"] = "fixed_wing".into();
        v["types"]["test_jet"] = jet;
        v["pose"]["attitude"]["fixed_wing"] = json!({
            "gain": 1,
            "max_deg": 35,
            "smoothing_s": 0.01
        });
    });
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut jet = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    jet.kind = "test_jet".into();
    jet.position[2] = 100.0;
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units, None);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(
            &frame(f64::from(time), vec![jet.clone()], &none),
            &units,
            None,
        );
    }
    jet.position = [900.0, 0.0, 90.0];
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units, None)
        .vehicles[0]
        .motion
        .expect("a diving jet tilts");
    assert!(
        motion[2] < -0.05,
        "nose points down from its flight path: {motion:?}"
    );
}

#[test]
fn a_climbing_jet_pitches_nose_up() {
    let units = units_with(|v| {
        let mut jet = v["types"]["capture_us_tank"].clone();
        jet["airborne"] = true.into();
        jet["flight"] = "fixed_wing".into();
        v["types"]["test_jet"] = jet;
        v["pose"]["attitude"]["fixed_wing"] = json!({
            "gain": 1,
            "max_deg": 35,
            "smoothing_s": 0.01
        });
    });
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut jet = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    jet.kind = "test_jet".into();
    jet.position[2] = 100.0;
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units, None);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(
            &frame(f64::from(time), vec![jet.clone()], &none),
            &units,
            None,
        );
    }
    jet.position = [900.0, 0.0, 110.0];
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units, None)
        .vehicles[0]
        .motion
        .expect("a climbing jet tilts");
    assert!(
        motion[2] > 0.05,
        "nose points up from its flight path: {motion:?}"
    );
}

#[test]
fn a_turning_jet_banks_into_the_turn() {
    let units = units_with(|v| {
        let mut jet = v["types"]["capture_us_tank"].clone();
        jet["airborne"] = true.into();
        jet["flight"] = "fixed_wing".into();
        v["types"]["test_jet"] = jet;
        v["pose"]["attitude"]["fixed_wing"] = json!({
            "gain": 1,
            "max_deg": 35,
            "smoothing_s": 0.01
        });
    });
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut jet = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    jet.kind = "test_jet".into();
    jet.position[2] = 100.0;
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units, None);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(
            &frame(f64::from(time), vec![jet.clone()], &none),
            &units,
            None,
        );
    }
    jet.position[0] = 900.0;
    jet.yaw = 0.4;
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units, None)
        .vehicles[0]
        .motion
        .expect("a turning jet banks");
    assert!(motion[6] < -0.05, "left turn banks left: {motion:?}");
}

#[test]
fn a_helicopters_attitude_is_unchanged() {
    let units = units_with(|v| {
        let mut heli = v["types"]["capture_us_tank"].clone();
        heli["airborne"] = true.into();
        v["types"]["test_heli"] = heli;
    });
    let none = fallen(&[]);
    let max = feel(&units, "/attitude/rotor/max_deg").to_radians();
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    heli.kind = "test_heli".into();
    heli.position[2] = 20.0;
    driver.update(&frame(0.0, vec![heli.clone()], &none), &units, None);
    heli.position[0] = 10.0;
    let motion = driver
        .update(&frame(1.0, vec![heli], &none), &units, None)
        .vehicles[0]
        .motion
        .expect("an accelerating helicopter leans");
    assert!(
        (motion[2] + max.sin()).abs() < 1e-9,
        "helicopters retain acceleration lean: {motion:?}"
    );
}

#[test]
fn a_tank_s_turret_slews_to_its_bearing_its_gun_eases_and_recoils_on_a_shot() {
    let units = units();
    let none = fallen(&[]);
    let recoil_m = feel(&units, "/mount/recoil_m");
    let return_s = feel(&units, "/mount/recoil_return_s");
    let rate = feel(&units, "/mount/gun_elevation_rad_s");
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, gun: (f64, f64, f64)| {
        let poses = driver.update(
            &frame(time, vec![tank(0.0, 0.5, gun, 1.0)], &none),
            &units,
            None,
        );
        poses.vehicles[0].articulation
    };
    // First seen with rounds counted: posed as published, nothing to recoil from.
    let a = at(0.0, (1.5, 0.3, 5.0));
    assert!(
        (a.turret_yaw - 1.0).abs() < 1e-12,
        "turret relative to the hull"
    );
    assert!(
        (a.hmg_yaw + 0.5).abs() < 1e-12,
        "the HMG relative to the turret it rides"
    );
    assert_eq!((a.gun_pitch, a.recoil), (0.3, 0.0));
    // A new elevation is eased toward, not snapped to; a shot runs the gun back.
    let a = at(0.5, (1.5, 0.0, 6.0));
    assert!((a.gun_pitch - (0.3 - rate * 0.5).max(0.0)).abs() < 1e-12);
    assert_eq!(a.recoil, recoil_m, "all the way back at once");
    let a = at(0.5 + return_s / 2.0, (1.5, 0.0, 6.0));
    assert!(
        0.0 < a.recoil && a.recoil < recoil_m,
        "returning: {}",
        a.recoil
    );
    assert_eq!(
        at(0.51 + return_s, (1.5, 0.0, 6.0)).recoil,
        0.0,
        "back in battery"
    );
}

#[test]
fn driving_rolls_both_tracks_and_turning_in_place_counter_rotates_them() {
    let units = units();
    let none = fallen(&[]);
    let half = units.types["capture_us_tank"].hull.unwrap().half_track;
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, x: f64, yaw: f64| {
        let poses = driver.update(
            &frame(time, vec![tank(x, yaw, (0.0, 0.0, 0.0), 0.0)], &none),
            &units,
            None,
        );
        let a = poses.vehicles[0].articulation;
        (a.travel_l, a.travel_r, a.rotor)
    };
    at(0.0, 0.0, 0.0);
    assert_eq!(
        at(1.0, 4.0, 0.0),
        (4.0, 4.0, 0.0),
        "both tracks roll; no rotor turns"
    );
    let (l, r, _) = at(2.0, 4.0, 0.2);
    assert!((l - (4.0 - 0.2 * half)).abs() < 1e-12 && (r - (4.0 + 0.2 * half)).abs() < 1e-12);
}

#[test]
fn an_aircraft_s_rotors_turn_and_blur_while_nothing_it_has_rolls() {
    let units = units_with(|v| {
        let mut heli = v["types"]["capture_us_tank"].clone();
        heli["airborne"] = true.into();
        v["types"]["capture_heli"] = heli;
    });
    let none = fallen(&[]);
    let tip = feel(&units, "/rotor/tip_mps");
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, x: f64| {
        let mut heli = tank(x, 0.0, (0.0, 0.0, 0.0), 0.0);
        heli.kind = "capture_heli".into();
        heli.position[2] = 20.0;
        driver
            .update(&frame(time, vec![heli], &none), &units, None)
            .vehicles[0]
            .clone()
    };
    at(0.0, 0.0);
    let hover = at(0.5, 0.0);
    assert!(
        hover.airborne && hover.motion.is_none(),
        "hovering, it sits level"
    );
    assert_eq!(hover.articulation.rotor, 0.5 * tip);
    assert_eq!(
        hover.articulation.rotor_blur, 1.0,
        "drawn as a shutter sees it"
    );
    let flown = at(1.5, 60.0);
    assert_eq!(flown.articulation.rotor, 1.5 * tip);
    assert_eq!(
        (flown.articulation.travel_l, flown.articulation.travel_r),
        (0.0, 0.0)
    );
    assert!(flown.motion.is_some(), "speeding up, it leans into it");
}

/// The fixture's units with a hull kick of the test's own: a main-gun round
/// kicks fully, an HMG's not at all.
fn kicking_units() -> UnitsSection {
    units_with(|v| {
        v["pose"]["hull_kick"] = json!({"rounds": {"default": 0, "tank_ap": 1},
            "rock_deg": 4, "slide_m": 0.2, "hz": 2, "damping": 0.3});
    })
}

/// The tank's drawn motion at each of `seconds` at `hz` frames a second,
/// firing its gun `gun` times along `bearing` (and its HMG `hmg` times) at 1 s.
fn kicked(bearing: f64, seconds: f64, hz: f64, gun: f64, hmg: f64) -> Vec<(f64, Option<Mat4>)> {
    let units = kicking_units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    (0..=(seconds * hz) as u32)
        .map(|k| {
            let time = f64::from(k) / hz;
            let fired = if time >= 1.0 { 1.0 } else { 0.0 };
            let mut unit = tank(0.0, 0.0, (bearing, 0.0, gun * fired), bearing);
            unit.mounts[1].shots = hmg * fired;
            let poses = driver.update(&frame(time, vec![unit], &none), &units, None);
            (time, poses.vehicles[0].motion)
        })
        .collect()
}

/// Where the hull's point `p` (its own frame) is drawn under `motion`.
fn moved(motion: Option<Mat4>, p: Vec3) -> Vec3 {
    motion.map_or(p, |m| {
        std::array::from_fn(|i| m[i] * p[0] + m[4 + i] * p[1] + m[8 + i] * p[2] + m[12 + i])
    })
}

#[test]
fn a_main_gun_shot_rocks_the_hull_away_from_its_target_on_its_springs_then_it_settles() {
    let frames = kicked(0.0, 6.0, 60.0, 1.0, 0.0);
    assert!(
        frames.iter().filter(|f| f.0 < 1.0).all(|f| f.1.is_none()),
        "still until it fires"
    );
    let roof = [0.0, 0.0, 2.0];
    let back: Vec<f64> = frames.iter().map(|f| -moved(f.1, roof)[0]).collect();
    let peak = (0..back.len())
        .max_by(|&a, &b| back[a].total_cmp(&back[b]))
        .unwrap();
    // Fired forward, the roof is thrown back and the nose lifts at once…
    assert!(frames[peak].0 - 1.0 < 0.25);
    assert!(moved(frames[peak].1, [4.0, 0.0, 0.0])[2] > 0.0);
    // …at most the full kick's rock (and slide) away from rest.
    let full = 2.0 * 4f64.to_radians().sin() + 0.2;
    assert!(
        back[peak] > 0.8 * full && back[peak] <= full + 1e-6,
        "{}",
        back[peak]
    );
    // The springs throw it back past rest, less far, before it settles.
    let past = back[peak..].iter().copied().fold(f64::INFINITY, f64::min);
    assert!(past < -0.1 * back[peak] && -past < back[peak], "{past}");
    assert!(frames.last().unwrap().1.is_none(), "settled, at rest");
    for f in &frames {
        assert!(moved(f.1, roof)[1].abs() < 1e-9, "no sideslip firing ahead");
    }
}

#[test]
fn a_shot_over_the_side_rocks_the_hull_away_from_that_side() {
    // Fired to the left (+Y): the roof is thrown to the right.
    let frames = kicked(FRAC_PI_2, 2.0, 60.0, 1.0, 0.0);
    let right: Vec<f64> = frames
        .iter()
        .map(|f| -moved(f.1, [0.0, 0.0, 2.0])[1])
        .collect();
    let widest = right
        .iter()
        .copied()
        .fold(0.0, |w: f64, r| if r.abs() > w.abs() { r } else { w });
    assert!(widest > 0.1, "{widest}");
    for f in &frames {
        assert!(moved(f.1, [0.0, 0.0, 2.0])[0].abs() < 1e-9);
    }
}

#[test]
fn a_round_with_no_kick_never_moves_the_hull_however_many_fly() {
    let frames = kicked(0.0, 3.0, 60.0, 0.0, 40.0);
    assert!(frames.iter().all(|f| f.1.is_none()));
}

#[test]
fn the_hull_rocks_the_same_however_often_it_is_drawn() {
    let fine = kicked(0.0, 3.0, 120.0, 1.0, 0.0);
    for (time, motion) in kicked(0.0, 3.0, 8.0, 1.0, 0.0) {
        let f = fine.iter().find(|f| (f.0 - time).abs() < 1e-9).unwrap();
        let (a, b) = (moved(motion, [0.0, 0.0, 2.0]), moved(f.1, [0.0, 0.0, 2.0]));
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-9, "at {time}: {a:?} vs {b:?}");
        }
    }
}

#[test]
fn the_drawn_muzzle_rides_the_rocking_hull_so_the_flash_stays_on_the_barrel() {
    let units = kicking_units();
    let motion = kicked(0.0, 1.5, 60.0, 1.0, 0.0)
        .into_iter()
        .find(|f| f.0 >= 1.15)
        .and_then(|f| f.1)
        .expect("rocking");
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut poses = driver
        .update(
            &frame(0.0, vec![tank(10.0, 0.0, (0.0, 0.0, 0.0), 0.0)], &none),
            &units,
            None,
        )
        .clone();
    let mut drawn = DrawnModels::new(Arc::new(units), Arc::new(ClipTables::new()), "blue");
    let mut muzzle = |m: Option<Mat4>| {
        poses.vehicles[0].motion = m;
        drawn.update(&poses);
        drawn.muzzle(side_key(5.0, true), 0, None).expect("drawn")
    };
    let still = muzzle(None);
    let rocking = muzzle(Some(motion));
    // The hull's motion carries the muzzle about the hull's foot.
    let local = moved(Some(motion), [still[0] - 10.0, still[1], still[2]]);
    let expected = [local[0] + 10.0, local[1], local[2]];
    for i in 0..3 {
        assert!(
            (rocking[i] - expected[i]).abs() < 1e-6,
            "{rocking:?} vs {expected:?}"
        );
    }
    assert!(
        rocking[2] > still[2] + 0.05,
        "the muzzle lifts with the nose"
    );
}
