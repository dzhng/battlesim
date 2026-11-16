//! Live order ingress: the battle stamps who sent an order and its place in
//! that sender's sequence.
use contract::command::{CommandAck, CommandEnvelope, Order};
use contract::ids::Side;

use super::Battle;

impl Battle {
    /// Take a live order from `player` (or, where the battle has no players,
    /// from `side`) as the next in that sender's sequence. A replay goes
    /// through [`Battle::accept`] instead: its envelopes are already stamped.
    pub fn accept_order(
        &mut self,
        side: Side,
        player: Option<String>,
        order: Order,
        queued: bool,
    ) -> CommandAck {
        let seq = self.expected_seq(side, self.player_index(player.as_deref()));
        self.accept(CommandEnvelope {
            side,
            player,
            seq,
            order,
            queued,
        })
    }

    /// Where `player` sits among the battle's players, if it has any.
    pub(super) fn player_index(&self, player: Option<&str>) -> Option<usize> {
        let skirmish = self.skirmish.as_ref()?;
        skirmish.player_index(player?)
    }

    /// The sequence number the next order must carry: its player's, or its
    /// side's when no player sent it.
    pub(super) fn expected_seq(&self, side: Side, player: Option<usize>) -> u64 {
        player.map_or(self.next_seq[side.index()], |i| self.player_next_seq[i])
    }
}
