//! Authored trunk geometry and concealment.
use serde::{Deserialize, Serialize};

/// The fixture's `forests` section: one rule every forest plays by (Q-G8b),
/// so a forest's art can never imply a different sight or movement rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForestRules {
    /// The prop type a forest's trees are (a `props` catalog entry).
    pub tree: String,
    pub rule: ForestRule,
}

/// How every forest's trunks stand and how much it hides (Q16, Q21).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForestRule {
    /// Trunk grid spacing; each trunk starts at its cell's centre, the
    /// first `spacing / 2` inside the shape's minimum corner.
    pub trunk_spacing_m: f64,
    /// Each trunk moves up to this fraction of the spacing off its cell's
    /// centre, on each axis (seeded by the forest, so every reader agrees).
    pub trunk_jitter: f64,
    /// Concealment of uncleared forest ground, independent of the target.
    pub concealment: f64,
    /// Foliage depth per metre a sight line crosses below the canopy.
    pub attenuation_per_m: f64,
    /// A trunk's crown: it attenuates sight through fog cells whose centres lie this near.
    pub canopy_radius_m: f64,
    pub canopy_height_m: f64,
    pub trunk_radius_m: f64,
    pub trunk_height_m: f64,
    /// Trunks are omitted within this distance of roads and props.
    pub trunk_clearance_m: f64,
}

impl ForestRules {
    pub fn check(&self) -> Result<(), String> {
        if !self.rule.trunk_spacing_m.is_finite() || self.rule.trunk_spacing_m <= 0.0 {
            return Err("rule.trunk_spacing_m must be finite and positive".into());
        }
        if !self.rule.concealment.is_finite()
            || self.rule.concealment <= 0.0
            || self.rule.concealment > 1.0
        {
            return Err("concealment must lie in (0, 1]".into());
        }
        Ok(())
    }
}

impl ForestRules {
    /// Every prop the forest may place must be an ordinary world body.
    pub fn check_placement(
        &self,
        catalog: &crate::catalog::Catalog,
    ) -> Result<(), super::RulesError> {
        catalog
            .props()
            .check_placement_of(&self.tree, crate::catalog::PropPlacement::Ordinary)
            .map_err(|e| super::RulesError::new("forests.tree", e))
    }
}
