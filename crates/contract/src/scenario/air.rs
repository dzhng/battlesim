//! Air rules admission: layer heights and each airframe's flight.
use serde::{Deserialize, Serialize};

/// How aircraft fly: the heights of each air layer, and the flight model of
/// each kind of airframe (D1, D2, D19). High air is the one layer that bends
/// physics: its heights are lowered from a real jet's for play, and every
/// treatment that follows from flying there keys on the layer.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirRules {
    pub low_air: LowAirLayer,
    pub high_air: HighAirLayer,
    pub rotor: RotorFlight,
    pub fixed_wing: FixedWingFlight,
    /// Aircraft closer than this, centre to centre, drift apart.
    pub separation_m: f64,
}

/// The low-air layer's heights.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LowAirLayer {
    pub cruise_agl_m: f64,
}

/// The high-air layer's heights: where it cruises, and the hard floor
/// nothing in it flies below (D20).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HighAirLayer {
    pub cruise_agl_m: f64,
    pub floor_agl_m: f64,
}

/// A helicopter's flight: clearance over obstacles and settling at a goal.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotorFlight {
    pub clearance_m: f64,
    pub ceiling_agl_m: f64,
    pub obstacle_m: f64,
    /// Seconds an aircraft takes to gather its cruise speed from a hover,
    /// and to shed it: a helicopter's, not a car's.
    pub acceleration_s: f64,
    /// Braking for its goal it runs past by this share of the last leg's
    /// length, at most `settle_max_m`, then eases back onto the point.
    pub settle_share: f64,
    pub settle_max_m: f64,
}

/// A winged aircraft's flight, which never stops: its attack and dogfight
/// bands, afterburner ramp, and the margin it turns inside the map's edge.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedWingFlight {
    pub strafe_ceiling_m: f64,
    pub bomb_ceiling_m: f64,
    pub dogfight_band_agl_m: [f64; 2],
    pub dogfight_break_m: f64,
    pub afterburner_ramp_s: f64,
    /// A ground attack fired on afterburner scatters this many times its
    /// weapon's authored dispersion: a fast pass is hard to stabilise.
    pub afterburner_scatter_multiplier: f64,
    pub edge_margin_m: f64,
    /// The cone, either side of a winged aircraft's heading, it counts as
    /// nose-on: where a row it aims by the nose fires, and what an attack
    /// pass needs before its dive begins.
    pub nose_on_deg: f64,
    /// Seconds a winged aircraft takes to roll from wings level into a turn
    /// at its `turn_deg_s`, or out of it: its turn rate never steps, and
    /// changes no faster on the way to its `max_turn_deg_s`.
    pub roll_in_s: f64,
}

impl AirRules {
    pub(super) fn check(&self) -> Result<(), String> {
        let low = &self.low_air;
        let high = &self.high_air;
        let rotor = &self.rotor;
        let wing = &self.fixed_wing;
        let rotor_ok = low.cruise_agl_m > 0.0
            && rotor.clearance_m > 0.0
            && rotor.obstacle_m + rotor.clearance_m <= rotor.ceiling_agl_m
            && low.cruise_agl_m <= rotor.ceiling_agl_m
            && rotor.acceleration_s > 0.0
            && (0.0..1.0).contains(&rotor.settle_share)
            && rotor.settle_max_m >= 0.0;
        let wing_ok = high.floor_agl_m > 0.0
            && high.floor_agl_m <= wing.strafe_ceiling_m
            && high.floor_agl_m <= wing.bomb_ceiling_m
            && wing.strafe_ceiling_m <= high.cruise_agl_m
            && wing.bomb_ceiling_m <= high.cruise_agl_m
            && wing.dogfight_band_agl_m[0] >= high.floor_agl_m
            && wing.dogfight_band_agl_m[0] <= wing.dogfight_band_agl_m[1]
            && wing.dogfight_break_m > 0.0
            && wing.afterburner_ramp_s > 0.0
            && wing.afterburner_scatter_multiplier >= 1.0
            && wing.edge_margin_m >= 0.0
            && wing.nose_on_deg > 0.0
            && wing.nose_on_deg < 180.0
            && wing.roll_in_s > 0.0;
        let ok = rotor_ok && wing_ok && self.separation_m > 0.0;
        if ok {
            Ok(())
        } else {
            Err(
                "air: low-air cruise must be positive and within the rotor ceiling; the high-air floor must not exceed the fixed-wing ceilings, those ceilings must not exceed high-air cruise, the dogfight band must be ordered from the floor up, and air timings/margins must be positive, afterburner scatter at least 1, and the nose-on cone within (0, 180) degrees"
                    .into(),
            )
        }
    }

    /// How high `mobility` cruises over the ground: its own override, else
    /// its layer's height, never under its layer's floor (D40). The one
    /// height lookup shared by simulation and presentation readers.
    pub fn cruise_agl_m(&self, mobility: &crate::catalog::Mobility) -> f64 {
        let layer = mobility.layer();
        let authored = match (mobility, layer) {
            (_, crate::catalog::AltitudeLayer::Ground) => return 0.0,
            (
                crate::catalog::Mobility::Air {
                    cruise_agl_m: Some(own),
                    ..
                },
                _,
            ) => *own,
            (_, crate::catalog::AltitudeLayer::LowAir) => self.low_air.cruise_agl_m,
            (_, crate::catalog::AltitudeLayer::HighAir) => self.high_air.cruise_agl_m,
        };
        authored.max(self.floor_agl_m(layer).unwrap_or(0.0))
    }

    /// The hard floor nothing in `layer` flies below (D20); None for a layer
    /// that has none, where no fixed-wing flight is admitted.
    pub fn floor_agl_m(&self, layer: crate::catalog::AltitudeLayer) -> Option<f64> {
        match layer {
            crate::catalog::AltitudeLayer::HighAir => Some(self.high_air.floor_agl_m),
            _ => None,
        }
    }
}
