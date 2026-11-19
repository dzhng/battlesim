//! An authored scenario: a fixed battle on a `scenario` saved map, fought
//! under today's rules and roster.
//!
//! A scenario document is a skirmish laid out in advance: its players, the
//! match rules it changes, its roads in and objectives, the forces it places,
//! the credits each player opens with and the named places its story talks
//! about. [`ScenarioDocument::on`] resolves it, against the rules and roster
//! of the build that reads it, into the one runtime setup
//! ([`ScenarioDefinition`]). It has no way to override a rule, by type: a
//! retune reaches every scenario on its next load. (An encounter may pin
//! rules for a film or a lab; a scenario-builder bundle freezes them to
//! travel. A scenario does neither.)
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, Faction};
use crate::encounter::SkirmishSites;
use crate::identity::Seed;
use crate::map::MapDefinition;
use crate::players::Player;
use crate::scenario::{Rules, ScenarioDefinition, UnitSetup};
use crate::skirmish::{MatchRules, SkirmishSetup};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDocument {
    /// The battle's seed: a scenario is one battle, the same each time.
    pub battle_seed: Seed,
    pub players: Vec<Player>,
    /// What this scenario changes in the match rules it is played under,
    /// key by key ([`MatchRules::patched`]).
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub match_rules: serde_json::Map<String, serde_json::Value>,
    pub sites: SkirmishSites,
    /// The forces placed on the map from the start, each by its name. Every
    /// force belongs to one of the players. A name is what the scenario's
    /// story calls a force; the battle numbers forces in name order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub forces: BTreeMap<String, UnitSetup>,
    /// What a player opens with, in place of the match rules' opening
    /// income.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub opening_credits: BTreeMap<String, CreditGrant>,
    /// The named places the scenario's story talks about.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub zones: BTreeMap<String, Zone>,
}

/// Credits a scenario hands a player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CreditGrant {
    /// Enough for these purchases at today's prices, so a price change
    /// never leaves the player short.
    Covering(Vec<PurchaseSpec>),
    /// This many credits.
    Exactly(u64),
}

/// A purchase as a card names it: a unit type, and the carrier a squad is
/// bought in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseSpec {
    pub variant: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
}

/// A named place on the map: a circle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub center: [f64; 2],
    pub radius_m: f64,
}

/// The credits that buy every one of `purchases` at `catalog`'s prices.
pub fn credits_covering(catalog: &Catalog, purchases: &[PurchaseSpec]) -> Result<u64, String> {
    let price = |kind: &str| {
        catalog
            .index(kind)
            .map(|index| u64::from(catalog.get(index).cost))
            .ok_or_else(|| format!("unknown unit type {kind:?}"))
    };
    purchases.iter().try_fold(0u64, |sum, purchase| {
        let carrier = purchase.carrier.as_deref().map_or(Ok(0), price)?;
        Ok(sum + price(&purchase.variant)? + carrier)
    })
}

impl ScenarioDocument {
    /// The battle this scenario makes on `map` under `rules` and the match
    /// rules `base`, or the refusal naming the document's field at fault.
    /// Every reader that plays a scenario makes its battle here.
    pub fn on(
        self,
        map: MapDefinition,
        rules: Rules,
        base: &MatchRules,
    ) -> Result<ScenarioDefinition, String> {
        let match_rules = base
            .patched(&self.match_rules)
            .map_err(|e| format!("match_rules.{e}"))?;
        let seat = |id: &str| self.players.iter().find(|player| player.id == id);
        for (name, force) in &self.forces {
            let Some(owner) = force.owner.as_deref().and_then(seat) else {
                return Err(format!("forces.{name}.owner: not a player of the scenario"));
            };
            if force.side != owner.side {
                return Err(format!("forces.{name}.side: not its owner's side"));
            }
            sold_to(&rules.catalog, &force.kind, owner.faction)
                .map_err(|e| format!("forces.{name}.kind: {e}"))?;
        }
        let mut opening_credits = BTreeMap::new();
        for (player, grant) in &self.opening_credits {
            let Some(seat) = seat(player) else {
                return Err(format!(
                    "opening_credits.{player}: not a player of the scenario"
                ));
            };
            let credits = match grant {
                CreditGrant::Exactly(credits) => *credits,
                CreditGrant::Covering(purchases) => {
                    for (i, purchase) in purchases.iter().enumerate() {
                        for kind in std::iter::once(&purchase.variant).chain(&purchase.carrier) {
                            sold_to(&rules.catalog, kind, seat.faction).map_err(|e| {
                                format!("opening_credits.{player}.covering[{i}]: {e}")
                            })?;
                        }
                    }
                    credits_covering(&rules.catalog, purchases)
                        .map_err(|e| format!("opening_credits.{player}: {e}"))?
                }
            };
            opening_credits.insert(player.clone(), credits);
        }
        for (name, zone) in &self.zones {
            let [x, y] = zone.center;
            let inside = x >= 0.0 && y >= 0.0 && x <= map.size[0] && y <= map.size[1];
            if !inside || !(zone.radius_m.is_finite() && zone.radius_m > 0.0) {
                return Err(format!("zones.{name}: a circle on the map"));
            }
        }
        let skirmish: SkirmishSetup = serde_json::from_value(serde_json::json!({
            "rules": match_rules,
            "sites": self.sites,
            "opening_credits": opening_credits,
        }))
        .map_err(|e| format!("sites: {e}"))?;
        let setup = ScenarioDefinition {
            players: self.players,
            skirmish: Some(skirmish),
            map,
            rules,
            units: self.forces.into_values().collect(),
            events: Vec::new(),
            scripts: Vec::new(),
            authored_scripts: Vec::new(),
            victory: None,
            opponent: None,
            encounter: None,
        };
        setup.validate_player_contract()?;
        Ok(setup)
    }
}

/// Refuses `kind` unless it is a roster card that `faction` may buy.
fn sold_to(catalog: &Catalog, kind: &str, faction: Option<Faction>) -> Result<(), String> {
    let card = catalog
        .card(kind)
        .ok_or_else(|| format!("{kind:?} is not a card of the game's roster"))?;
    let sold = faction.is_some_and(|faction| card.roster.factions.contains(&faction));
    if card.disabled_reason.is_some() || !sold {
        return Err(format!("{kind:?} is not sold to its player's faction"));
    }
    Ok(())
}
