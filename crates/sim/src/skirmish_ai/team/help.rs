//! Help requests and detachments between allied skirmish lanes.

use super::party::{force, on_duty, recruit, refill, reserved, PartyKey};
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

/// Where help may be asked for: each lane's objective, and each side
/// objective a hold party keeps (D55: a side objective under pressure raises
/// help exactly as a main lane does), asked by the party's owner. Each
/// requester asks once for an objective, though its lane and a hold share it.
fn help_sites(lanes: &[LaneContext], memory: &TeamMemory) -> Vec<LaneContext> {
    let holds = memory
        .parties
        .values()
        .filter(|party| party.kind == PartyKind::Hold)
        .filter_map(|party| {
            let (_, lane, _, staging) = lanes.iter().find(|lane| lane.0 == party.owner)?;
            Some((party.owner, *lane, party.objective.clone(), *staging))
        });
    let mut seen = BTreeSet::new();
    lanes
        .iter()
        .cloned()
        .chain(holds)
        .filter(|(requester, _, objective, _)| seen.insert((*requester, objective.id.clone())))
        .collect()
}

/// What `helper` can lend: its force beyond slice 13's margin for its own
/// lane, less its units already on party duty. The side-party budget bounds
/// claims and holds, not help: a helper with surplus answers however many
/// objectives it holds.
fn surplus(
    frame: &ObservationFrame,
    lanes: &[LaneContext],
    memory: &TeamMemory,
    helper: PlayerIndex,
    tuning: TeamTuning,
) -> f64 {
    let need = lanes
        .iter()
        .find(|(player, _, _, _)| *player == helper)
        .map_or(1.0, |(_, _, objective, _)| {
            memory
                .influence
                .as_ref()
                .map_or(1.0, |grid| (-grid.at(objective.center)).ceil().max(1.0))
        });
    force(frame, helper) as f64 - need * tuning.help_force_margin - on_duty(memory, helper) as f64
}

/// One plan step of help: keep, replace or release each help party, and
/// answer each new call from the nearest ally with surplus.
/// The help party itself (its units, lock, budget and refill) is the one
/// party concept in `party.rs`.
pub(super) fn update_help<P: TeamPlayer>(
    side: Side,
    frame: &ObservationFrame,
    players: &[P],
    lanes: &[LaneContext],
    memory: &mut TeamMemory,
    tuning: TeamTuning,
    rules: &Rules,
) -> Vec<HelpRequest> {
    if frame.skirmish.is_none() {
        return Vec::new();
    }
    let dwell = (tuning.help_dwell_s * f64::from(rules.tick_hz)) as Tick;
    let sites = help_sites(lanes, memory);
    let mut requests = Vec::new();
    for (requester, lane, objective, _) in &sites {
        let current = memory
            .influence
            .as_ref()
            .map_or(0.0, |grid| grid.at(objective.center));
        let previous = memory.previous_influence.get(&objective.id).copied();
        let trend = previous.map_or(0.0, |old| current - old);
        let raised = previous.is_some_and(|_| trend <= -tuning.help_trend_threshold);
        let need = ((-current).ceil() as usize).max(1);
        let key = PartyKey::Help(*requester, objective.id.clone());
        // Losses are replaced from the helper while it can spare them. A
        // detachment lost on the way with nobody to replace it is no help on
        // its way: the lock goes, and a lane still outgunned asks again.
        let mut lost = false;
        if memory.parties.contains_key(&key) {
            refill(rules, frame, memory, &key, &|memory, helper| {
                surplus(frame, lanes, memory, helper, tuning)
                    .floor()
                    .max(0.0) as usize
            });
            if memory.parties[&key].units.is_empty() {
                memory.parties.remove(&key);
                lost = true;
            }
        }
        if let Some(lock) = memory.parties.get_mut(&key) {
            // A rising threat raises help early; only relief clears it. A
            // lane still outgunned keeps its help on the way, or a
            // detachment is called back before it ever arrives.
            if current >= 0.0 {
                lock.clear_since.get_or_insert(frame.tick);
            } else {
                lock.clear_since = None;
            }
            let held = lock
                .clear_since
                .is_none_or(|since| frame.tick.saturating_sub(since) < dwell)
                || lock.engaged(frame);
            if held {
                requests.push(HelpRequest {
                    requester: *requester,
                    lane: *lane,
                    objective: objective.clone(),
                    answered_by: Some(lock.owner),
                    detachment: lock.units.clone(),
                    raised_tick: lock.raised_tick,
                });
                continue;
            }
            memory.parties.remove(&key);
        }
        if !(raised || lost && current < 0.0) {
            continue;
        }
        let reserved = reserved(memory);
        let answer = players
            .iter()
            .enumerate()
            .filter(|(index, player)| {
                player.side() == side
                    && PlayerIndex(*index as u16) != *requester
                    && *player.controller() == ControllerKind::ActiveAi
            })
            .filter_map(|(index, _)| {
                let helper = PlayerIndex(index as u16);
                let surplus = surplus(frame, lanes, memory, helper, tuning);
                let count = surplus.min(need as f64).floor().max(0.0) as usize;
                let units = recruit(
                    rules,
                    frame,
                    helper,
                    objective,
                    &reserved,
                    count,
                    PartyKind::Help,
                );
                (!units.is_empty()).then(|| {
                    let distance = player_anchor(frame, helper)
                        .map_or(f64::INFINITY, |at| distance(at, objective.center));
                    (distance, helper, units)
                })
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let Some((_, helper, units)) = answer else {
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
        memory.parties.insert(
            key,
            Party {
                kind: PartyKind::Help,
                owner: helper,
                objective: objective.clone(),
                units: units.clone(),
                size: units.len(),
                raised_tick: frame.tick,
                clear_since: None,
            },
        );
        requests.push(HelpRequest {
            requester: *requester,
            lane: *lane,
            objective: objective.clone(),
            answered_by: Some(helper),
            detachment: units,
            raised_tick: frame.tick,
        });
    }
    // Help for a call no longer made (its requester moved on, or the hold
    // it answered went home) goes home too, unless it is fighting.
    let asked: BTreeSet<_> = sites
        .iter()
        .map(|(requester, _, objective, _)| PartyKey::Help(*requester, objective.id.clone()))
        .collect();
    memory.parties.retain(|key, party| {
        !matches!(key, PartyKey::Help(..)) || asked.contains(key) || party.engaged(frame)
    });
    // Only this step's sites keep a reading: a site that comes back measures
    // its trend from its return, not from a reading minutes old.
    memory
        .previous_influence
        .retain(|id, _| sites.iter().any(|(_, _, objective, _)| objective.id == *id));
    let influence = memory.influence.clone();
    for (_, _, objective, _) in &sites {
        if let Some(current) = influence.as_ref().map(|grid| grid.at(objective.center)) {
            memory
                .previous_influence
                .insert(objective.id.clone(), current);
        }
    }
    requests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_requester_asks_once_where_its_lane_and_hold_share_an_objective() {
        let objective = ObjectiveSite {
            id: "held".into(),
            center: [100.0, 100.0],
            radius_m: 30.0,
            kind: contract::encounter::ObjectiveSiteKind::Field,
            counterpart: None,
        };
        let player = PlayerIndex(0);
        let lanes = vec![(player, EntryIndex(0), objective.clone(), [100.0, 0.0])];
        let mut memory = TeamMemory::default();
        memory.parties.insert(
            PartyKey::Side(objective.id.clone()),
            Party {
                kind: PartyKind::Hold,
                owner: player,
                objective,
                units: vec![],
                size: 2,
                raised_tick: 0,
                clear_since: None,
            },
        );
        assert_eq!(help_sites(&lanes, &memory).len(), 1);
    }
}
