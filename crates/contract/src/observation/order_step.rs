//! An own unit's orders as its side sees them: one step per order, the
//! current order first, then each queued order in turn.
use serde::{Deserialize, Serialize};

use crate::command::TargetRef;
use crate::ids::UnitId;

/// One order of an own unit. Every point is read from what the side knows,
/// where the order will take the unit or what it acts on: the overlay draws
/// the chain from them, and none of them is a plan.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrderStep {
    /// Drive to `point`; an attack-move halts there while it can engage.
    Move { point: [f64; 2], attack_move: bool },
    /// Fire on `target`. `target` is absent once the side no longer tracks
    /// the unit it names; `point` is where the side last knew the target to
    /// be (its track, the attack's last known spot, the contact, the
    /// ground), absent when it knows none.
    Attack {
        target: Option<TargetRef>,
        point: Option<[f64; 2]>,
    },
    /// Walk to `point` beside `building`, then enter it.
    Garrison { point: [f64; 2], building: u32 },
    /// Board `carrier`, which stands at `point`.
    Board { point: [f64; 2], carrier: UnitId },
    /// Drive to the squads it collects, the first of them at `point`;
    /// absent once none is left.
    Collect { point: Option<[f64; 2]> },
    /// Let the passengers out at `point`, where the step before ends; with
    /// `into`, they go on into that host.
    Unload {
        point: [f64; 2],
        into: Option<StepHost>,
    },
    /// Leave the building at `point`.
    Exit { point: [f64; 2] },
    /// Drive off the map at `point`, the way home from where the step
    /// before ends.
    ReturnToBase { point: [f64; 2] },
}

/// What a step enters: a placed building or a carrier, by id.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepHost {
    Building { id: u32 },
    Carrier { id: UnitId },
}
