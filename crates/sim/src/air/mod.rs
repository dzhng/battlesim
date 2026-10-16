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
use crate::weapons::Target;
use contract::catalog::AltitudeLayer;
use contract::random::{mix, Rng};

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
        .map_or((1.0, 1.0), |a| (a.speed_multiplier, a.fuel_multiplier));
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
    };
    let mut pass_release_now = false;
    let pass_before = unit.air.as_ref().and_then(|air| air.pass);
    let task = orders::task(
        unit,
        ctx.knowledge[unit.side.index()],
        ctx.arsenal,
        pass_before.as_ref(),
    );
    let linked_target = unit.air.as_ref().and_then(|air| air.dogfight_target);
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
            if matches!(unit.orders.front(), Some(UnitOrder::Attack { .. })) {
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
        orders::AirTask::Attack { target, explicit } => {
            let point =
                pass_before.map_or_else(|| target_point(ctx, unit, target), |pass| pass.aim);
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
                let weapons = pass_weapons(ctx, unit);
                let just_released = unit
                    .air
                    .as_ref()
                    .is_some_and(|air| air.pass_release && air.pass.is_some_and(|p| p.released));
                let chosen = unit
                    .air
                    .as_ref()
                    .and_then(|air| air.pass.filter(|_| just_released).map(|p| p.weapon))
                    .or_else(|| pass::choose_weapon(&weapons));
                if let Some(weapon) = chosen {
                    let restart = unit.air.as_ref().is_none_or(|air| {
                        air.pass
                            .is_none_or(|p| p.weapon != weapon || p.target != target)
                    });
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
                    let release = pass_release(ctx, unit, point, weapon);
                    let pass = unit
                        .air
                        .as_mut()
                        .expect("an aircraft's flight state")
                        .pass
                        .as_mut()
                        .unwrap();
                    let was_pull_out = pass.phase == pass::PassPhase::PullOut;
                    let (intent, cue) = pass::advance(pass, &state, &limits, point, &release);
                    pass_release_now = cue.is_some();
                    if was_pull_out && pass.phase == pass::PassPhase::LineUp {
                        let finished_target = pass.target;
                        unit.air.as_mut().expect("an aircraft's flight state").pass = None;
                        if !explicit
                            && matches!(
                                finished_target,
                                Target::Unit(id)
                                    if ctx.knowledge[unit.side.index()].knows_destroyed(id)
                            )
                            && orders::reaction(unit, ctx.knowledge[unit.side.index()])
                                == orders::AirTask::Hold
                        {
                            unit.orders.pop_front();
                            unit.air
                                .as_mut()
                                .expect("an aircraft's flight state")
                                .last_goal = Some(unit.position.xy());
                        }
                    }
                    intent
                } else {
                    unit.air.as_mut().expect("an aircraft's flight state").pass = None;
                    Intent::Orbit { centre: point.xy() }
                }
            }
        }
    };
    let bounds = Bounds::new(v2(0.0, 0.0), v2(ctx.world.width(), ctx.world.depth()));
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
    if flying_to_goal || air.last_goal.is_none() {
        air.last_goal = Some(next.pos.xy());
    }
    air.fuel_s = (fuel_s - dt * fuel_mult).max(0.0);
    unit.state = contract::observation::MoveState::Moving;
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
        Target::Unit(id) => knowledge.track(id).map(|t| (t.layer, t.fixed_wing)),
        Target::Contact(id) => knowledge.contact(id).map(|c| (c.layer, c.fixed_wing)),
        _ => None,
    };
    seen == Some((layer, true))
}

fn pass_weapons(ctx: &MovementContext, unit: &Unit) -> Vec<pass::PassWeapon> {
    let specs = ctx.arsenal.specs(unit.kind);
    let mut out = Vec::new();
    for mount in &unit.mounts {
        let spec = &specs[mount.spec];
        for (slot, &kind) in spec.kinds.iter().enumerate() {
            if !mount.ammo[slot].is_none_or(|n| n > 0) {
                continue;
            }
            let mode = pass::pass_weapon(&ctx.arsenal.weapons[kind].def);
            if let Some(mode) = mode.filter(|mode| !out.contains(mode)) {
                out.push(mode);
            }
        }
    }
    out
}

fn pass_release(
    ctx: &MovementContext,
    unit: &Unit,
    aim: V3,
    weapon: pass::PassWeapon,
) -> crate::flight::Release {
    let kind = unit
        .mounts
        .iter()
        .flat_map(|mount| {
            let spec = &ctx.arsenal.specs(unit.kind)[mount.spec];
            spec.kinds.iter().enumerate().filter_map(|(slot, &kind)| {
                if !mount.ammo[slot].is_none_or(|n| n > 0) {
                    return None;
                }
                (pass::pass_weapon(&ctx.arsenal.weapons[kind].def) == Some(weapon)).then_some(kind)
            })
        })
        .next();
    if let Some(kind) = kind.filter(|_| matches!(weapon, pass::PassWeapon::Bomb)) {
        return crate::flight::release(
            unit.position,
            unit.flight_velocity(),
            ctx.arsenal.weapons[kind]
                .profile
                .gravity(&ctx.arsenal.config),
            aim,
        );
    }
    crate::flight::Release {
        fall_s: 0.0,
        ahead_m: 0.0,
        at: aim.xy(),
    }
}
