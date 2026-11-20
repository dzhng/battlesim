//! Which card the doctrine buys (D56). The doctrine's recipe names roles;
//! within a role, cards are ranked by value per credit against the enemies
//! the side has identified, the cheaper first on a tie. The air-threat
//! pivot is the same ranking over the cards its weapon data makes
//! specialists in the aircraft it answers. A squad comes aboard its ride
//! as one purchase (M17). Where a ground purchase is sent from its lane's
//! entry is decided here too.

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

/// Whether the type moves on the ground, for the standing supplier's job.
pub(super) fn drives(rules: &Rules, kind: TypeIndex) -> bool {
    rules.catalog.get(kind).mobility.locomotion().is_some()
}

/// A supplier that drives to the standing support post.
pub(super) fn standing_supplier(rules: &Rules, kind: TypeIndex) -> bool {
    let t = rules.catalog.get(kind);
    t.has_role("logistics") && t.capabilities.supply.is_some() && drives(rules, kind)
}

/// A supplier that can land beside a group for emergency service.
pub(super) fn flying_supplier(rules: &Rules, kind: TypeIndex) -> bool {
    let t = rules.catalog.get(kind);
    t.has_role("logistics") && t.capabilities.supply.is_some() && t.landing().is_some()
}

/// The `ordinal`-th best card of `role` against `threat`, the enemies the
/// side has identified, if any (of `category`
/// when no role is named, or the deck has no card of it). An unarmed
/// carrier only carries: pairing alone buys it. Standing logistics drives.
pub(super) fn pick(
    rules: &Rules,
    eligible: &[TypeIndex],
    threat: Option<&Threat>,
    (role, category, ordinal): (Option<&str>, Category, usize),
) -> Option<TypeIndex> {
    let eligible = eligible
        .iter()
        .copied()
        .filter(|&k| role != Some("logistics") || standing_supplier(rules, k));
    let unarmed_carrier =
        |k: TypeIndex| crate::carriage::seats(rules, k) > 0 && rules.catalog.mounts(k).is_empty();
    let mut matching: Vec<_> = role
        .map(|role| {
            eligible
                .clone()
                .filter(|&k| rules.catalog.get(k).has_role(role) && !unarmed_carrier(k))
                .collect()
        })
        .unwrap_or_default();
    if matching.is_empty() && role != Some("artillery") {
        matching = eligible
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
    let defends = air_defender(rules, eligible, &threat);
    let mut cards: Vec<_> = eligible.iter().copied().filter(|&k| defends(k)).collect();
    rank(rules, &mut cards, Some(&threat));
    cards.first().copied()
}

/// Whether a unit answers `threat` in the air: it carries a weapon made for
/// it, or, when no card of `eligible` does, it can engage it at all.
fn air_defender<'a>(
    rules: &'a Rules,
    eligible: &[TypeIndex],
    threat: &'a Threat,
) -> impl Fn(TypeIndex) -> bool + 'a {
    let capable = move |kind: TypeIndex| unit_value(rules, kind, threat) > 0.0;
    let specialist = move |kind: TypeIndex| capable(kind) && specialized(rules, kind, threat);
    let deck_has_specialist = eligible.iter().any(|&k| specialist(k));
    move |kind| {
        if deck_has_specialist {
            specialist(kind)
        } else {
            capable(kind)
        }
    }
}

/// Whether the side's answer to aircraft in `layer` falls short of what it
/// faces there: the defenders this player and its `allies` field and have
/// bought cost less than the aircraft identified there, or, with only
/// contacts there, the side holds none. An answer that matches the threat
/// stops buying, so the rest of the force is still rebuilt.
fn air_defence_short(
    frame: &ObservationFrame,
    rules: &Rules,
    me: &PlayerView,
    allies: &[TypeIndex],
    eligible: &[TypeIndex],
    layer: AltitudeLayer,
) -> bool {
    if !frame_has_layer_threat(frame, rules, layer) {
        return false;
    }
    let seen: f64 = frame
        .identified
        .iter()
        .filter(|unit| rules.catalog.get(unit.kind).mobility.layer() == layer)
        .map(|unit| f64::from(unit.cost))
        .sum();
    let aircraft = Threat::aims(
        [TargetClass::AirRotor, TargetClass::AirFixedWing].map(|class| (layer, class)),
    );
    let defends = air_defender(rules, eligible, &aircraft);
    let held = frame
        .own
        .iter()
        .map(|unit| unit.kind)
        .chain(me.pending.iter().flat_map(|p| p.kinds()))
        .chain(allies.iter().copied())
        .filter(|&kind| defends(kind));
    let (count, worth) = held.fold((0, 0.0), |(n, sum), k| (n + 1, sum + cost(rules, k)));
    if seen > 0.0 {
        worth < seen
    } else {
        count == 0
    }
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
    // A missing or coincident staging point still needs to leave the road
    // mouth. The entry heading supplies the only useful direction then.
    let advance = if span > 0.0 {
        advance_m.max(0.0).min(span * 0.5)
    } else {
        advance_m.max(0.0)
    };
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

/// One purchase the AI sends: a card, and the carrier bought with it, in
/// which the squad arrives aboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Buy {
    pub kind: TypeIndex,
    pub carrier: Option<TypeIndex>,
}

impl Buy {
    pub(super) fn alone(kind: TypeIndex) -> Self {
        Self {
            kind,
            carrier: None,
        }
    }

    /// What it costs: both prices for a squad with its ride.
    fn price(self, rules: &Rules) -> f64 {
        self.kinds().map(|kind| cost(rules, kind)).sum()
    }

    /// The places it takes under the cap: one per unit.
    fn units(self) -> u32 {
        1 + u32::from(self.carrier.is_some())
    }

    fn kinds(self) -> impl Iterator<Item = TypeIndex> {
        std::iter::once(self.kind).chain(self.carrier)
    }
}

fn cost(rules: &Rules, kind: TypeIndex) -> f64 {
    f64::from(rules.catalog.get(kind).cost)
}

/// An external purchase plan: the cards it buys in order, the next one, and
/// where it repeats from once spent, if it does.
#[derive(Clone, Debug)]
pub(super) struct PurchasePlan {
    cards: Vec<TypeIndex>,
    next: usize,
    repeat_from: Option<usize>,
}

/// A scripted plan replaces the doctrine's own shopping.
impl BalancedDoctrine {
    pub fn set_purchase_plan(&mut self, cards: Vec<TypeIndex>) {
        self.set_purchase_plan_with_repeat(cards, None);
    }

    pub(crate) fn set_purchase_plan_with_repeat(
        &mut self,
        cards: Vec<TypeIndex>,
        repeat_from: Option<usize>,
    ) {
        let repeat_from = repeat_from.filter(|&index| index < cards.len());
        self.purchase_plan = Some(PurchasePlan {
            cards,
            next: 0,
            repeat_from,
        });
    }

    /// The battle's answer to a purchase this policy sent, of `cards`
    /// cards (two for a squad with its ride): an admitted one moves its
    /// plan past them, a refused one is tried again.
    pub(crate) fn purchase_result(&mut self, accepted: bool, cards: usize) {
        if let Some(plan) = self.purchase_plan.as_mut().filter(|_| accepted) {
            plan.advance(cards);
        }
    }

    pub(crate) fn has_purchase_plan(&self) -> bool {
        self.purchase_plan.is_some()
    }
}

impl PurchasePlan {
    /// The card after the one at `at`: the next, else the repeat's first,
    /// else none once the plan is spent.
    fn after(&self, at: usize) -> Option<usize> {
        if at + 1 < self.cards.len() {
            Some(at + 1)
        } else {
            self.repeat_from
        }
    }

    /// Move past `cards` cards the battle admitted.
    pub(super) fn advance(&mut self, cards: usize) {
        for _ in 0..cards {
            self.next = self.after(self.next).unwrap_or(self.cards.len());
        }
    }
}

/// The doctrine's shopping at one decision.
impl BalancedDoctrine {
    /// What `me` buys from `eligible` at this decision: the scripted plan's
    /// next cards when it has one, else the doctrine's whole opening while
    /// preparing, else one reinforcement while it has room under the cap.
    /// A squad bought where no carrier waits for it comes with its ride as
    /// one purchase (M17). The opening, a scripted plan and an answer to
    /// aircraft buy it alone when credits cover only the squad, and its
    /// carrier is bought once it asks for a ride (D17); the rotation saves
    /// for both. With room under the cap for one unit, the squad goes alone.
    pub(super) fn shopping(
        &mut self,
        frame: &ObservationFrame,
        rules: &Rules,
        me: &PlayerView,
        phase: Phase,
        eligible: &[TypeIndex],
        assignment: Option<&Assignment>,
    ) -> Vec<Buy> {
        if let Some(plan) = self.purchase_plan.as_ref() {
            planned(rules, me, plan)
        } else if phase == Phase::Preparation {
            self.opening(frame, rules, me, eligible)
        } else if me.unit_count < me.cap {
            let allies = assignment.map_or(&[][..], |a| &a.allies);
            self.reinforcement(frame, rules, me, eligible, allies)
                .into_iter()
                .collect()
        } else {
            vec![]
        }
    }

    /// The doctrine's whole opening, bought in one decision so the opponent
    /// is ready at once and the player's own readiness starts the battle.
    fn opening(
        &self,
        frame: &ObservationFrame,
        rules: &Rules,
        me: &PlayerView,
        eligible: &[TypeIndex],
    ) -> Vec<Buy> {
        let threat = threat(frame, rules);
        let count = |kind: TypeIndex| {
            frame.own.iter().filter(|u| u.kind == kind).count()
                + (me.pending.iter().flat_map(|p| p.kinds()))
                    .filter(|&k| k == kind)
                    .count()
        };
        let recipe = match self.doctrine {
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
        let desired = recipe.iter().filter_map(|&(role, category, ordinal)| {
            pick(
                rules,
                eligible,
                threat.as_ref(),
                (Some(role), category, ordinal),
            )
        });
        // AA is insurance every opening carries, picked by what its weapons
        // engage. It goes first, so it is bought before any other role
        // competes for credits or slots.
        let high_air_defender = best_air_defender(rules, eligible, frame, AltitudeLayer::HighAir);
        // Each squad's carrier follows it (D17), and rides on it: it is
        // bought only for a squad the player holds or buys with it.
        let desired = high_air_defender
            .into_iter()
            .chain(desired)
            .flat_map(|kind| {
                let size = rules.catalog.get(kind).squad_size();
                std::iter::once((kind, false)).chain(
                    transport::carrier_for(rules, eligible, size).map(|carrier| (carrier, true)),
                )
            });
        let mut credits = me.credits;
        let mut slots = me.unit_count;
        let mut needed = BTreeMap::new();
        let mut purchases: Vec<Buy> = Vec::new();
        // The squad before a ride: none when neither held nor bought, else
        // the purchase that bought it, if this decision did.
        let mut squad: Option<Option<usize>> = None;
        for (kind, rides) in desired {
            let wanted = needed.entry(kind).or_insert(0);
            *wanted += 1;
            let held = count(kind) >= *wanted;
            let buy = !held && cost(rules, kind) <= credits && slots < me.cap;
            if !rides {
                if buy {
                    purchases.push(Buy::alone(kind));
                }
                squad = (held || buy).then(|| buy.then(|| purchases.len() - 1));
            } else if buy {
                match squad {
                    // A squad bought now rides in aboard it.
                    Some(Some(index)) => purchases[index].carrier = Some(kind),
                    Some(None) => purchases.push(Buy::alone(kind)),
                    None => continue,
                }
            }
            if buy {
                credits -= cost(rules, kind);
                slots += 1;
            }
        }
        purchases
    }

    /// One reinforcement, or none while it saves for one.
    fn reinforcement(
        &mut self,
        frame: &ObservationFrame,
        rules: &Rules,
        me: &PlayerView,
        eligible: &[TypeIndex],
        allies: &[TypeIndex],
    ) -> Option<Buy> {
        let threat = threat(frame, rules);
        let pick = |role: Option<&str>, category: Category, ordinal: usize| {
            pick(rules, eligible, threat.as_ref(), (role, category, ordinal))
        };
        let affordable = |buy: Buy| (buy.price(rules) <= me.credits).then_some(buy);
        // A new squad no carrier waits for comes with its ride, while the
        // cap has room for both.
        let with_ride = |kind: TypeIndex| {
            let size = rules.catalog.get(kind).squad_size();
            Buy {
                kind,
                carrier: transport::carrier_for(rules, eligible, size)
                    .filter(|_| me.unit_count + 2 <= me.cap),
            }
        };
        let truck = pick(Some("logistics"), Category::Sup, 0);
        if truck.is_some()
            && !frame.own.iter().any(|u| standing_supplier(rules, u.kind))
            && !(me.pending.iter().flat_map(|p| p.kinds())).any(|k| standing_supplier(rules, k))
        {
            return truck.map(Buy::alone).and_then(affordable);
        }
        // Each role in turn, armour too (D2); helicopters as light vehicles
        // are (D35), then jets (the air category, no role of their own). A
        // deck without one buys the next role; an unaffordable one is saved
        // for.
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
        // A deck with no card that engages the aircraft has no answer to
        // buy: it shops as if none were there.
        let air_answer = [AltitudeLayer::HighAir, AltitudeLayer::LowAir]
            .into_iter()
            .filter(|&layer| air_defence_short(frame, rules, me, allies, eligible, layer))
            .find_map(|layer| best_air_defender(rules, eligible, frame, layer));
        let selected = if let Some(k) = air_answer {
            // An answer to aircraft can't wait: it goes alone when its
            // ride is more than the credits cover.
            affordable(with_ride(k))
                .or_else(|| affordable(Buy::alone(k)))
                .map(|buy| (Some(0), buy))
        } else if let Some(k) = self.ride_wanted(rules, me, eligible) {
            // A squad that asked for a ride and found none is bought one
            // before the rotation, saved for like its next role (D17).
            affordable(Buy::alone(k)).map(|buy| (None, buy))
        } else if let Some(k) = transport::squad_wanted(
            rules,
            eligible,
            frame
                .own
                .iter()
                .map(|u| u.kind)
                .chain(me.pending.iter().flat_map(|p| p.kinds())),
        ) {
            // A carrier whose squad was lost is bought a squad it seats.
            affordable(Buy::alone(k)).map(|buy| (None, buy))
        } else {
            // The next role in the rotation is a plan the AI saves for:
            // credits it can't yet spend on it wait, so an expensive card is
            // bought as surely as a cheap one. Only a role the deck has no
            // card for is passed over. With no squad wanted, every carrier
            // has its squad, so a squad this buys has none waiting for it:
            // it is saved for with its ride, and arrives aboard.
            (0..roles.len())
                .find_map(|offset| {
                    let index = (self.reinforcement + offset) % roles.len();
                    pick(roles[index].0, roles[index].1, 0).map(|k| (Some(index), k))
                })
                .and_then(|(index, k)| affordable(with_ride(k)).map(|buy| (index, buy)))
        };
        let (index, buy) = selected?;
        if let Some(index) = index {
            self.reinforcement = (index + 1) % roles.len();
        }
        Some(buy)
    }

    /// The carrier for the squad the last decision found no ride for, unless
    /// one bought on its own since, which seats it, is on its way (one
    /// bought with a squad is that squad's).
    fn ride_wanted(
        &self,
        rules: &Rules,
        me: &PlayerView,
        eligible: &[TypeIndex],
    ) -> Option<TypeIndex> {
        let size = self.ride_short?;
        let coming = me
            .pending
            .iter()
            .any(|p| p.carrier.is_none() && crate::carriage::seats_squad(rules, p.kind, size));
        (!coming)
            .then(|| transport::carrier_for(rules, eligible, size))
            .flatten()
    }
}

/// Within a role, the best card is judged against the enemies identified
/// (D56), when there are any.
fn threat(frame: &ObservationFrame, rules: &Rules) -> Option<Threat> {
    (!frame.identified.is_empty())
        .then(|| Threat::of(rules, frame.identified.iter().map(|unit| unit.kind)))
}

/// A scripted plan's next cards, in order, while credits and the cap cover
/// them. A squad's card followed by one that seats it is one purchase, the
/// squad aboard its ride, when both are covered; else the squad goes alone.
fn planned(rules: &Rules, me: &PlayerView, plan: &PurchasePlan) -> Vec<Buy> {
    let mut credits = me.credits;
    let mut slots = me.unit_count;
    let mut cursor = Some(plan.next).filter(|&at| at < plan.cards.len());
    let mut purchases = Vec::new();
    while let Some(at) = cursor {
        let kind = plan.cards[at];
        let after = plan.after(at);
        let pair = Buy {
            kind,
            carrier: after.map(|next| plan.cards[next]).filter(|&carrier| {
                crate::carriage::seats_squad(rules, carrier, rules.catalog.get(kind).squad_size())
            }),
        };
        let covered = |buy: Buy| buy.price(rules) <= credits && slots + buy.units() <= me.cap;
        let Some(buy) = [pair, Buy::alone(kind)]
            .into_iter()
            .find(|&buy| covered(buy))
        else {
            break;
        };
        purchases.push(buy);
        credits -= buy.price(rules);
        slots += buy.units();
        cursor = if buy.carrier.is_some() {
            after.and_then(|next| plan.after(next))
        } else {
            after
        };
    }
    purchases
}
