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
use crate::scenario_document::{CreditGrant, Zone};
use crate::skirmish::{Phase, PurchaseSpec};

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
        Ok(())
    }
}

/// The names a director may use, and the beats before the one checked.
struct Names<'a> {
    zones: &'a BTreeMap<String, Zone>,
    forces: &'a BTreeMap<String, UnitId>,
    players: &'a [crate::players::Player],
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
            Trigger::Bought { player, .. } => self.player(player),
            Trigger::Units { who, are } => {
                self.selector(who)?;
                match are {
                    UnitFact::In(zone) => self.zone(zone),
                    UnitFact::Engagement(_) => Ok(()),
                }
            }
            Trigger::Identifies { who, .. } => self.selector(who),
            Trigger::ContactIn { zone, .. } => self.zone(zone),
            Trigger::All(all) | Trigger::Any(all) => all.iter().try_for_each(|t| self.trigger(t)),
        }
    }
    fn action(&self, action: &Action) -> Result<(), String> {
        match action {
            Action::Order { units, order } => {
                self.selector(units)?;
                match order {
                    DirectedOrder::MoveTo(zone) | DirectedOrder::AttackGround(zone) => {
                        self.zone(zone)
                    }
                    DirectedOrder::Engage(_) => Ok(()),
                }
            }
            Action::Grant { player, .. }
            | Action::Offer { player, .. }
            | Action::Ready { player } => self.player(player),
            Action::Buy { player, at, .. } => {
                self.player(player)?;
                self.zone(at)
            }
        }
    }
}
