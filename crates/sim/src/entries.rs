//! The single owner of skirmish entry choice and arrival checks.

use contract::catalog::TypeIndex;
use contract::encounter::{EntrySite, SkirmishSites};
use contract::ids::Side;

use crate::math::V2;

/// An index into the ordered entry list for one side (centre, left, right).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EntryIndex(pub usize);

/// Pick the entry whose route to `point` is shortest. The route callback owns
/// the side's planning geometry; an unavailable route is treated as infinite.
/// Equal routes retain the ordered centre entry because the comparison only
/// replaces a candidate when it is strictly shorter.
pub fn select<F>(
    sites: &SkirmishSites,
    side: Side,
    kind: TypeIndex,
    point: [f64; 2],
    mut route_time: F,
) -> EntryIndex
where
    F: FnMut(&EntrySite, TypeIndex, [f64; 2]) -> Option<f64>,
{
    let mut selected = None;
    let mut best = f64::INFINITY;
    for (index, entry) in sites.entries.iter().enumerate() {
        if entry.side != side {
            continue;
        }
        let time = route_time(entry, kind, point)
            .filter(|time| time.is_finite() && *time >= 0.0)
            .unwrap_or(f64::INFINITY);
        if selected.is_none() || time < best {
            selected = Some(EntryIndex(index));
            best = time;
        }
    }
    selected.expect("combatant entry site")
}

/// Return the centre entry for aircraft that must cross the back edge there.
pub fn centre(sites: &SkirmishSites, side: Side) -> EntryIndex {
    sites
        .entries
        .iter()
        .position(|entry| entry.side == side)
        .map(EntryIndex)
        .expect("combatant entry site")
}

/// Look up an entry selected by [`select`].
pub fn site(sites: &SkirmishSites, index: EntryIndex) -> &EntrySite {
    sites.entries.get(index.0).expect("entry index")
}

/// Whether a withdrawing ground or rotor aircraft has reached any entry on
/// its side.
pub fn retire_at(sites: &SkirmishSites, side: Side, point: [f64; 2], half_extent: f64) -> bool {
    sites
        .entries
        .iter()
        .filter(|entry| entry.side == side)
        .any(|entry| {
            let delta = V2 {
                x: point[0] - entry.center[0],
                y: point[1] - entry.center[1],
            };
            delta.length() <= half_extent * 2.0
        })
}
