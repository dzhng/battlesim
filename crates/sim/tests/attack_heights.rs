//! How high attack runs fly: the seeded run height, the climb back toward
//! cruise between runs, and the dive from wherever that climb has reached.
use crate::attack_passes::{far, state, wing};
use crate::attack_runs::{run_battle, RUN_BOMB_CEILING_M, RUN_STRAFE_CEILING_M, RUN_WINDOW_S};
use crate::common::air_field::wing_limits;
use contract::ids::UnitId;
use contract::random::Rng;
use sim::air::pass::{advance, begin, PassAim, PassPhase, PassWeapon};
use sim::air::Intent;
use sim::flight::Release;
use sim::math::v3;

#[test]
fn pass_height_is_seeded_between_floor_and_ceiling() {
    let p = begin(
        &state(250.0),
        &wing_limits(),
        v3(1000.0, 0.0, 0.0),
        PassWeapon::Bomb,
        &wing(),
        &mut Rng::new(9),
    );
    assert!((100.0..=200.0).contains(&p.agl));
}

/// The height the intent flies at.
fn intent_agl(intent: Intent) -> f64 {
    match intent {
        Intent::Line { agl, .. } => agl,
        other => panic!("a pass flies a line, not {other:?}"),
    }
}

/// Lining up, the jet climbs toward cruise; the run it commits to dives to
/// its seeded height from above it, and from below it holds the height it
/// has reached rather than climbing into the run. That height is above the
/// ground under the jet, so a rise under it is not counted as its own.
#[test]
fn a_run_dives_from_the_height_its_climb_has_reached() {
    let wing = wing();
    let limits = wing_limits();
    let aim = v3(1000.0, 0.0, 0.0);
    let aim_at = PassAim {
        point: aim,
        release: Release {
            fall_s: 0.0,
            ahead_m: 0.0,
            at: aim.xy(),
        },
        reach_m: 0.0,
    };
    let fresh = || {
        let mut plan = begin(
            &state(limits.cruise_agl_m),
            &limits,
            aim,
            PassWeapon::Gun,
            &wing,
            &mut Rng::new(9),
        );
        // Lining up, just past the aim and flying away from it: it climbs.
        let mut off = state(0.0);
        off.pos.x = aim.x - 50.0;
        off.heading = std::f64::consts::PI;
        let (intent, _) = advance(&mut plan, &off, 0.0, &limits, &far(), &aim_at, 0.05);
        assert_eq!(plan.phase, PassPhase::LineUp);
        assert_eq!(intent_agl(intent), limits.cruise_agl_m, "lining up");
        plan
    };
    let seeded = fresh().agl;
    let under = (limits.floor_agl_m + seeded) * 0.5;
    for ground in [0.0, 60.0] {
        for (height, flown) in [(limits.cruise_agl_m, seeded), (under, under)] {
            let mut plan = fresh();
            let (intent, _) = advance(
                &mut plan,
                &state(ground + height),
                height,
                &limits,
                &far(),
                &aim_at,
                0.05,
            );
            assert_eq!(plan.phase, PassPhase::Dive);
            assert_eq!(
                intent_agl(intent),
                flown,
                "committing {height} m over {ground} m ground"
            );
        }
    }
}

/// Between runs the jet pulls up: off each run it climbs back toward its
/// cruise height while it lines up, and the next run dives again from
/// wherever that climb has got to, under its weapon's ceiling. It never
/// stays down at run height for the whole attack.
#[test]
fn a_jet_climbs_after_every_run_and_dives_on_the_next() {
    let aim = [2000.0, 2000.0];
    for (weapon, ceiling) in [
        ("air_bomb", RUN_BOMB_CEILING_M),
        ("air_gun", RUN_STRAFE_CEILING_M),
    ] {
        let mut battle = run_battle(&[weapon], aim);
        let hz = f64::from(battle.rules().tick_hz);
        let height = |battle: &sim::battle::Battle| battle.unit(UnitId(0)).unwrap().position.z;
        let diving = |battle: &sim::battle::Battle| {
            battle
                .unit(UnitId(0))
                .unwrap()
                .air
                .unwrap()
                .pass
                .map(|p| p.phase)
                == Some(PassPhase::Dive)
        };
        // Per run, from its commit: its lowest on the run, and the highest
        // the climb after it reaches before the next run commits.
        let mut runs: Vec<(f64, f64)> = Vec::new();
        let mut was_diving = false;
        for _ in 0..(RUN_WINDOW_S * hz) as u64 {
            battle.step();
            let (z, now) = (height(&battle), diving(&battle));
            if now && !was_diving {
                runs.push((z, z));
            }
            if let Some(run) = runs.last_mut() {
                if now {
                    run.0 = run.0.min(z);
                    run.1 = run.0;
                } else {
                    run.1 = run.1.max(z);
                }
            }
            was_diving = now;
        }
        assert!(
            runs.len() >= 3,
            "{weapon}: only {} runs in the window",
            runs.len()
        );
        // The field is flat at zero: height is height above the ground.
        for (i, &(low, _)) in runs.iter().enumerate() {
            assert!(
                low <= ceiling + 1.0,
                "{weapon}: run {i} stayed above its {ceiling} m ceiling, lowest {low:.0} m; runs {runs:?}"
            );
        }
        // The last run's climb may be cut off by the window's end.
        for (i, pair) in runs.windows(2).enumerate() {
            let (low, peak) = pair[0];
            let (next_low, _) = pair[1];
            assert!(
                peak >= low + 20.0,
                "{weapon}: after run {i} the jet climbed only from {low:.0} m to {peak:.0} m; runs {runs:?}"
            );
            assert!(
                next_low <= peak - 20.0,
                "{weapon}: run {} never dived from the climb ({peak:.0} m), lowest {next_low:.0} m; runs {runs:?}",
                i + 1
            );
        }
    }
}
