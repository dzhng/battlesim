//! Credits and reinforcement lifecycle; battle supplies physical admission.
use contract::catalog::TypeIndex;
use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::players::{ControllerKind, Player};
use contract::skirmish::{
    MatchEvent, MatchReport, MatchTotals, MatchUnit, PendingPurchase, Phase, PurchaseId,
    SkirmishSetup, SkirmishView,
};
use std::collections::{BTreeMap, VecDeque};

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
    pub next_dispatch: Tick,
    pub totals: MatchTotals,
}

pub struct Skirmish {
    pub setup: SkirmishSetup,
    pub objectives: crate::objectives::Objectives,
    pub settlement: crate::settlement::Settlement,
    pub phase: Phase,
    pub players: Vec<PlayerState>,
    pub entered: BTreeMap<UnitId, EnteredUnit>,
    pub report: MatchReport,
    income_remainder: u64,
}

impl Skirmish {
    pub fn new(setup: SkirmishSetup, source: &[Player]) -> Self {
        let budget = u64::from(setup.rules.credits_per_minute)
            * u64::from(setup.rules.starting_minutes)
            * CREDIT_SCALE;
        let players = source
            .iter()
            .map(|p| PlayerState {
                id: p.id.clone(),
                side: p.side,
                faction: p.faction.expect("validated combatant faction"),
                controller: p.controller.clone(),
                wallet: budget,
                ready: false,
                reservations: VecDeque::new(),
                next_purchase: 0,
                next_dispatch: 0,
                totals: MatchTotals::default(),
            })
            .collect::<Vec<_>>();
        let objectives =
            crate::objectives::Objectives::new(setup.sites.objectives.len(), &setup.rules);
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
        let first = players.first();
        let legacy_credits = first.map_or(0.0, |p| p.credits);
        let legacy_unit_count = first.map_or(0, |p| p.unit_count);
        let legacy_pending = first.map_or_else(Vec::new, |p| p.pending.clone());
        SkirmishView {
            phase: self.phase,
            players,
            ready: [false; 2],
            preparation_remaining_s: if self.phase == Phase::Preparation {
                (u64::from(self.setup.rules.preparation_s) * u64::from(hz)).saturating_sub(tick)
                    as f64
                    / f64::from(hz)
            } else {
                0.0
            },
            credits: legacy_credits,
            unit_count: legacy_unit_count,
            max_units: self.setup.rules.max_units,
            pending: legacy_pending,
            objectives: self.objectives.views(&self.setup.sites.objectives, hz),
            scores: self.objectives.scores(hz),
            result: self.objectives.result,
            report: (self.phase == Phase::Finished).then(|| self.report.clone()),
        }
    }
    pub fn reserve(
        &mut self,
        player: usize,
        kind: TypeIndex,
        destination: [f64; 2],
        price: u32,
        tick: Tick,
        living: u32,
    ) -> Result<PurchaseId, OrderError> {
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
                .u64(p.next_purchase as u64)
                .u64(p.next_dispatch)
                .u64(p.reservations.len() as u64);
            for r in &p.reservations {
                d.u64(r.data.id.0 as u64)
                    .u64(r.data.kind.0 as u64)
                    .f64(r.data.destination[0])
                    .f64(r.data.destination[1])
                    .u64(r.data.confirmed_tick)
                    .u64(r.data.blocked as u64)
                    .u64(r.price as u64)
                    .u64(r.data.arrival_tick.unwrap_or(u64::MAX));
            }
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
