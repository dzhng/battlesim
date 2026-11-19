//! Credits and reinforcement lifecycle; battle supplies physical admission.
use contract::catalog::TypeIndex;
use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::players::{ControllerKind, Player, Seating};
use contract::skirmish::{
    MatchEvent, MatchReport, MatchRuleSet, MatchRules, MatchTotals, MatchUnit, PendingPurchase,
    Phase, PurchaseId, SkirmishSetup, SkirmishView,
};
use std::collections::{BTreeMap, VecDeque};

/// Construct the one skirmish setup shape used by generated battles,
/// tournament inputs, and authored economy scenarios.
pub fn setup_for<P: AsRef<[Player]>>(
    players: P,
    sites: contract::encounter::SkirmishSites,
    rules: MatchRules,
) -> SkirmishSetup {
    let players = players.as_ref();
    debug_assert!(contract::players::validate_players(players, Seating::Skirmish).is_ok());
    debug_assert!(contract::players::validate_entry_counts(players, &sites.entries).is_ok());
    SkirmishSetup {
        rules,
        sites,
        openings: BTreeMap::new(),
        opening_credits: BTreeMap::new(),
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
    /// What `data.kind` cost; its unit keeps this price once it enters.
    pub price: u32,
    /// What `data.carrier` cost, when the purchase has one.
    pub carrier_price: Option<u32>,
    /// Original buyer, retained for historical spending and spawn reports.
    pub player: usize,
}

impl Reservation {
    /// Everything the purchase cost, which cancelling it pays back.
    pub fn total(&self) -> u32 {
        self.price + self.carrier_price.unwrap_or(0)
    }
}

/// A purchase being confirmed: the unit bought, and the carrier it arrives
/// aboard, each with its price.
pub struct Purchase {
    pub kind: TypeIndex,
    pub price: u32,
    pub carrier: Option<(TypeIndex, u32)>,
}

impl Purchase {
    /// How many units it fields: the unit, and its carrier when it has one.
    pub fn units(&self) -> u32 {
        1 + u32::from(self.carrier.is_some())
    }
    /// Everything it costs.
    pub fn total(&self) -> u32 {
        self.price + self.carrier.map_or(0, |(_, price)| price)
    }
}
pub const CREDIT_SCALE: u64 = 1_000_000;

/// Opening money in wallet microcredits, for both admission and seat reset.
pub(crate) fn opening_wallet(rules: &MatchRules) -> u64 {
    u64::from(rules.credits_per_minute) * u64::from(rules.starting_minutes) * CREDIT_SCALE
}

/// Passive income in microcredits per minute for a player whose army is
/// worth `army` credits.
pub(crate) fn income_per_minute(rules: &MatchRules, army: u64) -> u64 {
    let full = u64::from(rules.credits_per_minute) * CREDIT_SCALE;
    let Some(upkeep) = rules.upkeep else {
        return full;
    };
    let (from, to) = (u64::from(upkeep.taper_from), u64::from(upkeep.taper_to));
    let into = army.clamp(from, to) - from;
    let cut = u128::from(full) * u128::from(100 - upkeep.floor_percent) * u128::from(into)
        / (100 * u128::from(to - from));
    full - cut as u64
}

/// A player's living units: how many, and what they cost in credits.
#[derive(Clone, Copy, Default)]
pub struct Fielded {
    pub units: u32,
    pub cost: u64,
}

pub struct EnteredUnit {
    pub price: u32,
    pub entered_tick: Tick,
    /// Current refund beneficiary; price and entry time stay with the unit.
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
    /// Income owed below one microcredit, carried to the next tick.
    pub income_remainder: u64,
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
            income_remainder: 0,
        }
    }

    /// Whether this player lets preparation end: it said so, or its seat is
    /// directed and so never decides it for itself.
    pub fn is_ready(&self) -> bool {
        self.ready || self.controller == ControllerKind::DirectedAi
    }

    /// The places pending purchases hold under the unit cap: a squad bought
    /// in a carrier holds two.
    pub fn reserved_units(&self) -> u32 {
        self.reservations.iter().map(|r| r.data.units()).sum()
    }

    /// What this player's army is worth: its living units' purchase prices,
    /// `fielded`, plus every purchase still on its way.
    fn army(&self, fielded: u64) -> u64 {
        fielded
            + self
                .reservations
                .iter()
                .map(|r| u64::from(r.total()))
                .sum::<u64>()
    }
}

impl Skirmish {
    pub fn new(setup: SkirmishSetup, source: &[Player]) -> Self {
        let entry_count = setup.sites.entries.len();
        let budget = opening_wallet(&setup.rules);
        let players = source
            .iter()
            .map(|p| {
                let wallet = setup
                    .opening_credits
                    .get(&p.id)
                    .map_or(budget, |credits| credits * CREDIT_SCALE);
                PlayerState::new(p, wallet)
            })
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
        }
    }
    /// Each player's living units among `units`.
    pub fn fielded(&self, units: &[crate::units::Unit]) -> Vec<Fielded> {
        let mut fielded = vec![Fielded::default(); self.players.len()];
        for unit in units.iter().filter(|u| u.alive()) {
            let Some(player) = unit.owner.as_deref().and_then(|id| self.player_index(id)) else {
                continue;
            };
            fielded[player].units += 1;
            fielded[player].cost += self.entered.get(&unit.id).map_or(0, |e| u64::from(e.price));
        }
        fielded
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
    pub fn advance(&mut self, tick: Tick, hz: u32, fielded: &[Fielded]) {
        if self.phase == Phase::Preparation {
            if self.players.iter().all(PlayerState::is_ready)
                || tick >= u64::from(self.setup.rules.preparation_s) * u64::from(hz)
            {
                self.phase = Phase::Active;
            }
        } else if self.phase == Phase::Active {
            let den = 60 * u64::from(hz);
            for (index, p) in self.players.iter_mut().enumerate() {
                let army = p.army(fielded.get(index).map_or(0, |f| f.cost));
                let num = p.income_remainder + income_per_minute(&self.setup.rules, army);
                p.wallet += num / den;
                p.income_remainder = num % den;
            }
        }
    }
    pub fn view(&self, side: Side, tick: Tick, hz: u32, fielded: &[Fielded]) -> SkirmishView {
        let players: Vec<_> = self
            .side_players(side)
            .map(|(index, p)| (index, p, fielded.get(index).copied().unwrap_or_default()))
            .map(|(index, p, fielded)| contract::skirmish::PlayerView {
                index: contract::players::PlayerIndex(index as u16),
                id: p.id.clone(),
                controller: p.controller.clone(),
                ready: p.is_ready(),
                credits: p.wallet as f64 / CREDIT_SCALE as f64,
                income_per_minute: income_per_minute(&self.setup.rules, p.army(fielded.cost))
                    as f64
                    / CREDIT_SCALE as f64,
                unit_count: fielded.units + p.reserved_units(),
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
        purchase: Purchase,
        destination: [f64; 2],
        entry_tick: (crate::entries::EntryIndex, Tick),
        living: u32,
    ) -> Result<PurchaseId, OrderError> {
        let (entry, tick) = entry_tick;
        let p = self
            .players
            .get_mut(player)
            .ok_or(OrderError::MissingPlayer)?;
        if living + p.reserved_units() + purchase.units() > self.setup.rules.max_units {
            return Err(OrderError::UnitLimitReached);
        }
        let price = purchase.total();
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
            price: purchase.price,
            carrier_price: purchase.carrier.map(|(_, price)| price),
            player,
            data: PendingPurchase {
                id,
                kind: purchase.kind,
                carrier: purchase.carrier.map(|(kind, _)| kind),
                entry: entry.0,
                destination,
                confirmed_tick: tick,
                blocked: false,
                arrival_tick: None,
                condition: None,
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
        d.u64(self.phase as u64);
        for (index, p) in self.players.iter().enumerate() {
            d.u64(p.side.index() as u64)
                .u64(p.wallet)
                .u64(p.income_remainder)
                .u64(p.ready as u64)
                .u64(p.next_purchase as u64);
            d.u64(p.reservations.len() as u64);
            for r in &p.reservations {
                // Only transferred reservations need separate buyer identity.
                if r.player != index {
                    d.u64(u64::MAX).u64(r.player as u64);
                }
                d.u64(r.data.id.0 as u64)
                    .u64(r.data.kind.0 as u64)
                    .f64(r.data.destination[0])
                    .f64(r.data.destination[1])
                    .u64(r.data.confirmed_tick)
                    .u64(r.data.blocked as u64)
                    .u64(r.price as u64)
                    .u64(r.data.arrival_tick.unwrap_or(u64::MAX))
                    .u64(r.data.entry as u64);
                if let Some(condition) = &r.data.condition {
                    d.bytes(b"starting wear")
                        .bytes(&serde_json::to_vec(condition).expect("unit wear serializes"));
                }
                // Only a purchase with a carrier hashes it, so battles that
                // buy none keep their digests.
                if let (Some(kind), Some(price)) = (r.data.carrier, r.carrier_price) {
                    d.u64(kind.0 as u64).u64(price as u64);
                }
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
            p.wallet += u64::from(r.total()) * CREDIT_SCALE;
        }
        Ok(())
    }
}
pub fn starter_rules() -> MatchRules {
    resolve_rules(MatchRuleSet::Skirmish)
}

/// Resolve the selected override document over the one base match rule record.
/// Native tools and browser/server preparation all read these authored documents.
pub fn resolve_rules(selection: MatchRuleSet) -> MatchRules {
    let base: MatchRules = serde_json::from_str(include_str!("../../../fixtures/skirmish.json"))
        .expect("authored base match rules");
    match selection {
        MatchRuleSet::Skirmish => base,
        MatchRuleSet::Frontline => {
            let overrides: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(include_str!("../../../fixtures/frontline.json"))
                    .expect("authored match rule overrides");
            base.patched(&overrides)
                .expect("authored resolved match rules")
        }
    }
}
