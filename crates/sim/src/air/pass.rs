//! Fixed-wing ground attack: the runs a jet makes at one aim, and the weapon
//! it makes them with. Weapon release remains owned by the flight solver.

use contract::random::Rng;
use contract::scenario::FixedWingFlight;

use crate::flight::Release;
use crate::math::{v2, wrap_angle, V2, V3};
use crate::weapons::nose_on_bearing;

use super::fixed_wing::{FixedWingLimits, FixedWingState};
use super::steer::{edge_guarded, guard_short_of, inward_from, Bounds, Intent};
use crate::weapons::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassWeapon {
    Bomb,
    GroundMissile,
    Gun,
}

/// What a weapon row does on a pass, from how it is delivered: an
/// unpowered row falls (a bomb), the always-available gun (`default`)
/// strafes, and any other row reaching the ground is fired on a missile
/// pass. `None` for a row that never attacks the ground on a pass.
pub fn pass_weapon(def: &contract::weapons::WeaponDefinition) -> Option<PassWeapon> {
    if def.ballistics.delivery == contract::ballistics::Delivery::Unpowered {
        Some(PassWeapon::Bomb)
    } else if def
        .against(contract::catalog::AltitudeLayer::Ground)
        .is_none()
    {
        None
    } else if def.default {
        Some(PassWeapon::Gun)
    } else {
        Some(PassWeapon::GroundMissile)
    }
}

/// Where a jet attacking the ground is in its runs. A run is a short
/// commitment: it begins when the nose comes on the aim and is over once its
/// weapon has done its job or anything breaks it off; then the next is lined
/// up from wherever the jet is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassPhase {
    /// Lining up, committed to nothing: flying out along a line through the
    /// aim until a full-rate turn in on it leaves a full run-in.
    LineUp,
    /// Lining up, committed to nothing: holding that turn until the nose
    /// comes on the aim.
    TurnIn,
    /// The run: flying straight at the aim along the line it committed to,
    /// until it reaches its release point (a bomb's drop, or the aim itself:
    /// the gun has passed it) or the edge guard turns it away.
    Dive,
}

/// A jet's attack on one target with one pass weapon: the runs it makes
/// until the target, the weapon or the order changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassPlan {
    /// The order target the runs are flown against. Keeping it with the
    /// phase lets attack-move finish a run before choosing a new reaction.
    pub target: Target,
    /// Where the runs aim: the target's point while lining up, fixed for a
    /// run once it commits.
    pub aim: V3,
    pub phase: PassPhase,
    pub weapon: PassWeapon,
    /// The seeded height the runs are flown at, in metres above ground.
    pub agl: f64,
    /// The bearing of the line flown through the aim: away from it lining
    /// up, toward it turning in, and in a run the one it committed along.
    pub heading: f64,
}

/// Limited ground stores take a run before the unlimited fixed gun.
pub fn choose_weapon(weapons: &[PassWeapon]) -> Option<PassWeapon> {
    weapons
        .iter()
        .copied()
        .find(|w| matches!(w, PassWeapon::Bomb | PassWeapon::GroundMissile))
        .or_else(|| weapons.iter().copied().find(|w| *w == PassWeapon::Gun))
}

fn ceiling(weapon: PassWeapon, air: &FixedWingFlight) -> f64 {
    match weapon {
        PassWeapon::Bomb => air.bomb_ceiling_m,
        PassWeapon::Gun | PassWeapon::GroundMissile => air.strafe_ceiling_m,
    }
}

/// Start an attack from `state`, flying over ground `ground_m` high, lining
/// up. Its existing height above that ground is retained when already under
/// its weapon's ceiling; otherwise a deterministic height in the permitted
/// band is used.
pub fn begin(
    state: &FixedWingState,
    ground_m: f64,
    limits: &FixedWingLimits,
    aim: V3,
    weapon: PassWeapon,
    air: &FixedWingFlight,
    rng: &mut Rng,
) -> PassPlan {
    let max = ceiling(weapon, air);
    let floor = limits.floor_agl_m;
    let current = state.pos.z - ground_m;
    let agl = if current <= max {
        current.max(floor)
    } else {
        floor + rng.unit() * (max - floor)
    };
    let heading = libm::atan2(aim.y - state.pos.y, aim.x - state.pos.x);
    PassPlan {
        target: Target::Ground(aim),
        aim,
        phase: PassPhase::LineUp,
        weapon,
        agl,
        heading,
    }
}

fn line(plan: &PassPlan) -> Intent {
    Intent::Line {
        through: plan.aim.xy(),
        heading: plan.heading,
        agl: plan.agl,
    }
}

/// The circle a full-rate turn toward `aim` flies, on the side it lies:
/// its centre and radius.
fn turn_circle(state: &FixedWingState, limits: &FixedWingLimits, aim: V2) -> (V2, f64) {
    let to_aim = aim - state.pos.xy();
    let side = wrap_angle(libm::atan2(to_aim.y, to_aim.x) - state.heading).signum();
    let radius = state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE);
    let left = v2(-libm::sin(state.heading), libm::cos(state.heading));
    (state.pos.xy() + left * (radius * side), radius)
}

/// Fly one tick of lining up (`plan` is lining up or turning in and does not
/// commit this tick): move it to the phase it flies and return the intent.
fn line_up(
    plan: &mut PassPlan,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    release: &Release,
    guarded: bool,
    dt: f64,
) -> Intent {
    let aim = plan.aim.xy();
    let to_aim = aim - state.pos.xy();
    let distance = to_aim.length();
    let bearing = libm::atan2(to_aim.y, to_aim.x);
    let nose_on = nose_on_bearing(state.heading, bearing, limits.nose_on_rad);
    let past_release = distance <= release.ahead_m;
    let speed = state.speed_mps.max(1.0);
    let (centre, radius) = turn_circle(state, limits, aim);
    let from_centre = aim - centre;
    // Out along the line the run will come back on, and where on it the run
    // must end. Clear of the edges the line is straight away from the aim,
    // and the run ends at its release point. By an edge it is a fixed line
    // into the map, so the run comes back at the edge from inside rather
    // than along it, and ends at its release point or sooner where the edge
    // guard takes it. The jet flies out two turn radii to the side of that
    // line so that turning in lands it on it, and turns in only once out on
    // that leg, where the turn toward the aim is a turn away from the edge.
    let free_run_in = speed * 2.0 + release.ahead_m;
    let (out, through, on_leg, ends) =
        match inward_from(aim, limits.edge_margin_m + radius + free_run_in, bounds) {
            None => (bearing + core::f64::consts::PI, aim, true, release.ahead_m),
            Some(way) => {
                let out = libm::atan2(way.y, way.x);
                let out_dir = v2(libm::cos(out), libm::sin(out));
                let left = v2(-out_dir.y, out_dir.x);
                let side = if left.dot(-to_aim) < 0.0 { -1.0 } else { 1.0 };
                let through = aim + left * (2.0 * radius * side);
                let on_leg = wrap_angle(state.heading - out).abs() < core::f64::consts::FRAC_PI_4
                    && (state.pos.xy() - through).dot(left).abs() < radius;
                let guarded_at = guard_short_of(aim, -out_dir, state, limits, bounds, dt);
                (out, through, on_leg, release.ahead_m.max(guarded_at))
            }
        };
    let run_in = speed * 2.0 + ends;
    // A turn in the edge guard takes over is broken like a run: the jet
    // lines up again rather than riding the edge.
    plan.phase = if guarded {
        PassPhase::LineUp
    } else if on_leg && from_centre.at_least_radius(libm::hypot(radius, run_in)) {
        PassPhase::TurnIn
    } else if plan.phase == PassPhase::TurnIn
        && from_centre.at_least_radius(radius)
        && !(nose_on && past_release)
    {
        // Turning in, it holds the turn while one still brings the nose on;
        // nose-on past its release point, or inside its own turn, it lines
        // up again.
        PassPhase::TurnIn
    } else {
        PassPhase::LineUp
    };
    let (heading, through) = match plan.phase {
        PassPhase::TurnIn => (bearing, aim),
        _ => (out, through),
    };
    plan.heading = heading;
    Intent::Line {
        through,
        heading,
        agl: plan.agl,
    }
}

/// Advance the attack by one tick inside `bounds` and return the steering
/// intent, and whether this is the one tick its weapon is released.
///
/// Lining up is flown from where the jet is, the same for the first run as
/// for every one after: it flies out along a line through the aim (straight
/// away from it, or into the map from an aim by an edge) until turning in on
/// it would leave it pointed at the aim a full run-in short of where the run
/// must end, then holds the turn in. A run must end at its release point (a
/// bomb's lies its forward throw short of the aim) or where the edge guard
/// would take it, whichever comes first. The run commits the moment the
/// nose comes on the aim, short of the release point and clear of the edge
/// guard, however much run-in is left. The release cue comes on the tick
/// whose `dt` step carries the jet to its release point, so a bomb leaves
/// the jet within half a step of it, and the run is over with it. A run the
/// edge guard takes is broken and releases nothing. Either way the jet is
/// lining up again.
pub fn advance(
    plan: &mut PassPlan,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    aim: V3,
    release: &Release,
    dt: f64,
) -> (Intent, bool) {
    let speed = state.speed_mps.max(1.0);
    let guarded = edge_guarded(state, limits, bounds, dt);
    if plan.phase != PassPhase::Dive {
        let to_aim = aim.xy() - state.pos.xy();
        let distance = to_aim.length();
        let bearing = libm::atan2(to_aim.y, to_aim.x);
        plan.aim = aim;
        let nose_on = nose_on_bearing(state.heading, bearing, limits.nose_on_rad);
        // Short of where it would release on this run.
        let short = distance > release.ahead_m;
        if nose_on && short && !guarded {
            plan.phase = PassPhase::Dive;
            plan.heading = bearing;
        } else {
            return (
                line_up(plan, state, limits, bounds, release, guarded, dt),
                false,
            );
        }
    }
    let intent = line(plan);
    if guarded {
        plan.phase = PassPhase::LineUp;
        return (intent, false);
    }
    let forward = v2(libm::cos(plan.heading), libm::sin(plan.heading));
    let step = speed * dt;
    let short = (release.at - state.pos.xy()).dot(forward);
    let cue = short - step <= step * 0.5;
    if cue {
        plan.phase = PassPhase::LineUp;
    }
    (intent, cue)
}
