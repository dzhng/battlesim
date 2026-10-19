//! Shared fixtures for the team planner's focused tests.

use super::super::*;
use super::help::{capable_detachment, detachment, HelpLock};
use super::*;
use contract::encounter::{EntrySite, ObjectiveSiteKind};

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

fn team_rules() -> Rules {
    serde_json::from_value(crate::fixtures::test_game()).unwrap()
}

fn players() -> Vec<Player> {
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
        .collect()
}

mod cohesion;
mod help;
mod planning;
mod stance;
