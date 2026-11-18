//! The checks a movement scenario declares and the outcome a run reports.
use super::*;

pub struct Check {
    pub kind: CheckKind,
    /// `Some(reason)` until the rule the check needs has landed.
    pub pending: Option<&'static str>,
}

pub enum CheckKind {
    /// An impossible order is refused by this time and never plans again.
    Refused {
        unit: u32,
        by_s: f64,
    },
    /// Unit ends idle within `within_m` of `at`: a vehicle's centre, a
    /// squad's anchor (its soldiers spread over the area round it).
    Arrive {
        unit: u32,
        at: [f64; 2],
        within_m: f64,
    },
    /// Capture the issued marker at `tick`; end idle within `within_m` of it.
    ArriveAtMarker {
        unit: u32,
        tick: u64,
        within_m: f64,
    },
    /// No living soldier's disc ever overlaps a solid body (every prop that
    /// blocks infantry: all but the ground kinds, Q27).
    SoldiersClearOfProps,
    /// No vehicle hull ever overlaps a prop that blocks vehicles.
    VehiclesClearOfProps,
    /// No living soldier and no vehicle's centre ever stands on water (the
    /// points the rule reads): a river is crossed on a deck or not at all.
    NeverInWater,
    /// No hull corner over water ever hangs more than `max_m` off the deck
    /// that carries the hull.
    HullsOverWater {
        max_m: f64,
    },
    /// At the end, every squad's soldiers stand at least `min_m` apart.
    Spacing {
        min_m: f64,
    },
    /// Soldiers of different squads never come closer than `min_m`.
    SquadsNeverOverlap {
        min_m: f64,
    },
    /// Every living soldier stands within `within_m` of the ground under him.
    OnGround {
        within_m: f64,
    },
    /// At the end, at least `min` soldiers of `unit` have cover against
    /// `threat`'s position, or the ordered facing when absent: the body stands between.
    InCover {
        unit: u32,
        threat: Option<u32>,
        min: usize,
    },
    /// `unit` halts (attack-move) at least `min_m` short of `goal`, and is
    /// still halted at the end.
    HaltsShort {
        unit: u32,
        goal: [f64; 2],
        min_m: f64,
    },
    PropMoved {
        near: [f64; 2],
        min_m: f64,
    },
    PropStays {
        near: [f64; 2],
    },
    /// No living soldier's disc ever overlaps a live vehicle's hull (Q23).
    SoldiersClearOfHulls,
    /// Every soldier ends with the health he started with.
    NobodyHurt,
    /// No two live hulls ever overlap, whatever their sides (Q14).
    VehiclesNeverOverlap,
    /// `unit` waits for another vehicle at some point (the wait before a detour).
    Waits {
        unit: u32,
    },
    /// At the end, at least `min` soldiers of `unit` are each covered
    /// against `threat` by a body no squadmate shares: one tooth each.
    OneBodyEach {
        unit: u32,
        threat: u32,
        min: usize,
    },
    /// At the end, at least `min` soldiers of `unit` have a clear straight
    /// line from their muzzle to one of `threat`'s soldiers (D3: blocked ones
    /// step out).
    ClearLines {
        unit: u32,
        threat: u32,
        min: usize,
    },
    /// `unit` turns at least `min_deg` within four seconds, before its
    /// centre first moves half a metre: it pivots on the spot (Q29, tracks).
    PivotsInPlace {
        unit: u32,
        min_deg: f64,
    },
    /// `unit` never turns tighter than its turning radius, nor standing
    /// still (Q29, wheels).
    WithinRadius {
        unit: u32,
    },
    /// `unit` drives backwards at some point.
    Reverses {
        unit: u32,
    },
    /// `unit` never drives backwards.
    NeverReverses {
        unit: u32,
    },
    /// `unit` keeps the facing it starts with, within a degree (Q31).
    FacingHeld {
        unit: u32,
    },
    /// At the end, `unit` is at rest facing `deg` (a right-drag's facing, Q9).
    EndsFacing {
        unit: u32,
        deg: f64,
        within_deg: f64,
    },
    /// `first` arrives before `second`, and `second` arrives.
    ArrivesFirst {
        first: u32,
        second: u32,
    },
    /// At the end, at least `min` trees fewer stand than at the start (Q16).
    KnocksTrees {
        min: usize,
    },
    /// No tree ever falls.
    TreesStand,
    /// At the end, at least `min_share` of `rect`'s metre cells (`[x0, y0,
    /// x1, y1]`) are open ground: neither forest ground nor under foliage.
    OpenGround {
        rect: [f64; 4],
        min_share: f64,
    },
    /// The solid prop nearest `near` at the start ends destroyed, a prop of
    /// `into` standing in its place (Q17).
    PropBecomes {
        near: [f64; 2],
        into: &'static str,
    },
    /// While inside `rect` (`[x0, y0, x1, y1]`), `unit` averages at least `min_mps`.
    FastThrough {
        unit: u32,
        rect: [f64; 4],
        min_mps: f64,
    },
    /// `side` identifies `unit` on some tick while it is inside `rect`.
    SpottedIn {
        unit: u32,
        side: Side,
        rect: [f64; 4],
    },
    /// `observer`'s side first identifies `first` at least `ratio` times
    /// farther from `observer` than it first identifies `then`.
    SpottedFarther {
        first: u32,
        then: u32,
        observer: u32,
        ratio: f64,
    },
    /// Soldiers of `unit` lean out round tall cover to fire: at least
    /// `min` of them lean out on some tick, and none ever leans from inside
    /// a body.
    Leans {
        unit: u32,
        min: usize,
    },
    /// No soldier twitches while he has somewhere to go (his spot or post
    /// more than half a metre off, set off, his squad neither waiting nor
    /// halted by an attack-move): at most
    /// `max_reversals` reversals of his step (a step more than 135° off his
    /// last one), and at most `max_still_s` without leaving a metre's circle.
    /// Every scenario carries it (`scenarios`).
    NoTwitch {
        max_reversals: u32,
        max_still_s: f64,
    },
}

/// The no-twitch bound every scenario gets. A soldier twitching past a
/// corridor corner reversed 490 times in 40 s and stood 17 s within a metre
/// on its way; a clean walk reverses a handful of times, jostling in a crowd.
pub(super) const NO_TWITCH: CheckKind = CheckKind::NoTwitch {
    max_reversals: 20,
    max_still_s: 8.0,
};

pub(super) fn check(kind: CheckKind) -> Check {
    Check {
        kind,
        pending: None,
    }
}

/// A check's result after a run.
#[derive(Debug)]
pub struct Outcome {
    pub label: String,
    pub pending: Option<&'static str>,
    pub passed: bool,
    pub detail: String,
}
