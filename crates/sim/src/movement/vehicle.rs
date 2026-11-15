//! A vehicle's tick on its route: steer toward the next waypoint
//! ([`drive`]), wait for traffic, learn the bodies it meets, and roll only
//! where true geometry lets the hull go, shoving what it can ([`push`]).
use contract::map::MoverClass;
use contract::observation::MoveState;

use crate::math::{Obb2, V2};
use crate::units::Unit;
use crate::world::WorldGeometry;

use super::traffic::in_the_way;
use super::{
    arrive, drive, may_advance, push, MovementContext, Shove, SideGeometry, ENCOUNTER_RANGE_M,
    PROGRESS_EPSILON_M,
};

/// Where `motion`'s step from `current` lands on true geometry: free, with
/// the bodies it shoves and the ground under it; or held, by the body that
/// stops it (one it cannot shove, or one whose shove is jammed), or by
/// ground it cannot cross (`None`).
fn land(
    ctx: &MovementContext,
    units: &[Unit],
    i: usize,
    current: &Obb2,
    motion: &drive::Motion,
) -> Result<(V2, Vec<Shove>, crate::world::Surface), Option<Obb2>> {
    let next = current.center + motion.heading * motion.step;
    let hull = Obb2 {
        center: next,
        yaw: motion.yaw,
        half: current.half,
    };
    let met = push::meet(ctx.world, &hull, current, units[i].ground().push);
    if let Some(body) = met.solid.and_then(|id| ctx.world.prop(id)) {
        return Err(Some(body.footprint()));
    }
    let turn = ctx.rules.pushing.turn_deg_per_m.to_radians();
    let pusher = push::Pusher {
        unit: i,
        hull,
        heading: motion.heading,
    };
    let mut shoved = Vec::new();
    for prop in &met.shoved {
        match push::shove(ctx.world, units, pusher, prop, turn) {
            Some(s) => shoved.push(s),
            None => return Err(Some(prop.footprint())),
        }
    }
    let ground = ctx
        .world
        .surface_at(next.x, next.y)
        .filter(|s| s.traversable)
        .ok_or(None)?;
    Ok((next, shoved, ground))
}

pub(super) fn step_vehicle(
    ctx: &MovementContext,
    units: &mut [Unit],
    i: usize,
    sides: &mut [SideGeometry; 3],
    shoves: &mut Vec<Shove>,
    traffic: &[Option<Obb2>],
) {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    units[i].reversing = false;
    if !may_advance(ctx, &mut units[i]) {
        units[i].manoeuvre = None;
        // With no orders, tracks pivot a gun fixed in the hull onto its
        // target (D8); wheels cannot turn on the spot, so they hold fire.
        let unit = &mut units[i];
        let automatic = unit
            .reach
            .face
            .filter(|_| unit.orders.is_empty() && unit.ground().drive.is_some_and(|d| d.tracked));
        unit.turn_to = automatic.or(unit.turn_to);
        if units[i].route.is_none() && units[i].turn_to.is_some() {
            drive::pivot(ctx, &mut units[i], dt, automatic.is_none(), traffic);
        } else {
            units[i].drive_speed_mps = 0.0;
        }
        return;
    }
    if drive::prune(ctx, sides, traffic, &mut units[i]) {
        arrive(&mut units[i]);
        return;
    }
    let unit = &units[i];
    let target = unit.route.as_ref().unwrap()[0];
    let here = unit.position.xy();
    let footing = ctx.ground.footing(ctx.world, here, &ctx.rules.ground);
    let speed = footing.map_or(0.0, |g| unit.ground().speed(g));
    let before_manoeuvre = unit.manoeuvre;
    let mut motion = drive::steer(ctx.world, &mut units[i], target, speed, dt, traffic);
    let unit = &units[i];
    let mut next = here + motion.heading * motion.step;
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let current = Obb2 {
        center: here,
        yaw: unit.yaw,
        half,
    };
    let push = unit.ground().push;
    // Box against box (L8): a body it cannot shove stops it; the bodies it
    // can shove slow it by the heaviest's class ratio and slide aside (Q2).
    let hull_at = |center: V2, yaw: f64| Obb2 { center, yaw, half };
    let mut met = push::meet(ctx.world, &hull_at(next, motion.yaw), &current, push);
    if met.solid.is_none() && !met.shoved.is_empty() {
        let heaviest = met
            .shoved
            .iter()
            .map(|p| p.body.weight_class.rank())
            .max()
            .unwrap_or(0);
        // Shoving limits the target speed, not the already ramped velocity:
        // multiplying carried speed every tick would compound the slowdown.
        units[i].manoeuvre = before_manoeuvre;
        motion = drive::steer(
            ctx.world,
            &mut units[i],
            target,
            speed * push.shove_speed(heaviest),
            dt,
            traffic,
        );
        next = here + motion.heading * motion.step;
        met = push::meet(ctx.world, &hull_at(next, motion.yaw), &current, push);
    }
    // Traffic, whatever its side (Q14): a vehicle waits for whatever it
    // would run into; live vehicles are never shoved (Q15).
    let unit = &units[i];
    let mut blocker = motion
        .blocker
        .or_else(|| in_the_way(units, i, next, motion.yaw));

    // A detouring tank must not pivot through its neighbour, nor through
    // a body. Give the turn room instead of deadlocking: back off from
    // traffic; from a body, roll to where the whole turn clears.
    let room = (unit.ground().drive.is_some_and(|d| d.tracked)
        && motion.yaw != unit.yaw
        && stationary_yaw(ctx.world, units, i, motion.yaw) == unit.yaw)
        .then(|| match (blocker, met.solid) {
            (Some(_), _) => Some(drive::away(unit) * f64::INFINITY),
            (None, Some(_)) => drive::room_to_turn(ctx.world, unit, drive::away(unit), |at| {
                drive::yaw_toward(at, target, unit.direction())
            }),
            (None, None) => None,
        })
        .flatten();
    if let Some(room) = room {
        let back = drive::give_space(unit, room, speed, dt);
        let at = here + back.heading * back.step;
        let behind = push::meet(ctx.world, &hull_at(at, back.yaw), &current, push);
        let traffic = in_the_way(units, i, at, back.yaw);
        if behind.solid.is_none() && behind.shoved.is_empty() && traffic.is_none() {
            motion = back;
            next = at;
            blocker = None;
        }
    }
    if let Some(b) = blocker {
        let held_yaw = stationary_yaw(ctx.world, units, i, motion.yaw);
        let unit = &mut units[i];
        unit.state = MoveState::Waiting;
        unit.blocker = Some(b);
        unit.yaw = held_yaw;
        unit.drive_speed_mps = 0.0;
        return;
    }
    let unit = &mut units[i];
    unit.state = MoveState::Moving;
    unit.blocker = None;

    // True geometry decides; an obstacle met here becomes known to the side.
    let radius = unit.footprint_radius(ctx.rules.physics.soldier_radius_m);
    let side = &mut sides[unit.side.index()];
    for prop in ctx.world.props_near(next, radius + ENCOUNTER_RANGE_M) {
        if prop.blocks(MoverClass::Vehicle)
            && prop.footprint().contains(next, radius + ENCOUNTER_RANGE_M)
        {
            let resting = ctx.world.resting(prop.id, ctx.tick);
            side.learn(prop, ctx.authored, ctx.rules.pushing.relearn_m, resting);
        }
    }
    let mut landed = land(ctx, units, i, &current, &motion);
    // A forward step held at its trailing end (a turn swinging the tail
    // into a jammed body) rolls straight on instead, when that lands free.
    if let Err(Some(body)) = landed {
        let trailing = current.to_local(body.closest(here)).x * drive::gear(&units[i]) < 0.0;
        if trailing && !motion.backwards && motion.yaw != units[i].yaw {
            let straight = drive::Motion {
                yaw: units[i].yaw,
                heading: drive::travel_heading(&units[i]),
                ..motion
            };
            if let Ok(free) = land(ctx, units, i, &current, &straight) {
                motion = straight;
                landed = Ok(free);
            }
        }
    }
    let (next, shoved, ground) = match landed {
        Ok(free) => free,
        Err(held_by) => {
            let held_yaw = stationary_yaw(ctx.world, units, i, motion.yaw);
            let unit = &mut units[i];
            unit.yaw = held_yaw;
            unit.drive_speed_mps = 0.0;
            if let Some(body) = held_by.filter(|_| !motion.backwards) {
                drive::held_ahead(unit, body, target);
            }
            return;
        }
    };
    let unit = &mut units[i];
    shoves.extend(shoved);
    unit.yaw = motion.yaw;
    unit.position = next.with_z(ground.z);
    unit.reversing = motion.backwards && motion.step > 0.0;
    unit.drive_speed_mps = motion.step / dt * if motion.backwards { -1.0 } else { 1.0 };
    if let Some(m) = unit.manoeuvre.as_mut() {
        m.driven_m += motion.step;
    }
    // Tracks backing to clear a pivot may move away from the waypoint.
    // Wheeled shuffling must still close on it, or traffic recovery never runs.
    if motion.making_space && motion.step > 0.0 {
        unit.progress.1 = ctx.tick;
    }

    let route = unit.route.as_mut().unwrap();
    let left = (route[0] - next).length();
    if left < PROGRESS_EPSILON_M.min(motion.step + 1e-9) || left < 1e-6 {
        route.remove(0);
        unit.progress = (f64::INFINITY, ctx.tick);
        if route.is_empty() {
            arrive(unit);
        }
    } else if left < unit.progress.0 - PROGRESS_EPSILON_M {
        unit.progress = (left, ctx.tick);
    }
}

/// A rejected step may still let tracks pivot, but only into a clear pose.
fn stationary_yaw(world: &WorldGeometry, units: &[Unit], i: usize, yaw: f64) -> f64 {
    let unit = &units[i];
    if yaw == unit.yaw || !unit.ground().drive.is_some_and(|d| d.tracked) {
        return unit.yaw;
    }
    let here = unit.ground_footprint().expect("a vehicle has a hull");
    let turned = Obb2 { yaw, ..here };
    let met = push::meet(world, &turned, &here, unit.ground().push);
    // A stationary pivot checks true hulls; translation's traffic buffer
    // would forbid using the clearance reserved by the stopped approach.
    let occupied = met.solid.is_some()
        || !met.shoved.is_empty()
        || units.iter().enumerate().any(|(j, other)| {
            j != i
                && other.ground_footprint().is_some()
                && other.on_field()
                && turned.overlaps(&other.ground_footprint().expect("a vehicle has a hull"))
        });
    if occupied {
        unit.yaw
    } else {
        yaw
    }
}
