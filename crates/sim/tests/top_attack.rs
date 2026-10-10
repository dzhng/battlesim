//! A top-attack missile in the flight store: it climbs above its commanded
//! point, pitches over and dives onto it from above, so it meets a hull's
//! roof; the same row flown direct meets the front.

use crate::common::*;
use contract::ballistics::{TopAttack, WeaponBallistics};
use contract::random::Rng;
use contract::scenario::Face;
use sim::damage::struck_face;
use sim::digest::Digest;
use sim::flight::{
    fire_round, Aim, BodyId, FlightConfigError, FlightEvent, Guidance, Impact, Loft, Projectiles,
    Shape, Struck,
};
use sim::math::{v3, V3};

/// The generic missile row tuned for top attack: an agile seeker that pulls
/// up off the launcher, holds a loft a viewer reads at play camera and
/// pitches over steeply. At the generic row's 60°/s the pitch-over comes too
/// close for its turn radius and the missile loops or overshoots
/// (`specs/ground-admission/assets/top-attack/`).
fn top_attack_row() -> WeaponBallistics {
    WeaponBallistics {
        turn_deg_s: Some(360.0),
        top_attack: Some(TopAttack {
            loft_m: 60.0,
            dive_deg: 40.0,
        }),
        ..weapon("atgm")
    }
}

#[test]
fn the_aim_climbs_to_the_loft_holds_it_until_the_dive_cone_and_never_once_released() {
    let point = v3(500.0, 200.0, 1.0);
    let guidance = Guidance {
        point,
        turn_rad_s: 1.0,
        supported: true,
        loft: Some(Loft {
            height_m: 60.0,
            slope: 1.0,
        }),
    };
    let lofted = point + v3(0.0, 0.0, 60.0);
    // Far out and low: the loft height one climb's run (60 m at 45°) ahead.
    assert_eq!(guidance.aim(v3(100.0, 200.0, 2.0)), v3(160.0, 200.0, 61.0));
    // At the loft, far out: still level, a run ahead.
    assert_eq!(guidance.aim(v3(300.0, 200.0, 61.0)), v3(360.0, 200.0, 61.0));
    // Nearer than one run, 30 m above: shallower than 45°, so the lofted
    // point itself, never past it.
    assert_eq!(guidance.aim(v3(460.0, 200.0, 31.0)), lofted);
    // 40 m out and 41 m above: inside the cone, dive onto the point.
    assert_eq!(guidance.aim(v3(460.0, 200.0, 42.0)), point);
    // Released, it flies at its fixed point wherever it is.
    let released = Guidance {
        supported: false,
        ..guidance
    };
    assert_eq!(released.aim(v3(100.0, 200.0, 2.0)), point);
    // Without a loft, it always flies at its point.
    let direct = Guidance {
        loft: None,
        ..guidance
    };
    assert_eq!(direct.aim(v3(100.0, 200.0, 2.0)), point);
}

#[test]
fn top_attack_needs_a_guided_row_a_positive_loft_and_a_dive_between_level_and_vertical() {
    let good = top_attack_row().top_attack.unwrap();
    let row = |top_attack: TopAttack| WeaponBallistics {
        top_attack: Some(top_attack),
        ..top_attack_row()
    };
    let refused =
        |w: WeaponBallistics| matches!(config().profile(&w), Err(FlightConfigError::TopAttack(_)));
    assert!(refused(WeaponBallistics {
        turn_deg_s: None,
        accel_mps2: None,
        top_speed_mps: None,
        ..row(good)
    }));
    for loft_m in [0.0, -10.0, f64::NAN, f64::INFINITY] {
        assert!(refused(row(TopAttack { loft_m, ..good })), "{loft_m}");
    }
    for dive_deg in [0.0, 90.0, -10.0, 120.0, f64::NAN] {
        assert!(refused(row(TopAttack { dive_deg, ..good })), "{dive_deg}");
    }
    let loft = config().profile(&row(good)).unwrap().loft.unwrap();
    assert_eq!(loft.height_m, good.loft_m);
    assert!((loft.slope - good.dive_deg.to_radians().tan()).abs() < 1e-12);
    assert_eq!(profile("atgm").loft, None);
}

/// One shot from a launcher at infantry muzzle height at a still tank
/// `distance` away, its front toward the launcher, aimed where the battle
/// aims at a hull.
struct Shot {
    origin: V3,
    target: V3,
    /// Position at each tick's end, from the launch to the impact.
    path: Vec<V3>,
    impact: Option<Impact>,
    face: Option<Face>,
    seconds: f64,
}

impl Shot {
    fn apex(&self) -> f64 {
        self.path.iter().map(|p| p.z).fold(f64::MIN, f64::max)
    }

    /// Degrees below level the round was flying when it struck.
    fn descent_deg(&self) -> f64 {
        let v = self.impact.expect("it struck").velocity;
        (-v.z).atan2(v.xy().length()).to_degrees()
    }
}

fn shoot(w: &WeaponBallistics, distance: f64) -> Shot {
    let config = config();
    let p = config.profile(w).unwrap();
    let world = flat([distance + 400.0, 400.0], "");
    let origin = v3(100.0, 200.0, physics("infantry_muzzle_m"));
    let Shape::Box { half } = tank_shape() else {
        unreachable!("a tank is a box")
    };
    let base = v3(origin.x + distance, origin.y, 0.0);
    let target = base
        + v3(
            0.0,
            0.0,
            2.0 * half.z * physics("vehicle_aim_height_fraction"),
        );
    let aim = Aim {
        origin,
        target,
        target_velocity: V3::default(),
    };
    let (launch, _) = fire_round(
        &world,
        &config,
        &p,
        &aim,
        0.0,
        &mut Rng::new(1),
        None,
        |_| false,
        |_| false,
    )
    .unwrap()
    .expect("nothing holds the round");
    let tank = Mover {
        yaw: std::f64::consts::PI,
        ..Mover::standing(7, 1, tank_shape(), base)
    };
    let mut store = Projectiles::new(config.clone());
    let id = store.launch(launch);
    let dt = config.tick_s();
    let mut shot = Shot {
        origin,
        target,
        path: vec![origin],
        impact: None,
        face: None,
        seconds: 0.0,
    };
    for tick in 1..=(p.lifetime_s / dt).ceil() as u64 {
        let events = fly(&mut store, &world, 1, |_| vec![tank.body(tick, dt)]);
        if let Some(&(_, hit)) = impacts(&events).first() {
            shot.path.push(hit.point);
            shot.seconds = event_time(tick, &FlightEvent::Impact(hit), dt);
            shot.face = hit
                .pose
                .map(|pose| struck_face(half, pose, hit.point, hit.normal));
            shot.impact = Some(hit);
            break;
        }
        let Some(round) = store.get(id) else { break };
        shot.path.push(round.position);
    }
    shot
}

#[test]
fn a_top_attack_missile_climbs_well_above_the_line_of_sight_and_dives_onto_the_roof() {
    let w = top_attack_row();
    let TopAttack { loft_m, dive_deg } = w.top_attack.unwrap();
    let shot = shoot(&w, 500.0);
    // The trace a human reads (`--nocapture`): height against range.
    println!("top attack at 500 m: range m, height m");
    for p in &shot.path {
        println!("{:7.1} {:6.1}", p.x - shot.origin.x, p.z);
    }
    // It shoots up off the launcher, as steeply as it will come down: by
    // the time it has flown one loft's climb at the dive angle, it is past
    // half its loft, not on a ramp towards a point above the far target.
    let climb_m = loft_m / dive_deg.to_radians().tan();
    let risen = shot
        .path
        .iter()
        .find(|p| p.x - shot.origin.x >= climb_m)
        .expect("it flew past its climb")
        .z
        - shot.origin.z;
    assert!(
        risen >= 0.5 * loft_m,
        "{risen:.1} m up after {climb_m:.0} m; loft {loft_m} m"
    );
    let sight = shot.origin.z.max(shot.target.z);
    assert!(
        shot.apex() - sight >= 0.75 * loft_m,
        "climbs {:.1} m above the line of sight; loft {loft_m} m",
        shot.apex() - sight
    );
    assert_eq!(shot.impact.map(|i| i.struck), Some(Struck::Body(BodyId(7))));
    assert_eq!(shot.face, Some(Face::Roof));
    assert!(
        shot.descent_deg() >= dive_deg,
        "falls {:.1}° onto the roof; the dive is {dive_deg}°",
        shot.descent_deg()
    );
}

#[test]
fn the_same_row_flown_direct_meets_the_front() {
    let shot = shoot(
        &WeaponBallistics {
            top_attack: None,
            ..top_attack_row()
        },
        500.0,
    );
    assert_eq!(shot.impact.map(|i| i.struck), Some(Struck::Body(BodyId(7))));
    assert_eq!(shot.face, Some(Face::Front));
    assert!(
        shot.apex() < shot.target.z + 1.0,
        "flies flat: {}",
        shot.apex()
    );
}

#[test]
fn a_short_top_attack_shot_pops_up_and_still_dives_onto_the_roof_without_looping() {
    let w = top_attack_row();
    let TopAttack { loft_m, dive_deg } = w.top_attack.unwrap();
    let shot = shoot(&w, 100.0);
    assert_eq!(shot.face, Some(Face::Roof));
    assert!(shot.descent_deg() >= dive_deg, "{:.1}°", shot.descent_deg());
    // Never climbs past its loft: a missile turning back over the top
    // would read as a loop.
    assert!(
        shot.apex() <= shot.target.z + loft_m,
        "apex {:.1} m",
        shot.apex()
    );
}

#[test]
fn a_top_attack_shot_at_full_range_arrives_within_its_lifetime() {
    let w = top_attack_row();
    let shot = shoot(&w, w.range_m);
    assert_eq!(shot.face, Some(Face::Roof));
    assert!(
        shot.seconds < w.lifetime_s.unwrap(),
        "arrives at {:.2} s",
        shot.seconds
    );
}

#[test]
fn the_loft_is_part_of_the_digest_only_when_present() {
    let digest = |w: &WeaponBallistics| {
        let config = config();
        let p = config.profile(w).unwrap();
        let world = flat([800.0, 400.0], "");
        let origin = v3(100.0, 200.0, 1.4);
        let aim = Aim {
            origin,
            target: origin + v3(400.0, 0.0, 0.0),
            target_velocity: V3::default(),
        };
        let (launch, _) = fire_round(
            &world,
            &config,
            &p,
            &aim,
            p.scatter_mrad,
            &mut Rng::new(3),
            None,
            |_| false,
            |_| false,
        )
        .unwrap()
        .unwrap();
        let mut store = Projectiles::new(config.clone());
        store.launch(launch);
        fly(&mut store, &world, 20, |_| vec![]);
        let mut d = Digest::default();
        store.digest(&mut d);
        d.finish()
    };
    let lofted = top_attack_row();
    assert_eq!(digest(&lofted), digest(&lofted), "repeats from its seed");
    assert_ne!(
        digest(&lofted),
        digest(&WeaponBallistics {
            top_attack: None,
            ..lofted
        })
    );
}
