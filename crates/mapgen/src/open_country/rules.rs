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
    #[serde(deserialize_with = "homesteads")]
    pub homesteads: Steads<Homes>,
    /// Military depots: rarer than homes, a few prop bodies each.
    #[serde(deserialize_with = "depots")]
    pub depots: Steads<Vec<BodyGroup>>,
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
    pub depots: f64,
}

/// Groups of pieces outside the settlements, each at a station on a country
/// road or track: how many a map holds, and how each one stands. `P` is what
/// a group's yards hold: `Homes`, or a depot's body groups.
#[derive(Clone, Debug)]
pub struct Steads<P> {
    /// Groups for each kilometre of country road and track outside the
    /// settlements.
    pub per_road_km: f64,
    /// Places tried for each group.
    pub attempts: u32,
    pub steading: Steading<P>,
}

/// How one group stands: its station on the road, its yards and its pieces.
#[derive(Clone, Debug)]
pub struct Steading<P> {
    /// Two groups stand at least this far apart.
    pub apart_m: f64,
    /// The share of groups that stand back from the road at the end of a
    /// lane, and how long the lane is.
    pub lane_chance: f64,
    pub lane_m: [f64; 2],
    /// Between two yards of a group along their road, at most.
    pub spread_m: f64,
    /// A piece's yard: its footprint plus these setbacks.
    pub yard: Yard,
    pub pieces: P,
}

/// What a group's yards hold, as the station walk reads it.
#[derive(Clone, Copy, Debug)]
pub enum Pieces<'a> {
    /// A building lot each, furnished with trees and a body.
    Homes(&'a Homes),
    /// A prop body each (or a row of them), sized by its box in
    /// `street_props.bodies`.
    Bodies(&'a [BodyGroup]),
}

/// A steading's pieces, as the station walk reads them.
pub trait Pieced {
    fn pieces(&self) -> Pieces<'_>;
}

impl Pieced for Homes {
    fn pieces(&self) -> Pieces<'_> {
        Pieces::Homes(self)
    }
}

impl Pieced for Vec<BodyGroup> {
    fn pieces(&self) -> Pieces<'_> {
        Pieces::Bodies(self)
    }
}

impl Pieces<'_> {
    /// What a group of bodies is called: its lots are `country/depot-<n>/…`.
    pub const DEPOT: &'static str = "depot";

    /// The depot a lot is a yard of (`depot-3`), or none for any other lot.
    pub fn depot_of(lot: &str) -> Option<String> {
        let prefix = format!("country/{}-", Self::DEPOT);
        let number = lot.strip_prefix(&prefix)?.split_once('/')?.0;
        Some(format!("{}-{number}", Self::DEPOT))
    }

    /// The name a group's lots are numbered under: `country/<name>-<n>`.
    pub fn name(&self) -> &'static str {
        match self {
            Pieces::Homes(_) => "stead",
            Pieces::Bodies(_) => Self::DEPOT,
        }
    }
}

/// One kind of group of bodies, by weight: its pieces, placed in order.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyGroup {
    pub weight: f64,
    pub pieces: Vec<Piece>,
}

/// A piece of a group: `count` bodies of `kind` side by side in one yard,
/// with `net` (a body kind) drawn over them when it names one.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Piece {
    pub kind: String,
    #[serde(default = "one")]
    pub count: [u32; 2],
    #[serde(default)]
    pub net: Option<String>,
}

/// Bodies of a row lie this far apart, side by side.
pub const ROW_GAP_M: f64 = 0.1;

impl Piece {
    /// The box `count` bodies of half extents `half` fill, side by side.
    pub fn row(half: [f64; 3], count: u32) -> [f64; 3] {
        let n = f64::from(count);
        [n * half[0] + (n - 1.0) * ROW_GAP_M / 2.0, half[1], half[2]]
    }
}

/// Homes, each on its own lot.
#[derive(Clone, Debug)]
pub struct Homes {
    /// What a group is, by weight.
    pub groups: Vec<Group>,
    /// The chance a home has a clump of trees behind its yard, and the
    /// clump's sides.
    pub tree_chance: f64,
    pub tree_plot_m: [f64; 2],
    /// The chance a home has one of `bodies` beside it.
    pub body_chance: f64,
    pub bodies: Vec<Body>,
}

/// `homesteads` as the presets write it: one flat row of homes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HomesteadRow {
    per_road_km: f64,
    apart_m: f64,
    lane_chance: f64,
    lane_m: [f64; 2],
    spread_m: f64,
    yard: Yard,
    groups: Vec<Group>,
    tree_chance: f64,
    tree_plot_m: [f64; 2],
    body_chance: f64,
    bodies: Vec<Body>,
    attempts: u32,
}

/// `depots` as the presets write it: one flat row of body groups. A body
/// has no door, so its yard has no door clearance.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DepotRow {
    per_road_km: f64,
    apart_m: f64,
    lane_chance: f64,
    lane_m: [f64; 2],
    spread_m: f64,
    yard: DepotYard,
    groups: Vec<BodyGroup>,
    attempts: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DepotYard {
    front_m: f64,
    side_m: f64,
    rear_m: f64,
}

fn homesteads<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Steads<Homes>, D::Error> {
    HomesteadRow::deserialize(deserializer).map(Steads::from)
}

fn depots<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Steads<Vec<BodyGroup>>, D::Error> {
    let row = DepotRow::deserialize(deserializer)?;
    Ok(Steads {
        per_road_km: row.per_road_km,
        attempts: row.attempts,
        steading: Steading {
            apart_m: row.apart_m,
            lane_chance: row.lane_chance,
            lane_m: row.lane_m,
            spread_m: row.spread_m,
            yard: Yard {
                front_m: row.yard.front_m,
                side_m: row.yard.side_m,
                rear_m: row.yard.rear_m,
                door_clear_m: 0.0,
            },
            pieces: row.groups,
        },
    })
}

impl From<HomesteadRow> for Steads<Homes> {
    fn from(row: HomesteadRow) -> Self {
        Steads {
            per_road_km: row.per_road_km,
            attempts: row.attempts,
            steading: Steading {
                apart_m: row.apart_m,
                lane_chance: row.lane_chance,
                lane_m: row.lane_m,
                spread_m: row.spread_m,
                yard: row.yard,
                pieces: Homes {
                    groups: row.groups,
                    tree_chance: row.tree_chance,
                    tree_plot_m: row.tree_plot_m,
                    body_chance: row.body_chance,
                    bodies: row.bodies,
                },
            },
        }
    }
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
    pub fn errors(
        &self,
        presets: &crate::layout::PresetDefinitions,
    ) -> Vec<(String, &'static str)> {
        let wood_floor_m2 = presets.wood_floor_m2();
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
                && [e.homes, e.tree_line_m, e.copses, e.trees, e.cover, e.depots]
                    .iter()
                    .all(|v| length(*v)),
            "fairness",
            "evenness is a share and nonnegative allowances",
        );
        let h = &self.homesteads;
        let steading = &h.steading;
        let homes = &steading.pieces;
        check(
            length(h.per_road_km)
                && length(steading.apart_m)
                && chance(steading.lane_chance)
                && range(steading.lane_m)
                && length(steading.spread_m)
                && [
                    steading.yard.front_m,
                    steading.yard.side_m,
                    steading.yard.rear_m,
                    steading.yard.door_clear_m,
                ]
                .iter()
                .all(|v| length(*v))
                && chance(homes.tree_chance)
                && range(homes.tree_plot_m)
                && chance(homes.body_chance)
                && bodies(&homes.bodies)
                && h.attempts > 0,
            "homesteads",
            "homesteads need a density, a lane, a yard, chances and bodies with positive boxes",
        );
        check(
            !homes.groups.is_empty()
                && homes.groups.iter().all(|group| {
                    positive(group.weight)
                        && group.homes[0] >= 1
                        && group.homes[0] <= group.homes[1]
                        && !group.mix.is_empty()
                        && group.mix.iter().all(|(_, weight)| positive(*weight))
                }),
            "homesteads.groups",
            "a group is one or more homes of known building categories, by positive weight",
        );
        let d = &self.depots;
        let s = &d.steading;
        let groups = &s.pieces;
        check(
            length(d.per_road_km)
                && length(s.apart_m)
                && chance(s.lane_chance)
                && range(s.lane_m)
                && length(s.spread_m)
                && [s.yard.front_m, s.yard.side_m, s.yard.rear_m]
                    .iter()
                    .all(|v| length(*v))
                && d.attempts > 0,
            "depots",
            "depots need a density, a lane, a yard and attempts",
        );
        let boxes = &presets.street_props.bodies;
        // A piece is a body the presets size, in a row that fits under its net.
        let piece = |piece: &Piece| {
            boxes.get(&piece.kind).is_some_and(|body| {
                let row = Piece::row(body.half_extents_m, piece.count[1]);
                piece.count[0] >= 1
                    && piece.count[0] <= piece.count[1]
                    && piece.net.as_ref().is_none_or(|net| {
                        boxes
                            .get(net)
                            .is_some_and(|net| (0..3).all(|i| row[i] <= net.half_extents_m[i]))
                    })
            })
        };
        check(
            !groups.is_empty()
                && groups.iter().all(|group| {
                    positive(group.weight)
                        && !group.pieces.is_empty()
                        && group.pieces.iter().all(piece)
                }),
            "depots.groups",
            "a depot group is pieces of sized bodies, each row under its net, by positive weight",
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
                && homes.tree_plot_m[1] * homes.tree_plot_m[1] < k.area_m2[0]
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
