//! Who controls each player, and what its units may do under that
//! controller.

use super::*;

/// The engagement a unit fights with: neutral's units and a passive AI's
/// only return fire, whatever they author (neutral is a passive world actor:
/// it answers an attacker through the normal return-fire path but never
/// acquires a target on its own). Anyone else's keep `authored`, firing at
/// will by default.
pub(super) fn engagement(
    side: Side,
    controller: Option<&ControllerKind>,
    authored: Option<Engagement>,
) -> Engagement {
    if side == Side::Neutral || controller == Some(&ControllerKind::PassiveAi) {
        Engagement::ReturnFireOnly
    } else {
        authored.unwrap_or(Engagement::FireAtWill)
    }
}

impl Battle {
    /// Install the per-slot controller state chosen at play admission. AI
    /// orders still enter `accept`, while passive slots only retain return fire.
    pub fn set_controllers(&mut self, controllers: Vec<(String, ControllerKind)>) {
        for (slot, controller) in controllers {
            let transition = ControllerTransition {
                tick: self.tick,
                player: slot,
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
        // A player's units take its new controller's engagement; leaving a
        // passive AI, they fire at will again unless they hold another.
        for unit in &mut self.units {
            if unit.side == Side::Neutral || unit.owner.as_deref() == Some(&transition.player) {
                let held = Some(unit.engagement).filter(|e| *e != Engagement::ReturnFireOnly);
                unit.engagement = engagement(unit.side, Some(&transition.controller), held);
            }
        }
    }
}
