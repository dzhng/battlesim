//! What a unit is worth against what it faces (D56): the one measure of
//! strength the team planner and the doctrine's purchases read.
//!
//! A unit's value against one kind of target is its card cost when one of
//! its weapons can engage that kind (by the weapon rows' layers and target
//! classes), else nothing. Against a mix it is the cost-weighted share of
//! the mix it can engage. Cost is the only authored worth; specialization
//! comes from weapon data, never from a role name.

use super::super::capability::{aim_of, unit_engages, unit_specializes_in, Aim, GROUND};
use super::*;
use contract::catalog::TypeIndex;

/// What a side measures units against: the aims of the units it faces,
/// each weighted by their card cost. With nothing known it is the ground,
/// soldiers and hulls alike.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Threat(Vec<(Aim, f64)>);

impl Threat {
    pub(crate) fn of(rules: &Rules, kinds: impl IntoIterator<Item = TypeIndex>) -> Self {
        let mut weights: Vec<(Aim, f64)> = Vec::new();
        for kind in kinds {
            let (aim, cost) = (aim_of(rules, kind), card_cost(rules, kind));
            match weights.iter_mut().find(|(known, _)| *known == aim) {
                Some((_, weight)) => *weight += cost,
                None => weights.push((aim, cost)),
            }
        }
        if weights.is_empty() {
            return Self::aims(GROUND);
        }
        Self(weights)
    }

    /// `aims`, weighed alike: a threat known by where it is, not by kind.
    pub(crate) fn aims(aims: impl IntoIterator<Item = Aim>) -> Self {
        Self(aims.into_iter().map(|aim| (aim, 1.0)).collect())
    }
}

/// `kind`'s value against `threat`: its card cost times the share of the
/// threat its weapons can engage.
pub(crate) fn unit_value(rules: &Rules, kind: TypeIndex, threat: &Threat) -> f64 {
    let total: f64 = threat.0.iter().map(|(_, weight)| weight).sum();
    let engaged: f64 = threat
        .0
        .iter()
        .filter(|(aim, _)| unit_engages(rules, kind, *aim))
        .fold(0.0, |sum, (_, weight)| sum + weight);
    card_cost(rules, kind) * engaged / total
}

/// Whether `kind` is a specialist against `threat`: it carries a weapon
/// made for it (the user's "AT, AA"), not only guns that also reach it.
pub(crate) fn specialized(rules: &Rules, kind: TypeIndex, threat: &Threat) -> bool {
    let aims: Vec<_> = threat.0.iter().map(|(aim, _)| *aim).collect();
    unit_specializes_in(rules, kind, &aims)
}

fn card_cost(rules: &Rules, kind: TypeIndex) -> f64 {
    f64::from(rules.catalog.get(kind).cost.max(1))
}

/// One plan step's scale of strength: value over the mean roster card cost,
/// so a unit of that cost able to engage all it faces counts 1.
pub(super) struct Measure<'a> {
    rules: &'a Rules,
    reference: f64,
    /// What the side's own units face: the enemies it has identified.
    enemy: Threat,
}

impl<'a> Measure<'a> {
    pub(super) fn new(rules: &'a Rules, frame: &ObservationFrame) -> Self {
        let (sum, count) = rules
            .catalog
            .indices()
            .filter(|&kind| rules.catalog.get(kind).roster.is_some())
            .fold((0.0, 0usize), |(sum, count), kind| {
                (sum + card_cost(rules, kind), count + 1)
            });
        Self {
            rules,
            reference: if count == 0 { 1.0 } else { sum / count as f64 },
            enemy: Threat::of(rules, frame.identified.iter().map(|unit| unit.kind)),
        }
    }

    /// An own unit's strength against the enemies the side has identified.
    pub(super) fn own(&self, kind: TypeIndex) -> f64 {
        self.against(kind, &self.enemy)
    }

    pub(super) fn against(&self, kind: TypeIndex, threat: &Threat) -> f64 {
        unit_value(self.rules, kind, threat) / self.reference
    }
}

/// A player's force: the strength of its units in view and of those it has
/// bought that are still arriving.
pub(super) fn force(measure: &Measure, frame: &ObservationFrame, player: PlayerIndex) -> f64 {
    let fielded: f64 = frame
        .own
        .iter()
        .filter(|unit| unit.owner == Some(player))
        .map(|unit| measure.own(unit.kind))
        .sum();
    let arriving: f64 = frame
        .skirmish
        .as_ref()
        .and_then(|view| view.players.iter().find(|p| p.index == player))
        .map_or(0.0, |p| p.pending.iter().map(|p| measure.own(p.kind)).sum());
    fielded + arriving
}

/// The strength `owner` has on party duty, of every kind.
pub(super) fn on_duty(
    measure: &Measure,
    frame: &ObservationFrame,
    memory: &TeamMemory,
    owner: PlayerIndex,
) -> f64 {
    memory
        .parties
        .values()
        .filter(|party| party.owner == owner)
        .flat_map(|party| &party.units)
        .filter_map(|id| frame.own.iter().find(|unit| unit.id == *id))
        .map(|unit| measure.own(unit.kind))
        .sum()
}

/// What is left of `owner`'s one party budget: a share of its force, at
/// least one unit of the scale.
pub(super) fn budget_left(
    measure: &Measure,
    memory: &TeamMemory,
    frame: &ObservationFrame,
    owner: PlayerIndex,
    tuning: TeamTuning,
) -> f64 {
    let budget = (force(measure, frame, owner) * tuning.party_budget_share).max(1.0);
    (budget - on_duty(measure, frame, memory, owner)).max(0.0)
}
