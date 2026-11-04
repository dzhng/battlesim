//! How a unit type moves: on foot, on wheels or tracks within its own top
//! speeds, or through the air by its flight model.
use serde::{Deserialize, Serialize};

use super::{at_least_zero, positive, AltitudeLayer};

/// No ground mover's top speed may exceed this (light wheeled vehicles do 110).
pub const MAX_SPEED_KMH: f64 = 130.0;

/// How an airframe flies, apart from the layer it flies in: on rotors,
/// which hover, or on wings, which never stop and attack in passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightModel {
    Rotor,
    FixedWing,
}

impl FlightModel {
    /// Its name as authored (the serde name): the key the rules' per-flight
    /// tables use.
    pub fn name(self) -> &'static str {
        match self {
            FlightModel::Rotor => "rotor",
            FlightModel::FixedWing => "fixed_wing",
        }
    }
}

/// How a unit moves (Q29, Q30). A ground mover has its own two top speeds,
/// in km/h: `offroad_kmh` on open ground and `road_kmh` on a full road. Its
/// locomotion's speed factors scale them on each kind of ground
/// (`MovementRules::speed_factors`). An aircraft ignores the ground and
/// cruises at one speed, at the heights the rules' `air` section sets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Mobility {
    Foot {
        offroad_kmh: f64,
        road_kmh: f64,
    },
    /// Tracks pivot on the spot.
    Tracked {
        offroad_kmh: f64,
        road_kmh: f64,
        turn_deg_s: f64,
        /// Speed factor in reverse, of forward speed.
        reverse_speed_factor: f64,
    },
    /// Wheels hold a minimum turning radius and never pivot.
    Wheeled {
        offroad_kmh: f64,
        road_kmh: f64,
        turn_deg_s: f64,
        turning_radius_m: f64,
        reverse_speed_factor: f64,
    },
    /// Flies in an authored air layer, by its authored flight model.
    Air {
        cruise_kmh: f64,
        turn_deg_s: f64,
        /// How fast it climbs or sinks toward its target height.
        climb_mps: f64,
        /// The altitude layer this aircraft occupies.
        layer: AltitudeLayer,
        /// How it flies: always authored, never read off the layer.
        flight: FlightModel,
        /// How long a bought one takes to reach the map (D38); it appears at
        /// its back edge at once when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arrival: Option<Arrival>,
        /// Its own cruise height over the ground, overriding its layer's
        /// (D40). Read through `AirRules::cruise_agl_m`, never directly.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cruise_agl_m: Option<f64>,
    },
}

/// A bought aircraft's wait off the map (D38): seconds, drawn from the
/// battle's seed between `delay_s[0]` and `delay_s[1]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrival {
    pub delay_s: [f64; 2],
}

impl Mobility {
    pub fn layer(&self) -> AltitudeLayer {
        match self {
            Mobility::Air { layer, .. } => *layer,
            _ => AltitudeLayer::Ground,
        }
    }

    /// A bought aircraft's wait off the map; None for a ground mover and
    /// for an aircraft that appears at once.
    pub fn arrival(&self) -> Option<Arrival> {
        match self {
            Mobility::Air { arrival, .. } => *arrival,
            _ => None,
        }
    }

    /// How an aircraft flies; None for a ground mover.
    pub fn flight(&self) -> Option<FlightModel> {
        match self {
            Mobility::Air { flight, .. } => Some(*flight),
            _ => None,
        }
    }

    /// How it moves over the ground; `None` for an aircraft.
    pub fn locomotion(&self) -> Option<crate::movement::Locomotion> {
        use crate::movement::Locomotion;
        match self {
            Mobility::Foot { .. } => Some(Locomotion::Foot),
            Mobility::Wheeled { .. } => Some(Locomotion::Wheeled),
            Mobility::Tracked { .. } => Some(Locomotion::Tracked),
            Mobility::Air { .. } => None,
        }
    }

    /// `(offroad_kmh, road_kmh)`; an aircraft's cruise is both.
    pub fn speeds_kmh(&self) -> (f64, f64) {
        match *self {
            Mobility::Foot {
                offroad_kmh,
                road_kmh,
            }
            | Mobility::Tracked {
                offroad_kmh,
                road_kmh,
                ..
            }
            | Mobility::Wheeled {
                offroad_kmh,
                road_kmh,
                ..
            } => (offroad_kmh, road_kmh),
            Mobility::Air { cruise_kmh, .. } => (cruise_kmh, cruise_kmh),
        }
    }

    /// `(off-road, road)` top speeds in metres per second.
    pub fn speeds_mps(&self) -> (f64, f64) {
        let (offroad, road) = self.speeds_kmh();
        (offroad / 3.6, road / 3.6)
    }
}

/// Why no flight law flies `flight` in `layer` yet, naming the data it
/// lacks; None for a pair the simulation flies.
fn unflown(layer: AltitudeLayer, flight: FlightModel) -> Option<&'static str> {
    match (layer, flight) {
        (AltitudeLayer::HighAir, FlightModel::Rotor) => Some(
            "mobility: a rotor aircraft in high air has no flight law yet (rotors fly low air)",
        ),
        (AltitudeLayer::LowAir, FlightModel::FixedWing) => Some(
            "mobility: a fixed-wing aircraft in low air has no flight law yet: low air has no fixed-wing cruise height, floor or pass ceilings (air.low_air)",
        ),
        _ => None,
    }
}

/// Why an aircraft's mobility can't be flown: its layer, flight model and
/// numbers in the ranges the simulation divides and eases by. None for a
/// ground mover and for an aircraft it flies.
pub(super) fn air_problem(mobility: &Mobility) -> Option<&'static str> {
    let Mobility::Air {
        cruise_kmh,
        turn_deg_s,
        climb_mps,
        layer,
        flight,
        arrival,
        cruise_agl_m,
    } = *mobility
    else {
        return None;
    };
    if layer == AltitudeLayer::Ground {
        return Some(
            "mobility: an aircraft flies in low_air or high_air; in the ground layer it would sit at 0 m, a target for nothing",
        );
    }
    if let Some(missing) = unflown(layer, flight) {
        return Some(missing);
    }
    if cruise_agl_m.is_some_and(|h| !positive(h)) {
        return Some("mobility: an aircraft's cruise_agl_m must be positive");
    }
    if arrival.is_some_and(|a| {
        let [min, max] = a.delay_s;
        !(at_least_zero(min) && at_least_zero(max) && min <= max)
    }) {
        return Some("mobility: arrival.delay_s must be [min, max] with 0 <= min <= max");
    }
    // Admission checks its flight model's speed envelope.
    if !positive(cruise_kmh) {
        return Some("mobility: an aircraft's cruise_kmh must be positive");
    }
    if !positive(turn_deg_s) || !positive(climb_mps) {
        return Some("mobility: an aircraft's turn_deg_s and climb_mps must be positive");
    }
    None
}
