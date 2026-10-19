//! Help requests and detachments between allied skirmish lanes.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct HelpRequest {
    pub requester: PlayerIndex,
    pub lane: EntryIndex,
    pub objective: ObjectiveSite,
    pub answered_by: Option<PlayerIndex>,
    pub detachment: Vec<contract::ids::UnitId>,
    pub raised_tick: Tick,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HelpLock {
    pub(crate) answered_by: PlayerIndex,
    pub(crate) detachment: Vec<contract::ids::UnitId>,
    pub(crate) clear_since: Option<Tick>,
    pub(crate) raised_tick: Tick,
}

type LaneContext = (PlayerIndex, EntryIndex, ObjectiveSite, [f64; 2]);

pub(super) fn lane_contexts<P: TeamPlayer>(
    side: Side,
    players: &[P],
    sites: &SkirmishSites,
    plans: &[PlayerPlan],
    covered_by_player: &BTreeMap<PlayerIndex, ObjectiveSite>,
    tuning: TeamTuning,
) -> Vec<LaneContext> {
    let lanes: Vec<_> = sites
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.side == side)
        .map(|(i, _)| EntryIndex(i))
        .collect();
    if lanes.is_empty() || sites.objectives.is_empty() {
        return Vec::new();
    }
    players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.side() == side)
        .enumerate()
        .filter_map(|(local_index, (index, _))| {
            let player = PlayerIndex(index as u16);
            if let Some(plan) = plans.iter().find(|plan| plan.player == player) {
                return Some((
                    player,
                    plan.lane,
                    plan.main_objective.clone(),
                    plan.staging_point,
                ));
            }
            let is_human = players[index].controller() == &ControllerKind::Human;
            if is_human && !covered_by_player.contains_key(&player) {
                return None;
            }
            let lane = lanes[local_index % lanes.len()];
            let objective = covered_by_player
                .get(&player)
                .cloned()
                .unwrap_or_else(|| sites.objectives[local_index % sites.objectives.len()].clone());
            let staging_point = staging(
                sites.entries[lane.0].center,
                objective.center,
                tuning.staging_distance_m,
            );
            Some((player, lane, objective, staging_point))
        })
        .collect()
}

fn player_force(frame: &ObservationFrame, player: PlayerIndex) -> usize {
    let observed = frame
        .own
        .iter()
        .filter(|unit| unit.owner == Some(player))
        .count();
    frame
        .skirmish
        .as_ref()
        .and_then(|view| view.players.iter().find(|p| p.index == player))
        .map_or(observed, |p| observed.max(p.unit_count as usize))
}

fn player_anchor(frame: &ObservationFrame, player: PlayerIndex) -> Option<[f64; 2]> {
    let units: Vec<_> = frame
        .own
        .iter()
        .filter(|unit| unit.owner == Some(player))
        .collect();
    if units.is_empty() {
        return None;
    }
    let n = units.len() as f64;
    Some([
        units.iter().map(|u| u.position[0]).sum::<f64>() / n,
        units.iter().map(|u| u.position[1]).sum::<f64>() / n,
    ])
}

fn lane_threats(frame: &ObservationFrame, objective: &ObjectiveSite) -> Vec<TypeIndex> {
    frame
        .identified
        .iter()
        .filter(|target| {
            distance([target.position[0], target.position[1]], objective.center)
                <= objective.radius_m * 3.0
        })
        .map(|target| target.kind)
        .collect()
}

pub(super) fn capable_detachment(
    rules: &Rules,
    frame: &ObservationFrame,
    helper: PlayerIndex,
    objective: &ObjectiveSite,
    excluded: &BTreeSet<contract::ids::UnitId>,
) -> Vec<(f64, contract::ids::UnitId)> {
    let threats = lane_threats(frame, objective);
    let can_answer = |unit: &OwnUnit| {
        if threats.is_empty() {
            super::super::unit_can_engage_ground(rules, unit.kind)
        } else {
            threats
                .iter()
                .any(|target| super::super::unit_can_engage_target(rules, unit.kind, *target))
        }
    };
    let mut candidates: Vec<_> = frame
        .own
        .iter()
        .filter(|unit| {
            unit.owner == Some(helper) && !excluded.contains(&unit.id) && can_answer(unit)
        })
        .map(|unit| {
            (
                distance([unit.position[0], unit.position[1]], objective.center),
                unit.id,
            )
        })
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    candidates
}

/// The `count` nearest of `capable` (nearest first), holding at least one
/// unit that can hold an objective: help for a lane must be able to stand on
/// it, so an aircraft never answers alone.
pub(super) fn detachment(
    rules: &Rules,
    frame: &ObservationFrame,
    capable: Vec<(f64, contract::ids::UnitId)>,
    count: usize,
) -> Vec<contract::ids::UnitId> {
    let holds = |id: contract::ids::UnitId| {
        frame
            .own
            .iter()
            .find(|unit| unit.id == id)
            .is_some_and(|unit| crate::objectives::can_hold(rules.catalog.get(unit.kind)))
    };
    let mut chosen: Vec<_> = capable.iter().take(count).map(|(_, id)| *id).collect();
    if !chosen.iter().any(|id| holds(*id)) {
        if let Some((_, holder)) = capable.iter().find(|(_, id)| holds(*id)) {
            chosen.pop();
            chosen.push(*holder);
        }
    }
    chosen
}

pub(super) fn update_help<P: TeamPlayer>(
    side: Side,
    frame: &ObservationFrame,
    players: &[P],
    lanes: &[LaneContext],
    memory: &mut TeamMemory,
    tuning: TeamTuning,
    rules: &Rules,
) -> Vec<HelpRequest> {
    let tick_hz = rules.tick_hz;
    if frame.skirmish.is_none() {
        return Vec::new();
    }
    let dwell = (tuning.help_dwell_s * f64::from(tick_hz)) as Tick;
    let mut requests = Vec::new();
    let mut used_surplus: BTreeMap<PlayerIndex, usize> = memory
        .help_locks
        .values()
        .map(|lock| (lock.answered_by, lock.detachment.len()))
        .fold(BTreeMap::new(), |mut used, (player, count)| {
            *used.entry(player).or_default() += count;
            used
        });
    let mut assigned_units = BTreeSet::new();
    for (requester, lane, objective, _) in lanes {
        let current = memory
            .influence
            .as_ref()
            .map_or(0.0, |grid| grid.at(objective.center));
        let previous = memory.previous_influence.get(&objective.id).copied();
        let trend = previous.map_or(0.0, |old| current - old);
        let raised = previous.is_some_and(|_| trend <= -tuning.help_trend_threshold);
        let need = ((-current).ceil() as usize).max(1);
        // A detachment lost on the way is no help on its way: the lock goes,
        // and a lane still outgunned asks again.
        let mut lost = false;
        if let Some(lock) = memory.help_locks.get_mut(requester) {
            let sent = lock.detachment.len();
            lock.detachment
                .retain(|id| frame.own.iter().any(|unit| unit.id == *id));
            let remaining = lock.detachment.len();
            used_surplus
                .entry(lock.answered_by)
                .and_modify(|used| *used = used.saturating_sub(sent - remaining));
            if sent > 0 && remaining == 0 {
                memory.help_locks.remove(requester).expect("lock present");
                lost = true;
            }
        }
        if let Some(lock) = memory.help_locks.get_mut(requester) {
            // A rising threat raises help early; only relief clears it. A
            // lane still outgunned keeps its help on the way, or a
            // detachment is called back before it ever arrives.
            if current >= 0.0 {
                lock.clear_since.get_or_insert(frame.tick);
            } else {
                lock.clear_since = None;
            }
            let engaged = detachment_engaged(frame, &lock.detachment);
            let held = lock
                .clear_since
                .is_none_or(|since| frame.tick.saturating_sub(since) < dwell)
                || engaged;
            if held {
                requests.push(HelpRequest {
                    requester: *requester,
                    lane: *lane,
                    objective: objective.clone(),
                    answered_by: Some(lock.answered_by),
                    detachment: lock.detachment.clone(),
                    raised_tick: lock.raised_tick,
                });
                continue;
            }
            let released = memory.help_locks.remove(requester).expect("lock present");
            used_surplus
                .entry(released.answered_by)
                .and_modify(|used| *used = used.saturating_sub(released.detachment.len()));
        }
        if !(raised || lost && current < 0.0) {
            continue;
        }
        let candidates = players
            .iter()
            .enumerate()
            .filter(|(index, player)| {
                player.side() == side
                    && PlayerIndex(*index as u16) != *requester
                    && *player.controller() == ControllerKind::ActiveAi
            })
            .filter_map(|(index, _)| {
                let helper = PlayerIndex(index as u16);
                let force = player_force(frame, helper);
                let helper_need = lanes
                    .iter()
                    .find(|(player, _, _, _)| *player == helper)
                    .map_or(1.0, |(_, _, helper_objective, _)| {
                        memory.influence.as_ref().map_or(1.0, |grid| {
                            (-grid.at(helper_objective.center)).ceil().max(1.0)
                        })
                    });
                let used = used_surplus.get(&helper).copied().unwrap_or_default();
                let surplus = force as f64 - helper_need * tuning.help_force_margin - used as f64;
                let mut excluded = assigned_units.clone();
                excluded.extend(
                    memory
                        .help_locks
                        .values()
                        .flat_map(|lock| lock.detachment.iter().copied()),
                );
                let capable = capable_detachment(rules, frame, helper, objective, &excluded);
                (surplus > 0.0 && !capable.is_empty()).then(|| {
                    let distance = player_anchor(frame, helper)
                        .map_or(f64::INFINITY, |at| distance(at, objective.center));
                    (distance, helper, helper_need, surplus, capable)
                })
            });
        let Some((_, helper, _helper_need, surplus, capable)) =
            candidates.min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        else {
            requests.push(HelpRequest {
                requester: *requester,
                lane: *lane,
                objective: objective.clone(),
                answered_by: None,
                detachment: Vec::new(),
                raised_tick: frame.tick,
            });
            continue;
        };
        let count = surplus.min(need as f64).floor().max(1.0) as usize;
        let detachment = detachment(rules, frame, capable, count);
        assigned_units.extend(detachment.iter().copied());
        memory.help_locks.insert(
            *requester,
            HelpLock {
                answered_by: helper,
                detachment: detachment.clone(),
                clear_since: None,
                raised_tick: frame.tick,
            },
        );
        *used_surplus.entry(helper).or_default() += detachment.len();
        requests.push(HelpRequest {
            requester: *requester,
            lane: *lane,
            objective: objective.clone(),
            answered_by: Some(helper),
            detachment,
            raised_tick: frame.tick,
        });
    }
    let influence = memory.influence.clone();
    for (_, _, objective, _) in lanes {
        if let Some(current) = influence.as_ref().map(|grid| grid.at(objective.center)) {
            memory
                .previous_influence
                .insert(objective.id.clone(), current);
        }
    }
    requests
}

fn detachment_engaged(frame: &ObservationFrame, detachment: &[contract::ids::UnitId]) -> bool {
    frame.own.iter().any(|unit| {
        detachment.contains(&unit.id)
            && ((unit.engagement == contract::command::Engagement::FireAtWill
                && !unit.sees.is_empty())
                || unit
                    .mounts
                    .iter()
                    .any(|mount| mount.reason == contract::observation::ActionReason::Firing))
    })
}
