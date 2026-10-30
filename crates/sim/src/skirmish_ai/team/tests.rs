//! Shared fixtures for the team planner's focused tests.

use super::super::*;
use super::party::{recruit, Ask, PartyKey};
use super::*;
use contract::encounter::{EntrySite, ObjectiveSiteKind};
use contract::players::Player;

fn sites() -> SkirmishSites {
    SkirmishSites {
        entries: vec![
            EntrySite {
                side: Side::Blue,
                center: [100.0, 0.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Blue,
                center: [300.0, 0.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Blue,
                center: [500.0, 0.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Red,
                center: [100.0, 1000.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Red,
                center: [300.0, 1000.0],
                yaw: 0.0,
            },
            EntrySite {
                side: Side::Red,
                center: [500.0, 1000.0],
                yaw: 0.0,
            },
        ],
        objectives: vec![
            ObjectiveSite {
                id: "west".into(),
                center: [100.0, 500.0],
                radius_m: 30.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: None,
            },
            ObjectiveSite {
                id: "centre".into(),
                center: [300.0, 500.0],
                radius_m: 30.0,
                kind: ObjectiveSiteKind::Junction,
                counterpart: None,
            },
            ObjectiveSite {
                id: "east".into(),
                center: [500.0, 500.0],
                radius_m: 30.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: None,
            },
        ],
        candidates: vec![],
    }
}

/// The test cards on a roster, so the objective rule knows who can hold:
/// the tank and rifles hold ground, the helicopter flies over it, and
/// `test_gun_jeep` becomes a ground air-defence vehicle that can hold but
/// cannot engage ground (D57).
fn team_rules() -> Rules {
    let mut fixture = crate::fixtures::test_rules();
    for (id, category) in [
        ("test_tank", "veh"),
        ("test_rifle", "inf"),
        ("test_heli", "hel"),
        ("test_gun_jeep", "veh"),
    ] {
        crate::fixtures::patch_catalog(
            &mut fixture,
            "units",
            id,
            serde_json::json!({ "roster": { "factions": ["us"], "category": category,
                "family_name": "Test", "variant": "Test" } }),
        );
    }
    crate::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_gun_jeep",
        serde_json::json!({ "roles": ["air_defense"],
            "mounts": [{ "id": "HMG", "name": "AA missile", "weapons": ["stinger"] }] }),
    );
    serde_json::from_value(fixture).unwrap()
}

/// A party ask of up to `units` units, whatever their strength.
fn up_to(units: usize) -> Ask {
    Ask {
        units,
        strength: f64::INFINITY,
    }
}

fn players() -> Vec<PlayerState> {
    (0..3)
        .map(|index| Player {
            id: format!("blue-{}", index + 1),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: ControllerKind::ActiveAi,
            human_capable: false,
        })
        .chain((0..3).map(|index| Player {
            id: format!("red-{}", index + 1),
            side: Side::Red,
            faction: Some(contract::catalog::Faction::Europe),
            controller: ControllerKind::ActiveAi,
            human_capable: false,
        }))
        .map(|player| PlayerState::new(&player, 0))
        .collect()
}

mod cohesion;
mod help;
mod parties;
mod planning;
mod stance;
mod value;
