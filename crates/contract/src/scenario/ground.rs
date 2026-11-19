//! Ground-layer rules for craters, scorch, tracks and trampling.

use serde::{Deserialize, Serialize};

/// The ground layer (D2, Q8): per-cell craters, which are ground cover for
/// infantry (`CoverRules::crater`) and slow vehicles slightly, and cosmetic scorch, track wear and
/// trampling. Channels are bytes in [0, 255] that accumulate and saturate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundRules {
    /// Cell edge length.
    pub cell_m: f64,
    /// A weapon's crater radius as a fraction of its blast radius.
    pub crater_radius_fraction: f64,
    /// Crater depth a burst adds at its centre, per metre of crater radius,
    /// falling linearly to nothing at the rim.
    pub crater_depth_per_m: f64,
    /// Depth at which a crater gives its full cover and slowdown (saturation).
    pub crater_full_depth: f64,
    /// A track's wheel line: this share of the hull's half width off its
    /// axis, where tracks wear the ground.
    pub track_gauge: f64,
    /// A vehicle knocking through trees clears this much beyond its hull on
    /// either side (Q16).
    pub lane_margin_m: f64,
    /// A weapon's scorch radius as a fraction of its blast radius.
    pub scorch_radius_fraction: f64,
    /// Scorch a burst adds at its centre, falling linearly to the rim.
    pub scorch_per_burst: f64,
    /// A burst of at least this blast radius marks the ground fully; a
    /// smaller one adds scorch in proportion to its radius's share of this,
    /// and crater depth by the square of it, so a small cannon shell leaves
    /// a dark pock where a shell leaves a crater and a black scorch.
    pub mark_reference_blast_m: f64,
    /// Track wear a vehicle track adds to each cell it enters.
    pub tracks_per_pass: f64,
    /// Trampling a soldier adds to each cell it enters.
    pub trampled_per_pass: f64,
}
