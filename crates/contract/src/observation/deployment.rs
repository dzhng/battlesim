//! A deploying unit's set-up, as its own side and an enemy that sees it are told.
use serde::{Deserialize, Serialize};

/// The end state a deploying unit's progress is heading to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posture {
    Packed,
    Deployed,
}

/// A deploying unit's one progress value and where it is heading (L01).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeploymentState {
    /// In [0, 1]: 0 packed and free to move, 1 fully deployed.
    pub progress: f64,
    pub target: Posture,
}
