//! Detached parties (D55): units a player sends from its main group to an
//! objective.
//!
//! Help, hold and claim are one concept. They live in one map in
//! [`TeamMemory`] and share one lifecycle: one budget per player, one way of
//! choosing units ([`recruit`]), one way of replacing losses ([`refill`]),
//! and one engaged check. A unit is in at most one party, and an engaged
//! party is never re-tasked. `help.rs` only decides when a lane asks and who
//! answers; everything else about a party is here.

use super::*;
use contract::ids::UnitId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartyKind {
    /// Answers another lane's (or side objective's) request for help.
    Help,
    /// Stays on an objective the player took.
    Hold,
    /// Goes to take an open objective.
    Claim,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Party {
    pub kind: PartyKind,
    pub owner: PlayerIndex,
    pub objective: ObjectiveSite,
    pub units: Vec<UnitId>,
    /// The size it is kept at: losses are replaced up to it while the party
    /// is still needed.
    pub size: usize,
    pub raised_tick: Tick,
    /// Help only: when its lane stopped being outgunned.
    pub clear_since: Option<Tick>,
}

/// A party's place in the one map: help is per requester and objective; the
/// side has at most one hold or claim on an objective.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PartyKey {
    Help(PlayerIndex, String),
    Side(String),
}

impl Party {
    /// Whether any of its units is fighting. An engaged party keeps its
    /// units: it is never recalled or re-tasked mid-fight.
    pub fn engaged(&self, frame: &ObservationFrame) -> bool {
        self.units.iter().any(|id| unit_engaged(frame, *id))
    }
}

fn unit_engaged(frame: &ObservationFrame, id: UnitId) -> bool {
    frame.own.iter().any(|unit| {
        unit.id == id
            && ((unit.engagement == contract::command::Engagement::FireAtWill
                && !unit.sees.is_empty())
                || unit
                    .mounts
                    .iter()
                    .any(|mount| mount.reason == contract::observation::ActionReason::Firing))
    })
}

/// Every party, for the plan: by kind, owner, then objective.
pub(super) fn parties(memory: &TeamMemory) -> Vec<Party> {
    let mut result: Vec<_> = memory.parties.values().cloned().collect();
    result.sort_by(|a, b| {
        (a.kind, a.owner, &a.objective.id).cmp(&(b.kind, b.owner, &b.objective.id))
    });
    result
}

/// A player's force: its units in view, or the count the skirmish view
/// reports when that is larger (units still arriving).
pub(super) fn force(frame: &ObservationFrame, player: PlayerIndex) -> usize {
    let observed = frame.own.iter().filter(|u| u.owner == Some(player)).count();
    frame
        .skirmish
        .as_ref()
        .and_then(|view| view.players.iter().find(|p| p.index == player))
        .map_or(observed, |p| observed.max(p.unit_count as usize))
}

/// Units `owner` has on party duty, of every kind.
pub(super) fn on_duty(memory: &TeamMemory, owner: PlayerIndex) -> usize {
    memory
        .parties
        .values()
        .filter(|party| party.owner == owner)
        .map(|party| party.units.len())
        .sum()
}

/// What is left of `owner`'s one party budget: a share of its force.
pub(super) fn budget_left(
    memory: &TeamMemory,
    frame: &ObservationFrame,
    owner: PlayerIndex,
    tuning: TeamTuning,
) -> usize {
    let budget = ((force(frame, owner) as f64 * tuning.party_budget_share).floor() as usize).max(1);
    budget.saturating_sub(on_duty(memory, owner))
}

/// Units already in a party: no other party may take them.
pub(super) fn reserved(memory: &TeamMemory) -> BTreeSet<UnitId> {
    memory
        .parties
        .values()
        .flat_map(|party| party.units.iter().copied())
        .collect()
}

/// An open objective: neutral, no enemy seen within its radius, and nobody
/// on the side (main group or party, in `going`) already headed there.
pub(super) fn objective_open(
    frame: &ObservationFrame,
    objective: &ObjectiveSite,
    going: &BTreeSet<String>,
) -> bool {
    let Some(view) = frame.skirmish.as_ref() else {
        return false;
    };
    let Some(state) = view.objectives.iter().find(|o| o.id == objective.id) else {
        return false;
    };
    state.owner.is_none()
        && !going.contains(&objective.id)
        && !frame.identified.iter().any(|target| {
            distance([target.position[0], target.position[1]], objective.center)
                <= objective.radius_m
        })
}

/// The one way a party picks its units: the nearest of `owner`'s units, up
/// to `count`, not in a party (`reserved`) and not fighting, that can fight
/// the threat at `objective` by weapon target class (a ground fight with
/// none in view). A carrier and the squads aboard go together or not at
/// all: a passenger is never sent without its carrier, nor dropped off with
/// it. A unit that cannot engage ground never goes: air defence holds the
/// backline (D57). The party holds at least one unit that can hold the
/// objective, so aircraft never go alone; with none to spare it is empty.
/// A hold or claim never takes the main group's last holder; help is gated
/// by its lane's surplus instead (`help.rs`).
pub(super) fn recruit(
    rules: &Rules,
    frame: &ObservationFrame,
    owner: PlayerIndex,
    objective: &ObjectiveSite,
    reserved: &BTreeSet<UnitId>,
    count: usize,
    kind: PartyKind,
) -> Vec<UnitId> {
    if count == 0 {
        return Vec::new();
    }
    let holds = |unit: &OwnUnit| crate::objectives::can_hold(rules.catalog.get(unit.kind));
    let threats: Vec<_> = frame
        .identified
        .iter()
        .filter(|target| {
            distance([target.position[0], target.position[1]], objective.center)
                <= objective.radius_m * 3.0
        })
        .map(|target| target.kind)
        .collect();
    let capable = |unit: &OwnUnit| {
        !unit_engaged(frame, unit.id)
            && super::super::unit_can_engage_ground(rules, unit.kind)
            && (threats.is_empty()
                || threats
                    .iter()
                    .any(|kind| super::super::unit_can_engage_target(rules, unit.kind, *kind)))
    };
    // Each free unit with the squads aboard it; a passenger moves only with
    // its carrier.
    let groups: Vec<Vec<&OwnUnit>> = frame
        .own
        .iter()
        .filter(|unit| {
            unit.owner == Some(owner)
                && !reserved.contains(&unit.id)
                && super::super::transport::carrier_of(unit).is_none()
        })
        .map(|unit| {
            std::iter::once(unit)
                .chain(super::super::transport::passengers(frame, unit.id))
                .collect()
        })
        .filter(|group: &Vec<&OwnUnit>| group.iter().all(|u| !reserved.contains(&u.id)))
        .collect();
    let group_holds = |group: &[&OwnUnit]| {
        group
            .iter()
            .any(|unit| holds(unit) && super::super::unit_can_engage_ground(rules, unit.kind))
    };
    // The main group's holders are those that go with it: air defence at
    // the backline holds nothing for it.
    let keep = usize::from(kind != PartyKind::Help);
    let mut spare_holders = groups
        .iter()
        .filter(|group| group_holds(group))
        .count()
        .saturating_sub(keep);
    let mut candidates: Vec<_> = groups
        .iter()
        .filter(|group| {
            // Air defence holds the backline (D57), even aboard a carrier.
            group.len() <= count
                && group.iter().any(|unit| capable(unit))
                && group
                    .iter()
                    .all(|unit| super::super::unit_can_engage_ground(rules, unit.kind))
        })
        .map(|group| {
            (
                distance(
                    [group[0].position[0], group[0].position[1]],
                    objective.center,
                ),
                group,
            )
        })
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1[0].id.cmp(&b.1[0].id)));
    let mut chosen: Vec<&Vec<&OwnUnit>> = Vec::new();
    let size = |chosen: &[&Vec<&OwnUnit>]| chosen.iter().map(|group| group.len()).sum::<usize>();
    for (_, group) in &candidates {
        if size(&chosen) + group.len() > count {
            continue;
        }
        if group_holds(group) {
            if spare_holders == 0 {
                continue;
            }
            spare_holders -= 1;
        }
        chosen.push(group);
    }
    if !chosen.iter().any(|group| group_holds(group)) {
        // The nearest holder the group can spare replaces the farthest picks.
        let Some((_, holder)) = candidates
            .iter()
            .find(|(_, group)| group_holds(group))
            .filter(|_| spare_holders > 0)
        else {
            return Vec::new();
        };
        while size(&chosen) + holder.len() > count {
            chosen.pop();
        }
        chosen.push(holder);
    }
    chosen
        .into_iter()
        .flat_map(|group| group.iter().map(|unit| unit.id))
        .collect()
}

/// The one way a party lost on the way is replaced: drop its fallen and top
/// it up to its size from its owner's free units, up to the `room` its kind
/// allows the owner (the side-party budget for a hold or claim, the
/// helper's surplus for help). A party left empty has nobody to send; its
/// caller decides whether it still waits for units (a hold) or goes (help,
/// a claim).
pub(super) fn refill(
    rules: &Rules,
    frame: &ObservationFrame,
    memory: &mut TeamMemory,
    key: &PartyKey,
    room: &dyn Fn(&TeamMemory, PlayerIndex) -> usize,
) {
    let Some(party) = memory.parties.get_mut(key) else {
        return;
    };
    party
        .units
        .retain(|id| frame.own.iter().any(|unit| unit.id == *id));
    let (owner, kind, objective, short) = (
        party.owner,
        party.kind,
        party.objective.clone(),
        party.size.saturating_sub(party.units.len()),
    );
    let count = short.min(room(memory, owner));
    let units = recruit(
        rules,
        frame,
        owner,
        &objective,
        &reserved(memory),
        count,
        kind,
    );
    memory
        .parties
        .get_mut(key)
        .expect("party present")
        .units
        .extend(units);
}

/// What one plan step's side parties read.
pub(super) struct PartyInputs<'a> {
    pub rules: &'a Rules,
    pub frame: &'a ObservationFrame,
    pub plans: &'a [PlayerPlan],
    pub sites: &'a SkirmishSites,
    /// Objectives human teammates stand on or are ordered to.
    pub covered: &'a BTreeSet<String>,
    pub tuning: TeamTuning,
}

/// One plan step of the hold and claim parties, before help is raised:
/// - a main objective the side took and moved on from keeps a hold party;
/// - a claim whose objective the side took becomes its hold;
/// - a party goes home once it is no longer needed (a hold whose objective
///   the side lost, a claim whose objective is no longer neutral) unless it
///   is engaged;
/// - claims are replaced and raised first, then holds, while the budget lasts;
///   new help comes after both (`help.rs`).
pub(super) fn update_side_parties(side: Side, input: PartyInputs<'_>, memory: &mut TeamMemory) {
    let PartyInputs {
        rules,
        frame,
        plans,
        sites,
        covered,
        tuning,
    } = input;
    let Some(view) = frame.skirmish.as_ref() else {
        return;
    };
    let owner_of = |id: &str| view.objectives.iter().find(|o| o.id == id)?.owner;
    for plan in plans {
        let moved_from = memory
            .cached
            .as_ref()
            .and_then(|cached| cached.for_player(plan.player))
            .map(|old| old.main_objective.clone())
            .filter(|old| old.id != plan.main_objective.id && owner_of(&old.id) == Some(side));
        if let Some(objective) = moved_from {
            memory
                .parties
                .entry(PartyKey::Side(objective.id.clone()))
                .or_insert_with(|| Party {
                    kind: PartyKind::Hold,
                    owner: plan.player,
                    objective,
                    units: Vec::new(),
                    size: tuning.hold_party_size,
                    raised_tick: frame.tick,
                    clear_since: None,
                });
        }
    }
    for party in memory.parties.values_mut() {
        party
            .units
            .retain(|id| frame.own.iter().any(|unit| unit.id == *id));
    }
    memory.parties.retain(|key, party| {
        let PartyKey::Side(id) = key else {
            return true;
        };
        let owner = owner_of(id);
        if party.kind == PartyKind::Claim && owner == Some(side) {
            party.kind = PartyKind::Hold;
            party.size = tuning.hold_party_size;
        }
        needed(side, frame, party) || party.engaged(frame)
    });
    refill_side(side, PartyKind::Claim, rules, frame, memory, tuning);
    let mut going: BTreeSet<_> = plans
        .iter()
        .map(|plan| plan.main_objective.id.clone())
        .chain(memory.parties.values().map(|p| p.objective.id.clone()))
        .chain(covered.iter().cloned())
        .collect();
    for plan in plans {
        let entry = sites.entries[plan.lane.0].center;
        let reach = distance(entry, plan.main_objective.center) * 1.5;
        let mut open: Vec<_> = sites
            .objectives
            .iter()
            .filter(|objective| distance(entry, objective.center) <= reach)
            .collect();
        open.sort_by(|a, b| {
            distance(entry, a.center)
                .total_cmp(&distance(entry, b.center))
                .then(a.id.cmp(&b.id))
        });
        for objective in open {
            let left = budget_left(memory, frame, plan.player, tuning);
            if left == 0 {
                break;
            }
            if !objective_open(frame, objective, &going) {
                continue;
            }
            let units = recruit(
                rules,
                frame,
                plan.player,
                objective,
                &reserved(memory),
                tuning.claim_party_size.min(left),
                PartyKind::Claim,
            );
            if units.is_empty() {
                continue;
            }
            going.insert(objective.id.clone());
            memory.parties.insert(
                PartyKey::Side(objective.id.clone()),
                Party {
                    kind: PartyKind::Claim,
                    owner: plan.player,
                    objective: objective.clone(),
                    units,
                    size: tuning.claim_party_size,
                    raised_tick: frame.tick,
                    clear_since: None,
                },
            );
        }
    }
    refill_side(side, PartyKind::Hold, rules, frame, memory, tuning);
    // A claim nobody could be sent on blocks nothing: its objective is open
    // again for the next plan. A hold waits for units while the side owns it.
    memory.parties.retain(|key, party| {
        !(matches!(key, PartyKey::Side(_))
            && party.kind == PartyKind::Claim
            && party.units.is_empty())
    });
}

/// Whether a hold or claim is still needed: a hold while the side owns its
/// objective (contested or not), a claim while its objective is neutral.
fn needed(side: Side, frame: &ObservationFrame, party: &Party) -> bool {
    let owner = frame
        .skirmish
        .as_ref()
        .and_then(|view| view.objectives.iter().find(|o| o.id == party.objective.id))
        .and_then(|o| o.owner);
    match party.kind {
        PartyKind::Hold => owner == Some(side),
        PartyKind::Claim => owner.is_none(),
        PartyKind::Help => true,
    }
}

/// Replace the losses of every still-needed hold or claim of `kind`.
fn refill_side(
    side: Side,
    kind: PartyKind,
    rules: &Rules,
    frame: &ObservationFrame,
    memory: &mut TeamMemory,
    tuning: TeamTuning,
) {
    let keys: Vec<_> = memory
        .parties
        .iter()
        .filter(|(key, party)| {
            matches!(key, PartyKey::Side(_)) && party.kind == kind && needed(side, frame, party)
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in keys {
        refill(rules, frame, memory, &key, &|memory, owner| {
            budget_left(memory, frame, owner, tuning)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::catalog::TypeIndex;
    use contract::encounter::ObjectiveSiteKind;

    fn objective() -> ObjectiveSite {
        ObjectiveSite {
            id: "open".into(),
            center: [100.0, 100.0],
            radius_m: 20.0,
            kind: ObjectiveSiteKind::Field,
            counterpart: None,
        }
    }

    fn frame(owner: Option<Side>) -> ObservationFrame {
        ObservationFrame {
            skirmish: Some(contract::skirmish::SkirmishView {
                phase: contract::skirmish::Phase::Active,
                players: vec![],
                ready: [true, true],
                preparation_remaining_s: 0.0,
                credits: 0.0,
                unit_count: 0,
                max_units: 30,
                pending: vec![],
                objectives: vec![contract::skirmish::ObjectiveView {
                    id: "open".into(),
                    center: [100.0, 100.0],
                    radius_m: 20.0,
                    owner,
                    capturing: None,
                    capture_progress: 0.0,
                    contested: false,
                }],
                scores: [0.0; 2],
                result: None,
                report: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn open_requires_neutral_and_no_visible_enemy_in_the_radius() {
        let site = objective();
        assert!(objective_open(&frame(None), &site, &BTreeSet::new()));
        assert!(!objective_open(
            &frame(Some(Side::Red)),
            &site,
            &BTreeSet::new()
        ));
        assert!(!objective_open(
            &frame(None),
            &site,
            &["open".to_string()].into_iter().collect()
        ));
        let mut threatened = frame(None);
        threatened
            .identified
            .push(contract::observation::IdentifiedUnit {
                id: contract::observation::ObservedTargetId(1),
                kind: TypeIndex(0),
                cost: 1,
                position: [105.0, 100.0, 0.0],
                yaw: 0.0,
                velocity: [0.0, 0.0],
                members: vec![],
                member_ids: vec![],
                member_slots: vec![],
                member_active_mounts: vec![],
                weapon_poses: vec![],
                reversing: false,
                smoking: false,
                member_leans: vec![],
            });
        assert!(!objective_open(&threatened, &site, &BTreeSet::new()));
    }
}
