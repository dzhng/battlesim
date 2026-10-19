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

    /// Whether `player` is controlled by AI right now. The controller map is
    /// installed for every skirmish player at tick zero and updated by
    /// `apply_controller`; a slot absent from it remains client-controlled.
    pub(super) fn ai_controlled(&self, player: Option<&str>) -> bool {
        player
            .and_then(|player| self.controllers.get(player))
            .is_some_and(|controller| *controller != ControllerKind::Human)
    }

    pub(super) fn apply_controller(&mut self, transition: &ControllerTransition) {
        self.controllers
            .insert(transition.player.clone(), transition.controller.clone());
        let player_index = self
            .skirmish
            .as_ref()
            .and_then(|skirmish| skirmish.player_index(&transition.player));
        if let Some(index) = player_index {
            if let Some(player) = self
                .skirmish
                .as_mut()
                .and_then(|skirmish| skirmish.players.get_mut(index))
            {
                player.controller = transition.controller.clone();
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
