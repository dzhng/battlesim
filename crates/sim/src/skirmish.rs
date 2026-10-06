//! Credits and reinforcement lifecycle; battle supplies physical admission.
use contract::catalog::TypeIndex;
use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::skirmish::{PendingPurchase, Phase, PurchaseId, SkirmishSetup, SkirmishView};
use std::collections::BTreeMap;
use std::collections::VecDeque;

pub struct Reservation {
    pub data: PendingPurchase,
    pub price: u32,
}

/// Microcredits retain deterministic fractional rewards; passive remainder is exact.
pub const CREDIT_SCALE: u64 = 1_000_000;

pub struct EnteredUnit {
    pub price: u32,
    pub entered_tick: Tick,
}

pub struct Skirmish {
    pub setup: SkirmishSetup,
    pub objectives: crate::objectives::Objectives,
    pub phase: Phase,
    pub ready: [bool; 2],
    pub active_tick: Option<Tick>,
    pub wallets: [u64; 2],
    income_remainder: u64,
    pub reservations: [VecDeque<Reservation>; 2],
    next_purchase: [u32; 2],
    pub next_dispatch: [Tick; 2],
    pub entered: BTreeMap<UnitId, EnteredUnit>,
}

impl Skirmish {
    pub fn new(setup: SkirmishSetup) -> Self {
        let budget = u64::from(setup.rules.credits_per_minute)
            * u64::from(setup.rules.starting_minutes)
            * CREDIT_SCALE;
        let objectives = crate::objectives::Objectives::new(setup.sites.objectives.len());
        Self {
            objectives,
            setup,
            phase: Phase::Preparation,
            ready: [false; 2],
            active_tick: None,
            wallets: [budget; 2],
            income_remainder: 0,
            reservations: Default::default(),
            next_purchase: [0; 2],
            next_dispatch: [0; 2],
            entered: BTreeMap::new(),
        }
    }

    pub fn advance(&mut self, tick: Tick, hz: u32) {
        if self.phase == Phase::Preparation {
            if self.ready.iter().all(|r| *r)
                || tick >= u64::from(self.setup.rules.preparation_s) * u64::from(hz)
            {
                self.phase = Phase::Active;
                self.active_tick = Some(tick);
            }
        } else if self.phase == Phase::Active {
            let denominator = 60 * u64::from(hz);
            let numerator = self.income_remainder
                + u64::from(self.setup.rules.credits_per_minute) * CREDIT_SCALE;
            let income = numerator / denominator;
            self.income_remainder = numerator % denominator;
            for wallet in &mut self.wallets {
                *wallet += income;
            }
        }
    }

    pub fn view(&self, side: Side, tick: Tick, hz: u32, living: u32) -> SkirmishView {
        SkirmishView {
            phase: self.phase,
            ready: self.ready,
            preparation_remaining_s: if self.phase == Phase::Preparation {
                (u64::from(self.setup.rules.preparation_s) * u64::from(hz)).saturating_sub(tick)
                    as f64
                    / f64::from(hz)
            } else {
                0.0
            },
            credits: self.wallets[side.index()] as f64 / CREDIT_SCALE as f64,
            occupied_slots: living + self.reservations[side.index()].len() as u32,
            max_units: self.setup.rules.max_units,
            objectives: self.objectives.views(&self.setup.sites.objectives, hz),
            scores: self.objectives.scores(hz),
            result: self.objectives.result,
            pending: self.reservations[side.index()]
                .iter()
                .map(|r| r.data.clone())
                .collect(),
        }
    }

    pub fn reserve(
        &mut self,
        side: Side,
        kind: TypeIndex,
        destination: [f64; 2],
        price: u32,
        tick: Tick,
        living: u32,
    ) -> Result<(), OrderError> {
        let index = side.index();
        if living + self.reservations[index].len() as u32 >= self.setup.rules.max_units {
            return Err(OrderError::UnitLimitReached);
        }
        let price_micros = u64::from(price) * CREDIT_SCALE;
        if self.wallets[index] < price_micros {
            return Err(OrderError::InsufficientCredits);
        }
        self.wallets[index] -= price_micros;
        let id = PurchaseId(self.next_purchase[index]);
        self.next_purchase[index] += 1;
        self.reservations[index].push_back(Reservation {
            price,
            data: PendingPurchase {
                id,
                kind,
                destination,
                confirmed_tick: tick,
                blocked: false,
            },
        });
        Ok(())
    }

    pub fn digest(&self, digest: &mut crate::digest::Digest) {
        self.objectives.digest(digest);
        digest
            .u64(self.phase as u64)
            .u64(self.ready[0] as u64)
            .u64(self.ready[1] as u64)
            .u64(self.active_tick.unwrap_or(u64::MAX))
            .u64(self.wallets[0])
            .u64(self.wallets[1])
            .u64(self.income_remainder);
        digest
            .u64(self.next_purchase[0] as u64)
            .u64(self.next_purchase[1] as u64);
        digest.u64(self.next_dispatch[0]).u64(self.next_dispatch[1]);
        digest.u64(self.entered.len() as u64);
        for (id, unit) in &self.entered {
            digest
                .u64(id.0 as u64)
                .u64(unit.price as u64)
                .u64(unit.entered_tick);
        }
        for side in &self.reservations {
            digest.u64(side.len() as u64);
            for r in side {
                digest
                    .u64(r.data.id.0 as u64)
                    .u64(r.data.kind.0 as u64)
                    .f64(r.data.destination[0])
                    .f64(r.data.destination[1])
                    .u64(r.data.confirmed_tick)
                    .u64(r.data.blocked as u64)
                    .u64(r.price as u64);
            }
        }
    }

    pub fn cancel(&mut self, side: Side, purchase: PurchaseId) {
        let index = side.index();
        if let Some(at) = self.reservations[index]
            .iter()
            .position(|r| r.data.id == purchase)
        {
            let reservation = self.reservations[index].remove(at).unwrap();
            self.wallets[index] += u64::from(reservation.price) * CREDIT_SCALE;
        }
    }
}
