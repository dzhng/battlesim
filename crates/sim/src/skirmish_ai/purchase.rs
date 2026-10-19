//! Which card the doctrine buys (D56). The doctrine's recipe names roles;
//! within a role, cards are ranked by value per credit against the enemies
//! the side has identified, the cheaper first on a tie. The air-threat
//! pivot is the same ranking over the cards its weapon data makes
//! specialists in the aircraft it answers.

use super::team::{specialized, unit_value, Threat};
use super::*;
use contract::weapons::TargetClass;

/// `cards`, best first against `threat`: by value per credit, then the
/// cheaper, then the smaller id. With no threat known there is no matchup
/// to judge: the cheaper first.
fn rank(rules: &Rules, cards: &mut [TypeIndex], threat: Option<&Threat>) {
    let cost = |kind: TypeIndex| rules.catalog.get(kind).cost;
    let per_credit = |kind: TypeIndex| {
        threat.map_or(0.0, |threat| {
            unit_value(rules, kind, threat) / f64::from(cost(kind).max(1))
        })
    };
    cards.sort_by(|&a, &b| {
        per_credit(b)
            .total_cmp(&per_credit(a))
            .then(cost(a).cmp(&cost(b)))
            .then(rules.catalog.id(a).cmp(rules.catalog.id(b)))
    });
}

/// The `ordinal`-th best card of `role` against `threat`, the enemies the
/// side has identified, if any (of `category`
/// when no role is named, or the deck has no card of it). An unarmed
/// carrier only carries: pairing alone buys it.
pub(super) fn pick(
    rules: &Rules,
    eligible: &[TypeIndex],
    threat: Option<&Threat>,
    (role, category, ordinal): (Option<&str>, Category, usize),
) -> Option<TypeIndex> {
    let unarmed_carrier =
        |k: TypeIndex| crate::carriage::seats(rules, k) > 0 && rules.catalog.mounts(k).is_empty();
    let mut matching: Vec<_> = role
        .map(|role| {
            eligible
                .iter()
                .copied()
                .filter(|&k| rules.catalog.get(k).has_role(role) && !unarmed_carrier(k))
                .collect()
        })
        .unwrap_or_default();
    if matching.is_empty() && role != Some("artillery") {
        matching = eligible
            .iter()
            .copied()
            .filter(|&k| !unarmed_carrier(k))
            .filter(|&k| {
                let unit = rules.catalog.get(k);
                unit.roster.as_ref().unwrap().category == category
                    && (category == Category::Sup || !rules.catalog.mounts(k).is_empty())
            })
            .collect();
    }
    rank(rules, &mut matching, threat);
    matching
        .get(ordinal.min(matching.len().saturating_sub(1)))
        .copied()
}

/// The best card of `eligible` against aircraft in `layer`, against the
/// aircraft identified there, or either flight there when they are not
/// (a contact, or the opening's insurance). Its cards are those carrying a
/// weapon made for them (AA, not a rifle that also reaches the air), or
/// every card that can engage them when the deck has no such weapon; the
/// same ranking picks among them.
pub(super) fn best_air_defender(
    rules: &Rules,
    eligible: &[TypeIndex],
    frame: &ObservationFrame,
    layer: AltitudeLayer,
) -> Option<TypeIndex> {
    let seen: Vec<_> = frame
        .identified
        .iter()
        .map(|unit| unit.kind)
        .filter(|&kind| rules.catalog.get(kind).mobility.layer() == layer)
        .collect();
    let aircraft = Threat::aims(
        [TargetClass::AirRotor, TargetClass::AirFixedWing].map(|class| (layer, class)),
    );
    let threat = if seen.is_empty() {
        aircraft
    } else {
        Threat::of(rules, seen)
    };
    let capable: Vec<_> = eligible
        .iter()
        .copied()
        .filter(|&kind| unit_value(rules, kind, &threat) > 0.0)
        .collect();
    let mut cards: Vec<_> = capable
        .iter()
        .copied()
        .filter(|&kind| specialized(rules, kind, &threat))
        .collect();
    if cards.is_empty() {
        cards = capable;
    }
    rank(rules, &mut cards, Some(&threat));
    cards.first().copied()
}
