//! One side-known plan for an entry and its companions' gathering movement.
use crate::formation::{self, Member, Slot};
use crate::math::{v2, V2};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::navigation::{Journey, Leg, Plan, NAV_CELL_M};
use crate::units::{MoveOrder, Unit, UnitOrder};
use contract::command::{
    BuildingEntry, BuildingPlacement, BuildingPreviewRequest, MoveDestination, OrderError,
    RoutePolicy,
};
use contract::ids::UnitId;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
struct EntryPoint {
    at: V2,
    outward: V2,
}

#[derive(Clone, Copy)]
pub(crate) enum EntryAction {
    Keep,
    Reassert,
    Route,
}

#[derive(Clone)]
pub(crate) struct BuildingPlan {
    pub placement: BuildingPlacement,
    pub entry_action: EntryAction,
}

struct Candidate {
    unit: UnitId,
    entry: EntryPoint,
    route: Vec<V2>,
    distance: f64,
    ordinal: usize,
    action: EntryAction,
    held: bool,
}
struct RouteJob {
    candidate: Candidate,
    journey: Journey,
}

fn points(ctx: &MovementContext, known: &SideGeometry, owner: u32, from: V2) -> Vec<EntryPoint> {
    let Some(definition) = ctx.world.building(owner) else {
        return Vec::new();
    };
    let parts: Vec<_> = ctx
        .world
        .structure_parts(owner)
        .into_iter()
        .filter_map(|id| known.prop(ctx.world, ctx.authored, id))
        .collect();
    let mut points = Vec::new();
    for part in &parts {
        let Some(reference) = ctx
            .world
            .authored_prop(part.id)
            .and_then(|id| definition.parts.iter().find(|p| p.prop == id))
        else {
            continue;
        };
        for edge in definition
            .geometry
            .edges
            .iter()
            .filter(|e| e.part == reference.part && e.exposed)
        {
            let (normal, along, reach, _) =
                edge.facade.axes([part.half.x, part.half.y, part.half.z]);
            let n = v2(normal[0], normal[1]);
            let a = v2(along[0], along[1]);
            let projected = part
                .footprint()
                .to_local(from)
                .dot(a)
                .clamp(edge.span_m[0], edge.span_m[1]);
            let span = edge.span_m[1] - edge.span_m[0];
            let count = (span / NAV_CELL_M).ceil() as usize;
            let samples = std::iter::once(projected)
                .chain((0..=count).map(|k| edge.span_m[0] + span * k as f64 / count.max(1) as f64));
            for t in samples {
                let at = part.center
                    + (n * (reach + ctx.rules.garrison.entry_distance_m / 2.0) + a * t)
                        .rotated(part.yaw);
                if !parts.iter().any(|p| p.footprint().contains(at, 0.0))
                    && !points
                        .iter()
                        .any(|p: &EntryPoint| (p.at - at).length() < 1e-9)
                {
                    points.push(EntryPoint {
                        at,
                        outward: n.rotated(part.yaw),
                    });
                }
            }
        }
    }
    points
}

fn member(ctx: &MovementContext, u: &Unit) -> Member {
    Member {
        id: u.id,
        position: u.position.xy(),
        yaw: u.yaw,
        radius: u.hull.map_or_else(
            || crate::cover::area_radius(ctx.rules, u),
            |half| half.xy().length(),
        ),
    }
}

/// Every queued action must preserve the hold, even if an earlier attack expires.
fn entry_work_keeps_building(unit: &Unit, owner: u32) -> bool {
    for order in &unit.orders {
        match order {
            UnitOrder::Exit => return false,
            UnitOrder::Garrison { building, .. } if *building != owner => return false,
            order if order.movement().is_some() => return false,
            _ => {}
        }
    }
    true
}

pub(crate) fn occupy(
    ctx: &MovementContext,
    known: &SideGeometry,
    source: &[Unit],
    request: &BuildingPreviewRequest,
) -> Result<BuildingPlan, OrderError> {
    let mut ids = request.units.clone();
    ids.sort();
    ids.dedup();
    let side = source[ids[0].0 as usize].side;
    let world = ctx.world.planning_snapshot(
        |p| known.belief(p, ctx.authored),
        known.standing().values().cloned(),
    );
    let local = MovementContext {
        world: &world,
        ground: ctx.knowledge[side.index()].ground().layer(),
        ..*ctx
    };
    let ctx = &local;
    let mut owner = ctx.world.remembered_structure_owner(request.building);
    let mut target = known.prop(ctx.world, ctx.authored, request.building);
    if target.is_none() {
        target = ctx
            .world
            .historical_building_parts(owner)
            .iter()
            .filter_map(|&id| known.prop(ctx.world, ctx.authored, id))
            .next();
    }
    let Some(mut target) = target.filter(|p| ctx.world.building_of(p.id).is_some()) else {
        return Err(OrderError::NotABuilding);
    };
    owner = ctx.world.remembered_structure_owner(target.id);
    target = known.prop(ctx.world, ctx.authored, owner).unwrap_or(target);
    target.id = owner;
    target.center = ctx
        .world
        .structure_footprint(owner)
        .map_or(target.center, |footprint| footprint.center);
    let mut allowance = ctx.rules.navigation.move_validation_work as u64;
    let mut origins = source.to_vec();
    let mut eligible = vec![true; ids.len()];
    let mut prefix_unproven = false;
    if request.queued {
        let slots: Vec<_> = ids
            .iter()
            .map(|&id| {
                let u = &source[id.0 as usize];
                Slot {
                    id,
                    point: Some(
                        u.orders
                            .back()
                            .and_then(|o| match o {
                                UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                                    Some(m.destination)
                                }
                                UnitOrder::Garrison { approach, .. } => Some(*approach),
                                _ => None,
                            })
                            .unwrap_or(u.position.xy()),
                    ),
                }
            })
            .collect();
        // Reserve most work for the new action rather than spending it on a prefix.
        let mut prefix_work = allowance / 3;
        let before = prefix_work;
        let proof = movement::certify_orders(
            ctx,
            source,
            known,
            movement::ProofRequest {
                slots: &slots,
                orders: None,
                queued: true,
                reserve_repair: true,
            },
            &mut prefix_work,
        );
        prefix_unproven = proof.unproven || !proof.stable;
        allowance -= before - prefix_work;
        for (i, id) in ids.iter().enumerate() {
            eligible[i] = proof.stable && proof.arrivals[i].is_some();
            if eligible[i] {
                origins[id.0 as usize] = proof.arrivals[i].as_ref().unwrap().clone();
            }
        }
    }
    // A finite predecessor with no proven origin may still supply the
    // shortest entrant. Indefinite attacks are known noncandidates instead.
    let unresolved_competitor = prefix_unproven
        && ids.iter().enumerate().any(|(i, &id)| {
            !eligible[i]
                && source[id.0 as usize]
                    .orders
                    .iter()
                    .all(|o| !matches!(o, UnitOrder::Attack { .. }))
                && super::validate(
                    ctx.world,
                    source,
                    side,
                    &[id],
                    Some(target.clone()),
                    false,
                    ctx.rules,
                )
                .is_ok()
        });
    let centre = ids.iter().fold(v2(0.0, 0.0), |p, id| {
        p + origins[id.0 as usize].position.xy()
    }) * (1.0 / ids.len() as f64);
    let selected_holder = ids.iter().find_map(|&id| {
        let actual = &source[id.0 as usize];
        let i = ids.iter().position(|&u| u == id).unwrap();
        let u = if request.queued && eligible[i] {
            &origins[id.0 as usize]
        } else {
            actual
        };
        let g = u.garrison.as_ref().filter(|g| g.building == owner)?;
        let held = matches!(g.phase, super::Phase::Inside);
        let entering = matches!(g.phase, super::Phase::Entering(_));
        if request.queued {
            if !eligible[i] && !entry_work_keeps_building(actual, owner) {
                return None;
            }
            super::current_entry(u, owner).map(|at| {
                (
                    id,
                    at,
                    EntryAction::Keep,
                    held && super::held_entry(actual, owner).is_some(),
                )
            })
        } else if held || entering {
            let action = if super::current_entry(actual, owner).is_some()
                && entry_work_keeps_building(actual, owner)
            {
                EntryAction::Keep
            } else {
                EntryAction::Reassert
            };
            Some((id, g.entry, action, held))
        } else {
            None
        }
    });
    // Replacement supersedes selected deferred claims. Actual occupants still
    // reserve their building; queued intents keep every original claim.
    let mut validation_source = source.to_vec();
    if !request.queued {
        for &id in &ids {
            validation_source[id.0 as usize].orders.clear();
        }
    }
    let mut proof_source = source.to_vec();
    if let Some((id, at, EntryAction::Reassert, _)) = selected_holder {
        proof_source[id.0 as usize].enqueue(
            UnitOrder::Garrison {
                building: owner,
                approach: at,
            },
            false,
        );
    }
    if let Some((id, _, EntryAction::Keep, false)) = selected_holder {
        let i = ids.iter().position(|&u| u == id).unwrap();
        if request.queued && !eligible[i] {
            // Certify the current entry, whose later indefinite attack does
            // not move its holder. Application keeps the original suffix.
            proof_source[id.0 as usize].orders.truncate(1);
        }
    }
    let source = proof_source.as_slice();
    let mut nav = known.planning_snapshot();
    let grid = nav.grid(ctx.world, ctx.authored);
    let mut candidates = Vec::new();
    // Each search spends from one finite selection-wide allowance. Completed
    // shortest-policy paths are ranked by physical length, never travel time.
    let mut route_work = allowance / 3;
    let route_before = route_work;
    let mut jobs = Vec::new();
    if selected_holder.is_none() && !unresolved_competitor && target.body.garrison {
        for (i, &id) in ids.iter().enumerate() {
            if !eligible[i]
                || source[id.0 as usize].is_vehicle()
                || super::validate(
                    ctx.world,
                    &validation_source,
                    side,
                    &[id],
                    Some(target.clone()),
                    false,
                    ctx.rules,
                )
                .is_err()
            {
                continue;
            }
            let u = &origins[id.0 as usize];
            let from = u.position.xy();
            for (ordinal, entry) in points(ctx, known, owner, from).into_iter().enumerate() {
                if !crate::arrangement::standing_room(
                    ctx.world,
                    entry.at,
                    u.mobility.half_width_m,
                    &super::solid,
                ) {
                    continue;
                }
                let journey = Journey::new(
                    grid,
                    ctx.roads,
                    None,
                    Leg {
                        from,
                        goal: entry.at,
                        m: &u.mobility,
                        policy: RoutePolicy::Shortest,
                        avoid: &[],
                    },
                    &ctx.rules.navigation,
                );
                jobs.push(RouteJob {
                    candidate: Candidate {
                        unit: id,
                        entry,
                        route: vec![from],
                        distance: (entry.at - from).length(),
                        ordinal,
                        action: EntryAction::Route,
                        held: false,
                    },
                    journey,
                });
            }
        }
    }
    jobs.sort_by(|a, b| {
        a.candidate
            .distance
            .total_cmp(&b.candidate.distance)
            .then(a.candidate.unit.cmp(&b.candidate.unit))
            .then(a.candidate.ordinal.cmp(&b.candidate.ordinal))
    });
    let mut jobs = VecDeque::from(jobs);
    while route_work > 0 && !jobs.is_empty() {
        let share = route_work
            .div_ceil(jobs.len() as u64)
            .max(crate::navigation::LARGEST_STEP)
            .min(route_work);
        let mut job = jobs.pop_front().unwrap();
        let spent = job.journey.advance(grid, ctx.roads, share);
        route_work = route_work.saturating_sub(spent.max(1));
        match job.journey.plan() {
            Some(Plan::Route(route)) => {
                job.candidate.route.extend(route.iter().copied());
                job.candidate.distance = job
                    .candidate
                    .route
                    .windows(2)
                    .map(|w| (w[1] - w[0]).length())
                    .sum();
                candidates.push(job.candidate);
            }
            Some(Plan::Blocked(_)) => {}
            None => jobs.push_back(job),
        }
    }
    let route_unproven = !jobs.is_empty();
    allowance -= route_before - route_work;
    candidates.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then(a.unit.cmp(&b.unit))
            .then(a.ordinal.cmp(&b.ordinal))
    });
    if let Some((unit, at, action, held)) = selected_holder {
        candidates.insert(
            0,
            Candidate {
                unit,
                entry: EntryPoint {
                    at,
                    outward: (at - target.center).normalized(),
                },
                route: Vec::new(),
                distance: 0.0,
                ordinal: 0,
                action,
                held,
            },
        );
    }
    // Failed entrants remain at their source positions during the next proof.
    let fallback_work = allowance / 3;
    allowance -= fallback_work;
    let mut entry_unproven = false;
    for candidate in &candidates {
        // An unfinished route may still beat a completed path. Its straight
        // distance is a lower bound; ties use the same stable unit/edge order.
        if matches!(candidate.action, EntryAction::Route)
            && jobs.iter().any(|job| {
                job.candidate
                    .distance
                    .total_cmp(&candidate.distance)
                    .then(job.candidate.unit.cmp(&candidate.unit))
                    .then(job.candidate.ordinal.cmp(&candidate.ordinal))
                    .is_lt()
            })
        {
            break;
        }
        let mut outcome = gather(
            ctx,
            known,
            source,
            &origins,
            &ids,
            &eligible,
            request,
            &target,
            centre,
            Some(candidate),
            &mut allowance,
        );
        let undecided = outcome.unproven;
        entry_unproven |= undecided;
        outcome.unproven |= route_unproven || prefix_unproven;
        if outcome.entrant.is_some() {
            return Ok(BuildingPlan {
                placement: outcome,
                entry_action: candidate.action,
            });
        }
        if undecided || allowance == 0 {
            break;
        }
    }
    allowance += fallback_work;
    let mut outcome = gather(
        ctx,
        known,
        source,
        &origins,
        &ids,
        &eligible,
        request,
        &target,
        centre,
        None,
        &mut allowance,
    );
    outcome.unproven |= route_unproven || prefix_unproven || entry_unproven;
    Ok(BuildingPlan {
        placement: outcome,
        entry_action: EntryAction::Route,
    })
}

#[allow(clippy::too_many_arguments)]
fn gather(
    ctx: &MovementContext,
    known: &SideGeometry,
    source: &[Unit],
    origins: &[Unit],
    ids: &[UnitId],
    eligible: &[bool],
    request: &BuildingPreviewRequest,
    target: &crate::world::Prop,
    centre: V2,
    candidate: Option<&Candidate>,
    allowance: &mut u64,
) -> BuildingPlacement {
    let owner = target.id;
    let mut outward = (centre - target.center).normalized();
    if outward.length() == 0.0 {
        outward = candidate.map_or_else(
            || (origins[ids[0].0 as usize].position.xy() - target.center).normalized(),
            |c| c.entry.outward,
        );
        if outward.length() == 0.0 {
            outward = v2(1.0, 0.0);
        }
    }
    let entry = points(ctx, known, owner, centre)
        .into_iter()
        .min_by(|a, b| {
            (a.at - centre)
                .length()
                .total_cmp(&(b.at - centre).length())
        })
        .unwrap_or(EntryPoint {
            at: target.exterior_point(centre, ctx.rules.garrison.entry_distance_m / 2.0),
            outward,
        });
    let gatherers: Vec<_> = ids
        .iter()
        .filter(|&&id| candidate.is_none_or(|c| c.unit != id))
        .map(|id| member(ctx, &origins[id.0 as usize]))
        .collect();
    let max_radius = gatherers.iter().map(|m| m.radius).fold(0.0, f64::max);
    let anchor = entry.at + outward * (max_radius + ctx.rules.formation.spacing_m);
    let facing = request
        .facing
        .unwrap_or(libm::atan2(-outward.y, -outward.x));
    let mut nav = known.planning_snapshot();
    let grid = nav.grid(ctx.world, ctx.authored);
    let plan = formation::place(
        &gatherers,
        anchor,
        Some(facing),
        &ctx.rules.formation,
        libm::hypot(ctx.world.width(), ctx.world.depth()),
        |id, point| {
            let i = ids.iter().position(|&u| u == id).unwrap();
            if !eligible[i] {
                return None;
            }
            let u = &origins[id.0 as usize];
            let point = grid.placement_point(point, &u.mobility)?;
            if (point - entry.at).dot(outward) < u.mobility.half_width_m {
                return None;
            }
            if let Some(c) = candidate {
                let radius = member(ctx, u).radius
                    + origins[c.unit.0 as usize].mobility.half_width_m
                    + ctx.rules.formation.spacing_m;
                let mut left = 0.0;
                for w in c.route.windows(2).rev() {
                    if contract::ground::segment_distance(
                        [w[0].x, w[0].y],
                        [w[1].x, w[1].y],
                        [point.x, point.y],
                    ) < radius
                    {
                        return None;
                    }
                    left += (w[1] - w[0]).length();
                    if left >= ctx.rules.navigation.move_rehearsal_m {
                        break;
                    }
                }
            }
            Some(point)
        },
    );
    let mut slots = plan.slots;
    let mut orders: Vec<_> = slots
        .iter()
        .map(|s| {
            vec![UnitOrder::Move(MoveOrder {
                destination: s.point.unwrap_or(source[s.id.0 as usize].position.xy()),
                policy: RoutePolicy::Shortest,
                gesture: 0,
                direction: Default::default(),
                facing: request.facing,
            })]
        })
        .collect();
    if let Some(c) = candidate.filter(|c| !c.held) {
        slots.push(Slot {
            id: c.unit,
            point: Some(c.entry.at),
        });
        orders.push(if !matches!(c.action, EntryAction::Route) {
            Vec::new()
        } else {
            vec![
                UnitOrder::Move(MoveOrder {
                    destination: c.entry.at,
                    policy: RoutePolicy::Shortest,
                    gesture: 0,
                    direction: Default::default(),
                    facing: None,
                }),
                UnitOrder::Garrison {
                    building: owner,
                    approach: c.entry.at,
                },
            ]
        });
    }
    let proof = movement::certify_orders(
        ctx,
        source,
        known,
        movement::ProofRequest {
            slots: &slots,
            orders: Some(&orders),
            queued: request.queued,
            reserve_repair: true,
        },
        allowance,
    );
    let entrant = candidate
        .filter(|c| {
            c.held
                || proof.stable
                    && proof
                        .arrivals
                        .last()
                        .and_then(Option::as_ref)
                        .is_some_and(|u| {
                            u.garrisoned()
                                && u.garrison.as_ref().is_some_and(|g| g.building == owner)
                        })
        })
        .map(|c| BuildingEntry {
            unit: c.unit,
            approach: [c.entry.at.x, c.entry.at.y],
        });
    let destinations = slots
        .iter()
        .take(gatherers.len())
        .enumerate()
        .map(|(i, slot)| {
            let u = &source[slot.id.0 as usize];
            let at = slot.point.unwrap_or(u.position.xy());
            MoveDestination {
                unit: slot.id,
                goal: [at.x, at.y],
                placed: proof.stable && proof.facings[i].is_some(),
                facing: proof.facings[i].unwrap_or(u.yaw),
            }
        })
        .collect();
    BuildingPlacement {
        building: owner,
        entrant,
        destinations,
        unproven: proof.unproven || !proof.stable,
    }
}
