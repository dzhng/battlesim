//! The single owner of skirmish entry choice, arrival checks and lanes.
//!
//! Entries are listed centre first (I18), and arrivals, jets and ties read
//! that list. Lanes are a second ordering of the same entries: a side's
//! lanes are numbered from the map's high-x end, which the lobby picture
//! draws at the top, and the player in slot k of its side holds lane k.

use contract::catalog::TypeIndex;
use contract::encounter::{EntrySite, SkirmishSites};
use contract::ids::Side;
use contract::players::{ControllerKind, Player};

use crate::math::V2;

/// A returning unit may finish this far before the exact entry point. It is a
/// physical approach tolerance, independent of the marker's presentation.
const ENTRY_APPROACH_M: f64 = 8.0;

/// An index into the ordered entry list for one side (centre, left, right).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EntryIndex(pub usize);

/// Pick the entry whose route to `point` is shortest. The route callback owns
/// the side's planning geometry; an unavailable route is treated as infinite.
/// Equal routes go to the entry listed first, so the centre wins a tie.
///
/// `least_time` is a time no route from the entry can beat. Entries are
/// asked nearest bound first, and one whose bound is already slower than
/// the best route found is never routed: it could not be chosen.
pub fn select<F, L>(
    sites: &SkirmishSites,
    side: Side,
    kind: TypeIndex,
    point: [f64; 2],
    mut route_time: F,
    least_time: L,
) -> EntryIndex
where
    F: FnMut(&EntrySite, TypeIndex, [f64; 2]) -> Option<f64>,
    L: Fn(&EntrySite) -> f64,
{
    let mut candidates: Vec<(f64, usize)> = (sites.entries.iter().enumerate())
        .filter(|(_, entry)| entry.side == side)
        .map(|(index, entry)| (least_time(entry), index))
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut selected: Option<(f64, usize)> = None;
    for (least, index) in candidates {
        if selected.is_some_and(|(best, _)| least > best) {
            continue;
        }
        let time = route_time(&sites.entries[index], kind, point)
            .filter(|time| time.is_finite() && *time >= 0.0)
            .unwrap_or(f64::INFINITY);
        if selected.is_none_or(|best| (time, index) < best) {
            selected = Some((time, index));
        }
    }
    EntryIndex(selected.expect("combatant entry site").1)
}

/// Return the centre entry for aircraft that must cross the back edge there.
pub fn centre(sites: &SkirmishSites, side: Side) -> EntryIndex {
    centre_of(sites, side).expect("combatant entry site")
}

/// `side`'s centre entry, its first listed (I18), if it has any.
fn centre_of(sites: &SkirmishSites, side: Side) -> Option<EntryIndex> {
    sites
        .entries
        .iter()
        .position(|entry| entry.side == side)
        .map(EntryIndex)
}

/// Look up an entry selected by [`select`].
pub fn site(sites: &SkirmishSites, index: EntryIndex) -> &EntrySite {
    sites.entries.get(index.0).expect("entry index")
}

/// Whether a withdrawing ground or rotor aircraft is close enough to any entry
/// on its side to leave the field. The unit's footprint tolerance is retained
/// around the exit, with a small approach margin so it need not hit the exact
/// point.
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
            delta.length() <= half_extent * 2.0 + ENTRY_APPROACH_M
        })
}

/// A lane's number on its side, from 1 at the map's high-x end. It labels an
/// entry; it is never an index into the entry list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LaneNumber(pub u8);

/// The lane number of the entry at `index`: one more than the count of its
/// side's entries at higher x, the earlier-listed one first on equal x.
pub fn lane_number(sites: &SkirmishSites, index: EntryIndex) -> LaneNumber {
    let entry = site(sites, index);
    let before = sites
        .entries
        .iter()
        .enumerate()
        .filter(|(other, candidate)| {
            candidate.side == entry.side
                && (candidate.center[0] > entry.center[0]
                    || (candidate.center[0] == entry.center[0] && *other < index.0))
        })
        .count();
    LaneNumber(before as u8 + 1)
}

/// The entry that is `side`'s lane `lane`, if the side has that many.
pub fn lane_entry(sites: &SkirmishSites, side: Side, lane: LaneNumber) -> Option<EntryIndex> {
    (0..sites.entries.len())
        .map(EntryIndex)
        .find(|&index| sites.entries[index.0].side == side && lane_number(sites, index) == lane)
}

/// Each player's home entry, in player order: the player in slot k of its
/// side (its k-th player in `sides`) holds lane k, and a slot beyond the
/// side's lanes holds none.
pub fn homes(sites: &SkirmishSites, sides: &[Side]) -> Vec<Option<EntryIndex>> {
    let mut slots = [0u8; 3];
    sides
        .iter()
        .map(|&side| {
            slots[side.index()] += 1;
            lane_entry(sites, side, LaneNumber(slots[side.index()]))
        })
        .collect()
}

/// Where a seated player's view opens: the human's home entry, else the
/// centre entry of the human's side, else Blue's centre. None when that
/// side has no entry.
pub fn opening(sites: &SkirmishSites, players: &[Player]) -> Option<EntryIndex> {
    let sides: Vec<_> = players.iter().map(|player| player.side).collect();
    let human = players
        .iter()
        .position(|player| player.controller == ControllerKind::Human);
    if let Some(home) = human.and_then(|index| homes(sites, &sides)[index]) {
        return Some(home);
    }
    centre_of(sites, human.map_or(Side::Blue, |index| players[index].side))
}
