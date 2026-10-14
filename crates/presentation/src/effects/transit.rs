//! The short off-map tails around a fixed-wing aircraft's edge event.
//!
//! The simulation never owns an off-map position.  It publishes the edge
//! point and tick, and presentation reconstructs the part of the flight that
//! a player should see before arrival or after departure.

use std::sync::Arc;

use sim::publication::reader::AirTransit;

use crate::world::Vec3;

/// Presentation timing for an edge transit, in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
pub struct TransitFeel {
    pub inbound_s: f64,
    pub outbound_s: f64,
}

/// One aircraft pose reconstructed from an edge transit publication.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitPose {
    pub kind: Arc<str>,
    pub position: Vec3,
    pub heading: f64,
    pub outbound: bool,
}

/// Reconstruct the visible approach or departure at presentation time `now`
/// (simulation ticks), using `tick_hz` to turn travel into seconds.
pub fn tracks(
    transits: &[AirTransit],
    now: f64,
    tick_hz: f64,
    feel: &TransitFeel,
) -> Vec<TransitPose> {
    if !tick_hz.is_finite() || tick_hz <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for transit in transits {
        let outbound = match transit.dir.as_ref() {
            "in" => false,
            "out" => true,
            _ => continue,
        };
        let duration_s = if outbound {
            feel.outbound_s
        } else {
            feel.inbound_s
        };
        if !duration_s.is_finite() || duration_s <= 0.0 {
            continue;
        }
        let elapsed_ticks = now - transit.edge_tick;
        let duration_ticks = duration_s * tick_hz;
        if outbound {
            if !(0.0..duration_ticks).contains(&elapsed_ticks) {
                continue;
            }
        } else if !(-duration_ticks..=0.0).contains(&elapsed_ticks) {
            continue;
        }
        let (distance, forward) = if outbound {
            (transit.speed_mps * elapsed_ticks / tick_hz, 1.0)
        } else {
            (transit.speed_mps * -elapsed_ticks / tick_hz, -1.0)
        };
        let (sin, cos) = libm::sincos(transit.heading);
        out.push(TransitPose {
            kind: transit.kind.clone(),
            position: [
                transit.point[0] + forward * cos * distance,
                transit.point[1] + forward * sin * distance,
                transit.point[2],
            ],
            heading: transit.heading,
            outbound,
        });
    }
    out
}
