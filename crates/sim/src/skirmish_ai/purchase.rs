//! Which card the doctrine buys (D56). The doctrine's recipe names roles;
//! within a role, cards are ranked by value per credit against the enemies
//! the side has identified, the cheaper first on a tie. The air-threat
//! pivot is the same ranking over the cards its weapon data makes
//! specialists in the aircraft it answers. Where a ground purchase is sent
//! from its lane's entry is decided here too.

use super::team::{specialized, unit_value, Threat};
use super::*;
use contract::skirmish::{PlayerView, SkirmishView};
use contract::weapons::TargetClass;

/// The deciding player's own economy: its credits, cap, purchases and ready
/// flag. The battle narrows each AI's frame to its own player, so the
/// frame's one player row is that player's.
pub(super) fn own_player(view: &SkirmishView) -> Option<&PlayerView> {
    view.players.first()
}

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

/// A ground purchase's lane reservation: follow the entry road heading a
/// short distance, capped before the staging point.  The lane entry remains
/// the route owner's nearest choice, but the arriving unit is clear of its
/// footprint.
pub(super) fn lane_purchase_destination(
    entry: &EntrySite,
    staging: [f64; 2],
    advance_m: f64,
) -> [f64; 2] {
    let span = distance(entry.center, staging);
    let advance = advance_m.max(0.0).min(span * 0.5);
    let toward = [staging[0] - entry.center[0], staging[1] - entry.center[1]];
    let road = [libm::cos(entry.yaw), libm::sin(entry.yaw)];
    let heading = if road[0] * toward[0] + road[1] * toward[1] >= 0.0 {
        road
    } else {
        [-road[0], -road[1]]
    };
    [
        entry.center[0] + heading[0] * advance,
        entry.center[1] + heading[1] * advance,
    ]
}

/// The doctrine's shopping at one decision.
impl BalancedDoctrine {
    /// The cards `me` buys from `eligible` at this decision: the scripted
    /// plan's next cards when it has one, else the doctrine's whole opening
    /// while preparing, else one reinforcement while it has room under the
    /// cap.
    pub(super) fn shopping(
        &mut self,
        frame: &ObservationFrame,
        rules: &Rules,
        me: &PlayerView,
        phase: Phase,
        eligible: &[TypeIndex],
    ) -> Vec<TypeIndex> {
        // Within a role, the best card against the enemies identified (D56).
        let threat = (!frame.identified.is_empty())
            .then(|| team::Threat::of(rules, frame.identified.iter().map(|unit| unit.kind)));
        let pick = |role: Option<&str>, category: Category, ordinal: usize| {
            pick(rules, eligible, threat.as_ref(), (role, category, ordinal))
        };
        let count = |kind: TypeIndex| {
            frame.own.iter().filter(|u| u.kind == kind).count()
                + me.pending
                    .iter()
                    .flat_map(|p| p.kinds())
                    .filter(|&k| k == kind)
                    .count()
        };
        let low_air_demand = frame_has_layer_threat(frame, rules, AltitudeLayer::LowAir);
        let high_air_demand = frame_has_layer_threat(frame, rules, AltitudeLayer::HighAir);
        let opening = match self.doctrine {
            Doctrine::TankSwarm => [
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 0),
                ("mbt", Category::Veh, 1),
                ("recon", Category::Rec, 0),
                ("at", Category::Inf, 0),
                ("infantry", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("artillery", Category::Sup, 0),
                ("logistics", Category::Sup, 0),
            ],
            Doctrine::AirDenial => [
                ("recon", Category::Rec, 0),
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 1),
                ("at", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("mbt", Category::Veh, 0),
                ("recon", Category::Rec, 0),
                ("artillery", Category::Sup, 0),
                ("logistics", Category::Sup, 0),
            ],
            _ => [
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 0),
                ("infantry", Category::Inf, 1),
                ("mbt", Category::Veh, 0),
                ("recon", Category::Rec, 0),
                ("at", Category::Inf, 0),
                ("light_vehicle", Category::Veh, 0),
                ("artillery", Category::Sup, 0),
                ("logistics", Category::Sup, 0),
            ],
        };
        let desired: Vec<_> = opening
            .iter()
            .filter_map(|&(role, category, ordinal)| pick(Some(role), category, ordinal))
            .collect();
        // AA is insurance every opening carries, picked by what its weapons
        // engage. It goes first, so it is bought before any other role
        // competes for credits or slots.
        let high_air_defender = best_air_defender(rules, eligible, frame, AltitudeLayer::HighAir);
        let desired: Vec<_> = high_air_defender.iter().copied().chain(desired).collect();
        // Each squad's carrier follows it (D17), and rides on it: it is
        // bought only for a squad the player holds or buys with it.
        let desired: Vec<(TypeIndex, bool)> = desired
            .into_iter()
            .flat_map(|kind| {
                let size = rules.catalog.get(kind).squad_size();
                std::iter::once((kind, false)).chain(
                    transport::carrier_for(rules, eligible, size).map(|carrier| (carrier, true)),
                )
            })
            .collect();
        // The whole opening is bought in one decision, so the opponent is
        // ready at once and the player's own readiness starts the battle.
        if let Some(plan) = self.purchase_plan.as_ref() {
            let mut credits = me.credits;
            let mut slots = me.unit_count;
            let mut cursor = plan.next;
            let mut purchases = Vec::new();
            while slots < me.cap {
                let Some(&kind) = plan.cards.get(cursor) else {
                    break;
                };
                let cost = f64::from(rules.catalog.get(kind).cost);
                if cost > credits {
                    break;
                }
                purchases.push(kind);
                credits -= cost;
                slots += 1;
                cursor += 1;
                if cursor == plan.cards.len() {
                    let Some(repeat_from) = plan.repeat_from else {
                        break;
                    };
                    cursor = repeat_from;
                }
            }
            purchases
        } else if phase == Phase::Preparation {
            let mut credits = me.credits;
            let mut slots = me.unit_count;
            let mut needed = BTreeMap::new();
            let mut squad_held = false;
            desired
                .iter()
                .copied()
                .filter(|&(kind, rides)| {
                    let wanted = needed.entry(kind).or_insert(0);
                    *wanted += 1;
                    let cost = f64::from(rules.catalog.get(kind).cost);
                    let held = count(kind) >= *wanted;
                    let buy = !held && (!rides || squad_held) && cost <= credits && slots < me.cap;
                    if buy {
                        credits -= cost;
                        slots += 1;
                    }
                    squad_held = held || buy;
                    buy
                })
                .map(|(kind, _)| kind)
                .collect()
        } else if me.unit_count < me.cap {
            let purchase = {
                let truck = pick(Some("logistics"), Category::Sup, 0);
                if truck.is_some()
                    && !frame
                        .own
                        .iter()
                        .any(|u| rules.catalog.get(u.kind).has_role("logistics"))
                    && !(me.pending.iter().flat_map(|p| p.kinds()))
                        .any(|k| rules.catalog.get(k).has_role("logistics"))
                {
                    truck.filter(|&k| f64::from(rules.catalog.get(k).cost) <= me.credits)
                } else {
                    // Each role in turn, armour too (D2); helicopters as
                    // light vehicles are (D35), then jets (the air category,
                    // no role of their own). A deck without one buys the
                    // next role; an unaffordable one is saved for.
                    let roles = [
                        (Some("infantry"), Category::Inf),
                        (Some("recon"), Category::Rec),
                        (Some("at"), Category::Inf),
                        (Some("light_vehicle"), Category::Veh),
                        (Some("mbt"), Category::Veh),
                        (Some("artillery"), Category::Sup),
                        (Some("helicopter"), Category::Hel),
                        (None, Category::Air),
                    ];
                    let selected = if let Some(layer) = [
                        (AltitudeLayer::HighAir, high_air_demand),
                        (AltitudeLayer::LowAir, low_air_demand),
                    ]
                    .into_iter()
                    .find_map(|(layer, demanded)| demanded.then_some(layer))
                    {
                        best_air_defender(rules, eligible, frame, layer)
                            .filter(|&k| f64::from(rules.catalog.get(k).cost) <= me.credits)
                            .map(|k| (Some(0), k))
                    } else if let Some(want) = transport::want(
                        rules,
                        eligible,
                        frame
                            .own
                            .iter()
                            .map(|u| u.kind)
                            .chain(me.pending.iter().flat_map(|p| p.kinds())),
                    ) {
                        // One carrier per squad comes before the rotation,
                        // and is saved for like its next role: a lost
                        // transport is rebought, a lost squad replaced (D17).
                        let (transport::Want::Carrier(k) | transport::Want::Squad(k)) = want;
                        (f64::from(rules.catalog.get(k).cost) <= me.credits).then_some((None, k))
                    } else {
                        // The next role in the rotation is a plan the AI
                        // saves for: credits it can't yet spend on it wait,
                        // so an expensive card is bought as surely as a
                        // cheap one. Only a role the deck has no card for
                        // is passed over.
                        (0..roles.len())
                            .find_map(|offset| {
                                let index = (self.reinforcement + offset) % roles.len();
                                pick(roles[index].0, roles[index].1, 0).map(|k| (Some(index), k))
                            })
                            .filter(|&(_, k)| f64::from(rules.catalog.get(k).cost) <= me.credits)
                    };
                    selected.map(|(index, k)| {
                        if let Some(index) = index {
                            self.reinforcement = (index + 1) % roles.len();
                        }
                        k
                    })
                }
            };
            purchase.into_iter().collect()
        } else {
            vec![]
        }
    }
}
