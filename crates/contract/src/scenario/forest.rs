//! Authored forest rules and physical floor-body admission.
use serde::{Deserialize, Serialize};

/// The fixture's `forests` section: one rule every forest plays by (Q-G8b),
/// so a forest's art can never imply a different sight or movement rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForestRules {
    /// The prop type a forest's trees are (a `props` catalog entry).
    pub tree: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boulder: Option<String>,
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
    /// Zero when absent: frozen maps retain their original body-free floor.
    #[serde(default)]
    pub logs_per_ha: f64,
    #[serde(default)]
    pub boulders_per_ha: f64,
    #[serde(default)]
    pub log_half_extents_m: [f64; 3],
    #[serde(default)]
    pub boulder_half_extents_m: [f64; 3],
}

impl ForestRules {
    /// Physical floor bodies must be ordinary catalog props the world can stand.
    pub fn check(&self, catalog: &crate::catalog::Catalog) -> Result<(), String> {
        if !self.rule.concealment.is_finite()
            || self.rule.concealment <= 0.0
            || self.rule.concealment > 1.0
        {
            return Err("concealment must lie in (0, 1]".into());
        }
        for (name, kind, density, half) in [
            (
                "log",
                &self.log,
                self.rule.logs_per_ha,
                self.rule.log_half_extents_m,
            ),
            (
                "boulder",
                &self.boulder,
                self.rule.boulders_per_ha,
                self.rule.boulder_half_extents_m,
            ),
        ] {
            // One candidate per 100 square metres at most: a body floor,
            // not another dense vegetation population at map scale.
            if !density.is_finite() || !(0.0..=100.0).contains(&density) {
                return Err(format!(
                    "{name} density must be finite and in [0, 100] per hectare"
                ));
            }
            if density == 0.0 {
                continue;
            }
            if !(10_000.0 / density).sqrt().is_finite() {
                return Err(format!("{name} spacing must be finite"));
            }
            if half.iter().any(|v| !v.is_finite() || *v <= 0.0) {
                return Err(format!("{name} half extents must be finite and positive"));
            }
            let Some(kind) = kind.as_ref().and_then(|id| catalog.props().index(id)) else {
                return Err(format!(
                    "{name} needs a catalog prop type when its density is positive"
                ));
            };
            catalog
                .props()
                .check_placement(kind, crate::catalog::PropPlacement::Ordinary)
                .map_err(|error| format!("{name} floor body: {error}"))?;
            let body = catalog.props().get(kind).body;
            if body.topples || body.conceals != 0.0 {
                return Err(format!(
                    "{name} floor body cannot topple or conceal foliage"
                ));
            }
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
        let mut kinds = vec![("tree", Some(self.tree.as_str()))];
        if self.rule.logs_per_ha > 0.0 {
            kinds.push(("log", self.log.as_deref()));
        }
        if self.rule.boulders_per_ha > 0.0 {
            kinds.push(("boulder", self.boulder.as_deref()));
        }
        for (field, kind) in kinds {
            let checked = kind
                .ok_or_else(|| "no prop type is named".to_owned())
                .and_then(|id| {
                    catalog
                        .props()
                        .check_placement_of(id, crate::catalog::PropPlacement::Ordinary)
                });
            checked.map_err(|e| super::RulesError::new(format!("forests.{field}"), e))?;
        }
        Ok(())
    }
}
