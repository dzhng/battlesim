//! Physical inputs needed to construct a generated battlefield. The battle's
//! existing rules supply them; generation never owns a second settings file.
use crate::catalog::{AltitudeLayer, Catalog};
use crate::scenario::{BodyRules, ForestRules};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct GenerationPhysics {
    pub catalog: Catalog,
    pub forests: ForestRules,
    pub infantry_eye_m: f64,
    pub fog_target_height_m: f64,
}

#[derive(Deserialize)]
struct RuleFields {
    catalog: Catalog,
    forests: ForestRules,
    physics: BodyRules,
    sensors: GenerationSensors,
}

/// The one sensor field generation reads: the rest of the section (how
/// units see) does not shape a battlefield.
#[derive(Deserialize)]
struct GenerationSensors {
    fog_target_height_m: f64,
}

impl GenerationPhysics {
    /// Extract only contract-owned physical fields from the explicit rules
    /// record. The whole resolved Catalog is pinned; top-level combat/balance
    /// sections outside these fields do not enter generation identity.
    pub fn from_rules_json(text: &str) -> Result<Self, String> {
        let fields: RuleFields = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let physics = Self {
            catalog: fields.catalog,
            forests: fields.forests,
            infantry_eye_m: fields.physics.infantry_eye_m,
            fog_target_height_m: fields.sensors.fog_target_height_m,
        };
        physics.forests.check(&physics.catalog)?;
        let rule = physics.forests.rule;
        if !(physics.infantry_eye_m.is_finite()
            && physics.infantry_eye_m > 0.
            && physics.fog_target_height_m.is_finite()
            && physics.fog_target_height_m >= 0.
            && rule.trunk_spacing_m.is_finite()
            && rule.trunk_spacing_m > 0.
            && rule.trunk_jitter.is_finite()
            && rule.trunk_jitter >= 0.
            && rule.trunk_clearance_m.is_finite()
            && rule.trunk_clearance_m >= 0.
            && rule.trunk_height_m.is_finite()
            && rule.trunk_height_m > 0.
            && rule.trunk_radius_m.is_finite()
            && rule.trunk_radius_m > 0.
            && rule.canopy_radius_m.is_finite()
            && rule.canopy_radius_m > 0.
            && rule.canopy_height_m.is_finite()
            && rule.canopy_height_m > 0.
            && rule.attenuation_per_m.is_finite()
            && rule.attenuation_per_m > 0.)
        {
            return Err("generation needs finite positive ground eyes and forest geometry, nonnegative clearances, and finite nonnegative jitter".into());
        }
        let tree = physics
            .catalog
            .props()
            .index(&physics.forests.tree)
            .ok_or("forest tree is not in the physical catalog")?;
        physics
            .catalog
            .props()
            .check_placement(tree, crate::catalog::PropPlacement::Ordinary)?;
        physics.circular_range_m()?;
        Ok(physics)
    }

    /// The smallest circular ground observer. Narrow directional lobes are
    /// diagnostics, not an invented every-heading circle requirement.
    pub fn circular_range_m(&self) -> Result<f64, String> {
        self.circular_ground_observers()
            .map(|unit| unit.sensors.ground_m * unit.sensors.sight_shape.front)
            .reduce(f64::min)
            .ok_or_else(|| "no circular ground observer in the physical catalog".into())
            .and_then(|range| {
                if range.is_finite() && range > 0. {
                    Ok(range)
                } else {
                    Err("circular ground observer range must be finite and positive".into())
                }
            })
    }

    /// Ground observers with an actual circular sight shape. Aircraft never
    /// participate in ground furnishing coverage.
    pub fn circular_ground_observers(&self) -> impl Iterator<Item = &crate::catalog::UnitType> {
        self.catalog
            .indices()
            .map(|i| self.catalog.get(i))
            .filter(|unit| {
                let ground = unit.mobility.layer() == AltitudeLayer::Ground;
                let shape = unit.sensors.sight_shape;
                ground && shape.front == shape.side && shape.side == shape.rear
            })
    }

    pub fn hash(&self) -> Result<String, serde_json::Error> {
        crate::identity::json_hash(self)
    }
}
