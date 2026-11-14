use contract::catalog::TypeIndex;
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::ids::Side;
use contract::players::{ControllerKind, Player};
use sim::entries::{self, EntryIndex, LaneNumber};

fn sites() -> SkirmishSites {
    SkirmishSites {
        entries: vec![
            EntrySite {
                side: Side::Blue,
                center: [0.0, 0.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Blue,
                center: [100.0, 0.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Red,
                center: [0.0, 100.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Red,
                center: [100.0, 100.0],
                yaw: 0.0,
            },
        ],
        objectives: vec![ObjectiveSite {
            id: "centre".into(),
            center: [50.0, 50.0],
            radius_m: 10.0,
            kind: ObjectiveSiteKind::Junction,
            counterpart: None,
        }],
        candidates: vec![],
    }
}

#[test]
fn select_uses_the_shortest_route_and_centre_wins_a_tie() {
    let sites = sites();
    let kind = TypeIndex(7);
    let selected = entries::select(&sites, Side::Blue, kind, [90.0, 0.0], |entry, _, point| {
        let dx = entry.center[0] - point[0];
        let dy = entry.center[1] - point[1];
        Some((dx * dx + dy * dy).sqrt())
    });
    assert_eq!(selected, EntryIndex(1));

    let tie = entries::select(&sites, Side::Blue, kind, [50.0, 0.0], |entry, _, point| {
        let dx = entry.center[0] - point[0];
        let dy = entry.center[1] - point[1];
        Some((dx * dx + dy * dy).sqrt())
    });
    assert_eq!(tie, EntryIndex(0));
}

#[test]
fn retire_at_accepts_any_entry_on_the_side() {
    let mut sites = sites();
    sites.entries[0].center = [20.0, 30.0];
    sites.entries[1].center = [80.0, 30.0];
    assert!(entries::retire_at(&sites, Side::Blue, [20.0, 30.0], 2.0));
    assert!(entries::retire_at(&sites, Side::Blue, [80.0, 30.0], 2.0));
    assert!(!entries::retire_at(&sites, Side::Blue, [50.0, 30.0], 2.0));
}

#[test]
fn every_synthetic_side_lists_its_centre_road_first_then_flanks_left_to_right() {
    let width = 900.0;
    for roads in 1..=3 {
        let sites = sim::skirmish::synthetic_sites([width, 600.0], roads);
        for side in Side::COMBATANTS {
            let xs: Vec<f64> = sites
                .entries
                .iter()
                .filter(|entry| entry.side == side)
                .map(|entry| entry.center[0])
                .collect();
            assert_eq!(xs.len(), roads, "{side:?}");
            let centre = entries::site(&sites, entries::centre(&sites, side)).center[0];
            assert_eq!(xs[0], width / 2.0, "{roads} roads, {side:?} first entry");
            assert_eq!(centre, width / 2.0, "{roads} roads, {side:?} centre entry");
            assert!(
                xs[1..].windows(2).all(|pair| pair[0] < pair[1]),
                "{roads} roads, {side:?} flanks left to right: {xs:?}"
            );
        }
    }
}

fn seat(id: &str, side: Side, controller: ControllerKind) -> Player {
    Player {
        id: id.into(),
        side,
        faction: None,
        human_capable: controller == ControllerKind::Human,
        controller,
    }
}

/// `blue` against `red` players, AI except Blue's `human` slot (from 1).
fn seats(blue: usize, red: usize, human: Option<usize>) -> Vec<Player> {
    let side = |side: Side, count: usize, name: &str| {
        (1..=count)
            .map(|k| {
                let controller = if side == Side::Blue && human == Some(k) {
                    ControllerKind::Human
                } else {
                    ControllerKind::ActiveAi
                };
                seat(&format!("{name}-{k}"), side, controller)
            })
            .collect::<Vec<_>>()
    };
    [side(Side::Blue, blue, "blue"), side(Side::Red, red, "red")].concat()
}

fn x(sites: &SkirmishSites, index: EntryIndex) -> f64 {
    entries::site(sites, index).center[0]
}

#[test]
fn a_sides_lanes_count_down_from_its_high_x_end() {
    for roads in 1..=3 {
        let sites = sim::skirmish::synthetic_sites([900.0, 600.0], roads);
        for side in Side::COMBATANTS {
            let lanes: Vec<_> = (1..=roads as u8)
                .map(|lane| entries::lane_entry(&sites, side, LaneNumber(lane)).expect("lane"))
                .collect();
            assert!(
                lanes
                    .windows(2)
                    .all(|pair| x(&sites, pair[0]) > x(&sites, pair[1])),
                "{roads} roads, {side:?}: lanes descend in x"
            );
            for &index in &lanes {
                assert_eq!(entries::site(&sites, index).side, side);
            }
            let past = entries::lane_entry(&sites, side, LaneNumber(roads as u8 + 1));
            assert_eq!(past, None, "{roads} roads, {side:?}: no lane past the last");
        }
    }
}

#[test]
fn entries_at_the_same_x_number_in_list_order() {
    let mut sites = sites();
    sites.entries[1].center[0] = sites.entries[0].center[0];
    assert_eq!(entries::lane_number(&sites, EntryIndex(0)), LaneNumber(1));
    assert_eq!(entries::lane_number(&sites, EntryIndex(1)), LaneNumber(2));
}

#[test]
fn slot_k_holds_lane_k_and_later_slots_hold_none() {
    let sites = sim::skirmish::synthetic_sites([900.0, 600.0], 3);
    let players = seats(5, 2, None);
    let sides: Vec<_> = players.iter().map(|player| player.side).collect();
    let homes = entries::homes(&sites, &sides);
    for (player, home) in players.iter().zip(&homes) {
        let slot = players
            .iter()
            .filter(|other| other.side == player.side)
            .position(|other| other.id == player.id)
            .unwrap()
            + 1;
        let expected = entries::lane_entry(&sites, player.side, LaneNumber(slot as u8));
        assert_eq!(*home, expected, "{}", player.id);
    }
    assert_eq!(homes.iter().filter(|home| home.is_some()).count(), 3 + 2);
    // Red's third lane has no slot to hold it.
    let red_third = entries::lane_entry(&sites, Side::Red, LaneNumber(3));
    assert!(!homes.contains(&red_third));
}

#[test]
fn the_view_opens_on_the_humans_lane_or_else_its_sides_centre() {
    let sites = sim::skirmish::synthetic_sites([900.0, 600.0], 3);
    for slot in 1..=3 {
        let opening = entries::opening(&sites, &seats(3, 3, Some(slot)));
        let lane = entries::lane_entry(&sites, Side::Blue, LaneNumber(slot as u8));
        assert_eq!(opening, lane, "Blue {slot}");
    }
    let centre = Some(entries::centre(&sites, Side::Blue));
    assert_eq!(entries::opening(&sites, &seats(5, 5, Some(4))), centre);
    assert_eq!(entries::opening(&sites, &seats(3, 3, None)), centre);
}
