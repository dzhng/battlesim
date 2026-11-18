//! A vehicle's body.
use crate::scenario::{Armor, PushClass, WeightClass};
use serde::{Deserialize, Serialize};

/// A vehicle's body (Q3, Q14, Q19): a box like any prop, whose weight class
/// is also its cover tier's source (Q24), live or wrecked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hull {
    /// Half length (along heading), half width, half height.
    pub half_extents_m: [f64; 3],
    pub hp: f64,
    pub armor: Armor,
    pub weight_class: WeightClass,
    /// The prop type it leaves when destroyed (a `props` id).
    pub wreck: String,
}

impl Hull {
    /// What it shoves aside: every body lighter than itself (Q3).
    pub fn push_class(&self) -> PushClass {
        PushClass::ALL[usize::from(self.weight_class.rank())]
    }

    /// Whether it shoves aside a body of `weight`.
    pub fn shoves(&self, weight: WeightClass) -> bool {
        self.push_class().pushes(weight)
    }
}
