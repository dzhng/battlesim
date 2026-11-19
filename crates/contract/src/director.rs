//! An authored scenario's director: the rules of its progression.
//!
//! A director is chapters of beats. A beat fires once, on the first tick its
//! trigger holds while its chapter is open; it may tell sides it fired (its
//! audience) and act: order a directed seat's units, grant credits, offer
//! purchases. A chapter closes when its last beat fires, and the next opens on
//! the next tick. Triggers read what is true in the battle (where units are,
//! what a side knows, the match's phase), never what was commanded, so a
//! beat holds however its state came about. The director is part of the
//! battle's rules: it runs the same live and in replay, and its state is in
//! the digest.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command::Engagement;
use crate::ids::{Side, Tick, UnitId};
use crate::observation::SuppressionTier;
use crate::scenario_document::{CreditGrant, Zone};
use crate::skirmish::{MatchResult, Phase, PurchaseSpec};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Director {
    #[serde(default)]
    pub chapters: Vec<Chapter>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    pub id: String,
    pub beats: Vec<Beat>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub id: String,
    pub when: Trigger,
    /// A beat the chapter doesn't wait for: it answers trouble (a unit the
    /// player needs, lost), so it fires only if that comes about.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recovery: bool,
    /// The sides told it fired.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<Side>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Action>,
}

/// What must be true for a beat to fire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    /// Its chapter is open.
    ChapterOpens,
    /// `beat` fired at least `after_s` seconds ago.
    Fired {
        beat: String,
        #[serde(default)]
        after_s: f64,
    },
    /// The match is in this phase.
    Phase(Phase),
    /// `player` has bought `purchase`: it is on its way or on the field.
    Bought {
        player: String,
        purchase: PurchaseSpec,
    },
    /// Any of the selected living units is so.
    Units {
        who: Selector,
        are: UnitFact,
    },
    /// `side` identifies any of the selected units.
    Identifies {
        side: Side,
        who: Selector,
    },
    /// `side` holds a contact whose area meets the zone.
    ContactIn {
        side: Side,
        zone: String,
    },
    /// The objective, by its site's id, stands so.
    Objective {
        id: String,
        is: ObjectiveState,
    },
    /// Every unit the selector names is dead, and it named at least one.
    Destroyed(Selector),
    /// `player` has `purchase` on its way, not yet on the field.
    Pending {
        player: String,
        purchase: PurchaseSpec,
    },
    /// `side` holds a contact its radar receivers found.
    RadarContact {
        side: Side,
    },
    /// The match ended so.
    MatchResult(MatchResult),
    All(Vec<Trigger>),
    Any(Vec<Trigger>),
}

/// Units a beat speaks of.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Selector {
    /// A force the scenario places, by its name.
    Force(String),
    /// Every unit of `variant` that `player` owns.
    Bought { player: String, variant: String },
}

/// Something true of a unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitFact {
    /// On the field, inside the zone.
    In(String),
    /// At this fire policy.
    Engagement(Engagement),
    /// Aboard a carrier, or garrisoned in a building.
    Inside(Host),
    /// Suppressed at least this much (a vehicle never is).
    Suppression(SuppressionTier),
    /// Its current order is an attack-move.
    AttackMoving,
    /// It has spent smoke it hasn't been resupplied with.
    PoppedSmoke,
    /// It drives backwards this tick.
    Reversing,
    /// Its current order is so.
    Ordered(OrderFact),
    /// Set up, all the way (a unit that deploys).
    Deployed,
    /// Being replenished, repaired or reinforced by a supply vehicle now.
    Serviced,
    /// In the air.
    Airborne,
    /// On the field: a purchase that arrived, or a carried squad set down.
    Arrived,
}

/// What a unit's current order is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OrderFact {
    /// To attack one of the selected units.
    Attack(Selector),
    /// To fire on ground inside the zone.
    AttackGround(String),
}

/// What a unit can be inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Host {
    Carrier,
    Building,
}

/// How an objective stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveState {
    /// Held by the side, and not contested.
    OwnedBy(Side),
    /// The side is taking it.
    Capturing(Side),
    /// Both sides have combat units in its ring.
    Contested,
}

/// What a beat does when it fires.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    /// Order the selected units; only a directed seat's units obey.
    Order {
        units: Selector,
        order: DirectedOrder,
    },
    /// Add credits to `player`'s wallet.
    Grant {
        player: String,
        credits: CreditGrant,
    },
    /// Add purchases to `player`'s narrowed shop.
    Offer {
        player: String,
        purchases: Vec<PurchaseSpec>,
    },
    /// Buy `purchase` for `player`, sent to the zone's centre; only a
    /// directed seat buys.
    Buy {
        player: String,
        purchase: PurchaseSpec,
        at: String,
    },
    /// Say `player` is ready to start; only a directed seat says so.
    Ready { player: String },
    /// Give `player` back its faction's whole shop.
    OpenShop { player: String },
}

/// An order a beat gives, in the scenario's own terms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectedOrder {
    /// Drive to the zone's centre.
    MoveTo(String),
    /// Fire on the ground at the zone's centre.
    AttackGround(String),
    /// Take this fire policy.
    Engage(Engagement),
    /// Carriers set their squads down where they stand.
    Unload,
    /// Squads step into the building nearest the zone's centre, within it.
    Occupy(String),
    /// Attack-move to the zone's centre.
    AttackMoveTo(String),
    /// Drive home by the side's road in and leave the battle.
    Withdraw,
    /// Pop smoke where it stands.
    PopSmoke,
    /// Back up to the zone's centre, front to the threat.
    ReverseTo(String),
    /// Attack the first living unit the selector names, if the side
    /// identifies it.
    Attack(Selector),
    /// Set up where it stands (a unit that deploys).
    Deploy,
    /// Attack the contact the ordering side holds that meets the zone.
    AttackContactIn(String),
}

/// A scenario's solution: beats, in the director's own terms, that play its
/// human player through every chapter. The walkthrough merges them into the
/// director and hands the human's seat to it; nothing in a played battle
/// reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Solution {
    /// A loose tripwire on how long the walkthrough may take, never a target.
    pub max_minutes: f64,
    pub chapters: Vec<Chapter>,
}

/// An authored scenario's progress as one side is told it: the open chapter
/// (every side knows it) and the beats this side is in the audience of, with
/// the tick each fired. A beat is its place among all the director's beats,
/// in chapter order; the publication's layout names it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenarioView {
    /// The open chapter's place; past the last once every chapter closed.
    pub chapter: u32,
    pub beats: Vec<(u32, Tick)>,
}

/// A director as a battle runs it: its chapters, and the names they use
/// resolved to the battle's zones and units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectorSetup {
    pub chapters: Vec<Chapter>,
    pub zones: BTreeMap<String, Zone>,
    /// Each placed force's unit, by name.
    pub forces: BTreeMap<String, UnitId>,
}

impl Director {
    /// Refuses ids that repeat, a `Fired` that names no earlier beat, and a
    /// name no zone, force or player answers to. Each refusal names the beat.
    pub fn admit(
        &self,
        zones: &BTreeMap<String, Zone>,
        forces: &BTreeMap<String, UnitId>,
        players: &[crate::players::Player],
        objectives: &[String],
    ) -> Result<(), String> {
        let mut chapters = std::collections::BTreeSet::new();
        let mut earlier = std::collections::BTreeSet::new();
        for chapter in &self.chapters {
            if !chapters.insert(&chapter.id) {
                return Err(format!("director.{}: the chapter's id repeats", chapter.id));
            }
            for beat in &chapter.beats {
                let at = format!("director.{}.{}", chapter.id, beat.id);
                let names = Names {
                    zones,
                    forces,
                    players,
                    objectives,
                    earlier: &earlier,
                };
                names
                    .trigger(&beat.when)
                    .map_err(|e| format!("{at}: {e}"))?;
                for action in &beat.then {
                    names.action(action).map_err(|e| format!("{at}: {e}"))?;
                }
                if !earlier.insert(beat.id.clone()) {
                    return Err(format!("{at}: the beat's id repeats"));
                }
            }
        }
        // The story ends where the match does.
        if let Some(last) = self.chapters.last() {
            let ends = (last.beats.iter())
                .any(|b| !b.recovery && matches!(b.when, Trigger::MatchResult(_)));
            if !ends {
                return Err(format!(
                    "director.{}: the last chapter ends on a match result",
                    last.id
                ));
            }
        }
        Ok(())
    }
}

/// The names a director may use, and the beats before the one checked.
struct Names<'a> {
    zones: &'a BTreeMap<String, Zone>,
    forces: &'a BTreeMap<String, UnitId>,
    players: &'a [crate::players::Player],
    objectives: &'a [String],
    earlier: &'a std::collections::BTreeSet<String>,
}

impl Names<'_> {
    fn zone(&self, name: &str) -> Result<(), String> {
        self.zones
            .contains_key(name)
            .then_some(())
            .ok_or(format!("no zone {name:?}"))
    }
    fn player(&self, id: &str) -> Result<(), String> {
        self.players
            .iter()
            .any(|p| p.id == id)
            .then_some(())
            .ok_or(format!("no player {id:?}"))
    }
    fn selector(&self, who: &Selector) -> Result<(), String> {
        match who {
            Selector::Force(name) => self
                .forces
                .contains_key(name)
                .then_some(())
                .ok_or(format!("no force {name:?}")),
            Selector::Bought { player, .. } => self.player(player),
        }
    }
    fn trigger(&self, when: &Trigger) -> Result<(), String> {
        match when {
            Trigger::ChapterOpens | Trigger::Phase(_) => Ok(()),
            Trigger::Fired { beat, after_s } => {
                if !self.earlier.contains(beat) {
                    return Err(format!("{beat:?} is no earlier beat"));
                }
                (after_s.is_finite() && *after_s >= 0.0)
                    .then_some(())
                    .ok_or("a beat's delay is a time from it".into())
            }
            Trigger::Bought { player, .. } | Trigger::Pending { player, .. } => self.player(player),
            Trigger::RadarContact { .. } | Trigger::MatchResult(_) => Ok(()),
            Trigger::Units { who, are } => {
                self.selector(who)?;
                match are {
                    UnitFact::In(zone) => self.zone(zone),
                    UnitFact::Ordered(OrderFact::Attack(target)) => self.selector(target),
                    UnitFact::Ordered(OrderFact::AttackGround(zone)) => self.zone(zone),
                    UnitFact::Engagement(_)
                    | UnitFact::Inside(_)
                    | UnitFact::Suppression(_)
                    | UnitFact::AttackMoving
                    | UnitFact::PoppedSmoke
                    | UnitFact::Reversing
                    | UnitFact::Deployed
                    | UnitFact::Serviced
                    | UnitFact::Airborne
                    | UnitFact::Arrived => Ok(()),
                }
            }
            Trigger::Identifies { who, .. } | Trigger::Destroyed(who) => self.selector(who),
            Trigger::ContactIn { zone, .. } => self.zone(zone),
            Trigger::Objective { id, .. } => self
                .objectives
                .contains(id)
                .then_some(())
                .ok_or(format!("no objective {id:?}")),
            Trigger::All(all) | Trigger::Any(all) => all.iter().try_for_each(|t| self.trigger(t)),
        }
    }
    fn action(&self, action: &Action) -> Result<(), String> {
        match action {
            Action::Order { units, order } => {
                self.selector(units)?;
                match order {
                    DirectedOrder::MoveTo(zone)
                    | DirectedOrder::AttackGround(zone)
                    | DirectedOrder::Occupy(zone)
                    | DirectedOrder::AttackMoveTo(zone)
                    | DirectedOrder::ReverseTo(zone)
                    | DirectedOrder::AttackContactIn(zone) => self.zone(zone),
                    DirectedOrder::Attack(target) => self.selector(target),
                    DirectedOrder::Engage(_)
                    | DirectedOrder::Unload
                    | DirectedOrder::Withdraw
                    | DirectedOrder::PopSmoke
                    | DirectedOrder::Deploy => Ok(()),
                }
            }
            Action::Grant { player, .. }
            | Action::Offer { player, .. }
            | Action::Ready { player }
            | Action::OpenShop { player } => self.player(player),
            Action::Buy { player, at, .. } => {
                self.player(player)?;
                self.zone(at)
            }
        }
    }
}
