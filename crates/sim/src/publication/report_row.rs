//! A match report event's row: when, what (spawn 0, refund 1, kill 2), the
//! unit it is about and, for a kill, the killer; `otherSide` and the other
//! fields are -1 without one.
use contract::skirmish::MatchEvent;

use super::limbs;

pub(super) const REPORT_EVENT_FIELDS: [&str; 15] = [
    "tickLo",
    "tickHi",
    "kind",
    "side",
    "unitIdLo",
    "unitIdHi",
    "unitKind",
    "owner",
    "otherSide",
    "otherIdLo",
    "otherIdHi",
    "otherKind",
    "otherOwner",
    "amount",
    "otherAmount",
];

pub(super) fn report_event_row(event: &MatchEvent) -> Result<[f32; 15], String> {
    let (tick, kind, side, unit, other, amount, other_amount) = match event {
        MatchEvent::Spawn { tick, unit, cost } => {
            (*tick, 0.0, unit.side, *unit, None, *cost as f32, 0.0)
        }
        MatchEvent::Refund {
            tick,
            unit,
            credits,
        } => (*tick, 1.0, unit.side, *unit, None, *credits as f32, 0.0),
        MatchEvent::Kill {
            tick,
            victim,
            victim_cost,
            killer,
            killer_cost,
        } => (
            *tick,
            2.0,
            victim.side,
            *victim,
            *killer,
            *victim_cost as f32,
            killer_cost.unwrap_or_default() as f32,
        ),
    };
    let [tick_lo, tick_hi] = limbs(
        tick.try_into()
            .map_err(|_| "report tick exceeds publication range")?,
    );
    let [id_lo, id_hi] = limbs(unit.id.0);
    let owner = unit.owner.map_or(-1.0, |p| f32::from(p.0));
    let (other_side, other_id_lo, other_id_hi, other_kind, other_owner) =
        other.map_or((-1.0, -1.0, -1.0, -1.0, -1.0), |u| {
            let [lo, hi] = limbs(u.id.0);
            (
                u.side.index() as f32,
                lo,
                hi,
                u.kind.0 as f32,
                u.owner.map_or(-1.0, |p| f32::from(p.0)),
            )
        });
    Ok([
        tick_lo,
        tick_hi,
        kind,
        side.index() as f32,
        id_lo,
        id_hi,
        unit.kind.0 as f32,
        owner,
        other_side,
        other_id_lo,
        other_id_hi,
        other_kind,
        other_owner,
        amount,
        other_amount,
    ])
}
