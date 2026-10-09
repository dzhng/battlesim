//! The `open_country` rows of the presets: every density, size and
//! clearance the pass places by, refused at load when it cannot describe a
//! country.
use serde::Deserialize;

/// The `open_country` block of the presets.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// A tree line follows its road by points this far apart.
    pub line_step_m: f64,
    /// Road is measured, and a settlement's edge sampled, this often.
    pub walk_m: f64,
    /// The most things of one kind a map is given, whatever its rows ask.
    pub placed_max: u32,
    pub sight: SightRule,
    pub clear: Clearances,
    pub fairness: Evenness,
    pub homesteads: Homesteads,
    pub tree_lines: TreeLines,
    pub copses: Copses,
    pub lone_trees: LoneTrees,
    pub field_cover: FieldCover,
}

/// Construction sampling and the ordered physical features tried for a
/// missing whole-cell witness. Sight range belongs to resolved physics.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightRule {
    pub cell_m: f64,
    pub fill: Vec<FillKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillKind {
    Copse,
    TreeLine,
}

/// The least open ground kept round what the pass places.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clearances {
    pub edge_m: f64,
    /// From a settlement's outline: nearer is the settlement's own ground.
    pub settlement_m: f64,
    /// Between a wood and a home, a copse or a tree line.
    pub forest_m: f64,
    /// From a carriageway's edge.
    pub road_m: f64,
    pub water_m: f64,
    /// From a yard, and between two copses or tree lines.
    pub yard_m: f64,
    /// Beside a kept approach corridor, for what blocks sight.
    pub approach_m: f64,
}

/// `|top − bottom| ≤ max(rel × (top + bottom), least)`, in the thing's own
/// unit: buildings, metres of tree line, copses, trees, bodies.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evenness {
    pub rel: f64,
    pub homes: f64,
    pub tree_line_m: f64,
    pub copses: f64,
    pub trees: f64,
    pub cover: f64,
}

/// Homes outside the settlements, along the country roads and tracks.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Homesteads {
    /// Groups for each kilometre of country road and track outside the
    /// settlements.
    pub per_road_km: f64,
    /// Two groups stand at least this far apart.
    pub apart_m: f64,
    /// The share of groups that stand back from the road at the end of a
    /// lane, and how long the lane is.
    pub lane_chance: f64,
    pub lane_m: [f64; 2],
    /// Between two homes of a group along their road, at most.
    pub spread_m: f64,
    /// A home's yard: its template's footprint plus these setbacks.
    pub yard: Yard,
    /// What a group is, by weight.
    pub groups: Vec<Group>,
    /// The chance a home has a clump of trees behind its yard, and the
    /// clump's sides.
    pub tree_chance: f64,
    pub tree_plot_m: [f64; 2],
    /// The chance a home has one of `bodies` beside it.
    pub body_chance: f64,
    pub bodies: Vec<Body>,
    /// Places tried for each group.
    pub attempts: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Yard {
    pub front_m: f64,
    pub side_m: f64,
    pub rear_m: f64,
    /// A body in the yard keeps this far from a door and the way out of it.
    pub door_clear_m: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub weight: f64,
    /// How many homes, inclusive.
    pub homes: [u32; 2],
    /// Building categories by weight.
    #[serde(deserialize_with = "crate::layout::mix")]
    pub mix: crate::layout::Mix,
}

/// A body of the prop catalog and the box it is placed as.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub kind: String,
    pub half_extents_m: [f64; 3],
    pub weight: f64,
    /// How many stand together, inclusive; one when absent.
    #[serde(default = "one")]
    pub count: [u32; 2],
    /// Laid side by side on one heading (a stack of logs), not scattered.
    #[serde(default)]
    pub stacked: bool,
}

fn one() -> [u32; 2] {
    [1, 1]
}

/// Tree lines: long thin woods beside a road or across a field, in
/// stretches with gaps a vehicle drives through.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeLines {
    /// Lines for each square kilometre of open ground, at least.
    pub per_km2: f64,
    pub length_m: [f64; 2],
    pub width_m: f64,
    /// A line is broken into stretches this long by gaps this wide.
    pub stretch_m: [f64; 2],
    pub gap_m: f64,
    /// The share of lines that follow a road or a track, and how far from
    /// its edge their trees stand.
    pub roadside_chance: f64,
    pub road_gap_m: f64,
    /// A line in a field runs along or square to a road this near.
    pub align_m: f64,
    pub attempts: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Copses {
    /// Copses for each square kilometre of open ground, at least.
    pub per_km2: f64,
    pub area_m2: [f64; 2],
    pub aspect: [f64; 2],
    pub outline_points: u32,
    pub attempts: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoneTrees {
    pub per_km2: f64,
    /// The side of the plot one trunk stands on.
    pub plot_m: f64,
    pub attempts: u32,
}

/// Low cover in the fields: a few bodies of one kind together.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldCover {
    /// Clusters for each square kilometre of open ground.
    pub per_km2: f64,
    /// Bodies of a cluster stand within this of its middle, and keep this
    /// far from each other.
    pub spread_m: f64,
    pub gap_m: f64,
    pub bodies: Vec<Body>,
    pub attempts: u32,
}

impl Rules {
    /// What the rows cannot describe, as (field, message).
    pub fn errors(&self, wood_floor_m2: f64) -> Vec<(String, &'static str)> {
        let mut errors = Vec::new();
        if !self.line_step_m.is_finite() || self.line_step_m <= 0.0 {
            errors.push((
                "line_step_m".into(),
                "Tree-line spacing must be finite and positive",
            ));
        }
        if !self.walk_m.is_finite() || self.walk_m <= 0.0 {
            errors.push((
                "walk_m".into(),
                "Country search spacing must be finite and positive",
            ));
        }
        if self.placed_max == 0 || self.placed_max > 4096 {
            errors.push((
                "placed_max".into(),
                "Country placement allowance must be positive and no larger than 4096",
            ));
        }

        let mut check = |ok: bool, field: &str, message: &'static str| {
            if !ok {
                errors.push((field.to_string(), message));
            }
        };
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let length = |v: f64| v.is_finite() && v >= 0.0;
        let chance = |v: f64| (0.0..=1.0).contains(&v);
        let range = |r: [f64; 2]| positive(r[0]) && r[1].is_finite() && r[0] <= r[1];
        let bodies = |rows: &[Body]| {
            rows.iter().all(|row| {
                !row.kind.is_empty()
                    && row.half_extents_m.iter().all(|v| positive(*v))
                    && positive(row.weight)
                    && row.count[0] >= 1
                    && row.count[0] <= row.count[1]
            })
        };
        let s = &self.sight;
        check(
            positive(s.cell_m)
                && !s.fill.is_empty()
                && s.fill
                    .iter()
                    .enumerate()
                    .all(|(i, k)| !s.fill[..i].contains(k)),
            "sight",
            "sight needs a positive cell and distinct ordered fill kinds",
        );
        let c = &self.clear;
        check(
            [
                c.edge_m,
                c.settlement_m,
                c.forest_m,
                c.road_m,
                c.water_m,
                c.yard_m,
                c.approach_m,
            ]
            .iter()
            .all(|v| length(*v)),
            "clear",
            "clearances are nonnegative metres",
        );
        let e = &self.fairness;
        check(
            chance(e.rel)
                && [e.homes, e.tree_line_m, e.copses, e.trees, e.cover]
                    .iter()
                    .all(|v| length(*v)),
            "fairness",
            "evenness is a share and nonnegative allowances",
        );
        let h = &self.homesteads;
        check(
            length(h.per_road_km)
                && length(h.apart_m)
                && chance(h.lane_chance)
                && range(h.lane_m)
                && length(h.spread_m)
                && [
                    h.yard.front_m,
                    h.yard.side_m,
                    h.yard.rear_m,
                    h.yard.door_clear_m,
                ]
                .iter()
                .all(|v| length(*v))
                && chance(h.tree_chance)
                && range(h.tree_plot_m)
                && chance(h.body_chance)
                && bodies(&h.bodies)
                && h.attempts > 0,
            "homesteads",
            "homesteads need a density, a lane, a yard, chances and bodies with positive boxes",
        );
        check(
            !h.groups.is_empty()
                && h.groups.iter().all(|group| {
                    positive(group.weight)
                        && group.homes[0] >= 1
                        && group.homes[0] <= group.homes[1]
                        && !group.mix.is_empty()
                        && group.mix.iter().all(|(_, weight)| positive(*weight))
                }),
            "homesteads.groups",
            "a group is one or more homes of known building categories, by positive weight",
        );
        let t = &self.tree_lines;
        check(
            length(t.per_km2)
                && range(t.length_m)
                && positive(t.width_m)
                && range(t.stretch_m)
                && t.stretch_m[0] >= t.width_m
                && positive(t.gap_m)
                && chance(t.roadside_chance)
                && length(t.road_gap_m)
                && length(t.align_m)
                && t.attempts > 0,
            "tree_lines",
            "tree lines need ordered lengths, a width, stretches no shorter than it and a gap",
        );
        let k = &self.copses;
        check(
            length(k.per_km2)
                && range(k.area_m2)
                // Smaller than any wood, so the two are told apart by size.
                && k.area_m2[1] < wood_floor_m2
                && range(k.aspect)
                && k.aspect[1] <= 1.0
                && (8..=contract::ground::MAX_POLYGON_VERTICES as u32).contains(&k.outline_points)
                && k.attempts > 0,
            "copses",
            "a copse is smaller than the smallest wood, with an ordered aspect and 8 or more points",
        );
        let l = &self.lone_trees;
        check(
            length(l.per_km2)
                && positive(l.plot_m)
                // Smaller than any copse, so a tree is told from one by size.
                && h.tree_plot_m[1] * h.tree_plot_m[1] < k.area_m2[0]
                && l.plot_m * l.plot_m < k.area_m2[0]
                && l.attempts > 0,
            "lone_trees",
            "a tree's plot, and a yard's clump, is smaller than the smallest copse",
        );
        let f = &self.field_cover;
        check(
            length(f.per_km2)
                && length(f.spread_m)
                && length(f.gap_m)
                && !f.bodies.is_empty()
                && bodies(&f.bodies)
                && f.attempts > 0,
            "field_cover",
            "field cover needs a density, a spread and bodies with positive boxes",
        );
        errors
    }
}
