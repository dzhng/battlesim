//! The units an order refuses one by one as it is given, keeping the rest:
//! a squad too big for its carrier, a carrier with no room to unload.

use super::*;

impl Battle {
    /// The refusals of `command` at the command, beside a building order's
    /// own (`building_refused`), with the refused units taken out of its
    /// order; an error when it refuses the whole order.
    pub(super) fn refuse_at_command(
        &self,
        command: &mut CommandEnvelope,
        building_refused: Vec<contract::command::Refusal>,
    ) -> Result<Vec<contract::command::Refusal>, OrderError> {
        let mut refused =
            hold::refuse_oversized_riders(&self.units, &mut command.order, &self.rules)
                .unwrap_or(building_refused);
        if let Order::Unload {
            units, goal: None, ..
        } = &command.order
        {
            for &unit in units {
                if crate::carriage::unload_spots(&self.world, &self.units, unit, &self.rules)
                    .is_none()
                {
                    refused.push(contract::command::Refusal {
                        unit,
                        reason: OrderError::NoRoomToUnload,
                    });
                }
            }
        }
        Ok(refused)
    }
}
