//! A pending purchase's row: its id, its owner among the side's players,
//! the unit bought, where it is sent, when it was confirmed, whether its
//! entry is blocked, an aircraft's arrival tick (0 when it doesn't wait),
//! and the carrier a squad arrives aboard (-1 for none).
use super::limbs;
use contract::players::PlayerIndex;
use contract::skirmish::PendingPurchase;

pub(super) const PENDING_FIELDS: [&str; 12] = [
    "idLo",
    "idHi",
    "owner",
    "kind",
    "x",
    "y",
    "confirmedLo",
    "confirmedHi",
    "blocked",
    "arrivalLo",
    "arrivalHi",
    "carrier",
];

pub(super) fn pending_row(owner: PlayerIndex, p: &PendingPurchase) -> Result<[f32; 12], String> {
    let [lo, hi] = limbs(p.id.0);
    let [tick_lo, tick_hi] = limbs(
        p.confirmed_tick
            .try_into()
            .map_err(|_| "purchase tick exceeds publication range")?,
    );
    let [arrival_lo, arrival_hi] = limbs(
        p.arrival_tick
            .unwrap_or(0)
            .try_into()
            .map_err(|_| "arrival tick exceeds publication range")?,
    );
    Ok([
        lo,
        hi,
        f32::from(owner.0),
        p.kind.0 as f32,
        p.destination[0] as f32,
        p.destination[1] as f32,
        tick_lo,
        tick_hi,
        p.blocked as u8 as f32,
        arrival_lo,
        arrival_hi,
        p.carrier.map_or(-1.0, |kind| kind.0 as f32),
    ])
}
