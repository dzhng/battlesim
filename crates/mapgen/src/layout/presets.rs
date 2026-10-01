//! Versioned layout presets: every number the generator tunes, read from data
//! and refused at load when it cannot describe a map.
use super::{MapSize, MapType};
use crate::{Diagnostic, DiagnosticCode};
use contract::map::SurfaceKind;
use contract::templates::BuildingCategory;
use serde::Deserialize;
use std::collections::BTreeMap;

/// An inclusive `[low, high]` range a seed draws from.
pub type Range = [f64; 2];
/// What a district is built from: building categories by weight, the
/// dominant one first.
pub type Mix = Vec<(BuildingCategory, f64)>;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetDefinitions {
    /// A request pins this, so an edited file cannot answer an old request.
    pub revision: String,
    pub terrain: Terrain,
    pub approach: Approach,
    pub fairness: Fairness,
    pub transit: Transit,
    pub roads: Roads,
    pub sites: Sites,
    pub forests: Forests,
    pub retries: Retries,
    /// District id to what it is built from: one dominant building category
    /// and at most one minor one, the dominant first. A town is a mosaic of
    /// single-use districts, not a blend on every block.
    #[serde(deserialize_with = "district_mix")]
    pub districts: BTreeMap<String, Mix>,
    pub classes: BTreeMap<String, SettlementClass>,
    pub types: BTreeMap<MapType, TypePreset>,
}

/// The plan header the compiler and the battle read; flat ground has no more.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    pub fog_cell_m: f64,
    pub height_grid_m: f64,
    pub slope_cutoff_deg: f64,
}

/// The one open-approach rule (M19): the main settlement has, in each half,
/// ground open for `depth_m` beyond its edge across a front of `front_m`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approach {
    pub depth_m: f64,
    pub front_m: f64,
    /// What the generator keeps clear so the measured wedge meets the rule.
    pub reserve_front_m: f64,
    pub reserve_margin_m: f64,
    /// Bearings tried in each half for the reserved wedge.
    pub bearing_candidates: u32,
}

/// `|top − bottom| ≤ max(rel × (top + bottom), abs × playable area)`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    pub rel: f64,
    pub abs: f64,
}

impl Tolerance {
    pub fn allows(&self, top: f64, bottom: f64, playable_m2: f64) -> bool {
        (top - bottom).abs() <= self.allowance(top, bottom, playable_m2)
    }
    pub fn allowance(&self, top: f64, bottom: f64, playable_m2: f64) -> f64 {
        (self.rel * (top + bottom)).max(self.abs * playable_m2)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fairness {
    pub town: Tolerance,
    pub forest: Tolerance,
}

/// A light vehicle's road journey from the middle of each edge to the centre.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transit {
    pub road_kmh: f64,
    /// Planning, turns and slower stretches, added to the driving time.
    pub allowance_s: f64,
    pub max_s: f64,
    /// The share of an edge, centred on its midpoint, a main road may leave by.
    pub exit_window: f64,
}

impl Transit {
    pub fn road_mps(&self) -> f64 {
        self.road_kmh / 3.6
    }
    /// The longest road journey that still arrives in time.
    pub fn budget_m(&self) -> f64 {
        (self.max_s - self.allowance_s) * self.road_mps()
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roads {
    pub country_road_width_m: f64,
    pub dirt_track_width_m: f64,
    /// A track's speed against a road's, for timing a journey that uses one.
    pub dirt_track_speed_factor: f64,
    /// Distance between a road's authored points.
    pub bend_step_m: f64,
    /// How far a road may swing off its straight line, as a share of its length.
    pub bend_amplitude: f64,
    pub bend_max_m: f64,
    /// How far the main junction may sit from the exact centre.
    pub hub_jitter_m: f64,
    /// An edge road may join an earlier one this far, by road, from the main
    /// junction,
    pub junction_reach_m: f64,
    /// and does so this often when the journey time allows it.
    pub junction_chance: f64,
    /// An edge road swings through a settlement this near its line.
    pub waypoint_reach_m: f64,
    /// How far past its centre, as a share of the way to its edge, the road
    /// that joins a settlement carries on as its main street.
    pub main_street_reach: f64,
    /// Longest link between neighbouring settlements.
    pub link_max_m: f64,
    /// Most extra roads in from the edges, on the richest network.
    pub corridors_max: u32,
}

impl Roads {
    pub fn width_m(&self, kind: SurfaceKind) -> f64 {
        match kind {
            SurfaceKind::DirtTrack => self.dirt_track_width_m,
            _ => self.country_road_width_m,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sites {
    pub edge_margin_m: f64,
    /// How far from the main settlement's edge a clustered site may lie.
    pub cluster_reach_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forests {
    pub large_radius_m: Range,
    pub small_radius_m: Range,
    /// Chance a wood is large, from the sparsest to the most wooded seed.
    pub large_chance: Range,
    pub min_radius_m: f64,
    pub aspect: Range,
    pub outline_noise: f64,
    pub outline_points: u32,
    pub settlement_gap_m: f64,
    pub forest_gap_m: f64,
    /// How far measured coverage may fall outside a type's `forest_share`.
    pub share_tolerance: f64,
    pub max_woods: u32,
    /// Chance that ground a settlement leaves open between its districts is
    /// wooded rather than field.
    pub infill_chance: f64,
}

/// Bounds on every search; running out is a named refusal, never a new seed.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retries {
    pub centre: u32,
    pub site: u32,
    pub forest: u32,
    pub repair_settlements: u32,
    pub repair_woods: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineShape {
    /// Superellipse exponent: 2 is an ellipse, higher is squarer.
    pub exponent: f64,
    pub noise: f64,
    pub points: u32,
}

/// One settlement size. Every map size draws from the same classes (M05).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementClass {
    /// Larger classes are placed first and take the better roads.
    pub rank: u32,
    /// A fixed size in hectares, or
    pub area_ha: Option<Range>,
    /// a share of the playable area, for the one place that grows with the map.
    pub area_share: Option<Range>,
    /// Short axis over long axis.
    pub aspect: Range,
    /// Bearing of the long axis, radians.
    pub rotation: Range,
    pub outline: OutlineShape,
    /// The road that joins it to the network.
    pub road: SurfaceKind,
    /// Whether open approaches to it are measured.
    pub approach: bool,
    /// Rings of districts from the centre out; the last reaches the edge.
    pub bands: Vec<Band>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// Outer limit as a share of the distance from centre to edge.
    pub to: f64,
    pub sectors: u32,
    /// District id to the weight a sector picks it with.
    pub districts: BTreeMap<String, f64>,
    /// The weights a sector on a main road picks with instead, so that
    /// industry, say, lines the road.
    #[serde(default)]
    pub roadside: Option<BTreeMap<String, f64>>,
    /// Share of the band's sectors left unbuilt: field or wood reaching into
    /// the town. The innermost band is always built.
    #[serde(default)]
    pub open: f64,
    /// A built sector stops at a seed-drawn share of the band's depth, from
    /// this up to 1, so the town's edge is uneven.
    #[serde(default = "full")]
    pub ragged: f64,
}

fn full() -> f64 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypePreset {
    /// The building categories this type may select (M07).
    pub categories: Vec<BuildingCategory>,
    /// Tallest building this type admits; absent means no limit.
    pub max_floors: Option<u32>,
    pub centre: Centre,
    pub siting: Siting,
    /// Least open ground between two settlements.
    pub gap_m: f64,
    pub forest_share: Range,
    pub sizes: BTreeMap<MapSize, SizePreset>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Centre {
    pub class: String,
    /// How far its centre may sit from the map's, as shares of the extent
    /// along X and Y. Y stays small so the two halves share it.
    pub offset: [f64; 2],
}

/// How often a settlement is sited on a main road's line or beside the main
/// settlement; otherwise it lies anywhere there is room.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Siting {
    pub on_road: f64,
    pub near_main: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizePreset {
    /// Ceiling on settlement area over playable area.
    pub urban_share_max: f64,
    /// Class id to the inclusive count range a seed draws from.
    pub settlements: BTreeMap<String, [u32; 2]>,
}

#[derive(Deserialize)]
struct DistrictWire(BTreeMap<String, f64>);

impl PresetDefinitions {
    pub fn from_json(source: &str) -> Result<Self, Vec<Diagnostic>> {
        let presets: Self = serde_json::from_str(source)
            .map_err(|error| vec![refusal("$.presets".into(), error.to_string())])?;
        let errors = presets.errors();
        if errors.is_empty() {
            Ok(presets)
        } else {
            Err(errors)
        }
    }

    fn errors(&self) -> Vec<Diagnostic> {
        let mut errors = Vec::new();
        let mut check = |ok: bool, location: String, message: &str| {
            if !ok {
                errors.push(refusal(format!("$.presets.{location}"), message.into()));
            }
        };
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let share = |v: f64| v.is_finite() && v > 0.0 && v <= 1.0;
        let range = |r: Range| r[0].is_finite() && r[1].is_finite() && r[0] <= r[1];
        let positive_range = |r: Range| range(r) && r[0] > 0.0;

        check(
            contract::identity::validate_version_identifier(&self.revision).is_ok(),
            "revision".into(),
            "a preset revision must be nonempty",
        );
        check(
            contract::map::validate_header(
                [1.0, 1.0],
                self.terrain.fog_cell_m,
                self.terrain.height_grid_m,
                self.terrain.slope_cutoff_deg,
            )
            .is_empty(),
            "terrain".into(),
            "terrain resolution and slope cutoff must be a valid map header",
        );
        let a = &self.approach;
        check(
            positive(a.depth_m)
                && positive(a.front_m)
                && a.reserve_front_m >= a.front_m
                && a.reserve_margin_m >= 0.0
                && a.reserve_front_m.is_finite()
                && a.reserve_margin_m.is_finite()
                && a.bearing_candidates > 0,
            "approach".into(),
            "an approach needs a positive depth and front, and a reserve no smaller",
        );
        for (name, tolerance) in [
            ("town", self.fairness.town),
            ("forest", self.fairness.forest),
        ] {
            check(
                (0.0..=1.0).contains(&tolerance.rel) && (0.0..=1.0).contains(&tolerance.abs),
                format!("fairness.{name}"),
                "a fairness tolerance is a share between 0 and 1",
            );
        }
        let t = &self.transit;
        check(
            positive(t.road_kmh)
                && t.allowance_s >= 0.0
                && t.max_s.is_finite()
                && t.max_s > t.allowance_s
                && share(t.exit_window),
            "transit".into(),
            "transit needs a road speed, a time above its allowance and an exit window",
        );
        let r = &self.roads;
        check(
            positive(r.country_road_width_m)
                && positive(r.dirt_track_width_m)
                && share(r.dirt_track_speed_factor)
                && positive(r.bend_step_m)
                && (0.0..=0.5).contains(&r.bend_amplitude)
                && r.bend_max_m >= 0.0
                && r.bend_max_m.is_finite()
                && r.hub_jitter_m >= 0.0
                && r.hub_jitter_m.is_finite()
                && r.junction_reach_m >= 0.0
                && r.junction_reach_m.is_finite()
                && (0.0..=1.0).contains(&r.junction_chance)
                && r.waypoint_reach_m >= 0.0
                && r.waypoint_reach_m.is_finite()
                && share(r.main_street_reach)
                && positive(r.link_max_m),
            "roads".into(),
            "road widths, bends and junction spreads must be finite and nonnegative",
        );
        check(
            self.sites.edge_margin_m >= 0.0
                && self.sites.edge_margin_m.is_finite()
                && positive(self.sites.cluster_reach_m),
            "sites".into(),
            "sites need a nonnegative edge margin and a cluster reach",
        );
        let f = &self.forests;
        check(
            positive_range(f.large_radius_m)
                && positive_range(f.small_radius_m)
                && range(f.large_chance)
                && f.large_chance[0] >= 0.0
                && f.large_chance[1] <= 1.0
                && positive(f.min_radius_m)
                && f.min_radius_m <= f.small_radius_m[0]
                && positive_range(f.aspect)
                && f.aspect[1] <= 1.0
                && outline_ok(f.outline_noise, f.outline_points)
                && f.settlement_gap_m >= 0.0
                && f.forest_gap_m >= 0.0
                && f.settlement_gap_m.is_finite()
                && f.forest_gap_m.is_finite()
                && (0.0..1.0).contains(&f.share_tolerance)
                && (0.0..=1.0).contains(&f.infill_chance)
                && f.max_woods > 0,
            "forests".into(),
            "forest sizes, outline and gaps must be finite, ordered and positive",
        );
        let n = &self.retries;
        check(
            n.centre > 0 && n.site > 0 && n.forest > 0,
            "retries".into(),
            "every search needs at least one attempt",
        );
        for (id, mix) in &self.districts {
            check(
                (1..=2).contains(&mix.len()) && mix.iter().all(|(_, weight)| positive(*weight)),
                format!("districts.{id}"),
                "a district is one building category, or one with a minor second, by positive weight",
            );
        }
        for (id, class) in &self.classes {
            let at = |field: &str| format!("classes.{id}.{field}");
            let size_ok = match (class.area_ha, class.area_share) {
                (Some(area), None) => positive_range(area),
                (None, Some(area)) => positive_range(area) && area[1] < 1.0,
                _ => false,
            };
            check(
                size_ok,
                at("area_ha"),
                "a class has one positive ordered size: area_ha or area_share",
            );
            check(
                positive_range(class.aspect) && class.aspect[1] <= 1.0 && range(class.rotation),
                at("aspect"),
                "aspect is an ordered range in (0, 1] and rotation an ordered range",
            );
            check(
                class.outline.exponent >= 1.0
                    && class.outline.exponent.is_finite()
                    && outline_ok(class.outline.noise, class.outline.points),
                at("outline"),
                "an outline needs exponent ≥ 1, noise in [0, 0.7] and 8..=256 points",
            );
            check(
                class.road.is_road(),
                at("road"),
                "a settlement joins the network by a carriageway",
            );
            let mut from = 0.0;
            for (index, band) in class.bands.iter().enumerate() {
                let sectors_ok = band.sectors >= if from > 0.0 { 2 } else { 1 }
                    && band.sectors * 2 <= class.outline.points;
                check(
                    band.to > from && band.to <= 1.0 && sectors_ok,
                    at(&format!("bands[{index}]")),
                    "bands grow outward to 1; a ring needs 2 or more sectors, each two outline points wide",
                );
                let picks = |weights: &BTreeMap<String, f64>| {
                    !weights.is_empty()
                        && weights
                            .iter()
                            .all(|(d, w)| self.districts.contains_key(d) && positive(*w))
                };
                check(
                    picks(&band.districts) && band.roadside.as_ref().is_none_or(picks),
                    at(&format!("bands[{index}].districts")),
                    "a band picks known districts with positive weights",
                );
                check(
                    (0.0..1.0).contains(&band.open)
                        && share(band.ragged)
                        && (index > 0 || band.open == 0.0),
                    at(&format!("bands[{index}].open")),
                    "open is a share below 1, zero at the centre, and ragged a share of the band's depth",
                );
                from = band.to;
            }
            check(
                from == 1.0,
                at("bands"),
                "the last band must reach the settlement's edge (to: 1)",
            );
        }
        for map_type in MapType::ALL {
            let name = map_type.name();
            let Some(preset) = self.types.get(&map_type) else {
                check(
                    false,
                    format!("types.{name}"),
                    "every map type needs a preset",
                );
                continue;
            };
            let at = |field: &str| format!("types.{name}.{field}");
            check(
                self.classes.contains_key(&preset.centre.class)
                    && preset.centre.offset.iter().all(|v| (0.0..0.5).contains(v)),
                at("centre"),
                "the centre names a known class and an offset below half the map",
            );
            check(
                preset.siting.on_road >= 0.0
                    && preset.siting.near_main >= 0.0
                    && preset.siting.on_road + preset.siting.near_main <= 1.0,
                at("siting"),
                "siting chances are nonnegative and sum to at most 1",
            );
            check(
                preset.gap_m >= 0.0 && preset.gap_m.is_finite(),
                at("gap_m"),
                "the gap between settlements must be finite and nonnegative",
            );
            check(
                range(preset.forest_share)
                    && preset.forest_share[0] >= 0.0
                    && preset.forest_share[1] < 1.0,
                at("forest_share"),
                "forest share is an ordered range of shares",
            );
            let mut classes: Vec<&String> = vec![&preset.centre.class];
            for size in MapSize::ALL {
                let Some(cell) = preset.sizes.get(&size) else {
                    check(
                        false,
                        at(&format!("sizes.{}", size.name())),
                        "every map size needs a preset",
                    );
                    continue;
                };
                check(
                    share(cell.urban_share_max),
                    at(&format!("sizes.{}.urban_share_max", size.name())),
                    "the urban ceiling is a share of the playable area",
                );
                for (class, count) in &cell.settlements {
                    check(
                        self.classes.contains_key(class) && count[0] <= count[1],
                        at(&format!("sizes.{}.settlements.{class}", size.name())),
                        "settlement counts are ordered ranges of known classes",
                    );
                    classes.push(class);
                }
            }
            // M07 holds by construction: no district this type can reach
            // offers a category the type does not admit.
            for class in classes {
                let districts = self
                    .classes
                    .get(class)
                    .into_iter()
                    .flat_map(|class| &class.bands)
                    .flat_map(|band| {
                        band.districts
                            .keys()
                            .chain(band.roadside.iter().flatten().map(|(d, _)| d))
                    });
                for district in districts {
                    for (category, _) in self.districts.get(district).into_iter().flatten() {
                        if !preset.categories.contains(category) {
                            let category = serde_json::to_string(category).unwrap_or_default();
                            check(
                                false,
                                at("categories"),
                                &format!(
                                    "class {class} reaches district {district}, which builds {category}, a category this type does not admit"
                                ),
                            );
                        }
                    }
                }
            }
        }
        errors
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

fn outline_ok(noise: f64, points: u32) -> bool {
    // Past 0.7 the harmonics could fold an outline through its own centre.
    (0.0..=0.7).contains(&noise)
        && (8..=contract::ground::MAX_POLYGON_VERTICES as u32).contains(&points)
}

fn refusal(location: String, message: String) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::InvalidPresets,
        feature: None,
        location,
        message,
    }
}

/// A district's mix is written as an object and kept dominant first (then by
/// name), so the plan lists categories the same way whatever order the file used.
fn district_mix<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<BTreeMap<String, Mix>, D::Error> {
    let wire = BTreeMap::<String, DistrictWire>::deserialize(decoder)?;
    wire.into_iter()
        .map(|(id, DistrictWire(mix))| {
            let mut out = Vec::new();
            for (name, weight) in mix {
                let category: BuildingCategory =
                    serde_json::from_value(serde_json::Value::String(name))
                        .map_err(serde::de::Error::custom)?;
                out.push((category, weight));
            }
            out.sort_by(|a, b| b.1.total_cmp(&a.1));
            Ok((id, out))
        })
        .collect()
}
