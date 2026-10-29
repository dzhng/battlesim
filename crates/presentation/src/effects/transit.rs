//! The short off-map tails around a fixed-wing aircraft's edge event.
//!
//! The simulation never owns an off-map position.  It publishes the edge
//! point and tick, and presentation reconstructs the part of the flight that
//! a player should see before arrival or after departure.

use std::sync::Arc;

use sim::publication::reader::AirTransit;

use crate::world::Vec3;

/// Presentation timing for an edge transit, in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
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
}

/// Edge transits as presentation draws them. An inbound approach is
/// published every tick while its reservation waits, so each publication
/// replaces the approaches; a departure is published for one tick, so it is
/// retained until its visual window ends.
#[derive(Default)]
pub struct TransitHistory {
    seen: Vec<AirTransit>,
}

impl TransitHistory {
    /// Take a publication's approaches as the current ones and retain its
    /// newly observed departures, keyed by their kind, edge tick and edge
    /// point.
    pub fn note(&mut self, transits: &[AirTransit], now: f64, tick_hz: f64, feel: &TransitFeel) {
        self.prune(now, tick_hz, feel);
        self.seen.retain(|seen| seen.dir.as_ref() != "in");
        for transit in transits {
            if !self.seen.iter().any(|seen| same_key(seen, transit)) {
                self.seen.push(transit.clone());
            }
        }
    }

    /// Reconstruct all retained tracks at presentation time and discard those
    /// whose visual windows have ended.
    pub fn tracks(&mut self, now: f64, tick_hz: f64, feel: &TransitFeel) -> Vec<TransitPose> {
        self.prune(now, tick_hz, feel);
        tracks(&self.seen, now, tick_hz, feel)
    }

    pub fn clear(&mut self) {
        self.seen.clear();
    }

    fn prune(&mut self, now: f64, tick_hz: f64, feel: &TransitFeel) {
        self.seen.retain(|transit| match transit.dir.as_ref() {
            "in" => now <= transit.edge_tick,
            "out" => {
                let duration_ticks = feel.outbound_s * tick_hz;
                duration_ticks.is_finite()
                    && duration_ticks > 0.0
                    && now < transit.edge_tick + duration_ticks
            }
            _ => false,
        });
    }
}

fn same_key(a: &AirTransit, b: &AirTransit) -> bool {
    a.kind == b.kind && a.dir == b.dir && a.edge_tick == b.edge_tick && a.point == b.point
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
        });
    }
    out
}
