//! What a player sees the pose driver do, beyond the browser's vectors: each
//! soldier moves and faces by his own motion, a squad under fire is never a
//! row of copies, the dying fall from the posture they are in and lie as
//! they fell, a lean slides out and back, a tank's turret, guns and tracks
//! follow what carries them, rotors turn, a shot kicks its shooter (a hull
//! rocks on its springs and settles, a soldier snaps back about his waist,
//! and the drawn muzzle rides both), a paused clock holds, and the corpse
//! cap gives way oldest first. Driven by synthetic feed frames over
//! the fixture's real appearances and pose feel.
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use presentation::effects::MuzzleSource;
use presentation::seed::side_key;
use presentation::units::drawn::{ClipTables, DrawnModels};
use presentation::units::driver::{
    FeedFallen, FeedFrame, FeedMount, FeedSoldier, FeedUnit, PoseDriver,
};
use presentation::units::pose::{ClipBlend, Kick, PoseFrame, SoldierPose};
use presentation::units::skin::{joint_worlds, kick_worlds, mul, trs, Mat4};
use presentation::world::{Appearance, UnitsSection, Vec3};
use serde_json::{json, Value};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/paris-corner-units.json"
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

/// Paused mid-battle, everything is drawn exactly as it was: a walker
/// mid-stride, a pinned man mid-crawl, a dying man mid-fall, a tank on the
/// move. Resumed, each carries on as if the pause never happened.
#[test]
fn paused_everyone_holds_mid_stride_and_resumed_carries_on_as_if_never_paused() {
    let units = units();
    let none = fallen(&[]);
    let down = fallen(&[9.0]);
    // At `time`: soldier 1 walks east, pinned soldier 2 crawls (his squad fired
    // at bearing 0), soldier 9 is shot at 1.5, and the tank drives east.
    let at = |time: f64, d: &mut PoseDriver| {
        let walker = squad(vec![man(1.0, 1.2 * time, 0.0, 0.0)], false);
        let mut crawler = squad(vec![man(2.0, 0.0, 5.0 + 0.5 * time, 0.0)], true);
        crawler.id = 2.0;
        crawler.mounts[0].shots = 3.0;
        let mut victim = squad(vec![man(9.0, 9.0, 0.0, 0.0)], false);
        victim.id = 3.0;
        let (alive, gone) = if time < 1.5 {
            (vec![victim.soldiers.pop().unwrap()], &none)
        } else {
            (vec![], &down)
        };
        victim.soldiers = alive;
        let tank = tank(20.0 + 4.0 * time, 0.0, (0.0, 0.0, 0.0), 0.0);
        let units_now = vec![walker, crawler, victim, tank];
        format!("{:?}", d.update(&frame(time, units_now, gone), &units))
    };
    let mut paused = PoseDriver::new(&units).unwrap();
    let mut straight = PoseDriver::new(&units).unwrap();
    for time in [0.0, 0.5, 1.0, 1.6] {
        at(time, &mut paused);
        at(time, &mut straight);
    }
    let before = at(1.7, &mut paused);
    at(1.7, &mut straight);
    let poses = paused.poses();
    assert_eq!(clip_of(poses, 1.0), "walk");
    assert!(
        clip_of(poses, 2.0).starts_with("crawl_") || clip_of(poses, 2.0).starts_with("crouch_")
    );
    assert!(clip_of(poses, 9.0).starts_with("death"), "mid-fall");
    for _ in 0..3 {
        assert_eq!(at(1.7, &mut paused), before, "paused, the same frame again");
    }
    assert_eq!(
        at(1.8, &mut paused),
        at(1.8, &mut straight),
        "resumed, as if never paused"
    );
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
fn a_diving_jet_pitches_nose_down() {
    let units = units_with(|v| {
        let mut jet = v["types"]["menu_us_tank"].clone();
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
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(&frame(f64::from(time), vec![jet.clone()], &none), &units);
    }
    jet.position = [900.0, 0.0, 90.0];
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units)
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
        let mut jet = v["types"]["menu_us_tank"].clone();
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
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(&frame(f64::from(time), vec![jet.clone()], &none), &units);
    }
    jet.position = [900.0, 0.0, 110.0];
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units)
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
        let mut jet = v["types"]["menu_us_tank"].clone();
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
    driver.update(&frame(0.0, vec![jet.clone()], &none), &units);
    for time in 1..=8 {
        jet.position[0] = f64::from(time) * 100.0;
        driver.update(&frame(f64::from(time), vec![jet.clone()], &none), &units);
    }
    jet.position[0] = 900.0;
    jet.yaw = 0.4;
    let motion = driver
        .update(&frame(9.0, vec![jet], &none), &units)
        .vehicles[0]
        .motion
        .expect("a turning jet banks");
    assert!(motion[6] < -0.05, "left turn banks left: {motion:?}");
}

#[test]
fn a_helicopters_attitude_is_unchanged() {
    let units = units_with(|v| {
        let mut heli = v["types"]["menu_us_tank"].clone();
        heli["airborne"] = true.into();
        v["types"]["test_heli"] = heli;
    });
    let none = fallen(&[]);
    let max = feel(&units, "/attitude/rotor/max_deg").to_radians();
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut heli = tank(0.0, 0.0, (0.0, 0.0, 0.0), 0.0);
    heli.kind = "test_heli".into();
    heli.position[2] = 20.0;
    driver.update(&frame(0.0, vec![heli.clone()], &none), &units);
    heli.position[0] = 10.0;
    let motion = driver
        .update(&frame(1.0, vec![heli], &none), &units)
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
        at(0.51 + return_s, (1.5, 0.0, 6.0)).recoil,
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

#[test]
fn each_of_his_own_shots_kicks_a_soldier_back_by_his_weapon_s_kick_and_he_recovers() {
    // The test's own kick: a rifle's kicks fully, anything else half as hard.
    let units = units_with(|v| {
        v["pose"]["soldier_kick"] = json!({"rounds": {"default": 0.5, "rifle": 1},
            "pitch_deg": 10, "back_m": 0.06, "return_s": 0.3});
    });
    let none = fallen(&[]);
    let mut driver = PoseDriver::new(&units).unwrap();
    // Soldier 1 has fired `own` rounds of the squad's `own`, soldier 2 none.
    let mut at = |time: f64, own: f64| {
        let mut unit = squad(
            vec![man(1.0, 0.0, 0.0, own), man(2.0, 3.0, 0.0, 0.0)],
            false,
        );
        unit.mounts[0].shots = own;
        let poses = driver.update(&frame(time, vec![unit], &none), &units);
        (pose_of(poses, 1.0).kick, pose_of(poses, 2.0).kick)
    };
    let rest = Kick::default();
    let full = Kick {
        pitch: 10f64.to_radians(),
        back: 0.06,
    };
    assert_eq!(at(0.0, 0.0), (rest, rest));
    // His shot snaps him back the full kick at once; his mate never moves.
    let (shot, mate) = at(1.0, 1.0);
    assert!((shot.pitch - full.pitch).abs() < 1e-9 && (shot.back - full.back).abs() < 1e-9);
    assert_eq!(mate, rest);
    // He comes back, quickly at first, and is at rest by `return_s`.
    let early = at(1.1, 1.0).0.pitch;
    let late = at(1.2, 1.0).0.pitch;
    assert!(early < full.pitch && late > 0.0);
    assert!(full.pitch - early > early - late, "fast at first");
    assert_eq!(at(1.31, 1.0).0, rest);
    // Another shot kicks him again.
    assert!((at(2.0, 2.0).0.pitch - full.pitch).abs() < 1e-9);
}

/// A clip table written beside the fixture run's manifest: raw little-endian f32.
fn clip_table(name: &str) -> Vec<f32> {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/parity/capture/run/paris-corner/units-"
    );
    let bytes = std::fs::read(format!("{at}{name}.u32")).unwrap();
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&b| f32::from_le_bytes(b))
        .collect()
}

fn point(m: &Mat4) -> Vec3 {
    [m[12], m[13], m[14]]
}

#[test]
fn a_kick_turns_his_upper_body_back_about_his_waist_the_muzzle_climbs_the_hips_stay() {
    let units = units();
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        muzzle: Some(socket),
        ..
    }) = units.appearances.get("rifle_squad_active_a")
    else {
        panic!("the fixture's rifleman is skinned, with a muzzle");
    };
    let table = clip_table(table);
    let joint = |name: &str| joints.iter().position(|j| j.name == name).unwrap();
    let at = |kick: Kick| {
        let mut worlds = Vec::new();
        joint_worlds(joints, clips, &table, "stand_aim", 0.0, None, &mut worlds);
        kick_worlds(joints, kick, &mut worlds);
        let o = &socket.offset;
        let muzzle = mul(&worlds[socket.joint], &trs(o.t, o.r, o.s));
        (
            point(&muzzle),
            point(&worlds[joint("Head")]),
            point(&worlds[joint("pelvis")]),
        )
    };
    let (muzzle, head, pelvis) = at(Kick::default());
    let kicked = at(Kick {
        pitch: 0.15,
        back: 0.05,
    });
    // The muzzle (ahead of the waist) climbs; the head (above it) goes back.
    assert!(kicked.0[2] > muzzle[2] + 0.02, "{kicked:?}");
    assert!(kicked.1[0] < head[0] - 0.05, "{kicked:?}");
    // Nothing turns sideways, and the hips and legs never move.
    assert!((kicked.0[1] - muzzle[1]).abs() < 1e-9);
    assert_eq!(kicked.2, pelvis);
    // Thrown back alone, every carried point moves straight back by it.
    let shoved = at(Kick {
        pitch: 0.0,
        back: 0.05,
    });
    for (moved, rest) in [(shoved.0, muzzle), (shoved.1, head)] {
        assert!((moved[0] - (rest[0] - 0.05)).abs() < 1e-9 && (moved[2] - rest[2]).abs() < 1e-9);
    }
}

#[test]
fn a_soldier_s_flash_sits_on_his_muzzle_as_his_body_is_drawn_mid_crossfade_and_kicked() {
    let units = units();
    let mut pose = SoldierPose {
        soldier: 3.0,
        unit: 1.0,
        kind: "menu_rifle".into(),
        slot: 0,
        operator_mount: None,
        active_mount: Some(0),
        side: "blue".into(),
        position: [10.0, 5.0, 0.0],
        facing: 0.7,
        clip: "stand_aim".into(),
        phase: 0.0,
        blend: Some(ClipBlend {
            clip: "kneel_fire".into(),
            phase: 0.3,
            weight: 0.5,
        }),
        kick: Kick {
            pitch: 0.15,
            back: 0.05,
        },
    };
    let name = units
        .resolve(&pose.kind, &pose.side, pose.soldier, 0, None, Some(0))
        .expect("a rifleman with his rifle up")
        .to_owned();
    let Some(Appearance::Skinned {
        joints,
        clips,
        table,
        muzzle: Some(socket),
        ..
    }) = units.appearances.get(&name)
    else {
        panic!("{name} is skinned, with a muzzle");
    };
    let samples = clip_table(table);
    // Where his body is drawn: his clip crossfaded and kicked, as his palette is.
    let body_muzzle = |pose: &SoldierPose| {
        let mut worlds = Vec::new();
        joint_worlds(
            joints,
            clips,
            &samples,
            &pose.clip,
            pose.phase,
            pose.blend.as_ref(),
            &mut worlds,
        );
        kick_worlds(joints, pose.kick, &mut worlds);
        let o = &socket.offset;
        let m = mul(&worlds[socket.joint], &trs(o.t, o.r, o.s));
        let (s, c) = pose.facing.sin_cos();
        [
            pose.position[0] + m[12] * c - m[13] * s,
            pose.position[1] + m[12] * s + m[13] * c,
            pose.position[2] + m[14],
        ]
    };
    let mut tables = ClipTables::new();
    tables.insert(table.clone(), samples.clone());
    let mut drawn = DrawnModels::new(Arc::new(units.clone()), Arc::new(tables), "blue");
    let mut flash_at = |pose: &SoldierPose| {
        drawn.update(&PoseFrame {
            soldiers: vec![pose.clone()],
            ..PoseFrame::default()
        });
        drawn
            .muzzle(side_key(1.0, true), 0, Some(pose.soldier))
            .expect("his rifle is up")
    };
    let blended = flash_at(&pose);
    let want = body_muzzle(&pose);
    for i in 0..3 {
        assert!(
            (blended[i] - want[i]).abs() < 1e-9,
            "{blended:?} vs {want:?}"
        );
    }
    // The crossfade moves the muzzle: without it he would be drawn elsewhere.
    pose.blend = None;
    let unblended = body_muzzle(&pose);
    assert!(
        (0..3).any(|i| (unblended[i] - want[i]).abs() > 0.05),
        "{unblended:?} vs {want:?}"
    );
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
            let poses = driver.update(&frame(time, vec![unit], &none), &units);
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
