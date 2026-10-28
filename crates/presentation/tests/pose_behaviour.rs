//! What a player sees the pose driver do with soldiers, beyond the
//! browser's vectors: each moves and faces by his own motion, a squad under
//! fire is never a row of copies, the dying fall from the posture they are
//! in and lie as they fell, a lean slides out and back, a shot kicks its
//! shooter back about his waist (and the drawn muzzle rides it), a paused
//! clock holds, and the corpse cap gives way oldest first. Driven by
//! synthetic feed frames over the fixture's real appearances and pose feel.
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use presentation::effects::MuzzleSource;
use presentation::seed::side_key;
use presentation::units::drawn::{ClipTables, DrawnModels};
use presentation::units::driver::PoseDriver;
use presentation::units::pose::{ClipBlend, Kick, PoseFrame, SoldierPose};
use presentation::units::skin::{joint_worlds, kick_worlds, mul, trs, Mat4};
use presentation::world::{Appearance, Vec3};
use serde_json::json;

mod common;
use common::*;

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
        format!(
            "{:?}",
            d.update(&frame(time, vec![unit], &none), &units, None)
        )
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
        format!(
            "{:?}",
            d.update(&frame(time, units_now, gone), &units, None)
        )
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
        let poses = driver.update(&frame(time, vec![squad(men, pinned)], &none), &units, None);
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
    let fell = driver.update(&frame(3.1, vec![squad(vec![], true)], &down), &units, None);
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
    let lain = driver.update(&frame(5.0, vec![squad(vec![], true)], &down), &units, None);
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
    driver.update(
        &frame(0.0, vec![squad(men, false)], &fallen(&[])),
        &units,
        None,
    );
    let down = fallen(&ids);
    let falls = driver.update(&frame(1.0, vec![squad(vec![], false)], &down), &units, None);
    let falls: Vec<(f64, String, f64)> = ids
        .iter()
        .map(|&id| (id, clip_of(falls, id), pose_of(falls, id).facing))
        .collect();
    let kinds: BTreeSet<_> = falls.iter().map(|f| f.1.as_str()).collect();
    assert_eq!(kinds, BTreeSet::from(["death", "death_back"]));
    let lain = driver.update(&frame(4.0, vec![squad(vec![], false)], &down), &units, None);
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
            pose_of(
                driver.update(&frame(time, vec![unit], &none), &units, None),
                1.0,
            )
            .clone()
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
            None,
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

#[test]
fn past_the_cap_a_battle_joined_lies_its_newest_and_a_reset_forgets_the_gone() {
    let units = units_with(|v| v["pose"]["corpses"]["max"] = 3.into());
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut at = |time: f64, ids: &[f64]| {
        let poses = driver.update(&frame(time, vec![], &fallen(ids)), &units, None);
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
        let poses = driver.update(&frame(time, vec![unit], &none), &units, None);
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
        kind: "capture_rifle".into(),
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
