//! Optical sensing rule admission.
use serde::{Deserialize, Serialize};

/// Optical sensing and concealment (the fixture's `sensors` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorRules {
    /// Minimum fraction of living soldiers one observer must see to identify a squad.
    pub squad_identification_fraction: f64,
    /// Fraction of a unit's visibility samples (living soldiers, or a hull's
    /// two) that must be concealed for HIDDEN.
    pub squad_hidden_fraction: f64,
    /// A ground ray whose foliage depth (the sum of each crossed metre's
    /// `attenuation_per_m` below the canopy) reaches this is blocked
    /// outright; below it, reach is `range · exp(−depth)`.
    pub foliage_full_block: f64,
    /// Detection-range multiplier for infantry garrisoned in a building at
    /// full building strength (the strongest concealment source wins).
    pub building_range_multiplier: f64,
    /// Movement scales target concealment while its move order is in progress.
    pub moving_concealment: f64,
    /// Seconds an acquisition survives lost identification (V12).
    pub acquisition_grace_s: f64,
    /// A contact's area is this many times the footprint radius of the unit
    /// that caused it (its catalog type's: a hull's half-diagonal, or half a
    /// full squad's spread plus a soldier's body). So the area's size tells
    /// a vehicle's from a squad's, never which unit it is or where exactly.
    pub contact_radius_factor: f64,
    pub contact_lifetime_s: f64,
    /// Units carry their own loudness (their type's `sound`); shots carry
    /// this far.
    pub hearing_shot_m: f64,
    pub sound_bucket_s: f64,
    /// Height above ground tested for ground visibility.
    pub fog_target_height_m: f64,
    /// A line of sight squeezing between bodies that hide what lies behind
    /// them, one either side, needs at least this much room on its two sides
    /// together: no one is seen through the crack between two houses, nor the
    /// sliver between a near one and a far one (`WorldGeometry::squeezed`).
    pub min_sight_gap_m: f64,
    /// How coarsely a radar receiver reports what it finds.
    pub radar_report: RadarReport,
}

/// A radar receiver's report of an emitter: its bearing and range rounded
/// to these steps, and an area at least this wide around it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarReport {
    pub bearing_step_deg: f64,
    pub range_step_m: f64,
    pub min_radius_m: f64,
}

impl SensorRules {
    pub(super) fn check(&self) -> Result<(), String> {
        if !(self.squad_identification_fraction.is_finite()
            && self.squad_identification_fraction > 0.0
            && self.squad_identification_fraction <= 1.0)
        {
            return Err("squad_identification_fraction must lie in (0, 1]".into());
        }
        if !(0.0..1.0).contains(&self.squad_hidden_fraction) {
            return Err("squad_hidden_fraction must lie in [0, 1)".into());
        }
        if !self.foliage_full_block.is_finite() || self.foliage_full_block <= 0.0 {
            return Err("foliage_full_block must be finite and positive".into());
        }
        if !(self.building_range_multiplier > 0.0 && self.building_range_multiplier <= 1.0) {
            return Err("building_range_multiplier must lie in (0, 1]".into());
        }
        if !self.moving_concealment.is_finite() || self.moving_concealment < 1.0 {
            return Err("moving_concealment must be finite and at least 1".into());
        }
        let r = self.radar_report;
        if [r.bearing_step_deg, r.range_step_m, r.min_radius_m]
            .iter()
            .any(|m| !(m.is_finite() && *m > 0.0))
        {
            return Err(
                "radar_report: every step and the radius must be finite and positive".into(),
            );
        }
        Ok(())
    }
}
