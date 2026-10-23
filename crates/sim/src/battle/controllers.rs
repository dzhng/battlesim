//! Who controls each player, and what its units may do under that
//! controller.

use super::*;

/// The engagement a unit chooses for itself, before any controller's say:
/// neutral's units only return fire, whatever they author (neutral is a
/// passive world actor: it answers an attacker through the normal
/// return-fire path but never acquires a target on its own). Anyone else's
/// keep `authored`, firing at will by default.
pub(super) fn own_engagement(side: Side, authored: Option<Engagement>) -> Engagement {
    if side == Side::Neutral {
        Engagement::ReturnFireOnly
    } else {
        authored.unwrap_or(Engagement::FireAtWill)
    }
}

/// Put `unit` under `controller`. A passive AI holds it at return fire and
/// keeps its own engagement to give back; any other controller leaves it
/// with its own.
pub(super) fn put_under(unit: &mut Unit, controller: &ControllerKind) {
    let own = unit.held_engagement.take().unwrap_or(unit.engagement);
    if *controller == ControllerKind::PassiveAi {
        unit.held_engagement = Some(own);
        unit.engagement = Engagement::ReturnFireOnly;
    } else {
        unit.engagement = own;
    }
}

/// An attack or attack-move order chooses fire at will (W14).
pub(super) fn fire_at_will(unit: &mut Unit) {
    order_engagement(unit, Engagement::FireAtWill);
}

/// An order chooses the unit's own engagement; a passive AI holding it at
/// return fire keeps holding it there.
pub(super) fn order_engagement(unit: &mut Unit, policy: Engagement) {
    match unit.held_engagement.as_mut() {
        Some(held) => *held = policy,
        None => unit.engagement = policy,
    }
}

impl Battle {
    /// Install each player's controller chosen at play admission. The
    /// battle's AI issues its orders itself (`issue`), past the refusal a
    /// client order to an AI's player meets in `accept`.
    pub fn set_controllers(&mut self, controllers: Vec<(String, ControllerKind)>) {
        for (player, controller) in controllers {
            let transition = ControllerTransition {
                tick: self.tick,
                player,
                controller,
            };
            self.apply_controller(&transition);
            self.controller_transitions.push(transition);
        }
    }

    /// Whether `player` is controlled by AI right now. Every skirmish
    /// player's seat holds a controller from tick zero, updated by
    /// `apply_controller`; a player with none remains client-controlled.
    pub(super) fn ai_controlled(&self, player: Option<&str>) -> bool {
        player
            .and_then(|player| self.controller(player))
            .is_some_and(|controller| *controller != ControllerKind::Human)
    }

    /// Who controls `player`: its skirmish seat's controller, or for a
    /// player with no seat, the one installed for it.
    pub(super) fn controller(&self, player: &str) -> Option<&ControllerKind> {
        match self.skirmish.as_ref().and_then(|s| s.player_index(player)) {
            Some(index) => Some(&self.skirmish.as_ref()?.players[index].controller),
            None => self.controllers.get(player),
        }
    }

    /// Every player's controller, by id, as the replay records it.
    pub(super) fn controller_list(&self) -> Vec<(String, ControllerKind)> {
        let seats = self.skirmish.iter().flat_map(|s| &s.players);
        let seated = seats.map(|p| (p.id.clone(), p.controller.clone()));
        let unseated = self
            .controllers
            .iter()
            .map(|(id, c)| (id.clone(), c.clone()));
        let all: BTreeMap<_, _> = unseated.chain(seated).collect();
        all.into_iter().collect()
    }

    pub(super) fn apply_controller(&mut self, transition: &ControllerTransition) {
        let seat = self.skirmish.as_mut().and_then(|skirmish| {
            let index = skirmish.player_index(&transition.player)?;
            skirmish.players.get_mut(index)
        });
        match seat {
            Some(seat) => seat.controller = transition.controller.clone(),
            None => {
                self.controllers
                    .insert(transition.player.clone(), transition.controller.clone());
            }
        }
        for unit in &mut self.units {
            if unit.owner.as_deref() == Some(&transition.player) {
                put_under(unit, &transition.controller);
            }
        }
    }
}
