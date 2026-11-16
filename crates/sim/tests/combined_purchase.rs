//! A squad bought together with its ride: one purchase that is two units
//! against the cap, costs both prices, arrives with the squad already aboard,
//! and is paid back per unit.
use crate::common::{self, Seat};
use contract::command::{CommandEnvelope, Order, OrderError};
use contract::ids::{Side, UnitId};
use contract::observation::{HoldPhase, HostRef, ObservationFrame, OwnUnit};
use contract::skirmish::MatchEvent;
use serde_json::json;
use sim::battle::Battle;

const SQUAD: &str = "test_recon";
const DESTINATION: [f64; 2] = [400.0, 200.0];

/// Seats in each carrier the test sells.
const SEATS: usize = 3;

/// A duel on open ground where blue sells `SQUAD` with `soldiers`, the ground
/// carrier `test_apc`, the helicopter `test_transport_heli` and a
/// carrier-less `test_jeep`. The test sets every bound it turns on: the
/// squad's size, each carrier's seats, each price, the cap and the credits.
fn setup(soldiers: usize, max_units: u32, credits: u32) -> contract::scenario::ScenarioDefinition {
    let mut rules = sim::fixtures::test_rules();
    for (kind, category) in [
        (SQUAD, "inf"),
        ("test_apc", "veh"),
        ("test_transport_heli", "hel"),
        ("test_jeep", "veh"),
    ] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"roster":{"factions":["us"],"category":category,"family_name":"Test Ride","variant":kind}}),
        );
    }
    let scouts = vec!["test_scout"; soldiers];
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        SQUAD,
        json!({"cost": 100, "body": {"squad": {"slots": scouts}}}),
    );
    for (carrier, cost) in [("test_apc", 150), ("test_transport_heli", 150)] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            carrier,
            json!({"cost": cost, "mounts": [], "capabilities": {"carriage": {"seats": SEATS}}}),
        );
    }
    serde_json::from_value(json!({
        "map":{"size":[800,600],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"props":[],"buildings":[]},
        "rules":rules,"units":[],"events":[],"scripts":[],
        "players": common::human_duel(),
        "skirmish":{"rules":{"credits_per_minute":credits,"starting_minutes":1,"preparation_s":600,"max_units":max_units,"max_players_per_side":10,"dispatch_interval_s":1,"capture_s":20,"majority_s":1800,"victory_score":1000},"sites":{"entries":[{"side":"blue","center":[400,10],"yaw":1.57},{"side":"red","center":[400,590],"yaw":-1.57}],"objectives":[{"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null},{"id":"south","center":[200,300],"radius_m":50,"kind":"field","counterpart":"north"},{"id":"north","center":[600,300],"radius_m":50,"kind":"field","counterpart":"south"}]}}
    }))
    .unwrap()
}

struct Blue {
    battle: Battle,
    seq: u64,
}

impl Blue {
    fn new(setup: &contract::scenario::ScenarioDefinition) -> Self {
        Self {
            battle: Battle::new(setup, 5),
            seq: 0,
        }
    }

    /// Give `order` and step once, so the frame shows what it did.
    fn order(&mut self, order: Order) -> Option<OrderError> {
        self.seq += 1;
        let error = self
            .battle
            .accept(CommandEnvelope {
                side: Side::Blue,
                player: Some("blue-1".into()),
                seq: self.seq,
                order,
                queued: false,
            })
            .error;
        self.battle.step();
        error
    }

    fn buy(&mut self, variant: &str, carrier: Option<&str>) -> Option<OrderError> {
        self.order(Order::ConfirmPurchase {
            variant: variant.into(),
            carrier: carrier.map(Into::into),
            destination: DESTINATION,
        })
    }

    fn frame(&self) -> &ObservationFrame {
        self.battle.observe(Side::Blue)
    }

    fn price(&self, variant: &str) -> f64 {
        let rules = self.battle.rules();
        f64::from(
            rules
                .catalog
                .get(rules.catalog.index(variant).unwrap())
                .cost,
        )
    }

    fn own(&self, variant: &str) -> Vec<OwnUnit> {
        let kind = self.battle.rules().catalog.index(variant).unwrap();
        (self.frame().own.iter())
            .filter(|u| u.kind == kind)
            .cloned()
            .collect()
    }

    fn start(&mut self) {
        assert_eq!(self.order(Order::Ready), None);
        self.battle.accept(CommandEnvelope {
            side: Side::Red,
            player: Some("red-1".into()),
            seq: 1,
            order: Order::Ready,
            queued: false,
        });
    }

    fn step_until(&mut self, seconds: u32, mut done: impl FnMut(&Self) -> bool) -> bool {
        for _ in 0..seconds * self.battle.rules().tick_hz {
            self.battle.step();
            if done(self) {
                return true;
            }
        }
        false
    }
}

fn aboard(squad: &OwnUnit, carrier: UnitId) -> bool {
    squad.hold.as_ref().is_some_and(|hold| {
        hold.host == HostRef::Carrier { id: carrier.0 } && hold.phase == HoldPhase::Inside
    })
}

fn spawn_cost(battle: &Battle, unit: UnitId) -> Option<u32> {
    battle
        .match_report()?
        .events
        .iter()
        .find_map(|event| match event {
            MatchEvent::Spawn { unit: u, cost, .. } if u.id == unit => Some(*cost),
            _ => None,
        })
}

fn refunded(battle: &Battle, unit: UnitId) -> Option<f64> {
    battle
        .match_report()?
        .events
        .iter()
        .find_map(|event| match event {
            MatchEvent::Refund {
                unit: u, credits, ..
            } if u.id == unit => Some(*credits),
            _ => None,
        })
}

/// Buy `SQUAD` in `carrier` and follow it from the shop to its destination:
/// the pending pair holds two places under the cap and both prices, a
/// cancelled pair gives both back, and the squad arrives already aboard on
/// the carrier's tick, still aboard where the carrier was sent.
fn bought_aboard(carrier: &str) -> (Blue, UnitId, UnitId) {
    // Room for exactly three units and credits for two pairs.
    let mut blue = Blue::new(&setup(SEATS, 3, 500));
    let start = blue.frame().seat().credits;
    let pair = blue.price(SQUAD) + blue.price(carrier);

    assert_eq!(blue.buy(SQUAD, Some(carrier)), None);
    let seat = blue.frame().seat();
    assert_eq!(seat.unit_count, 2, "a pending pair is two units");
    assert_eq!(seat.credits, start - pair, "a pair costs both prices");
    let pending = seat.pending[0].clone();
    assert_eq!(
        pending.carrier,
        blue.battle.rules().catalog.index(carrier),
        "the pending purchase names its carrier"
    );
    assert_eq!(
        blue.buy(SQUAD, Some(carrier)),
        Some(OrderError::UnitLimitReached),
        "a second pair needs two places and only one is left"
    );
    assert_eq!(
        blue.order(Order::CancelPending {
            purchase: pending.id
        }),
        None
    );
    let seat = blue.frame().seat();
    assert_eq!(seat.credits, start, "cancelling a pair refunds both prices");
    assert_eq!(seat.unit_count, 0);

    // One squad on foot, then a pair: exactly the cap.
    assert_eq!(blue.buy(SQUAD, None), None);
    assert_eq!(blue.buy(SQUAD, Some(carrier)), None);
    assert_eq!(blue.frame().seat().unit_count, 3);
    blue.start();
    assert!(
        blue.step_until(60, |b| !b.own(carrier).is_empty()),
        "the carrier never arrived"
    );
    let ride = blue.own(carrier)[0].id;
    let squads = blue.own(SQUAD);
    let rider = (squads.iter())
        .find(|s| aboard(s, ride))
        .unwrap_or_else(|| panic!("no squad arrived aboard on the carrier's tick: {squads:?}"))
        .id;
    assert!(!blue.battle.unit(rider).unwrap().on_field());
    assert_eq!(
        blue.frame().seat().unit_count,
        3,
        "the pair is now two units"
    );
    assert_eq!(
        spawn_cost(&blue.battle, ride).map(f64::from),
        Some(blue.price(carrier)),
        "the carrier records its own price"
    );
    assert_eq!(
        spawn_cost(&blue.battle, rider).map(f64::from),
        Some(blue.price(SQUAD)),
        "the squad records its own price"
    );

    let there = |b: &Blue| {
        let unit = b.battle.unit(ride).unwrap();
        let [x, y] = DESTINATION;
        libm::hypot(unit.position.x - x, unit.position.y - y) < 12.0
            && unit.orders.is_empty()
            && unit.drive_speed_mps.abs() < 0.1
    };
    assert!(blue.step_until(90, there), "the carrier never got there");
    blue.step_until(5, |_| false);
    let squad = (blue.own(SQUAD).into_iter())
        .find(|s| s.id == rider)
        .unwrap();
    assert!(aboard(&squad, ride), "the squad got off without an order");
    (blue, ride, rider)
}

#[test]
fn a_squad_bought_in_a_ground_carrier_arrives_aboard_and_is_refunded_per_unit() {
    let (mut blue, ride, rider) = bought_aboard("test_apc");
    let before = blue.frame().seat().credits;
    assert_eq!(blue.order(Order::Refund { units: vec![ride] }), None);
    assert!(
        blue.step_until(90, |b| b.battle.unit(ride).unwrap().retired),
        "the refunded carrier never left"
    );
    let gained = blue.frame().seat().credits - before;
    let (carrier, squad) = (
        refunded(&blue.battle, ride).expect("the carrier is refunded"),
        refunded(&blue.battle, rider).expect("the squad aboard is refunded"),
    );
    // Both entered together, unhurt and unfired, so each is paid the same
    // share back of its own price.
    let (carrier_share, squad_share) =
        (carrier / blue.price("test_apc"), squad / blue.price(SQUAD));
    assert!(carrier_share > 0.0 && carrier_share <= 1.0);
    assert!(
        (carrier_share - squad_share).abs() < 1e-3,
        "the carrier got {carrier_share} of its price, the squad {squad_share}"
    );
    // Income runs on while the carrier drives off.
    assert!(
        gained >= carrier + squad - 1e-3,
        "the wallet gained {gained}"
    );
}

#[test]
fn a_squad_bought_in_a_helicopter_arrives_aboard_and_gets_off_on_order() {
    let (mut blue, ride, rider) = bought_aboard("test_transport_heli");
    assert_eq!(
        blue.order(Order::Unload {
            units: vec![ride],
            goal: None,
            gesture: 1,
            route: Default::default(),
        }),
        None
    );
    assert!(
        blue.step_until(60, |b| b.battle.unit(rider).unwrap().on_field()),
        "the squad never got off the helicopter"
    );
}

#[test]
fn a_pair_is_refused_without_seats_for_the_squad_room_for_two_or_credits_for_both() {
    let mut short = Blue::new(&setup(SEATS + 1, 30, 1000));
    assert_eq!(
        short.buy(SQUAD, Some("test_apc")),
        Some(OrderError::CapacityFull),
        "one soldier more than the seats"
    );
    assert_eq!(
        short.buy(SQUAD, Some("test_jeep")),
        Some(OrderError::NotACarrier),
        "a vehicle with no seats"
    );
    assert_eq!(
        short.buy("test_jeep", Some("test_apc")),
        Some(OrderError::NotACarrier),
        "a vehicle rides in no carrier"
    );

    let mut full = Blue::new(&setup(SEATS, 2, 1000));
    assert_eq!(full.buy(SQUAD, None), None);
    assert_eq!(
        full.buy(SQUAD, Some("test_apc")),
        Some(OrderError::UnitLimitReached),
        "room for one unit, not two"
    );
    assert_eq!(full.buy(SQUAD, None), None, "the place left takes one unit");

    // Credits for either unit alone, not both.
    let mut poor = Blue::new(&setup(SEATS, 30, 249));
    assert_eq!(
        poor.buy(SQUAD, Some("test_apc")),
        Some(OrderError::InsufficientCredits)
    );
    assert_eq!(poor.frame().seat().credits, 249.0);
    assert!(poor.frame().seat().pending.is_empty());
    assert_eq!(poor.buy("test_apc", None), None);
}
