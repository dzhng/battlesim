//! Drawn motion as sound hears it, decoded from canonical frame emitters.
use serde::Deserialize;

use crate::records::{Emitter, NONE};
use crate::world::Vec3;

/// A drawn vehicle, as posed this frame.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundVehicle {
    /// Unique across both sides (the effects' shooter key).
    pub key: f64,
    /// Its `audio.vehicles` row.
    pub vehicle_class: String,
    pub position: Vec3,
    /// Metres the left and right running gear have rolled.
    pub travel_l: f64,
    pub travel_r: f64,
    /// Turret heading relative to the hull, radians.
    pub turret: f64,
    #[serde(default)]
    pub reverse: bool,
    /// Normalized rotor speed; none for engines without a rotor input.
    pub rotor_speed: Option<f64>,
}

/// A drawn soldier, as posed this frame.
#[derive(Clone, Debug, Deserialize)]
pub struct SoundSoldier {
    pub id: f64,
    pub position: Vec3,
}

/// What the side draws moving this frame.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Motion {
    pub vehicles: Vec<SoundVehicle>,
    pub soldiers: Vec<SoundSoldier>,
}

impl Motion {
    /// The frame's emitters, each vehicle as its class (`class(emitter.class)`,
    /// `default` when it has none), keyed apart by side as the effects key them.
    pub fn of<'a>(emitters: &[Emitter], class: impl Fn(u32) -> Option<&'a str>) -> Self {
        let mut motion = Self::default();
        for e in emitters {
            let position = [0, 1, 2].map(|i| f64::from(e.position[i]));
            if e.member != NONE {
                motion.soldiers.push(SoundSoldier {
                    id: f64::from(e.member),
                    position,
                });
                continue;
            }
            motion.vehicles.push(SoundVehicle {
                key: f64::from(e.unit) * 2.0 + f64::from(e.flags & 1),
                vehicle_class: class(e.class).unwrap_or("default").to_owned(),
                position,
                travel_l: f64::from(e.motion[0]),
                travel_r: f64::from(e.motion[1]),
                turret: f64::from(e.motion[2]),
                reverse: e.flags & 2 != 0,
                rotor_speed: (e.motion[3] >= 0.0).then_some(f64::from(e.motion[3])),
            });
        }
        motion
    }
}
