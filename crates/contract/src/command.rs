//! Player and controller intent. Commands are validated by the authority,
//! acknowledged in order, and applied at a tick boundary.
use crate::ids::{Side, Tick, UnitId};
use crate::observation::{ContactId, ObservedTargetId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicy {
    /// Minimise distance (ordinary right-click move).
    Shortest,
    /// Minimise travel time using roads and terrain speeds (double right-click).
    Fastest,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Order {
    Move {
        units: Vec<UnitId>,
        /// Client-issued token identifying this gesture for a later upgrade.
        gesture: u64,
        goal: [f64; 2],
        route: RoutePolicy,
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
    /// Set up in place (`deployed`) or pack for movement, for units that
    /// deploy (L01, L02). Deploying cancels movement; others ignore it.
    SetDeployment {
        units: Vec<UnitId>,
        deployed: bool,
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
    OutOfBounds,
    /// Input is disabled while a replay feeds recorded commands.
    ReplayInProgress,
    /// The target reference is not one this side currently holds.
    UnknownTarget,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub seq: u64,
    /// The tick whose step applies the command (accepted or not, the tick it was judged against).
    pub applied_tick: Tick,
    pub error: Option<OrderError>,
}
