//! Fixed-wing flight: the flight law, steering, attack passes, dogfights and
//! edge transit, for every aircraft whose flight model is `fixed_wing`, at
//! its own layer's heights. Rotor aircraft fly in [`crate::movement::air`].

pub mod dogfight;
pub mod edge;
pub mod fixed_wing;
pub mod orders;
pub mod pass;
pub mod steer;

pub use fixed_wing::{fly, velocity, FixedWingLimits, FixedWingState, Steer};
pub use steer::{steer, Bounds, Intent};

use crate::knowledge::SideKnowledge;
use crate::math::{v2, V2, V3};
use crate::movement::MovementContext;
use crate::units::{Motion, Unit, UnitOrder};
use crate::weapons::{Arsenal, Target, Weapon};
use contract::catalog::AltitudeLayer;
use contract::random::{mix, Rng};
use contract::weapons::{Fire, TargetClass};

/// Advance one winged aircraft by the fixed-wing flight law, without
/// consulting the rotor air grid.
pub(crate) fn step_fixed_wing(ctx: &MovementContext, unit: &mut Unit) {
    let Motion::Air(flight) = unit.motion else {
        unreachable!("only aircraft fly");
    };
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let (afterburner, burner, turn_sign, fuel_s) = {
        let air = unit.air.as_ref().expect("an aircraft's flight state");
        (air.afterburner, air.burner, air.turn_sign, air.fuel_s)
    };
    // The afterburner's speed is the type's; the toggle only says which way
    // the ramp runs, so switching it off eases back to cruise.
    let boost = unit
        .unit_type(ctx.rules)
        .capabilities
        .air
        .as_ref()
        .and_then(|a| a.afterburner)
        .map_or((1.0, 1.0), |a| (a.speed_factor, a.fuel_factor));
    let burner_mult = boost.0;
    let fuel_mult = if afterburner { boost.1 } else { 1.0 };
    let cruise = flight.cruise_mps;
    let speed = unit
        .air
        .as_ref()
        .map_or(cruise, |air| air.velocity.xy().length().max(cruise));
    let current_order = unit.orders.front().cloned();
    let goal = current_order.as_ref().and_then(|order| match order {
        UnitOrder::Move(m) | UnitOrder::AttackMove(m) => Some(m.destination),
        _ => None,
    });
    let on_circle = goal.is_some_and(|centre| {
        let radius = speed / flight.turn_rad_s.max(f64::MIN_POSITIVE);
        ((unit.position.xy() - centre).length() - radius).abs() <= radius * 0.08 + 2.0
    });
    if on_circle && !unit.withdrawing {
        unit.orders.pop_front();
        // It goes on circling the goal it reached.
        unit.air
            .as_mut()
            .expect("an aircraft's flight state")
            .last_goal = goal;
        unit.state = if unit.orders.is_empty() {
            contract::observation::MoveState::Idle
        } else {
            contract::observation::MoveState::Moving
        };
    }
    let current_order = unit.orders.front().cloned();
    // While it flies for a goal, an order taken off it leaves it circling
    // where it was.
    let flying_to_goal = matches!(
        current_order,
        Some(UnitOrder::Move(_) | UnitOrder::AttackMove(_))
    );
    let state = FixedWingState {
        pos: unit.position,
        heading: unit.yaw,
        speed_mps: speed,
        burner,
        turn_sign,
    };
    let limits = FixedWingLimits {
        cruise_mps: cruise,
        burner_mult,
        turn_rad_s: flight.turn_rad_s,
        climb_mps: flight.climb_mps,
        ramp_s: ctx.rules.air.fixed_wing.afterburner_ramp_s,
        floor_agl_m: ctx
            .rules
            .air
            .floor_agl_m(unit.layer())
            .expect("the catalog admits fixed-wing flight only in a layer with a floor"),
        edge_margin_m: ctx.rules.air.fixed_wing.edge_margin_m,
        cruise_agl_m: flight.cruise_agl_m,
        nose_on_rad: ctx.rules.air.fixed_wing.nose_on_deg.to_radians(),
    };
    let bounds = Bounds::new(v2(0.0, 0.0), v2(ctx.world.width(), ctx.world.depth()));
    let mut pass_release_now = false;
    let pass_before = unit.air.as_ref().and_then(|air| air.pass);
    let task = orders::task(
        unit,
        ctx.knowledge[unit.side.index()],
        ctx.arsenal,
        pass_before.as_ref(),
    );
    let linked_target = unit.air.as_ref().and_then(|air| air.dogfight_target);
    // While it attacks, by order or answering fire, in passes or a
    // dogfight, where it is is where it circles once the attack ends.
    let attacking = matches!(task, orders::AirTask::Attack { .. });
    let intent = match task {
        // A withdrawing jet only flies for its back edge (air::edge).
        orders::AirTask::Withdraw => {
            unit.air.as_mut().expect("an aircraft's flight state").pass = None;
            match current_order {
                Some(UnitOrder::Move(m)) => Intent::Exit { to: m.destination },
                _ => Intent::Exit {
                    to: unit.position.xy(),
                },
            }
        }
        orders::AirTask::Hold => {
            unit.air.as_mut().expect("an aircraft's flight state").pass = None;
            // An attack, or an attack-move whose target it killed, is over (D15).
            if matches!(
                unit.orders.front(),
                Some(UnitOrder::Attack { .. } | UnitOrder::AttackMove(_))
            ) {
                unit.orders.pop_front();
            }
            Intent::Orbit {
                centre: unit
                    .air
                    .as_ref()
                    .and_then(|air| air.last_goal)
                    .unwrap_or_else(|| unit.position.xy()),
            }
        }
        orders::AirTask::Orbit { centre } => {
            unit.air.as_mut().expect("an aircraft's flight state").pass = None;
            Intent::Orbit { centre }
        }
        orders::AirTask::Attack { target } => {
            // A run flies at the aim it committed to; lining up, at the
            // target's point now.
            let point = pass_before
                .filter(|p| p.phase == pass::PassPhase::Dive && p.target == target)
                .map_or_else(|| target_point(ctx, unit, target), |p| p.aim);
            if dogfightable(ctx.knowledge[unit.side.index()], target, unit.layer()) {
                unit.air.as_mut().expect("an aircraft's flight state").pass = None;
                let ordered = match target {
                    Target::Unit(id) => Some(id),
                    _ => None,
                };
                // It flies at a jet beyond the break range; the dogfight
                // forms once it is inside.
                let in_range = |id| {
                    ctx.knowledge[unit.side.index()]
                        .track(id)
                        .is_some_and(|track| {
                            !dogfight::breaks(
                                crate::math::body_distance(unit.position, track.position),
                                &ctx.rules.air.fixed_wing,
                            )
                        })
                };
                if linked_target.is_none() {
                    if let Some(id) = ordered.filter(|&id| in_range(id)) {
                        dogfight::set_target(unit, id);
                    }
                }
                let chase = unit
                    .air
                    .as_ref()
                    .and_then(|air| air.dogfight_target)
                    .or(ordered);
                dogfight_intent(ctx, unit, &state, chase, point.xy())
            } else {
                let weapons = pass_weapons(ctx, unit, target, point);
                let chosen =
                    pass::choose_weapon(&weapons.iter().map(|(w, _)| *w).collect::<Vec<_>>());
                if let Some(weapon) = chosen {
                    let restart =
                        pass_before.is_none_or(|p| p.weapon != weapon || p.target != target);
                    if restart {
                        let mut rng = Rng::new(mix(ctx.seed ^ u64::from(unit.id.0)));
                        let mut plan = pass::begin(
                            &state,
                            ctx.world.height_at(state.pos.x, state.pos.y).unwrap_or(0.0),
                            &limits,
                            point,
                            weapon,
                            &ctx.rules.air.fixed_wing,
                            &mut rng,
                        );
                        plan.target = target;
                        unit.air.as_mut().expect("an aircraft's flight state").pass = Some(plan);
                    }
                    let release = weapons
                        .iter()
                        .find(|(w, _)| *w == weapon)
                        .map_or_else(|| at_the_aim(point), |(_, release)| *release);
                    let lost = !knows_target(ctx.knowledge[unit.side.index()], target);
                    let pass = unit
                        .air
                        .as_mut()
                        .expect("an aircraft's flight state")
                        .pass
                        .as_mut()
                        .expect("a pass was begun");
                    // A run at a target its side has lost is over.
                    if lost {
                        pass.phase = pass::PassPhase::LineUp;
                    }
                    let (intent, cue) =
                        pass::advance(pass, &state, &limits, &bounds, point, &release, dt);
                    pass_release_now = cue;
                    intent
                } else {
                    unit.air.as_mut().expect("an aircraft's flight state").pass = None;
                    Intent::Orbit { centre: point.xy() }
                }
            }
        }
    };
    let mut command = steer(&intent, &state, &limits, &bounds, dt);
    command.burner = afterburner;
    let next = fly(
        state,
        command,
        &limits,
        &|p| ctx.world.height_at(p.x, p.y).unwrap_or(0.0),
        dt,
    );
    let vertical = (next.pos.z - unit.position.z) / dt;
    unit.position = next.pos;
    unit.yaw = next.heading;
    unit.drive_speed_mps = next.speed_mps;
    let air = unit.air.as_mut().expect("an aircraft's flight state");
    let horizontal = velocity(&next);
    air.velocity = crate::math::v3(horizontal.x, horizontal.y, vertical);
    air.burner = next.burner;
    air.turn_sign = next.turn_sign;
    air.pass_release = pass_release_now;
    if flying_to_goal || attacking || air.last_goal.is_none() {
        air.last_goal = Some(next.pos.xy());
    }
    air.fuel_s = (fuel_s - dt * fuel_mult).max(0.0);
    unit.state = contract::observation::MoveState::Moving;
}

/// End whatever run or fight a jet is in: no attack run, release cue or
/// dogfight link survives an order it is given (order application, and
/// turning for home on low fuel). If its order still attacks, it lines up
/// afresh from where it is.
pub(crate) fn break_off(unit: &mut Unit) {
    if let Some(air) = unit.air.as_mut() {
        air.pass = None;
        air.pass_release = false;
        air.dogfight_target = None;
    }
}

/// Whether the side still knows where `target` is: a ground point always,
/// a unit while it holds a track of it, a contact while it lives.
fn knows_target(knowledge: &SideKnowledge, target: Target) -> bool {
    match target {
        Target::Unit(id) => knowledge.track(id).is_some(),
        Target::Contact(id) => knowledge.contact(id).is_some(),
        Target::Ground(_) | Target::Round(_) => true,
    }
}

/// How a jet flies its dogfight against `target`, as its side knows it;
/// circling `fallback` while it knows nothing of one.
fn dogfight_intent(
    ctx: &MovementContext,
    unit: &Unit,
    state: &FixedWingState,
    target: Option<contract::ids::UnitId>,
    fallback: V2,
) -> Intent {
    target
        .and_then(|target| ctx.knowledge[unit.side.index()].track(target))
        .map(|track| {
            dogfight::intent(
                state,
                track,
                mix(ctx.seed ^ u64::from(unit.id.0)),
                &ctx.rules.air.fixed_wing,
            )
        })
        .unwrap_or(Intent::Orbit { centre: fallback })
}

fn target_point(ctx: &MovementContext, unit: &Unit, target: Target) -> V3 {
    match target {
        Target::Unit(id) => ctx.knowledge[unit.side.index()]
            .track(id)
            .map(|track| track.position)
            .or_else(|| unit.pursuit.map(|p| p.to.with_z(unit.position.z)))
            .unwrap_or(unit.position),
        Target::Ground(point) => point,
        // No order names a round; a mount's lock is never a flight goal.
        Target::Round(_) => unit.position,
        Target::Contact(contact) => ctx.knowledge[unit.side.index()]
            .contact(contact)
            .map(|c| c.center)
            .or_else(|| unit.pursuit.map(|p| p.to.with_z(unit.position.z)))
            .unwrap_or(unit.position),
    }
}

/// What the side knows `target` to be: its layer and, for an identified
/// unit, its class. A contact is an area in the layer its cause flew in,
/// which no class bars; a ground point, or a unit lost from view, is a
/// point on the ground. Read from what was seen or heard, never from
/// height.
fn known_target(knowledge: &SideKnowledge, target: Target) -> (AltitudeLayer, Option<TargetClass>) {
    match target {
        Target::Unit(id) => knowledge.track(id).map(|t| (t.layer, Some(t.class))),
        Target::Contact(id) => knowledge.contact(id).map(|c| (c.layer, None)),
        _ => None,
    }
    .unwrap_or((AltitudeLayer::Ground, None))
}

/// Whether the side knows `target` as a winged aircraft in `layer`, the
/// pursuer's own: the only kind a fixed-wing aircraft dogfights (D29c);
/// anything else it attacks in passes. Read from what was seen or heard of
/// it, never from height.
pub(crate) fn dogfightable(
    knowledge: &SideKnowledge,
    target: Target,
    layer: AltitudeLayer,
) -> bool {
    let seen = match target {
        Target::Unit(id) => knowledge
            .track(id)
            .map(|t| (t.layer, t.class == TargetClass::AirFixedWing)),
        Target::Contact(id) => knowledge.contact(id).map(|c| (c.layer, c.fixed_wing)),
        _ => None,
    };
    seen == Some((layer, true))
}

/// What the jet can fly a pass with at `aim`: each pass weapon one of its
/// rows with rounds left engages the target with, as its side knows it,
/// and the release it flies to; at a bare ground point, only a row the fire
/// rule lets fire at one. A bomb's is where it must leave the jet to
/// fall onto the aim; a bomb that can't fall to the aim from here (the aim
/// is above the jet) flies no run. Any other row fires at the aim itself.
fn pass_weapons(
    ctx: &MovementContext,
    unit: &Unit,
    target: Target,
    aim: V3,
) -> Vec<(pass::PassWeapon, crate::flight::Release)> {
    let (layer, class) = known_target(ctx.knowledge[unit.side.index()], target);
    let mut out: Vec<(pass::PassWeapon, crate::flight::Release)> = Vec::new();
    let point = matches!(target, Target::Ground(_));
    for (weapon, _) in engaging_rows(ctx.arsenal, unit, layer, class)
        .filter(|(weapon, _)| !point || crate::weapons::area_capable(&weapon.def))
    {
        let Some(mode) = pass::pass_weapon(&weapon.def) else {
            continue;
        };
        if out.iter().any(|(m, _)| *m == mode) {
            continue;
        }
        let release = match mode {
            pass::PassWeapon::Bomb => crate::flight::release(
                unit.position,
                unit.flight_velocity(),
                weapon.profile.gravity(&ctx.arsenal.config),
                aim,
            ),
            pass::PassWeapon::Gun | pass::PassWeapon::GroundMissile => Some(at_the_aim(aim)),
        };
        if let Some(release) = release {
            out.push((mode, release));
        }
    }
    out
}

/// What the jet can attack a target of `class` in `layer` with: each row
/// it carries with rounds left that engages such a target there, and how
/// it fires there. The one test of what a jet can attack.
pub(crate) fn engaging_rows<'a>(
    arsenal: &'a Arsenal,
    unit: &'a Unit,
    layer: AltitudeLayer,
    class: Option<TargetClass>,
) -> impl Iterator<Item = (&'a Weapon, Fire)> + 'a {
    let specs = arsenal.specs(unit.kind);
    unit.mounts.iter().flat_map(move |mount| {
        specs[mount.spec]
            .kinds
            .iter()
            .enumerate()
            .filter(|&(slot, _)| mount.has_rounds(slot))
            .filter_map(move |(_, &kind)| {
                let weapon = &arsenal.weapons[kind];
                weapon.def.engages(layer, class).map(|fire| (weapon, fire))
            })
    })
}

/// The release of a row fired at the aim itself, not dropped short of it.
fn at_the_aim(aim: V3) -> crate::flight::Release {
    crate::flight::Release {
        fall_s: 0.0,
        ahead_m: 0.0,
        at: aim.xy(),
    }
}
