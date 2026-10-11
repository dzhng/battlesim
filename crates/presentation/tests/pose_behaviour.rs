//! What a player sees the pose driver do, beyond the browser's vectors: each
//! soldier moves and faces by his own motion, a squad under fire is never a
//! row of copies, the dying fall from the posture they are in and lie as
//! they fell, a lean slides out and back, a tank's turret, guns and tracks
//! follow what carries them, rotors turn, a paused clock holds, and the
//! corpse cap gives way oldest first. Driven by synthetic feed frames over
//! the fixture's real appearances and pose feel.
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use presentation::units::driver::{
    FeedFallen, FeedFrame, FeedMount, FeedSoldier, FeedUnit, PoseDriver,
};
use presentation::units::pose::{PoseFrame, SoldierPose};
use presentation::world::UnitsSection;
use serde_json::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../native/godot/tests/fixtures/poses/paris-corner-units.json"
);

/// The fixture's units section, edited by `edit`.
fn units_with(edit: impl FnOnce(&mut Value)) -> UnitsSection {
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    edit(&mut v);
    serde_json::from_value(v).unwrap()
}

fn units() -> UnitsSection {
    units_with(|_| {})
}

fn feel(units: &UnitsSection, path: &str) -> f64 {
    units.pose.pointer(path).and_then(Value::as_f64).unwrap()
}

/// A rifleman at (x, y) with his rifle up, `shots` rounds fired.
fn man(id: f64, x: f64, y: f64, shots: f64) -> FeedSoldier {
    FeedSoldier {
        id,
        active_mount: Some(0),
        position: [x, y, 0.0],
        shots,
        ..FeedSoldier::default()
    }
}

fn squad(soldiers: Vec<FeedSoldier>, pinned: bool) -> FeedUnit {
    FeedUnit {
        id: 1.0,
        kind: "menu_rifle".into(),
        side: "blue".into(),
        soldiers,
        mounts: vec![FeedMount::default(); 2],
        pinned,
        ..FeedUnit::default()
    }
}

fn frame(time: f64, units: Vec<FeedUnit>, fallen: &Arc<Vec<FeedFallen>>) -> FeedFrame {
    FeedFrame {
        time,
        units,
        fallen: fallen.clone(),
    }
}

fn fallen(ids: &[f64]) -> Arc<Vec<FeedFallen>> {
    Arc::new(
        ids.iter()
            .map(|&id| FeedFallen {
                soldier: id,
                position: [id, 0.0, 0.0],
                yaw: 0.0,
                kind: "menu_rifle".into(),
                slot: 0,
                side: "blue".into(),
            })
            .collect(),
    )
}

fn clip_of(poses: &PoseFrame, id: f64) -> String {
    pose_of(poses, id).clip.to_string()
}

fn pose_of(poses: &PoseFrame, id: f64) -> &SoldierPose {
    poses.soldiers.iter().find(|s| s.soldier == id).unwrap()
}

fn same_angle(a: f64, b: f64) -> bool {
    ((a - b + PI).rem_euclid(2.0 * PI) - PI).abs() < 1e-6
}

#[test]
fn each_soldier_walks_runs_or_stands_by_his_own_motion_and_faces_his_own_way() {
    let units = units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let at = |time: f64, places: [(f64, f64); 3], d: &mut PoseDriver| {
        let men = (0..3).map(|i| man(i as f64 + 1.0, places[i].0, places[i].1, 0.0));
        let mut unit = squad(men.collect(), false);
        unit.yaw = PI; // the squad's heading is nobody's way
        format!("{:?}", d.update(&frame(time, vec![unit], &none), &units))
    };
    at(0.0, [(0.0, 0.0), (0.0, 5.0), (0.0, 9.0)], &mut driver);
    // Over a second: the first walks east, the second runs north, the third stands.
    let moved = [(1.2, 0.0), (0.0, 8.0), (0.0, 9.0)];
    let seen = at(1.0, moved, &mut driver);
    let poses = driver.poses();
    assert_eq!(clip_of(poses, 1.0), "walk");
    assert_eq!(clip_of(poses, 2.0), "run");
    assert_eq!(clip_of(poses, 3.0), "idle");
    assert!(same_angle(pose_of(poses, 1.0).facing, 0.0), "he faces east");
    assert!(
        same_angle(pose_of(poses, 2.0).facing, FRAC_PI_2),
        "he faces north"
    );
    // The same men always move the same way: replays are stable.
    let mut replay = PoseDriver::new(&units).unwrap();
    at(0.0, [(0.0, 0.0), (0.0, 5.0), (0.0, 9.0)], &mut replay);
    assert_eq!(at(1.0, moved, &mut replay), seen);
}

/// Under fire every man takes his own stance, pinned nobody stands, a man
/// who fires lying down stays down, and each falls from where he is.
#[test]
fn a_squad_under_fire_is_never_a_row_of_copies_and_each_man_falls_from_his_posture() {
    let units = units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let ids: Vec<f64> = (1..=8).map(f64::from).collect();
    let mut at = |time: f64, shots: f64, pinned: bool| {
        let men = ids
            .iter()
            .map(|&id| man(id, id * 2.0, 0.0, shots))
            .collect();
        let poses = driver.update(&frame(time, vec![squad(men, pinned)], &none), &units);
        ids.iter().map(|&id| clip_of(poses, id)).collect::<Vec<_>>()
    };
    at(0.0, 0.0, false);
    let firing = at(0.5, 1.0, false);
    let set = |clips: &[String]| clips.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        set(&firing),
        set(&[
            "stand_aim".into(),
            "kneel_fire".into(),
            "prone_pinned".into()
        ]),
        "firing, some stand, some kneel, some lie: {firing:?}"
    );
    // Long after their shots, pinned: some hug the ground, some crouch.
    let pinned = at(3.0, 1.0, true);
    assert_eq!(
        set(&pinned),
        set(&["kneel_fire".into(), "prone_pinned".into()]),
        "pinned, nobody stands: {pinned:?}"
    );
    for (f, p) in firing.iter().zip(&pinned) {
        if f == "prone_pinned" {
            assert_eq!(p, "prone_pinned", "a man who fires lying down stays down");
        }
    }

    // All fall at once: kneeling he slumps, lying he goes slack.
    let down = fallen(&ids);
    let fell = driver.update(&frame(3.1, vec![squad(vec![], true)], &down), &units);
    for (id, was) in ids.iter().zip(&pinned) {
        let fall = if was == "prone_pinned" {
            "death_prone"
        } else {
            "death_kneel"
        };
        assert_eq!(clip_of(fell, *id), fall, "soldier {id} falls from {was}");
    }
    assert!(
        fell.corpses.is_empty(),
        "nobody lies before his fall has played"
    );
    // Every fall played out: each lies static, and none is posed.
    let lain = driver.update(&frame(5.0, vec![squad(vec![], true)], &down), &units);
    assert!(lain.soldiers.is_empty());
    let lying: BTreeSet<_> = lain.corpses.iter().map(|c| c.soldier as u32).collect();
    assert_eq!(lying, (1..=8).collect());
}

#[test]
fn killed_standing_some_fall_back_and_some_pitch_forward_each_lying_as_he_fell() {
    let units = units();
    let mut driver = PoseDriver::new(&units).unwrap();
    let ids: Vec<f64> = (1..=20).map(f64::from).collect();
    let men = ids.iter().map(|&id| man(id, id * 2.0, 0.0, 0.0)).collect();
    driver.update(&frame(0.0, vec![squad(men, false)], &fallen(&[])), &units);
    let down = fallen(&ids);
    let falls = driver.update(&frame(1.0, vec![squad(vec![], false)], &down), &units);
    let falls: Vec<(f64, String, f64)> = ids
        .iter()
        .map(|&id| (id, clip_of(falls, id), pose_of(falls, id).facing))
        .collect();
    let kinds: BTreeSet<_> = falls.iter().map(|f| f.1.as_str()).collect();
    assert_eq!(kinds, BTreeSet::from(["death", "death_back"]));
    let lain = driver.update(&frame(4.0, vec![squad(vec![], false)], &down), &units);
    for (id, fall, facing) in &falls {
        let corpse = lain.corpses.iter().find(|c| c.soldier == *id).unwrap();
        assert_eq!(
            corpse.lies,
            usize::from(fall == "death_back"),
            "{id} lies as he fell"
        );
        assert_eq!(corpse.yaw, *facing, "{id} lies facing as he fell");
    }
}

#[test]
fn lying_down_he_crawls_whichever_way_he_goes_facing_the_threat_and_runs_when_fast() {
    let units = units();
    let none = fallen(&[]);
    // Soldier 1 hugs the ground when pinned. He has seen his squad fire at
    // bearing 0, then moves for half a second at `speed` toward `heading`.
    let mover = |heading: f64, speed: f64| {
        let mut driver = PoseDriver::new(&units).unwrap();
        let mut at = |time: f64, x: f64, y: f64, shots: f64| {
            let mut unit = squad(vec![man(1.0, x, y, 0.0)], true);
            unit.mounts[0].shots = shots;
            pose_of(driver.update(&frame(time, vec![unit], &none), &units), 1.0).clone()
        };
        at(0.0, 0.0, 0.0, 0.0);
        assert_eq!(&*at(0.5, 0.0, 0.0, 3.0).clip, "prone_pinned");
        let step = speed * 0.5;
        at(1.0, heading.cos() * step, heading.sin() * step, 3.0)
    };
    for (way, heading) in [("f", 0.0), ("l", FRAC_PI_2), ("b", PI), ("r", -FRAC_PI_2)] {
        let s = mover(heading, 0.6);
        assert_eq!(*s.clip, format!("crawl_{way}"));
        assert!(
            same_angle(s.facing, 0.0),
            "crawling {way}, he still faces the threat"
        );
    }
    let crawl = feel(&units, "/gait/crawl_mps");
    assert_eq!(
        &*mover(PI, crawl + 0.2).clip,
        "crouch_b",
        "too fast to crawl"
    );
    let ran = mover(FRAC_PI_2, feel(&units, "/gait/run_mps") + 0.5);
    assert_eq!(&*ran.clip, "run");
    assert!(ran.facing > 0.0, "at a run he turns toward his way");
}

#[test]
fn out_on_his_lean_he_kneels_and_slides_out_then_eases_back_never_walking() {
    let units = units();
    let none = fallen(&[]);
    let (out_s, back_s) = (feel(&units, "/lean/out_s"), feel(&units, "/lean/back_s"));
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, lean: Option<[f64; 2]>| {
        let mut soldier = man(1.0, 0.0, 0.0, 0.0);
        soldier.lean = lean;
        let poses = driver.update(
            &frame(time, vec![squad(vec![soldier], true)], &none),
            &units,
        );
        let s = pose_of(poses, 1.0);
        assert!(
            !["walk", "run"].contains(&&*s.clip),
            "a lean is never a walk"
        );
        (s.clip.to_string(), s.position[0])
    };
    at(0.0, None);
    let (tucked, _) = at(1.0, None);
    assert_eq!(tucked, "prone_pinned", "pinned, he hugs his cover");
    let (clip, x) = at(1.1, Some([0.8, 0.0]));
    assert_eq!(
        clip, "kneel_fire",
        "out on his lean he kneels to fire, pinned or not"
    );
    assert!(0.0 < x && x < 0.8, "partway out at first: {x}");
    assert!((at(1.1 + out_s, Some([0.8, 0.0])).1 - 0.8).abs() < 1e-9);
    let (clip, x) = at(1.5, None);
    assert_eq!(clip, "prone_pinned");
    assert!(0.0 < x && x < 0.8, "eased back, not snapped: {x}");
    assert!(at(1.5 + back_s, None).1.abs() < 1e-9);
}

/// The fixture's tank: hull yaw, cannon (bearing, elevation, shots), HMG bearing.
fn tank(x: f64, yaw: f64, gun: (f64, f64, f64), hmg: f64) -> FeedUnit {
    FeedUnit {
        id: 5.0,
        kind: "menu_us_tank".into(),
        side: "blue".into(),
        position: [x, 0.0, 0.0],
        yaw,
        mounts: vec![
            FeedMount {
                bearing: gun.0,
                elevation: gun.1,
                shots: gun.2,
                ..FeedMount::default()
            },
            FeedMount {
                bearing: hmg,
                elevation: 0.2,
                ..FeedMount::default()
            },
        ],
        ..FeedUnit::default()
    }
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
        let poses = driver.update(&frame(time, vec![tank(0.0, 0.5, gun, 1.0)], &none), &units);
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
        at(0.5 + return_s, (1.5, 0.0, 6.0)).recoil,
        0.0,
        "back in battery"
    );
}

#[test]
fn driving_rolls_both_tracks_and_turning_in_place_counter_rotates_them() {
    let units = units();
    let none = fallen(&[]);
    let half = units.types["menu_us_tank"].hull.unwrap().half_track;
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, x: f64, yaw: f64| {
        let poses = driver.update(
            &frame(time, vec![tank(x, yaw, (0.0, 0.0, 0.0), 0.0)], &none),
            &units,
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
        let mut heli = v["types"]["menu_us_tank"].clone();
        heli["airborne"] = true.into();
        v["types"]["menu_heli"] = heli;
    });
    let none = fallen(&[]);
    let tip = feel(&units, "/rotor/tip_mps");
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, x: f64| {
        let mut heli = tank(x, 0.0, (0.0, 0.0, 0.0), 0.0);
        heli.kind = "menu_heli".into();
        heli.position[2] = 20.0;
        driver
            .update(&frame(time, vec![heli], &none), &units)
            .vehicles[0]
            .clone()
    };
    at(0.0, 0.0);
    let hover = at(0.5, 0.0);
    assert!(
        hover.airborne && hover.tilt.is_none(),
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
    assert!(flown.tilt.is_some(), "speeding up, it leans into it");
}

#[test]
fn a_paused_clock_holds_every_pose_and_an_earlier_time_is_a_new_battle() {
    let units = units();
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    let walkers = |y: f64| squad(vec![man(1.0, 0.0, y, 0.0), man(2.0, 3.0, y, 1.0)], false);
    driver.update(&frame(0.0, vec![walkers(0.0)], &none), &units);
    let walked = format!(
        "{:?}",
        driver.update(&frame(0.4, vec![walkers(0.5)], &none), &units)
    );
    // Paused: the same moment again, rendered frame after frame.
    for _ in 0..3 {
        let held = format!(
            "{:?}",
            driver.update(&frame(0.4, vec![walkers(0.5)], &none), &units)
        );
        assert_eq!(held, walked);
    }
    // Time running backwards starts over: posed as first seen.
    let mut fresh = PoseDriver::new(&units).unwrap();
    let first = format!(
        "{:?}",
        fresh.update(&frame(0.0, vec![walkers(0.0)], &none), &units)
    );
    let restarted = format!(
        "{:?}",
        driver.update(&frame(0.0, vec![walkers(0.0)], &none), &units)
    );
    assert_eq!(
        restarted.replace("corpses_version: 2", "corpses_version: 0"),
        first
    );
}

#[test]
fn past_the_cap_a_battle_joined_lies_its_newest_and_a_reset_forgets_the_gone() {
    let units = units_with(|v| v["pose"]["corpses"]["max"] = 3.into());
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, ids: &[f64]| {
        let poses = driver.update(&frame(time, vec![], &fallen(ids)), &units);
        let lying: Vec<f64> = poses.corpses.iter().map(|c| c.soldier).collect();
        let fading: Vec<f64> = poses.fading.iter().map(|f| f.corpse.soldier).collect();
        (lying, fading)
    };
    // Joined with six already down: the newest three lie, and none it never drew fades.
    let none: Vec<f64> = vec![];
    let all = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    assert_eq!(at(0.0, &all), (vec![4.0, 5.0, 6.0], none.clone()));
    // A seventh falls: the oldest drawn sinks away, and stays gone while listed.
    let more = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    assert_eq!(at(1.0, &more), (vec![5.0, 6.0, 7.0], vec![4.0]));
    assert_eq!(at(10.0, &more), (vec![5.0, 6.0, 7.0], none.clone()));
    // An earlier time is a new battle: the gone may lie again.
    assert_eq!(at(0.0, &[4.0]), (vec![4.0], none));
}
