use serde::{Deserialize, Serialize};

/// A fixed simulation step count since battle start (30 Hz by default).
pub type Tick = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Blue,
    Red,
    Neutral,
}

impl Side {
    pub const ALL: [Side; 3] = [Side::Blue, Side::Red, Side::Neutral];
    /// The two sides accepted by binary encounter/economy owners. Neutral is
    /// intentionally excluded until those modes define its wallet/objective
    /// semantics.
    pub const COMBATANTS: [Side; 2] = [Side::Blue, Side::Red];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Authority-wide unit handle. A side only ever sees its own units' ids;
/// enemies appear through side-scoped observation ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitId(pub u32);
