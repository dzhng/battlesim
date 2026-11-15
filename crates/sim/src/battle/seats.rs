//! Recorded administrative transfers at the next tick boundary.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatEvent {
    HandOver {
        tick: Tick,
        player: String,
    },
    Reset {
        tick: Tick,
        player: String,
        opening: bool,
    },
}

impl SeatEvent {
    fn tick(&self) -> Tick {
        match self {
            Self::HandOver { tick, .. } | Self::Reset { tick, .. } => *tick,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatError {
    ReplayInProgress,
    MatchFinished,
    NotSkirmish,
    MissingPlayer,
    NoReceiver,
    Occupied,
}

impl std::fmt::Display for SeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ReplayInProgress => "the replay supplies its own seat events",
            Self::MatchFinished => "the match has finished",
            Self::NotSkirmish => "the battle has no player wallets",
            Self::MissingPlayer => "the battle has no such player",
            Self::NoReceiver => "no other AI holds a seat on this side",
            Self::Occupied => "the seat still holds units or pending purchases",
        })
    }
}

impl std::error::Error for SeatError {}

impl Battle {
    fn seat_player(&self, player: &str) -> Result<usize, SeatError> {
        self.skirmish
            .as_ref()
            .ok_or(SeatError::NotSkirmish)?
            .player_index(player)
            .ok_or(SeatError::MissingPlayer)
    }

    fn live_seat_player(&self, player: &str) -> Result<usize, SeatError> {
        if self.replaying.is_some() {
            return Err(SeatError::ReplayInProgress);
        }
        if self.finished() {
            return Err(SeatError::MatchFinished);
        }
        self.seat_player(player)
    }

    fn seat_receiver(&self, source: usize) -> Result<usize, SeatError> {
        let skirmish = self.skirmish.as_ref().ok_or(SeatError::NotSkirmish)?;
        skirmish
            .side_players(skirmish.players[source].side)
            .filter(|(index, p)| *index != source && p.controller != ControllerKind::Human)
            .min_by_key(|(index, p)| {
                let living = self
                    .units
                    .iter()
                    .filter(|u| u.alive() && u.owner.as_deref() == Some(p.id.as_str()))
                    .count();
                (living, *index)
            })
            .map(|(index, _)| index)
            .ok_or(SeatError::NoReceiver)
    }

    /// Record a transfer for the next tick. Like an order, acceptance checks
    /// today's state; application checks it again at the boundary. Later
    /// controller changes can leave an accepted transfer with no receiver;
    /// that event then has no effect, live or replayed.
    pub fn hand_over(&mut self, player: &str) -> Result<(), SeatError> {
        let source = self.live_seat_player(player)?;
        self.seat_receiver(source)?;
        let event = SeatEvent::HandOver {
            tick: self.tick + 1,
            player: player.into(),
        };
        self.pending_seat_events.push_back(event.clone());
        self.seat_events.push(event);
        Ok(())
    }

    /// Record a fresh wallet for an empty seat at the next boundary. The
    /// caller decides whether opening credits are due; the simulation tracks
    /// no human identity. Units and pending purchases must already be gone.
    /// Player sequence, purchase numbering, controller, readiness, AI memory
    /// and historical totals continue unchanged.
    pub fn reset_seat(&mut self, player: &str, opening: bool) -> Result<(), SeatError> {
        let index = self.live_seat_player(player)?;
        self.empty_seat(index)?;
        let event = SeatEvent::Reset {
            tick: self.tick + 1,
            player: player.into(),
            opening,
        };
        self.pending_seat_events.push_back(event.clone());
        self.seat_events.push(event);
        Ok(())
    }

    fn empty_seat(&self, index: usize) -> Result<(), SeatError> {
        let seat = &self.skirmish.as_ref().unwrap().players[index];
        if !seat.reservations.is_empty()
            || self
                .units
                .iter()
                .any(|u| u.alive() && u.owner.as_deref() == Some(seat.id.as_str()))
        {
            return Err(SeatError::Occupied);
        }
        Ok(())
    }

    fn clear_seat(&mut self, player: &str, opening: bool) -> Result<(), SeatError> {
        let index = self.seat_player(player)?;
        self.empty_seat(index)?;
        let skirmish = self.skirmish.as_mut().unwrap();
        skirmish.players[index].wallet = if opening {
            crate::skirmish::opening_wallet(&skirmish.setup.rules)
        } else {
            0
        };
        Ok(())
    }

    /// Boundary order is fixed: controllers at the preceding boundary, then
    /// order admission (including AI orders, purchases and cancellations),
    /// execution of admitted orders and this tick's income, then seat events
    /// in recording order, then arrivals and the physical tick. Already
    /// admitted orders remain orders of the units; only future orders require
    /// their new owner's authority. Events never consume an order sequence.
    /// Invalid replay events and events invalidated since acceptance are
    /// deterministic no-ops.
    pub(super) fn apply_seat_events(&mut self) {
        while self
            .pending_seat_events
            .front()
            .is_some_and(|e| e.tick() <= self.tick)
        {
            let event = self.pending_seat_events.pop_front().unwrap();
            let _ = match event {
                SeatEvent::HandOver { player, .. } => self.transfer_seat(&player),
                SeatEvent::Reset {
                    player, opening, ..
                } => self.clear_seat(&player, opening),
            };
        }
    }

    fn transfer_seat(&mut self, player: &str) -> Result<(), SeatError> {
        let source = self.seat_player(player)?;
        let receiver = self.seat_receiver(source)?;
        let skirmish = self.skirmish.as_mut().unwrap();
        let receiver_id = skirmish.players[receiver].id.clone();
        let controller = skirmish.players[receiver].controller.clone();
        for unit in &mut self.units {
            if unit.alive() && unit.owner.as_deref() == Some(player) {
                unit.owner = Some(receiver_id.clone());
                controllers::put_under(unit, &controller);
                if let Some(receipt) = skirmish.entered.get_mut(&unit.id) {
                    receipt.player = receiver;
                }
            }
        }
        let wallet = std::mem::take(&mut skirmish.players[source].wallet);
        skirmish.players[receiver].wallet += wallet;
        let reservations = std::mem::take(&mut skirmish.players[source].reservations);
        let target = &mut skirmish.players[receiver];
        for mut reservation in reservations {
            reservation.data.id = contract::skirmish::PurchaseId(target.next_purchase);
            target.next_purchase += 1;
            // `player` stays the original buyer for the eventual Spawn report.
            // The containing seat owns cancellation, arrival and future refunds.
            target.reservations.push_back(reservation);
        }
        Ok(())
    }
}
