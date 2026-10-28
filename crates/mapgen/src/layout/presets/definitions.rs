//! The presets document as a whole: its sections, reading it, and what a
//! request reads of it.
use super::*;

#[path = "swamps.rs"]
mod swamps;
pub use swamps::Swamps;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetDefinitions {
    /// A request pins this, so an edited file cannot answer an old request.
    pub revision: String,
    pub joints: JointPolicy,
    pub terrain: Terrain,
    pub approach: Approach,
    pub skirmish: SkirmishPreset,
    pub fairness: Fairness,
    pub transit: Transit,
    pub roads: Roads,
    pub sites: Sites,
    pub towns: Towns,
    pub forests: Forests,
    pub rivers: Rivers,
    pub swamps: Swamps,
    pub retries: Retries,
    /// District id to what stands there and how its ground is cut. A town
    /// is a mosaic of single-use districts, not a blend on every block.
    pub districts: BTreeMap<String, DistrictPreset>,
    pub parcels: Parcels,
    /// What stands in the streets, yards and open parcels of a built town.
    pub street_props: StreetProps,
    pub classes: BTreeMap<String, SettlementClass>,
    pub types: BTreeMap<MapType, TypePreset>,
    /// What stands between the settlements and the woods (M24).
    pub open_country: crate::open_country::Rules,
}

impl PresetDefinitions {
    /// The presets as `request` reads them: a skirmish request's own
    /// approach, transit, river margin, hub and settlement rows in place of
    /// the standard ones; a standard request's as written.
    pub fn for_request(&self, request: &contract::generation::GenerationRequest) -> Self {
        let mut resolved = self.clone();
        if request.profile == contract::generation::GenerationProfile::Skirmish {
            resolved.approach = self.skirmish.approach.clone();
            resolved
                .approach
                .depth_m
                .values_mut()
                .for_each(|v| *v = self.skirmish.approach.depth_m[&request.size]);
            resolved.transit.max_s = resolved.transit.allowance_s
                + request.extent_m() * self.skirmish.main_road_length_factor
                    / resolved.transit.road_mps();
            resolved.transit.exit_window = self.skirmish.exit_window;
            resolved.rivers.side_margin_m = self.skirmish.river_side_margin_m;
            resolved.roads.hub_offset_m = self.skirmish.hub_offset_m;
            for (kind, preset) in &mut resolved.types {
                preset.gap_m = self.skirmish.settlement_gap_m;
                preset.sizes = self.skirmish.types[kind].clone();
            }
        }
        resolved
    }

    pub fn from_json(source: &str) -> Result<Self, Vec<Diagnostic>> {
        let presets: Self = serde_json::from_str(source)
            .map_err(|error| vec![refusal("$.presets".into(), error.to_string())])?;
        let mut errors = presets.errors();
        errors.extend(presets.swamps.errors(
            presets.terrain.height_grid_m,
            presets.terrain.slope_cutoff_deg,
            presets.rivers.bank_m(),
        ));
        if errors.is_empty() {
            Ok(presets)
        } else {
            Err(errors)
        }
    }

    /// The streets the layout lays between the blocks of a settlement whose
    /// own road is `road`: their surface and width. Lanes where that road
    /// is a dirt track, avenues elsewhere.
    pub fn avenue(&self, road: SurfaceKind) -> (SurfaceKind, f64) {
        match road {
            SurfaceKind::DirtTrack => (SurfaceKind::DirtTrack, self.roads.dirt_track_width_m),
            _ => (SurfaceKind::Road, self.towns.avenue_width_m),
        }
    }

    /// From a carriageway's middle to the front of the parcels along it,
    /// at most: half the widest carriageway and the verge.
    pub fn setback_m(&self) -> f64 {
        self.towns
            .avenue_width_m
            .max(self.roads.country_road_width_m)
            / 2.0
            + self.parcels.verge_m
    }

    /// The smallest wood the layout stands: anything smaller is a copse or
    /// a tree of the open country.
    pub fn wood_floor_m2(&self) -> f64 {
        core::f64::consts::PI * self.forests.min_radius_m * self.forests.min_radius_m
    }

    pub fn class(&self, id: &str) -> &SettlementClass {
        &self.classes[id]
    }

    /// The cell a request selects; `from_json` proved every cell exists.
    pub fn cell(&self, map_type: MapType, size: MapSize) -> (&TypePreset, &SizePreset) {
        let preset = &self.types[&map_type];
        (preset, &preset.sizes[&size])
    }
}
