//! Credits and reinforcement lifecycle; battle supplies physical admission.
use contract::catalog::TypeIndex;
use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::players::{ControllerKind, Player, Seating};
use contract::skirmish::{
    MatchEvent, MatchReport, MatchTotals, MatchUnit, PendingPurchase, Phase, PurchaseId,
    SkirmishSetup, SkirmishView,
};
use std::collections::{BTreeMap, VecDeque};

/// Construct the one skirmish setup shape used by generated battles,
/// tournament inputs, and authored economy scenarios.
pub fn setup_for<P: AsRef<[Player]>>(
    players: P,
    sites: contract::encounter::SkirmishSites,
) -> SkirmishSetup {
    let players = players.as_ref();
    debug_assert!(contract::players::validate_players(players, Seating::Skirmish).is_ok());
    debug_assert!(contract::players::validate_entry_counts(players, &sites.entries).is_ok());
    SkirmishSetup {
        rules: starter_rules(),
        sites,
        openings: BTreeMap::new(),
    }
}

/// Synthetic sites for fixtures that have a map but no authored encounter.
/// Entries follow the generated map order: centre, then left-to-right flanks,
/// for each side.
pub fn synthetic_sites(
    size: [f64; 2],
    entries_per_side: usize,
) -> contract::encounter::SkirmishSites {
    use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
    use contract::ids::Side;
    let [width, depth] = size;
    let fractions: &[f64] = match entries_per_side {
        1 => &[0.5],
        // Generation lays the second road as one flank arm (0.22-0.38 or
        // 0.62-0.78 of the edge).
        2 => &[0.5, 0.3],
        _ => &[0.5, 0.2, 0.8],
    };
    let mut entries = Vec::with_capacity(fractions.len() * 2);
    for side in [Side::Blue, Side::Red] {
        for &fraction in fractions {
            let (y, yaw) = if side == Side::Blue {
                (12.0, 1.57)
            } else {
                (depth - 12.0, -1.57)
            };
            entries.push(EntrySite {
                side,
                center: [width * fraction, y],
                yaw,
            });
        }
    }
    SkirmishSites {
        entries,
        objectives: vec![
            ObjectiveSite {
                id: "center".into(),
                center: [width * 0.5, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Junction,
                counterpart: None,
            },
            ObjectiveSite {
                id: "west".into(),
                center: [width * 0.25, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: Some("east".into()),
            },
            ObjectiveSite {
                id: "east".into(),
                center: [width * 0.75, depth * 0.5],
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: Some("west".into()),
            },
        ],
        candidates: vec![],
    }
}

#[derive(Clone, Debug)]
pub struct Reservation {
    pub data: PendingPurchase,
    pub price: u32,
    pub player: usize,
}
pub const CREDIT_SCALE: u64 = 1_000_000;
pub struct EnteredUnit {
    pub price: u32,
    pub entered_tick: Tick,
    pub player: usize,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub id: String,
    pub side: Side,
    pub faction: contract::catalog::Faction,
    pub controller: ControllerKind,
    pub wallet: u64,
    pub ready: bool,
    pub reservations: VecDeque<Reservation>,
    pub next_purchase: u32,
    pub totals: MatchTotals,
}

pub struct Skirmish {
    pub setup: SkirmishSetup,
    pub objectives: crate::objectives::Objectives,
    pub settlement: crate::settlement::Settlement,
    pub phase: Phase,
    pub players: Vec<PlayerState>,
    /// Independent ground-arrival clocks and round-robin cursors for entries.
    pub next_dispatch: Vec<Tick>,
    pub dispatch_turn: Vec<usize>,
    pub entered: BTreeMap<UnitId, EnteredUnit>,
    pub report: MatchReport,
    income_remainder: u64,
}

impl PlayerState {
    /// `player`'s seat in a match, with `wallet` to spend.
    pub fn new(player: &Player, wallet: u64) -> Self {
        Self {
            id: player.id.clone(),
            side: player.side,
            faction: player.faction.expect("validated combatant faction"),
            controller: player.controller.clone(),
            wallet,
            ready: false,
            reservations: VecDeque::new(),
            next_purchase: 0,
            totals: MatchTotals::default(),
        }
    }
}

impl Skirmish {
    pub fn new(setup: SkirmishSetup, source: &[Player]) -> Self {
        let entry_count = setup.sites.entries.len();
        let budget = u64::from(setup.rules.credits_per_minute)
            * u64::from(setup.rules.starting_minutes)
            * CREDIT_SCALE;
        let players = source
            .iter()
            .map(|p| PlayerState::new(p, budget))
            .collect::<Vec<_>>();
        let objectives =
            crate::objectives::Objectives::new(setup.sites.objectives.len(), &setup.rules);
        let dispatch_turn = setup
            .sites
            .entries
            .iter()
            .map(|entry| {
                source
                    .iter()
                    .position(|player| player.side == entry.side)
                    .unwrap_or_default()
            })
            .collect();
        Self {
            setup,
            objectives,
            settlement: Default::default(),
            phase: Phase::Preparation,
            report: MatchReport {
                totals: vec![MatchTotals::default(); players.len()],
                events: Vec::new(),
            },
            players,
            next_dispatch: vec![0; entry_count],
            dispatch_turn,
            entered: BTreeMap::new(),
            income_remainder: 0,
        }
    }
    pub fn player_index(&self, id: &str) -> Option<usize> {
        self.players.iter().position(|p| p.id == id)
    }
    pub fn side_players(&self, side: Side) -> impl Iterator<Item = (usize, &PlayerState)> {
        self.players
            .iter()
            .enumerate()
            .filter(move |(_, p)| p.side == side)
    }

    pub fn next_entry_player(&self, entry: crate::entries::EntryIndex, current: usize) -> usize {
        let side = self.setup.sites.entries[entry.0].side;
        (1..=self.players.len())
            .map(|step| (current + step) % self.players.len())
            .find(|&index| self.players[index].side == side)
            .unwrap_or(current)
    }

    pub fn entry_turn_has_purchase(&self, entry: crate::entries::EntryIndex) -> bool {
        self.players
            .get(self.dispatch_turn[entry.0])
            .is_some_and(|player| {
                player.reservations.iter().any(|reservation| {
                    reservation.data.entry == entry.0 && reservation.data.arrival_tick.is_none()
                })
            })
    }
    pub fn advance(&mut self, tick: Tick, hz: u32) {
        if self.phase == Phase::Preparation {
            if self.players.iter().all(|p| p.ready)
                || tick >= u64::from(self.setup.rules.preparation_s) * u64::from(hz)
            {
                self.phase = Phase::Active;
            }
        } else if self.phase == Phase::Active {
            let den = 60 * u64::from(hz);
            let num = self.income_remainder
                + u64::from(self.setup.rules.credits_per_minute) * CREDIT_SCALE;
            let income = num / den;
            self.income_remainder = num % den;
            for p in &mut self.players {
                p.wallet += income;
            }
        }
    }
    pub fn view(&self, side: Side, tick: Tick, hz: u32, living: &[u32]) -> SkirmishView {
        let players: Vec<_> = self
            .side_players(side)
            .map(|(index, p)| contract::skirmish::PlayerView {
                index: contract::players::PlayerIndex(index as u16),
                id: p.id.clone(),
                controller: p.controller.clone(),
                ready: p.ready,
                credits: p.wallet as f64 / CREDIT_SCALE as f64,
                unit_count: living.get(index).copied().unwrap_or(0) + p.reservations.len() as u32,
                cap: self.setup.rules.max_units,
                pending: p.reservations.iter().map(|r| r.data.clone()).collect(),
            })
            .collect();
        SkirmishView {
            phase: self.phase,
            players,
            preparation_remaining_s: if self.phase == Phase::Preparation {
                (u64::from(self.setup.rules.preparation_s) * u64::from(hz)).saturating_sub(tick)
                    as f64
                    / f64::from(hz)
            } else {
                0.0
            },
            objectives: self.objectives.views(&self.setup.sites.objectives, hz),
            scores: self.objectives.scores(hz),
            result: self.objectives.result,
            report: (self.phase == Phase::Finished).then(|| self.report.clone()),
        }
    }
    pub fn reserve_at(
        &mut self,
        player: usize,
        kind: TypeIndex,
        destination: [f64; 2],
        price: u32,
        entry_tick: (crate::entries::EntryIndex, Tick),
        living: u32,
    ) -> Result<PurchaseId, OrderError> {
        let (entry, tick) = entry_tick;
        let p = self
            .players
            .get_mut(player)
            .ok_or(OrderError::MissingPlayer)?;
        if living + p.reservations.len() as u32 >= self.setup.rules.max_units {
            return Err(OrderError::UnitLimitReached);
        }
        let micros = u64::from(price) * CREDIT_SCALE;
        if p.wallet < micros {
            return Err(OrderError::InsufficientCredits);
        }
        p.wallet -= micros;
        p.totals.spent += price;
        self.report.totals[player].spent += price;
        let id = PurchaseId(p.next_purchase);
        p.next_purchase += 1;
        p.reservations.push_back(Reservation {
            price,
            player,
            data: PendingPurchase {
                id,
                kind,
                entry: entry.0,
                destination,
                confirmed_tick: tick,
                blocked: false,
                arrival_tick: None,
            },
        });
        Ok(id)
    }
    pub fn record_spawn(
        &mut self,
        player: usize,
        id: UnitId,
        kind: TypeIndex,
        cost: u32,
        tick: Tick,
    ) {
        let side = self.players[player].side;
        self.players[player].totals.spawned += 1;
        self.report.totals[player].spawned += 1;
        self.report.events.push(MatchEvent::Spawn {
            tick,
            unit: MatchUnit {
                side,
                id,
                kind,
                owner: Some(contract::players::PlayerIndex(player as u16)),
            },
            cost,
        });
    }
    pub fn record_refund(
        &mut self,
        player: usize,
        id: UnitId,
        kind: TypeIndex,
        credits: f64,
        tick: Tick,
    ) {
        let side = self.players[player].side;
        self.players[player].totals.refunded += credits;
        self.report.totals[player].refunded += credits;
        self.report.events.push(MatchEvent::Refund {
            tick,
            unit: MatchUnit {
                side,
                id,
                kind,
                owner: Some(contract::players::PlayerIndex(player as u16)),
            },
            credits,
        });
    }
    pub fn record_kill(
        &mut self,
        victim: MatchUnit,
        victim_cost: u32,
        killer: Option<MatchUnit>,
        killer_cost: Option<u32>,
        tick: Tick,
        killer_player: Option<usize>,
    ) {
        if let Some(k) = killer {
            if k.side != victim.side {
                if let Some(i) = killer_player {
                    self.players[i].totals.killed += 1;
                    self.report.totals[i].killed += 1;
                }
            }
            self.report.events.push(MatchEvent::Kill {
                tick,
                victim,
                victim_cost,
                killer: Some(k),
                killer_cost,
            });
        } else {
            self.report.events.push(MatchEvent::Kill {
                tick,
                victim,
                victim_cost,
                killer: None,
                killer_cost: None,
            });
        }
    }
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        self.objectives.digest(d);
        self.settlement.digest(d);
        d.u64(self.phase as u64).u64(self.income_remainder);
        for p in &self.players {
            d.u64(p.side.index() as u64)
                .u64(p.wallet)
                .u64(p.ready as u64)
                .u64(p.next_purchase as u64);
            d.u64(p.reservations.len() as u64);
            for r in &p.reservations {
                d.u64(r.data.id.0 as u64)
                    .u64(r.data.kind.0 as u64)
                    .f64(r.data.destination[0])
                    .f64(r.data.destination[1])
                    .u64(r.data.confirmed_tick)
                    .u64(r.data.blocked as u64)
                    .u64(r.price as u64)
                    .u64(r.data.arrival_tick.unwrap_or(u64::MAX))
                    .u64(r.data.entry as u64);
            }
        }
        for (clock, turn) in self.next_dispatch.iter().zip(&self.dispatch_turn) {
            d.u64(*clock).u64(*turn as u64);
        }
        d.u64(self.entered.len() as u64);
        for (id, u) in &self.entered {
            d.u64(id.0 as u64)
                .u64(u.price as u64)
                .u64(u.entered_tick)
                .u64(u.player as u64);
        }
        for t in &self.report.totals {
            d.u64(u64::from(t.spawned))
                .u64(u64::from(t.spent))
                .u64(u64::from(t.killed))
                .u64(t.refunded.to_bits());
        }
        d.u64(self.report.events.len() as u64);
    }
    pub fn cancel(&mut self, player: usize, purchase: PurchaseId) -> Result<(), OrderError> {
        let p = self
            .players
            .get_mut(player)
            .ok_or(OrderError::MissingPlayer)?;
        if let Some(i) = p.reservations.iter().position(|r| r.data.id == purchase) {
            let r = p.reservations.remove(i).expect("reservation index");
            p.wallet += u64::from(r.price) * CREDIT_SCALE;
        }
        Ok(())
    }
}
pub fn starter_rules() -> contract::skirmish::MatchRules {
    serde_json::from_str(include_str!("../../../fixtures/skirmish.json"))
        .expect("authored match rules")
}
