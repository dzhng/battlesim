use contract::catalog::TypeIndex;
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::ids::Side;
use sim::entries::{self, EntryIndex};

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
