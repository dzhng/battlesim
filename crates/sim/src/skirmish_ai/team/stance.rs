//! Team stance hysteresis: hold, push, and fall back.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TeamStance {
    #[default]
    Hold,
    Push,
    FallBack,
}

pub(super) fn update_stance(
    side: Side,
    frame: &ObservationFrame,
    plans: &[PlayerPlan],
    memory: &mut TeamMemory,
    tuning: TeamTuning,
    tick_hz: u32,
) -> TeamStance {
    let Some(view) = frame.skirmish.as_ref() else {
        return memory.stance;
    };
    let opponent = match side {
        Side::Blue => Side::Red,
        Side::Red => Side::Blue,
        Side::Neutral => Side::Neutral,
    };
    let score_balance = view.scores[side.index()] - view.scores[opponent.index()];
    let score_trend = memory.previous_score.map_or(0.0, |old| score_balance - old);
    memory.previous_score = Some(score_balance);
    let lane_balance: Vec<_> = plans
        .iter()
        .map(|plan| {
            memory
                .influence
                .as_ref()
                .map_or(0.0, |grid| grid.on(&plan.main_objective))
        })
        .collect();
    memory.outgunned = plans
        .iter()
        .zip(&lane_balance)
        .filter_map(|(plan, balance)| {
            (*balance < tuning.stance_fallback_influence).then_some(plan.player)
        })
        .collect();
    let balance = lane_balance.iter().sum::<f64>() / plans.len().max(1) as f64;
    let dwell = (tuning.stance_dwell_s * f64::from(tick_hz)) as Tick;
    let team_losing = !plans.is_empty() && memory.outgunned.len() * 2 > plans.len();
    if team_losing {
        memory.ready_groups.clear();
        memory.stance = TeamStance::FallBack;
        memory.stance_since = frame.tick;
        memory.push_wait_since = None;
        memory.ready_after = frame.tick.saturating_add(dwell);
        return memory.stance;
    }
    if memory.stance == TeamStance::FallBack
        && frame.tick.saturating_sub(memory.stance_since) < dwell
    {
        return memory.stance;
    }
    if memory.stance == TeamStance::Push {
        let release_threshold =
            (tuning.stance_fallback_influence + tuning.stance_push_influence) * 0.5;
        if balance >= release_threshold || frame.tick.saturating_sub(memory.stance_since) < dwell {
            return memory.stance;
        }
        memory.ready_groups.clear();
        memory.push_wait_since = Some(frame.tick);
        memory.stance_since = frame.tick;
        memory.ready_after = frame.tick.saturating_add(dwell);
        memory.stance = TeamStance::Hold;
        return memory.stance;
    }
    if memory.stance == TeamStance::Hold
        && memory
            .push_wait_since
            .is_some_and(|since| frame.tick.saturating_sub(since) < dwell)
    {
        return memory.stance;
    }
    let ready = plans
        .iter()
        .all(|plan| memory.ready_groups.contains(&plan.player));
    let push_candidate = score_trend < 0.0
        || (memory.outgunned.len() * 2 <= plans.len() && balance >= tuning.stance_push_influence);
    if !push_candidate {
        memory.stance = TeamStance::Hold;
        return memory.stance;
    }
    let wait = *memory.push_wait_since.get_or_insert(frame.tick);
    let timeout = (tuning.push_timeout_s * f64::from(tick_hz)) as Tick;
    if ready || frame.tick.saturating_sub(wait) >= timeout {
        let started = memory.stance != TeamStance::Push;
        memory.stance = TeamStance::Push;
        memory.stance_since = frame.tick;
        if started {
            memory.ready_groups.clear();
        }
    } else {
        memory.stance = TeamStance::Hold;
    }
    memory.stance
}
