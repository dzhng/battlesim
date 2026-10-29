//! The pose driver's test feed: the fixture's real appearances and pose
//! feel, and synthetic feed frames of riflemen, squads and a tank.
#![allow(dead_code)]
use std::sync::Arc;

use presentation::units::driver::{FeedFallen, FeedFrame, FeedMount, FeedSoldier, FeedUnit};
use presentation::units::pose::{PoseFrame, SoldierPose};
use presentation::world::UnitsSection;
use serde_json::Value;

pub const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/paris-corner-units.json"
);

/// The fixture's units section, edited by `edit`.
pub fn units_with(edit: impl FnOnce(&mut Value)) -> UnitsSection {
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    edit(&mut v);
    serde_json::from_value(v).unwrap()
}

pub fn units() -> UnitsSection {
    units_with(|_| {})
}

pub fn feel(units: &UnitsSection, path: &str) -> f64 {
    units.pose.pointer(path).and_then(Value::as_f64).unwrap()
}

/// A rifleman at (x, y) with his rifle up, `shots` rounds fired.
pub fn man(id: f64, x: f64, y: f64, shots: f64) -> FeedSoldier {
    FeedSoldier {
        id,
        active_mount: Some(0),
        position: [x, y, 0.0],
        shots,
        ..FeedSoldier::default()
    }
}

pub fn squad(soldiers: Vec<FeedSoldier>, pinned: bool) -> FeedUnit {
    FeedUnit {
        id: 1.0,
        kind: "capture_rifle".into(),
        side: "blue".into(),
        soldiers,
        mounts: vec![FeedMount::default(); 2],
        pinned,
        ..FeedUnit::default()
    }
}

pub fn frame(time: f64, units: Vec<FeedUnit>, fallen: &Arc<Vec<FeedFallen>>) -> FeedFrame {
    FeedFrame {
        time,
        units,
        fallen: fallen.clone(),
    }
}

pub fn fallen(ids: &[f64]) -> Arc<Vec<FeedFallen>> {
    Arc::new(
        ids.iter()
            .map(|&id| FeedFallen {
                soldier: id,
                position: [id, 0.0, 0.0],
                yaw: 0.0,
                kind: "capture_rifle".into(),
                slot: 0,
                side: "blue".into(),
            })
            .collect(),
    )
}

pub fn clip_of(poses: &PoseFrame, id: f64) -> String {
    pose_of(poses, id).clip.to_string()
}

pub fn pose_of(poses: &PoseFrame, id: f64) -> &SoldierPose {
    poses.soldiers.iter().find(|s| s.soldier == id).unwrap()
}

/// The fixture's tank: hull yaw, cannon (bearing, elevation, shots), HMG bearing.
pub fn tank(x: f64, yaw: f64, gun: (f64, f64, f64), hmg: f64) -> FeedUnit {
    FeedUnit {
        id: 5.0,
        kind: "capture_us_tank".into(),
        side: "blue".into(),
        position: [x, 0.0, 0.0],
        yaw,
        mounts: vec![
            FeedMount {
                bearing: gun.0,
                elevation: gun.1,
                shots: gun.2,
                ..FeedMount::default()
            },
            FeedMount {
                bearing: hmg,
                elevation: 0.2,
                ..FeedMount::default()
            },
        ],
        ..FeedUnit::default()
    }
}
