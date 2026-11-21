//! Fixed-wing ground attack: the runs a jet makes at one aim, and the weapon
//! it makes them with. Weapon release remains owned by the flight solver.

use contract::random::Rng;
use contract::scenario::FixedWingFlight;

use crate::flight::Release;
use crate::math::{v2, wrap_angle, V2, V3};
use crate::weapons::nose_on_bearing;

use super::fixed_wing::{pitch_swing_s, pull_out_m, FixedWingLimits, FixedWingState, NoseAt};
use super::steer::{
    edge_guard, guard_short_of, inward_from, level_roll_lag_s, roll_lag_s, tightest_radius, Bounds,
    Intent,
};
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
/// up from wherever the jet is. A bomb need not wait for a run: it leaves
/// in any phase once it would land within reach of the aim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassPhase {
    /// Lining up, committed to nothing: flying out along a line through the
    /// aim until a turn in on it at its turn rate leaves a full run-in.
    LineUp,
    /// Lining up, committed to nothing: holding that turn until the nose
    /// comes on the aim.
    TurnIn,
    /// The run: flying straight at the aim along the line it committed to,
    /// until where its round would land comes no nearer the aim (a bomb's
    /// drop point, or the jet itself: the gun has passed it) or the edge
    /// guard turns it away. A run fired along the nose dives at the aim on
    /// the way ([`Dive`]).
    Run,
}

/// How far down a run fired along the nose (one whose gun engages its
/// target, a missile run's included) has come: it flies in level at its
/// entry height, dives with its nose on the aim once the aim lies its dive
/// angle below it, and pulls out over its run height. A bomb's run stays
/// level, as does one that fires nothing along the nose. A missile the run
/// fires leaves in the dive, its nose on the aim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dive {
    Level,
    Diving,
    PulledOut,
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
    /// The seeded height a run is flown at, in metres above ground, between
    /// the floor and its weapon's ceiling: the lowest a run comes down to.
    pub agl: f64,
    /// The height this run is flown at, in metres above ground, once its
    /// descent has begun: its seeded height or, lower than that, its own.
    /// `None` while it climbs back toward cruise between runs.
    pub run_agl: Option<f64>,
    /// The bearing of the line flown through the aim: away from it lining
    /// up, toward it turning in, and in a run the one it committed along.
    pub heading: f64,
    /// Where a bomb dropped last tick would have landed was still closing
    /// on the aim: the tick it stops is a bomb's one chance on that
    /// approach.
    pub closing: bool,
    /// How far down this run's dive has come; `None` for a run that does
    /// not dive (a bomb's, or one that fires nothing along the nose).
    pub dive: Option<Dive>,
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

/// How far from its aim a pass weapon fires: a bomb its slant from the
/// release a run at its ceiling, `speed_mps` fast, drops it from (the highest
/// a run is flown, so the farthest it falls); a gun or ground missile from its
/// reach, nose on. `None` for a row a pass never fires at the ground.
pub fn firing_range_m(
    weapon: &crate::weapons::Weapon,
    config: &crate::flight::FlightConfig,
    air: &FixedWingFlight,
    speed_mps: f64,
) -> Option<f64> {
    match pass_weapon(&weapon.def)? {
        PassWeapon::Bomb => {
            let height = ceiling(PassWeapon::Bomb, air);
            let release = crate::flight::release(
                crate::math::v3(0.0, 0.0, height),
                crate::math::v3(speed_mps, 0.0, 0.0),
                weapon.profile.gravity(config),
                V3::default(),
            )?;
            Some(libm::hypot(release.ahead_m, height))
        }
        PassWeapon::Gun | PassWeapon::GroundMissile => weapon
            .def
            .against(contract::catalog::AltitudeLayer::Ground)
            .map(|fire| fire.range_m),
    }
}

/// Start an attack from `state`, lining up, with a seeded run height
/// between the floor and its weapon's ceiling.
pub fn begin(
    state: &FixedWingState,
    limits: &FixedWingLimits,
    aim: V3,
    weapon: PassWeapon,
    air: &FixedWingFlight,
    rng: &mut Rng,
) -> PassPlan {
    let floor = limits.floor_agl_m;
    let agl = floor + rng.unit() * (ceiling(weapon, air) - floor);
    let heading = libm::atan2(aim.y - state.pos.y, aim.x - state.pos.x);
    PassPlan {
        target: Target::Ground(aim),
        aim,
        phase: PassPhase::LineUp,
        weapon,
        agl,
        run_agl: None,
        heading,
        closing: false,
        dive: None,
    }
}

/// Where a pass aims this tick, and where its weapon must leave the jet to
/// reach it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassAim {
    pub point: V3,
    pub release: Release,
    /// How far off the aim a dropped round may fall and still be steered
    /// onto it: a bomb's glide over its drop. A release that would land
    /// farther off is held.
    pub reach_m: f64,
    /// The dive a run fired along the nose flies; `None` for a level run (a
    /// bomb's, or one that fires nothing along the nose).
    pub dive: Option<DiveRun>,
}

/// The dive of a run fired along the nose: the height above the ground it
/// flies in at, from which a dive at `angle_rad` toward the aim comes onto
/// its line where it meets its weapon's reach (never under the floor), how
/// far that lies above the aim, that angle, and how far ahead of where it
/// starts pushing over its dive comes onto that line: half the ground it
/// covers pushing over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiveRun {
    pub entry_agl_m: f64,
    /// How far its entry height lies above the aim.
    pub above_aim_m: f64,
    pub angle_rad: f64,
    pub lead_m: f64,
}

impl DiveRun {
    /// The dive into a weapon's `reach_m` at `angle_rad` for a jet `speed_mps`
    /// fast, onto an aim `aim_agl_m` above its ground, flown in no lower than
    /// its floor.
    pub fn new(
        reach_m: f64,
        angle_rad: f64,
        aim_agl_m: f64,
        limits: &FixedWingLimits,
        speed_mps: f64,
    ) -> Self {
        // Pushing over, it sinks about its lead's worth of the line before
        // it is on it, so it flies in that much higher.
        let lead_m = 0.5 * speed_mps * pitch_swing_s(limits, angle_rad);
        let on_line = reach_m * libm::sin(angle_rad) + lead_m * libm::tan(angle_rad);
        let entry_agl_m = (aim_agl_m + on_line).max(limits.floor_agl_m);
        Self {
            entry_agl_m,
            above_aim_m: entry_agl_m - aim_agl_m,
            angle_rad,
            lead_m,
        }
    }
}

impl PassAim {
    /// How far short of the aim a run must have its nose on it, over the
    /// ground: a bomb's release point, its forward throw short; a dive's
    /// push-over, its lead short of where the aim comes its dive angle
    /// below its entry height.
    pub fn ends_short_m(&self) -> f64 {
        self.dive.map_or(self.release.ahead_m, |d| {
            d.above_aim_m / libm::tan(d.angle_rad).max(f64::MIN_POSITIVE) + d.lead_m
        })
    }
}

/// The circle a turn toward `aim` at its turn rate flies, on the side it lies,
/// once the jet has rolled into it: its centre and radius.
fn turn_circle(state: &FixedWingState, limits: &FixedWingLimits, aim: V2) -> (V2, f64) {
    let to_aim = aim - state.pos.xy();
    let side = wrap_angle(libm::atan2(to_aim.y, to_aim.x) - state.heading).signum();
    let radius = state.speed_mps / limits.turn_rad_s.max(f64::MIN_POSITIVE);
    let forward = v2(libm::cos(state.heading), libm::sin(state.heading));
    let left = v2(-forward.y, forward.x);
    let lag = state.speed_mps * roll_lag_s(state, limits, side, limits.turn_rad_s);
    (
        state.pos.xy() + forward * lag + left * (radius * side),
        radius,
    )
}

/// How far the jet has left to fly to its release point turning in: round
/// its turn toward the aim until the nose comes on it, then straight along
/// that line to the release, its forward throw short of the aim.
fn run_in_m(state: &FixedWingState, limits: &FixedWingLimits, aim: &PassAim) -> f64 {
    let at = aim.point.xy();
    let (centre, radius) = turn_circle(state, limits, at);
    let from_centre = at - centre;
    let d = from_centre.length();
    if d <= radius {
        // Inside its own turn the nose never comes on: it lines up again.
        return f64::INFINITY;
    }
    let to_aim = at - state.pos.xy();
    let side = wrap_angle(libm::atan2(to_aim.y, to_aim.x) - state.heading).signum();
    // The nose comes on where the line to the aim leaves the circle.
    let tangent = libm::atan2(from_centre.y, from_centre.x) - side * libm::acos(radius / d);
    let jet = state.pos.xy() - centre;
    let turn =
        (side * wrap_angle(tangent - libm::atan2(jet.y, jet.x))).rem_euclid(core::f64::consts::TAU);
    let straight = libm::sqrt(d * d - radius * radius) - aim.ends_short_m();
    turn * radius + straight.max(0.0)
}

/// Fly one tick of lining up (`plan` is lining up or turning in and does not
/// commit this tick): move it to the phase it flies and return the line it
/// flies, a point on it and its heading.
fn line_up(
    plan: &mut PassPlan,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    run: &PassAim,
    guard: Option<f64>,
    dt: f64,
) -> (V2, f64) {
    let (ends_short, dives) = (run.ends_short_m(), run.dive.is_some());
    let aim = plan.aim.xy();
    let to_aim = aim - state.pos.xy();
    let distance = to_aim.length();
    let bearing = libm::atan2(to_aim.y, to_aim.x);
    let nose_on = nose_on_bearing(state.heading, bearing, limits.nose_on_rad);
    let past_release = distance <= ends_short;
    let speed = state.speed_mps.max(1.0);
    let (centre, radius) = turn_circle(state, limits, aim);
    let from_centre = aim - centre;
    // Out along the line the run will come back on, and where on it the run
    // must end. Where it fits, the line is straight away from the aim on the
    // jet's side, and the run ends at its release point or sooner where the
    // edge guard takes it. It fits when flying out along it, a full run-in
    // and a turn radius, and the turn round at its end keep the margin
    // inside every edge, turning as tight as it can: it turns round at the
    // rate it turns at by choice, and the edge guard pulls harder where
    // that would not keep the margin. An edge the line runs along, however
    // near the aim, never bends it. Where it doesn't fit it is a fixed line
    // into the map, so the run comes back at the edge from inside rather
    // than along it. The jet flies out two turn radii to the side of that
    // line so that turning in lands it on it, and turns in only once out on
    // that leg, where the turn toward the aim is a turn away from the edge.
    // Its settle is the lag of its roll out of the turn in onto the line
    // (half its roll-in, as rolling into it costs) and, for a gun or
    // missile, two seconds' flight to fire along. A bomb needs no run to
    // fire along: it leaves wherever it would land on the aim; nor does a
    // dive, which fires on its way down.
    let fire_along_s = if plan.weapon == PassWeapon::Bomb || dives {
        0.0
    } else {
        2.0
    };
    let settle = speed * (fire_along_s + level_roll_lag_s(limits));
    let away = bearing + core::f64::consts::PI;
    let away_dir = v2(libm::cos(away), libm::sin(away));
    let straight_ends = ends_short.max(guard_short_of(aim, -away_dir, state, limits, bounds, dt));
    let tightest = tightest_radius(state, limits);
    let straight_fits = bounds.edges(aim).into_iter().all(|(room, outward)| {
        let along = away_dir.dot(outward).max(0.0);
        room >= limits.edge_margin_m + tightest + (settle + straight_ends) * along
    });
    let free_run_in = settle + ends_short;
    let edge_line = (!straight_fits)
        .then(|| inward_from(aim, limits.edge_margin_m + tightest + free_run_in, bounds))
        .flatten();
    let (out, through, on_leg, ends) = match edge_line {
        None => (away, aim, true, straight_ends),
        Some(way) => {
            let out = libm::atan2(way.y, way.x);
            let out_dir = v2(libm::cos(out), libm::sin(out));
            let left = v2(-out_dir.y, out_dir.x);
            let side = if left.dot(-to_aim) < 0.0 { -1.0 } else { 1.0 };
            let through = aim + left * (2.0 * radius * side);
            let on_leg = wrap_angle(state.heading - out).abs() < core::f64::consts::FRAC_PI_4
                && (state.pos.xy() - through).dot(left).abs() < radius;
            let guarded_at = guard_short_of(aim, -out_dir, state, limits, bounds, dt);
            (out, through, on_leg, ends_short.max(guarded_at))
        }
    };
    let run_in = settle + ends;
    // The edge guard breaking the other way breaks a turn in like a run:
    // the jet lines up again rather than riding the edge. Breaking the same
    // way only tightens it.
    let turning = wrap_angle(bearing - state.heading);
    plan.phase = if guard.is_some_and(|turn| turn * turning < 0.0) {
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
    (through, heading)
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
/// would take it, whichever comes first. The run commits the moment the nose comes on
/// the aim, short of the release point and clear of the edge guard, however
/// much run-in is left. A bomb's release cue comes, in whatever phase the
/// jet is, on the tick whose `dt` step carries it to where a drop would
/// land stops closing on the aim, if that lies within the aim's reach
/// (how far the bomb can glide onto it). A run is over then, whether or not it
/// released, or when the edge guard takes it: the jet lines up again at
/// once, rolling into its break from wings level.
///
/// Off each bomb run the jet climbs back toward its cruise height, from the
/// tick after its release. It dives to the run's height from wherever the
/// climb has reached (`agl_m`, its height above the ground under it) once,
/// turning in, it is near enough its release point that a descent at its
/// climb rate just gets it down there, or as it commits, whichever comes
/// first. A turn in it abandons climbs again.
///
/// A run fired along the nose flies its lining up at its dive's entry
/// height, and the run itself level there until the aim lies its dive angle
/// below it. Then it points its nose at the aim, so the weapon fixed along
/// the nose bears on it, until pulling out at its hardest would only just
/// keep it above the run's seeded height (`PassPlan::agl`); it pulls out
/// and holds that height until the run is over.
pub fn advance(
    plan: &mut PassPlan,
    state: &FixedWingState,
    agl_m: f64,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    aim: &PassAim,
    dt: f64,
) -> (Intent, bool) {
    let (through, heading, cue) = advance_phase(plan, state, limits, bounds, aim, dt);
    if let Some(dive) = aim.dive {
        let nose_at = dive_nose(plan, state, agl_m, limits, aim.point, dive, dt);
        // In the run it climbs no more: pushing over from a climb would
        // swing its nose through more than its dive.
        let agl = match (plan.phase, plan.dive) {
            (_, Some(Dive::PulledOut)) => plan.agl,
            (PassPhase::Run, _) => dive.entry_agl_m.min(agl_m),
            _ => dive.entry_agl_m,
        };
        let intent = Intent::Line {
            through,
            heading,
            agl,
            nose_at,
        };
        return (intent, cue);
    }
    plan.dive = None;
    match plan.phase {
        // It pulls up only after the release: a bomb leaving a climbing jet
        // would carry the climb and fall long.
        PassPhase::LineUp if !cue => plan.run_agl = None,
        PassPhase::LineUp => {}
        PassPhase::TurnIn | PassPhase::Run if plan.run_agl.is_none() => {
            let descent_s = (agl_m - plan.agl).max(0.0) / limits.climb_mps.max(f64::MIN_POSITIVE);
            let near = run_in_m(state, limits, aim) <= state.speed_mps * descent_s;
            if near || plan.phase == PassPhase::Run {
                plan.run_agl = Some(plan.agl.min(agl_m).max(limits.floor_agl_m));
            }
        }
        PassPhase::TurnIn | PassPhase::Run => {}
    }
    let intent = Intent::Line {
        through,
        heading,
        agl: plan.run_agl.unwrap_or(limits.cruise_agl_m),
        nose_at: None,
    };
    (intent, cue)
}

/// Move a dive on by one tick and return where it points the nose: at the
/// aim from the tick the aim lies the dive's angle below the point it would
/// come onto that line pushing over, until pulling out at its hardest would
/// only just keep it above the run's height; nothing level or pulled out.
/// Every new run starts level.
fn dive_nose(
    plan: &mut PassPlan,
    state: &FixedWingState,
    agl_m: f64,
    limits: &FixedWingLimits,
    aim: V3,
    dive: DiveRun,
    dt: f64,
) -> Option<NoseAt> {
    if plan.phase != PassPhase::Run {
        plan.dive = Some(Dive::Level);
        return None;
    }
    // It pushes over half the ground pushing over from its pitch now covers
    // early, so its dive comes onto the line through the aim at its angle.
    let swing = dive.angle_rad + state.pitch.max(0.0);
    let lead = 0.5 * state.speed_mps * pitch_swing_s(limits, swing);
    let to = aim - state.pos;
    let below = libm::atan2(-to.z, to.xy().length() - lead);
    let mut now = plan.dive.unwrap_or(Dive::Level);
    if now == Dive::Level && below >= dive.angle_rad {
        now = Dive::Diving;
    }
    if now == Dive::Diving && agl_m - plan.agl <= pull_out_m(state, limits, dt) {
        now = Dive::PulledOut;
    }
    plan.dive = Some(now);
    (now == Dive::Diving).then_some(NoseAt {
        point: aim,
        lowest_agl_m: plan.agl,
    })
}

/// One tick of the runs' phases: the line flown (a point on it and its
/// heading), and whether this is the tick the weapon is released.
fn advance_phase(
    plan: &mut PassPlan,
    state: &FixedWingState,
    limits: &FixedWingLimits,
    bounds: &Bounds,
    aim: &PassAim,
    dt: f64,
) -> (V2, f64, bool) {
    let guard = edge_guard(state, limits, bounds, dt);
    let guarded = guard.is_some();
    let landing = Landing::of(state, aim, dt);
    // A bomb may leave on the tick where it would land stops closing on the
    // aim, if that lies within reach: a run drops level with the aim, a
    // turn that swings the nose onto it a little too close for a level run
    // drops as the swing ends, and the edge guard's break drops as it takes
    // the jet. A bomb held then (for a friendly in its blast, say) waits for
    // the next approach.
    let opening = guarded || landing.after >= landing.now;
    let drop =
        plan.weapon == PassWeapon::Bomb && plan.closing && opening && landing.now <= aim.reach_m;
    plan.closing = !opening;
    let run = aim;
    let ends_short = run.ends_short_m();
    let aim = run.point;
    if plan.phase != PassPhase::Run {
        let to_aim = aim.xy() - state.pos.xy();
        let bearing = libm::atan2(to_aim.y, to_aim.x);
        plan.aim = aim;
        let nose_on = nose_on_bearing(state.heading, bearing, limits.nose_on_rad);
        // Short of where it must have its nose on for this run.
        let short = to_aim.length() > ends_short;
        if nose_on && short && !guarded {
            plan.phase = PassPhase::Run;
            plan.heading = bearing;
        } else {
            let (through, heading) = line_up(plan, state, limits, bounds, run, guard, dt);
            if drop {
                plan.phase = PassPhase::LineUp;
            }
            return (through, heading, drop);
        }
    }
    // The run is over once where its round would land comes no nearer the
    // aim: a bomb's drop point, or the jet itself for a gun or missile.
    if opening {
        plan.phase = PassPhase::LineUp;
    }
    (plan.aim.xy(), plan.heading, drop)
}

/// How far from the aim a round released after this tick's step lands
/// (`now`), and one released a step later (`after`), holding the turn the
/// jet is in. It flies on with the jet's velocity as it falls, so it lands
/// its throw (`aim - release.at`, nothing for a round fired at the aim)
/// ahead of where it leaves the jet.
struct Landing {
    now: f64,
    after: f64,
}

impl Landing {
    fn of(state: &FixedWingState, aim: &PassAim, dt: f64) -> Self {
        let throw = aim.point.xy() - aim.release.at;
        let step = state.speed_mps.max(1.0) * dt;
        let turn = state.turn_rate_rad_s * dt;
        let miss = |n: u32| {
            let flown = (1..=n)
                .map(|k| {
                    let heading = state.heading + turn * f64::from(k);
                    v2(libm::cos(heading), libm::sin(heading)) * step
                })
                .fold(v2(0.0, 0.0), |sum, leg| sum + leg);
            let (sin, cos) = libm::sincos(turn * f64::from(n));
            let thrown = v2(throw.x * cos - throw.y * sin, throw.x * sin + throw.y * cos);
            (state.pos.xy() + flown + thrown - aim.point.xy()).length()
        };
        Self {
            now: miss(1),
            after: miss(2),
        }
    }
}
