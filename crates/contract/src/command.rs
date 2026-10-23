//! Player and controller intent. Commands are validated by the authority,
//! acknowledged in order, and applied at a tick boundary.
use crate::ids::{Side, Tick, UnitId};
use crate::observation::{ContactId, ObservedTargetId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicy {
    /// Minimise distance (ordinary right-click move).
    #[default]
    Shortest,
    /// Minimise travel time using roads and terrain speeds (double right-click).
    Fastest,
}

/// Which way a move drives (Q31): forwards, or backwards along the route
/// with the hull's facing held, at the vehicle's reverse speed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveDirection {
    #[default]
    Forward,
    /// A vehicle backs along its route; infantry ignore it.
    Reverse,
}

/// What an attack aims at, in the ordering side's own vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetRef {
    Identified { id: ObservedTargetId },
    Contact { id: ContactId },
    Ground { point: [f64; 3] },
}

/// A whole unit's fire policy (W12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engagement {
    FireAtWill,
    /// "Hold fire": only retaliate against an enemy attacking this unit.
    ReturnFireOnly,
}

/// A side-scoped, read-only resolution of a proposed group move.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveDestination {
    pub unit: UnitId,
    pub goal: [f64; 2],
    pub placed: bool,
    /// An explicit facing is the arrival target for pivot-capable units.
    /// Without one, and for wheels, routing may refine this approach estimate.
    pub facing: f64,
    /// Where a squad's living soldiers stand round `goal`, in member order,
    /// as a squad arriving there spreads out on the ground its side knows.
    /// The order draws its own arrangement and cover moves it, so this shows
    /// the squad's shape, not each soldier's final spot. Empty for a vehicle,
    /// and for a gathering or a withdrawal, which spread out on arrival.
    pub spots: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovePlacement {
    pub gesture: u64,
    pub destinations: Vec<MoveDestination>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MovePreviewRequest {
    pub units: Vec<UnitId>,
    pub goal: [f64; 2],
    pub facing: Option<f64>,
    #[serde(default)]
    pub direction: MoveDirection,
    #[serde(default)]
    pub route: RoutePolicy,
    #[serde(default)]
    pub queued: bool,
}

/// One side's combined entry and gathering intent, without accepting an order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildingPreviewRequest {
    pub units: Vec<UnitId>,
    pub building: u32,
    pub facing: Option<f64>,
    #[serde(default)]
    pub queued: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingEntry {
    pub unit: UnitId,
    pub approach: [f64; 2],
}

/// The exact entry and certified companion destinations prepared by admission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingPlacement {
    pub building: u32,
    pub entrant: Option<BuildingEntry>,
    pub destinations: Vec<MoveDestination>,
    /// Work ended before every possibility was proved; this is not impossibility.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unproven: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Order {
    Refund {
        units: Vec<UnitId>,
    },
    Ready,
    CancelPending {
        purchase: crate::skirmish::PurchaseId,
    },
    ConfirmPurchase {
        variant: String,
        destination: [f64; 2],
    },
    Move {
        units: Vec<UnitId>,
        /// Client-issued token identifying this gesture for a later upgrade.
        gesture: u64,
        goal: [f64; 2],
        route: RoutePolicy,
        /// R, or a right-click behind a single selected vehicle, reverses.
        #[serde(default)]
        direction: MoveDirection,
        /// A right-drag's facing (Q9): the world bearing (radians,
        /// counter-clockwise from +X) the units face once there. `None`: the
        /// direction of travel. Group offsets rotate from the first selected
        /// unit's heading to this bearing, about the clicked front-center anchor.
        /// Wheeled vehicles keep their approach
        /// heading; a seen or engaged enemy overrides ordered facing.
        #[serde(default)]
        facing: Option<f64>,
    },
    Stop {
        units: Vec<UnitId>,
    },
    /// Focus every weapon that can damage the target; pursue to regain a
    /// firing position. Switches the units to fire at will (W14, W17).
    Attack {
        units: Vec<UnitId>,
        target: TargetRef,
    },
    /// Move, halting while any weapon has an engageable target (W16).
    AttackMove {
        units: Vec<UnitId>,
        gesture: u64,
        goal: [f64; 2],
    },
    SetEngagement {
        units: Vec<UnitId>,
        policy: Engagement,
    },
    /// Persistently toggle an aircraft's afterburner. Other units ignore it.
    SetAfterburner {
        units: Vec<UnitId>,
        enabled: bool,
    },
    /// Set up in place (`deployed`) or pack for movement, for units that
    /// deploy (L01, L02). Deploying cancels movement; others ignore it.
    SetDeployment {
        units: Vec<UnitId>,
        deployed: bool,
    },
    /// Walk one squad to a building, then enter after a stationary timer.
    /// The complete squad must fit, or the order is refused.
    Garrison {
        units: Vec<UnitId>,
        building: u32,
    },
    /// One eligible squad enters; selected companions gather on the approach side.
    OccupyBuilding {
        units: Vec<UnitId>,
        building: u32,
        gesture: u64,
        #[serde(default)]
        facing: Option<f64>,
    },
    /// Leave the building after a stationary timer, to free ground outside.
    ExitBuilding {
        units: Vec<UnitId>,
    },
    /// Walk selected squads to their own carrier and board after the timer.
    Board {
        units: Vec<UnitId>,
        carrier: UnitId,
        gesture: u64,
    },
    /// Drive the carrier to selected squads and take them aboard.
    Collect {
        units: Vec<UnitId>,
        squads: Vec<UnitId>,
        gesture: u64,
    },
    /// Drive carriers to a point and unload, or unload in place when no goal
    /// is supplied. Shift queues a goal unload; an in-place unload replaces
    /// the queue.
    Unload {
        units: Vec<UnitId>,
        goal: Option<[f64; 2]>,
        gesture: u64,
        route: RoutePolicy,
    },
    /// Double right-click: switch the orders issued by `gesture` to `route`.
    /// A gesture whose orders have all completed is an acknowledged no-op.
    UpgradeMove {
        gesture: u64,
        route: RoutePolicy,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub side: Side,
    /// Player issuing the order. Required when the scenario assigns units to
    /// players; omitted by legacy side-owned scenarios.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<String>,
    /// Strictly increasing per side, starting at 1.
    pub seq: u64,
    pub order: Order,
    /// Shift: append to the queue instead of replacing it.
    #[serde(default)]
    pub queued: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum OrderError {
    /// Neutral is a passive world side and cannot receive player commands.
    PassiveSide,
    NotSkirmish,
    MatchFinished,
    UnitUnavailable,
    WrongFaction,
    InsufficientCredits,
    UnitLimitReached,
    /// `seq` is not the next number for this side.
    OutOfSequence {
        expected: u64,
    },
    NoUnits,
    UnknownUnit {
        unit: UnitId,
    },
    NotOwnUnit {
        unit: UnitId,
    },
    /// A portable scenario assigned the unit to another player.
    WrongPlayer {
        unit: UnitId,
    },
    /// A player-owned scenario command omitted its player.
    MissingPlayer,
    /// The player is under AI control, so only its controller orders its
    /// units: a client commands no AI player, nor one the AI took over.
    AiControlled,
    OutOfBounds,
    /// No unit has a place to stand there that it can reach.
    NoValidDestination,
    /// Input is disabled while a replay feeds recorded commands.
    ReplayInProgress,
    /// More orders than the side may give this second
    /// (`rules.commands.orders_per_s`): dropped, and never shown.
    RateLimited,
    /// The target reference is not one this side currently holds.
    UnknownTarget,
    /// The contact is in the air: area fire has no ground point to aim at (D22).
    AirContact,
    /// The unit has been destroyed.
    Destroyed {
        unit: UnitId,
    },
    /// Only infantry squads garrison buildings.
    NotInfantry {
        unit: UnitId,
    },
    /// The prop is not a standing building.
    NotABuilding,
    /// The squad has more living soldiers than a building takes.
    CapacityFull,
    /// A building takes one squad: the order names more than one.
    OneSquadPerBuilding,
    /// Another of the side's squads holds the building, is entering it, or
    /// is on its way in.
    BuildingOccupied,
    /// The unit is already riding in a carrier.
    Aboard {
        unit: UnitId,
    },
    /// The named host cannot carry passengers.
    NotACarrier,
    /// No reachable standing spots exist for every passenger.
    NoRoomToUnload,
}

impl OrderError {
    /// The order broke the command contract: its sender could have seen the
    /// refusal coming from what it was shown (its own units, its contacts,
    /// the map, its sequence). Any other refusal is the world's state at the
    /// tick the order is judged (credits, the order rate, room to stand or
    /// enter), which a sender with a correct view still meets.
    pub const fn is_malformed(&self) -> bool {
        match self {
            Self::PassiveSide
            | Self::NotSkirmish
            | Self::WrongFaction
            | Self::OutOfSequence { .. }
            | Self::NoUnits
            | Self::UnknownUnit { .. }
            | Self::NotOwnUnit { .. }
            | Self::WrongPlayer { .. }
            | Self::MissingPlayer
            | Self::AiControlled
            | Self::OutOfBounds
            | Self::ReplayInProgress
            | Self::UnknownTarget
            | Self::AirContact
            | Self::Destroyed { .. }
            | Self::NotInfantry { .. }
            | Self::NotABuilding
            | Self::OneSquadPerBuilding
            | Self::Aboard { .. }
            | Self::NotACarrier => true,
            Self::MatchFinished
            | Self::UnitUnavailable
            | Self::InsufficientCredits
            | Self::UnitLimitReached
            | Self::NoValidDestination
            | Self::RateLimited
            | Self::CapacityFull
            | Self::BuildingOccupied
            | Self::NoRoomToUnload => false,
        }
    }

    /// The stable wire name used by refusal publications.
    pub const fn reason_name(&self) -> &'static str {
        match self {
            Self::PassiveSide => "passive_side",
            Self::NotSkirmish => "not_skirmish",
            Self::MatchFinished => "match_finished",
            Self::UnitUnavailable => "unit_unavailable",
            Self::WrongFaction => "wrong_faction",
            Self::InsufficientCredits => "insufficient_credits",
            Self::UnitLimitReached => "unit_limit_reached",
            Self::OutOfSequence { .. } => "out_of_sequence",
            Self::NoUnits => "no_units",
            Self::UnknownUnit { .. } => "unknown_unit",
            Self::NotOwnUnit { .. } => "not_own_unit",
            Self::WrongPlayer { .. } => "wrong_player",
            Self::MissingPlayer => "missing_player",
            Self::AiControlled => "ai_controlled",
            Self::OutOfBounds => "out_of_bounds",
            Self::NoValidDestination => "no_valid_destination",
            Self::ReplayInProgress => "replay_in_progress",
            Self::RateLimited => "rate_limited",
            Self::UnknownTarget => "unknown_target",
            Self::AirContact => "air_contact",
            Self::Destroyed { .. } => "destroyed",
            Self::NotInfantry { .. } => "not_infantry",
            Self::NotABuilding => "not_a_building",
            Self::CapacityFull => "capacity_full",
            Self::OneSquadPerBuilding => "one_squad_per_building",
            Self::BuildingOccupied => "building_occupied",
            Self::Aboard { .. } => "aboard",
            Self::NotACarrier => "not_a_carrier",
            Self::NoRoomToUnload => "no_room_to_unload",
        }
    }
}

/// One selected unit refused an order. Command-level validation stays in
/// [`CommandAck::error`]; this is the per-unit channel used when a group order
/// partially succeeds or a later step is refused by the simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Refusal {
    pub unit: UnitId,
    pub reason: OrderError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u64,
    /// The tick whose step applies the command (accepted or not, the tick it was judged against).
    pub applied_tick: Tick,
    pub error: Option<OrderError>,
    pub refused: Vec<Refusal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<MovePlacement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingPlacement>,
}
