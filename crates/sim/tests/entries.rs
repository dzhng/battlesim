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
