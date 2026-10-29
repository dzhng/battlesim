//! What a side hears at a completed tick: the sounds it can place and the
//! barks its soldiers shout.
use super::ObservedTargetId;
use crate::catalog::TypeIndex;
use crate::ids::UnitId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundCategory {
    Infantry,
    Vehicle,
    Shot,
    /// An unseen enemy shouted: never which bark, voice or unit.
    Shout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundBand {
    Near,
    Far,
}

/// A friendly listener heard an unseen enemy: broad character, one of eight
/// directions and a near/far band. Never a position; never a visual contact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundCue {
    pub listener: UnitId,
    pub category: SoundCategory,
    /// 0 = east, counter-clockwise in 45° steps.
    pub sector: u8,
    pub band: SoundBand,
    /// The source was moving (engine working, footsteps) rather than idle.
    pub moving: bool,
}

/// A bark emitted by the simulation. The client chooses and plays a take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShoutTrigger {
    B1ContactInfantry,
    B2ContactVehicle,
    B3ContactAir,
    B4OpensFire,
    B5UnderFire,
    B6Suppressed,
    B7Pinned,
    B8Hit,
    B9ManDown,
    B10BadlyDamaged,
    B11Kill,
    B12Reloading,
    B13OutOfAmmo,
    B14MissileAway,
    B15IncomingMissile,
    B16Destroyed,
    B17FriendlyDestroyedNearby,
}

impl ShoutTrigger {
    pub const ALL: [ShoutTrigger; 17] = [
        ShoutTrigger::B1ContactInfantry,
        ShoutTrigger::B2ContactVehicle,
        ShoutTrigger::B3ContactAir,
        ShoutTrigger::B4OpensFire,
        ShoutTrigger::B5UnderFire,
        ShoutTrigger::B6Suppressed,
        ShoutTrigger::B7Pinned,
        ShoutTrigger::B8Hit,
        ShoutTrigger::B9ManDown,
        ShoutTrigger::B10BadlyDamaged,
        ShoutTrigger::B11Kill,
        ShoutTrigger::B12Reloading,
        ShoutTrigger::B13OutOfAmmo,
        ShoutTrigger::B14MissileAway,
        ShoutTrigger::B15IncomingMissile,
        ShoutTrigger::B16Destroyed,
        ShoutTrigger::B17FriendlyDestroyedNearby,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intensity {
    Steady,
    Stressed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShoutSpeaker {
    Own(UnitId),
    Seen(ObservedTargetId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shout {
    pub speaker: ShoutSpeaker,
    pub kind: TypeIndex,
    pub position: [f64; 3],
    pub trigger: ShoutTrigger,
    pub intensity: Intensity,
}
