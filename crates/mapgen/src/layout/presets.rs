//! Versioned layout presets: every number the generator tunes, read from data
//! and refused at load when it cannot describe a map.
use super::{MapSize, MapType};
use crate::street_props::{fence_inset, side_panels};
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
    pub joints: JointPolicy,
    pub terrain: Terrain,
    pub approach: Approach,
    pub fairness: Fairness,
    pub transit: Transit,
    pub roads: Roads,
    pub sites: Sites,
    pub towns: Towns,
    pub forests: Forests,
    pub rivers: Rivers,
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

/// The plan header the compiler and the battle read; flat ground has no more.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    /// The map's visual-only surrounding landscape; it never expands the physical height grid.
    #[serde(default, deserialize_with = "contract::map::render_margin")]
    pub render_margin_m: f64,
    pub fog_cell_m: f64,
    pub height_grid_m: f64,
    pub slope_cutoff_deg: f64,
}

/// The one open-approach rule (M19): the main settlement has, in each half,
/// a corridor of open ground `front_m` wide for `depth_m` beyond its edge.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approach {
    pub depth_m: f64,
    pub front_m: f64,
    /// What the generator keeps clear so the measured corridor meets the
    /// rule: a wider front, and a margin past the depth.
    pub reserve_front_m: f64,
    pub reserve_margin_m: f64,
    /// Bearings tried in each half for the reserved corridor.
    pub bearing_candidates: u32,
}

/// `|top − bottom| ≤ max(rel × (top + bottom), abs × whole)`, where `whole`
/// is the playable area for an area and the map's side for a length.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    pub rel: f64,
    pub abs: f64,
}

impl Tolerance {
    pub fn allows(&self, top: f64, bottom: f64, whole: f64) -> bool {
        (top - bottom).abs() <= self.allowance(top, bottom, whole)
    }
    pub fn allowance(&self, top: f64, bottom: f64, whole: f64) -> f64 {
        (self.rel * (top + bottom)).max(self.abs * whole)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fairness {
    pub town: Tolerance,
    pub forest: Tolerance,
    /// The length of river in each half.
    pub river: Tolerance,
}

/// A light vehicle's road journey from the middle of the top edge and of the
/// bottom edge, where the two sides start, to the centre (M22).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transit {
    pub road_kmh: f64,
    /// Planning, turns and slower stretches, added to the driving time.
    pub allowance_s: f64,
    pub max_s: f64,
    /// The share of an edge, centred on its midpoint, a main road leaves by.
    pub exit_window: f64,
    /// The centre is reached at the road junction within this of the map's
    /// middle.
    pub centre_reach_m: f64,
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
    /// How many places are tried for each secondary road a settlement has.
    pub side_road_tries: usize,

    /// Two roads meet no nearer alongside than this (the sine of 45 degrees).
    pub fork_sin: f64,

    pub country_road_width_m: f64,
    pub dirt_track_width_m: f64,
    /// A track's speed against a road's, for timing a journey that uses one.
    pub dirt_track_speed_factor: f64,
    /// Distance between a road's authored points.
    pub bend_step_m: f64,
    /// How far a road may swing off its straight line, as a share of its length.
    pub bend_amplitude: f64,
    pub bend_max_m: f64,
    /// How far the main junction, and the main settlement about it, may sit
    /// from the exact centre along X and Y. Y stays small so the two halves
    /// share it.
    pub hub_offset_m: [f64; 2],
    /// The share of maps with a road across the middle from side to side.
    pub cross_road_chance: f64,
    /// On the other maps, how often one road comes in from the left or the
    /// right edge,
    pub side_road_chance: f64,
    /// and the share of that edge, centred on its midpoint, it may leave by.
    pub side_exit_window: f64,
    /// An edge road may join an earlier one this far, by road, from the main
    /// junction,
    pub junction_reach_m: f64,
    /// and does so this often when the journey time allows it.
    pub junction_chance: f64,
    /// Two junctions of the edge roads lie at least this far apart: nearer
    /// than this the roads meet at one.
    pub junction_apart_m: f64,
    /// An edge road swings through a settlement this near its line.
    pub waypoint_reach_m: f64,
    /// How far past its centre, as a share of the way to its edge, the road
    /// that joins a settlement carries on as its main street.
    pub main_street_reach: f64,
    /// A road runs straight across a settlement's ground, and turns this far
    /// outside it.
    pub gate_margin_m: f64,
    /// The sharpest turn a road makes where it leaves a settlement's main
    /// street or carries on from another road's end. It is also the least
    /// angle at which a road joins a road that passes, and the farthest off
    /// square to its edge that an extra road in from the edge sets out.
    pub turn_max_deg: f64,
    /// A road that passes this near a settlement's centre, as a share of
    /// its ground's reach, is its main street already.
    pub through_reach: f64,
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

/// How a settlement grows over the blocks its ground is cut into. Every
/// length is metres, the same at every map size.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Towns {
    pub geometry: TownGeometry,
    /// The street laid along a block's edge where no road runs.
    pub avenue_width_m: f64,
    /// How unevenly a settlement grows: the share by which a block's
    /// distance from the centre is drawn longer or shorter, one draw for
    /// each patch of ground this many of its longest blocks across.
    pub growth_noise: f64,
    pub growth_patch_blocks: f64,
    /// The fewest such patches a settlement's ground has.
    pub growth_patches_min: u32,
    /// A piece of ground with a corner sharper than this is left open.
    pub corner_min_deg: f64,
    /// Where a row of up to two blocks is cut across, as a share of its
    /// length.
    pub block_split: Range,
    /// The most a cut between blocks turns off square, either way.
    pub block_skew_deg: f64,
    /// How many times its length an edge on a road counts for, when a
    /// piece of ground picks the edge it fronts.
    pub road_frontage: f64,
    /// How far a street's end is moved along the road it meets, to meet it
    /// where a street already does from the other side: nearer than this
    /// the two make one crossroads, not two junctions a few metres apart.
    pub align_m: f64,
    /// A cut between blocks ends on a road no nearer than this to where
    /// another road leaves it.
    pub junction_clear_m: f64,
    /// A park is the smallest of this many blocks nearest the centre,
    pub park_reach_blocks: u32,
    /// that leaves at least this much ground for its trees.
    pub park_min_ha: f64,
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
    /// Chance that a wood is tried on a block a settlement leaves open
    /// beside its districts, and the wood's size as a share of the block's.
    pub infill_chance: f64,
    pub infill_cover: Range,
}

/// A river: a meandering line of water from the north edge to the south, and
/// what keeps clear of it. Every length is metres, the same at every size.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rivers {
    /// The water's width; a river draws one for each end and runs between.
    pub width_m: Range,
    /// How far the width swells and narrows along the way, as a share of it.
    pub width_swing: f64,
    /// Depth over half the width: the fall of the bed and of the bank.
    pub bank_grade: f64,
    /// How far the land stands above the water's surface.
    pub freeboard_m: f64,
    /// The most a river turns at one authored point, and the farthest two
    /// points lie apart: the contract only takes the corners off a line.
    pub point_turn_deg: f64,
    pub point_step_m: f64,
    pub meander: Meander,
    /// Least ground between the water's edge and the east and west edges.
    pub side_margin_m: f64,
    /// Least ground between the water's edge and a settlement's outline, a
    /// wood, a road that runs beside it, and a junction or road exit.
    pub settlement_gap_m: f64,
    pub forest_gap_m: f64,
    pub road_gap_m: f64,
    pub junction_gap_m: f64,
    pub bridge: BridgeRule,
}

impl Forests {
    /// A wood's shape: an ellipse with its edge drawn in and out.
    pub fn shape(&self) -> OutlineShape {
        OutlineShape {
            exponent: 2.0,
            noise: self.outline_noise,
            points: self.outline_points,
        }
    }
}

impl Rivers {
    /// How far a bank runs from the water's edge up to the land.
    pub fn bank_m(&self) -> f64 {
        self.freeboard_m / self.bank_grade
    }
}

/// A meander is a wave across the river's course with one overtone.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meander {
    pub wavelength_m: Range,
    /// The wave's reach to either side, as a share of its length.
    pub amplitude: Range,
    /// The overtone's reach against the wave's, and how many times shorter
    /// it is.
    pub overtone_gain: f64,
    pub overtone_ratio: Range,
}

impl Meander {
    /// The tightest any drawn meander can bend, as a curvature (1/m): both
    /// waves at their crests together.
    pub fn sharpest_bend(&self) -> f64 {
        let tau = core::f64::consts::TAU;
        let ratio = self.overtone_ratio[1];
        self.amplitude[1] * tau * tau / self.wavelength_m[0]
            * (1.0 + self.overtone_gain * ratio * ratio)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeRule {
    /// The prop type a deck is (a `props` catalog entry).
    pub deck: String,
    pub width_m: f64,
    /// How far past the water's edge a deck runs before it ends.
    pub landing_m: f64,
    /// The deck's top above the land, and its thickness.
    pub deck_z: f64,
    pub thickness_m: f64,
    /// A road runs straight for this far before and after a deck.
    pub approach_m: f64,
    /// The longest deck: a crossing that would need more is sited elsewhere.
    pub span_max_m: f64,
    /// A road that meets the river within this of square on is bridged on
    /// its own line; one more askew turns to cross square on.
    pub skew_max_deg: f64,
    /// A road that crosses within this of a bridge already built uses it.
    pub reuse_m: BTreeMap<SurfaceKind, f64>,
    /// How much farther a settlement's road goes to join the network on its
    /// own bank before it crosses.
    pub worth_m: f64,
}

/// Bounds on every search; running out is a named refusal, never a new seed.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retries {
    pub centre: u32,
    pub site: u32,
    pub forest: u32,
    pub river: u32,
    pub repair_settlements: u32,
    pub repair_woods: u32,
    /// The most blocks one settlement builds or leaves open at a time to
    /// even out the two halves.
    pub repair_blocks: u32,
    /// Woods drawn for one open block of a settlement before it is left a
    /// field.
    pub infill: u32,
    /// Templates tried at one place along a street before it is left open.
    pub fit: u32,
}

/// One kind of district: its buildings, its streets and its parcels. Every
/// length is metres, the same at every map size.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistrictPreset {
    /// One dominant building category and at most one minor one, by weight,
    /// kept dominant first.
    #[serde(deserialize_with = "mix")]
    pub mix: Mix,
    /// The least ground it is built on: a rectangle this long against a
    /// street and this deep from it, room for its largest parcel. A block
    /// that cannot hold it is another kind, or open.
    pub ground_m: [f64; 2],
    pub streets: StreetPattern,
    pub lots: LotRule,
    /// What stands in its streets and yards; absent is bare streets.
    #[serde(default)]
    pub props: DistrictProps,
}

/// One district kind's street furniture: how much of each body, by the
/// length of kerb or by the parcel. The bodies themselves, and the room each
/// keeps, are `street_props`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistrictProps {
    /// The share of a street's kerb, on a side cars park along, that parked
    /// cars stand beside; 0 is none.
    #[serde(default)]
    pub parking: f64,
    /// Bodies on the verge, placed in this order.
    #[serde(default)]
    pub verge: Vec<VergeRow>,
    /// Bodies beside each building, on its own parcel.
    #[serde(default)]
    pub yard: Vec<CountRow>,
    /// The garden behind each building; absent is a bare lawn.
    #[serde(default)]
    pub gardens: Option<Gardens>,
    /// The chance a parcel the parcel pass left open is a construction site.
    #[serde(default)]
    pub site_chance: f64,
    /// Its block interior; absent is grass.
    #[serde(default)]
    pub courts: Courts,
}

/// A district's block interior (`parcels::courts`) and the amenity groups
/// it is dressed with (`street_props::courts`). Its yards, car parks and
/// lawn paths are every paved district's (`street_props.courts`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Courts {
    /// Paved between its buildings, out to its carriageways and in to its
    /// last parcels' rears where none runs.
    #[serde(default)]
    pub paved: bool,
    /// Open groups are tried one to each square of a grid this far apart.
    #[serde(default)]
    pub spacing_m: f64,
    /// Groups of `street_props.groups` any map's court may hold, by weight.
    #[serde(default)]
    pub groups: BTreeMap<String, f64>,
    /// Groups only a map of that regional family's courts hold, by weight,
    /// drawn among the shared ones.
    #[serde(default)]
    pub families: BTreeMap<String, BTreeMap<String, f64>>,
}

/// What every paved district's courts are made of, and the parts they take.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourtRule {
    /// Courts take at most this share of the authored parts a map has left
    /// after its streets; its gardens take what the courts leave.
    pub share: f64,
    /// How a district's share is spent where parts run short.
    pub split: CourtSplit,
    /// Wall groups are tried one to each stretch this long of a building
    /// wall that faces into a court.
    pub wall_spacing_m: f64,
    /// What bounds each built parcel's yard; absent is an open yard.
    #[serde(default)]
    pub yards: Option<Yards>,
    /// The car parks cut from the ground the parcels leave; absent is none,
    /// and all that ground is lawn.
    #[serde(default)]
    pub parking: Option<CourtParking>,
    /// The paths laid across a lawn; absent is none.
    #[serde(default)]
    pub lawn: Option<Lawn>,
}

/// Each kind of court structure, in this order, takes at most one n-th of
/// the district's parts its predecessors left; the open groups take the
/// rest. Where parts run short each kind gets some.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourtSplit {
    pub yards: usize,
    pub parking: usize,
    pub paths: usize,
    pub walls: usize,
}

/// The paths across a dense district's lawn: paved strips from the streets
/// it meets and the yard gates that open on it to its middle, one of those
/// from a street a lane the widest hull drives, with groups planted beside.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lawn {
    /// A path's paved width.
    pub path_m: f64,
    /// The width of lawn a lane's strip finds clear, its path in the
    /// middle. Once laid, it keeps open this or a vehicle's way
    /// (`street_props.hull_way_margin_m`), the wider.
    pub lane_m: f64,
    /// A piece of lawn smaller than this has no paths.
    pub least_m2: f64,
    /// At most this many paths from streets, the lane among them, and from
    /// yard gates, to a piece of lawn's middle.
    pub streets: u32,
    pub gates: u32,
    /// Groups of `street_props.groups` planted beside a path, facing it, by
    /// weight: one tried to each stretch this long of each side.
    pub beside: BTreeMap<String, f64>,
    pub beside_spacing_m: f64,
}

/// The boundary run round a built parcel of a dense district, its yard:
/// along its street edge, its sides and its rear where it leaves a squad's
/// way (`street_props.squad_way_m`) to the building, with a gate before
/// each door, one in the middle of its rear, and one a hull wide in the
/// first of its street side, rear and sides that leaves a vehicle's way
/// (`street_props.hull_way_margin_m`) to the building.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Yards {
    /// The bodies a boundary is made of, for each regional family by
    /// weight: one kind to a district, so its yards' runs continue each other.
    pub boundary: BTreeMap<String, BTreeMap<String, f64>>,
    /// The width of a yard's one vehicle gate: the widest hull drives
    /// through.
    pub gate_m: f64,
    /// The width of each other gate: a squad walks through.
    pub door_gate_m: f64,
    /// The street side runs this far in from the parcel's street edge, or
    /// less where the building stands nearer: back from what stands at the
    /// kerb.
    pub front_inset_m: f64,
}

/// A car park cut from a dense district's ground beside a carriageway: an
/// aisle running in from the street with a row of bays either side of it.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourtParking {
    /// One bay's width along its row and depth from the aisle.
    pub bay_m: [f64; 2],
    /// The aisle's width: a car turns out of a bay into it.
    pub aisle_m: f64,
    /// Bays in a row: a car park this short or longer, the longest that fits.
    pub bays: [u32; 2],
    /// Along a side of a street, at most one car park this often.
    pub every_m: f64,
}

/// One kind of body along a district's verges.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VergeRow {
    /// A row of `street_props.bodies`.
    pub kind: String,
    /// One every this many metres of carriageway.
    pub spacing_m: f64,
    pub sides: VergeSides,
    /// Only beside a carriageway at least an avenue wide.
    #[serde(default)]
    pub avenue: bool,
}

/// Which side of the carriageway a verge row's bodies stand on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VergeSides {
    /// Evenly spaced, each on the other side from the one before.
    Alternate,
    /// Evenly spaced, one on each side.
    Both,
    /// That many to the length, each at a drawn place on a drawn side.
    Scatter,
}

/// What a built lot's garden holds: pieces in its rear setback, and a
/// boundary along its rear and side edges, never in the front garden.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gardens {
    /// The strip in from the lot's rear edge that pieces stand in.
    pub depth_m: f64,
    /// Pieces keep this far inside the lot's edges: room for its boundary
    /// and the ground that boundary keeps.
    pub room_m: f64,
    /// The chance a lot's garden has a boundary.
    pub boundary_chance: f64,
    /// The bodies a boundary is made of, by weight: one kind to a lot.
    pub boundary: BTreeMap<String, f64>,
    /// Pieces, each row's count drawn in order.
    pub pieces: Vec<CountRow>,
    /// The most bodies one garden holds, its boundary's included.
    pub max_per_lot: u32,
}

/// A body kind and how many of it: an inclusive range a seed draws from.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountRow {
    pub kind: String,
    pub count: [u32; 2],
}

/// Street furniture (C46): the bodies a built town's streets, yards and
/// open parcels are dressed with, as prop types of the catalog. Every body
/// goes through one legality check, so these rows are the whole of a kind.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreetProps {
    /// A fence panel, a cabin and loose stock keep this far inside the fence.
    pub site_room_m: f64,

    /// A side of a carriageway is asked whose district it is this often.
    pub owner_step_m: f64,

    /// How far ahead of a door a carriageway is looked for.
    pub door_look_m: f64,

    /// A door's way to the street is this long where no carriageway lies ahead.
    pub door_reach_m: f64,

    /// A body keeps this far inside the map's edge.
    pub edge_m: f64,

    /// No body stands nearer a carriageway's middle than the catalog's
    /// widest hull plus this. A vehicle keeps to the right of the middle,
    /// and the simulation checks its lane on a 2 m grid: a body that stops
    /// it, standing nearer than this, can close the road to it.
    pub lane_margin_m: f64,
    /// No body stands on a carriageway, nor within this of its edge.
    pub kerb_gap_m: f64,
    /// Open ground between a body and a building's wall: a soldier passes.
    pub wall_gap_m: f64,
    /// Open ground a squad walks through on the simulation's navigation
    /// grid (its clearance either side and a cell), at least the wall gap:
    /// kept between a court's wall group and its wall and round every court
    /// group's open sides, and the least room between a yard's boundary and
    /// its building for the boundary to stand.
    pub squad_way_m: f64,
    /// Open ground a court group keeps round its open sides: a squad
    /// passes it abreast, and a vehicle does not, so a court's inside is
    /// infantry ground the lanes and gates let a vehicle into.
    pub group_ring_m: f64,
    /// Kept clear either side of a door's line to the street.
    pub door_clear_m: f64,
    /// No body beside a carriageway stands within this of another
    /// carriageway's edge: a junction's corners stay open.
    pub corner_clear_m: f64,
    /// How far along its street a body is moved to find legal ground, and
    /// by what step.
    pub slide_m: f64,
    pub slide_step_m: f64,
    /// Places tried for a body that has no street to slide along.
    pub attempts: u32,
    /// Each kind that may be placed: its box, and the ground it keeps.
    pub bodies: BTreeMap<String, PropBox>,
    /// Amenities a court is dressed with, each placed whole or not at all.
    #[serde(default)]
    pub groups: BTreeMap<String, Group>,
    /// A vehicle's way is the catalog's widest hull and this: the
    /// simulation judges a vehicle's room on a 2 m grid and can lose a cell
    /// of it at each side. A lawn's lane keeps that much open, and a yard's
    /// vehicle gate stands only in a side that leaves that much between
    /// itself and the building.
    pub hull_way_margin_m: f64,
    pub courts: CourtRule,
    pub parking: Parking,
    pub site: ConstructionSite,
}

/// A placed body's box, and the open ground kept round it.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropBox {
    /// Half its length, width and height.
    pub half_extents_m: [f64; 3],
    /// No other body stands within this of it.
    pub clear_m: f64,
    /// A boundary of it is cut to fit its run, in panels any length up to
    /// its box's; its catalog appearance must be modular, repeating one
    /// module along the box rather than stretching.
    #[serde(default)]
    pub cut_to_fit: bool,
}

/// Amenities that stand together (a playground, a fenced basketball court,
/// a row of garages), in their own frame: `x` along the group, `y` across
/// it, from its middle.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Its length and width: every piece, and its fence, stands inside.
    pub size_m: [f64; 2],
    pub pieces: Vec<GroupPiece>,
    /// A body run round its edge as a fence, with a gate.
    #[serde(default)]
    pub fence: Option<String>,
    /// The opening left at the middle of the fence's first side (`y` least).
    #[serde(default)]
    pub gate_m: Option<f64>,
    /// It stands with its back (`y` most) to a building's wall, as a row of
    /// parking bays or a bench does, rather than out in the open.
    #[serde(default)]
    pub wall: bool,
}

/// The most a fenced group's side may leave open at its two corners
/// together, between its last whole panels and the corners: no soldier
/// squeezes through.
const CORNER_GAP_M: f64 = 0.6;

/// One body of a group: where it stands and how it is turned against the
/// group's own `x`, a quarter turn at a time.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupPiece {
    pub kind: String,
    pub at: [f64; 2],
    pub yaw_deg: f64,
}

impl GroupPiece {
    /// Its footprint's half extents along the group's `x` and `y`.
    fn reach(&self, body: &PropBox) -> [f64; 2] {
        let [a, b] = [body.half_extents_m[0], body.half_extents_m[1]];
        if (self.yaw_deg / 90.0).rem_euclid(2.0) == 1.0 {
            [b, a]
        } else {
            [a, b]
        }
    }
}

/// Cars parked along a street, in runs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parking {
    pub kind: String,
    /// Cars in one run.
    pub run: [u32; 2],
    /// Between two cars of a run: no way through for a soldier.
    pub bumper_gap_m: f64,
    /// The least gap between two runs: a squad passes.
    pub run_gap_m: f64,
    /// A carriageway at least this wide parks along both sides; a narrower
    /// one along one side, drawn for the whole street.
    pub both_sides_min_width_m: f64,
}

/// A construction site on a parcel left open: a cabin, a fence round the
/// parcel with a gate on the street, and loose stock inside.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionSite {
    /// The most sites one settlement has.
    pub max_per_settlement: u32,
    /// The least parcel a site fits: along the street, and deep.
    pub min_lot_m: [f64; 2],
    pub cabin: String,
    pub fence: String,
    /// From the parcel's edge in to its fence.
    pub fence_inset_m: f64,
    /// The opening left in the fence on the street side.
    pub gate_m: f64,
    pub stock: Vec<CountRow>,
}

/// A district's streets are a grid in its own frame: long streets
/// `block_depth_m` apart and cross streets every `block_length_m`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreetPattern {
    /// What its streets are: paved `road`, or `dirt_track` for a lane.
    pub surface: SurfaceKind,
    pub block_depth_m: f64,
    pub block_length_m: f64,
    /// Swings both street families off their straight lines; absent is a
    /// straight grid.
    #[serde(default)]
    pub bend: Option<Bend>,
    /// Chance a cross street is left out between two long streets, which
    /// leaves longer blocks and T-junctions.
    pub cross_skip: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bend {
    pub amplitude_m: f64,
    pub wavelength_m: f64,
}

/// A parcel is its template's footprint plus these margins: the template is
/// never scaled to a parcel.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LotRule {
    /// From the parcel's street edge to the building.
    pub front_m: f64,
    /// On each side: neighbours stand twice this apart.
    pub side_m: f64,
    pub rear_m: f64,
    /// Share of parcels built on; the rest stay open ground.
    pub coverage: f64,
    /// Depth of the paved apron across the parcel's front (parking, a
    /// loading yard), from the street's edge; never under the building.
    pub apron_m: f64,
}

/// What the parcel pass shares across districts.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parcels {
    pub geometry: StreetGeometry,
    /// The regional families a map may be built in; a seed draws one, and
    /// every building of the map comes from it (M08).
    pub regional_families: Vec<String>,
    /// The prop type every placed building is (a `props` catalog entry).
    pub prop_kind: String,
    pub street_width_m: f64,
    /// Between a carriageway's edge and a parcel's.
    pub verge_m: f64,
    /// Distance between a bent street's authored points.
    pub street_step_m: f64,
    /// How far along a street the next parcel is tried when none fits.
    pub lot_step_m: f64,
    /// A street that stops with a carriageway this near ahead of it runs on
    /// to it: no ground between is worth a dead end.
    pub run_on_m: f64,
    /// A street that runs on past its last junction for less than this and
    /// stops in the open is cut back to that junction.
    pub tail_min_m: f64,
    /// A street lands on a road no nearer than this to a junction the road
    /// already has, unless it makes a crossroads of it.
    pub junction_clear_m: f64,
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
    /// The share of its ground (the class's size) that is built on. The rest is
    /// field or wood beside and between its districts.
    pub built_share: Range,
    pub block: BlockRule,
    /// How much a block's distance from the nearest road counts against its
    /// distance from the centre when the settlement grows: 0 grows a compact
    /// town, more strings it along its roads.
    pub ribbon: f64,
    /// The roads it has besides the ones that meet at its centre: none
    /// where absent.
    #[serde(default)]
    pub side_roads: Option<SideRoads>,
    /// How many blocks near its centre are parks: open ground with trees
    /// and a street all round. None where absent.
    #[serde(default)]
    pub parks: Option<[u32; 2]>,
    /// The chance a block away from its centre is left unbuilt, so that
    /// its built ground has gaps. Its built share falls by as much.
    #[serde(default)]
    pub open_blocks: f64,
    /// How many blocks side by side take one district kind together.
    pub neighbourhood: [u32; 2],
    /// What its blocks are, from the centre out; the last zone reaches the
    /// edge.
    pub zones: Vec<Zone>,
}

/// A settlement's secondary roads. Each leaves one of the roads through
/// the centre part of the way out to the edge of its ground, turns off it
/// into the widest sector that has no road yet, and runs straight to that
/// edge: a second way out of town that does not pass the central junction.
/// It runs beside the next road round its sector, or square off its own.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SideRoads {
    /// How many a seed draws.
    pub count: [u32; 2],
    /// Where one leaves its road, as a share of the way from the centre to
    /// the edge of the settlement's ground.
    pub from: Range,
    /// The angles to its own road at which it may run beside another;
    /// outside them it leaves its own road square.
    pub turn_deg: Range,
    /// The least distance between two of them where they leave one road.
    pub apart_m: f64,
}

/// A block is one district: ground bounded by roads, streets and the
/// settlement's edge, this deep from the road it fronts and this long
/// along it.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockRule {
    pub depth_m: Range,
    pub length_m: Range,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    /// Outer limit as a share of the settlement's built ground, counted
    /// block by block from the centre out.
    pub to: f64,
    /// District id to the weight a block picks it with.
    pub districts: BTreeMap<String, f64>,
    /// The weights a block at the settlement's edge on a country road picks
    /// with instead, so that industry stands where the road comes in.
    #[serde(default)]
    pub roadside: Option<BTreeMap<String, f64>>,
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
    /// The share of this type's maps that have a river (M11).
    pub river_chance: f64,
    pub sizes: BTreeMap<MapSize, SizePreset>,
}

/// The main settlement: it stands on the main junction.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Centre {
    pub class: String,
}

/// How often a settlement is sited on a main road's line, beside the main
/// settlement or, on a map with a river, on its bank; otherwise it lies
/// anywhere there is room.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Siting {
    pub on_road: f64,
    pub near_main: f64,
    pub beside_river: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizePreset {
    /// Ceiling on settlement area over playable area.
    pub urban_share_max: f64,
    /// Class id to the inclusive count range a seed draws from.
    pub settlements: BTreeMap<String, [u32; 2]>,
}

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
        let length = |v: f64| v.is_finite() && v >= 0.0;

        check(
            self.joints.meet_m.is_finite() && self.joints.meet_m > 0.0,
            "joints.meet_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.near_m.is_finite() && self.joints.near_m > 0.0,
            "joints.near_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.through_cos.is_finite() && (0.0..=1.0).contains(&self.joints.through_cos),
            "joints.through_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.tail_spare_m.is_finite() && self.joints.tail_spare_m > 0.0,
            "joints.tail_spare_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.carry_rounds > 0 && self.joints.carry_rounds <= 4096,
            "joints.carry_rounds".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.beside_widths.is_finite() && self.joints.beside_widths > 0.0,
            "joints.beside_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.junction_m.is_finite() && self.joints.junction_m > 0.0,
            "joints.junction_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.swing_widths.is_finite() && self.joints.swing_widths > 0.0,
            "joints.swing_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.slant_cos.is_finite() && (0.0..=1.0).contains(&self.joints.slant_cos),
            "joints.slant_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.square_widths.is_finite() && self.joints.square_widths > 0.0,
            "joints.square_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.stub_widths.is_finite() && self.joints.stub_widths > 0.0,
            "joints.stub_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.alongside_cos.is_finite()
                && (0.0..=1.0).contains(&self.joints.alongside_cos),
            "joints.alongside_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.gate_widths.is_finite() && self.joints.gate_widths > 0.0,
            "joints.gate_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.trim_margin_m.is_finite() && self.joints.trim_margin_m > 0.0,
            "joints.trim_margin_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.pin_rounds > 0 && self.joints.pin_rounds <= 4096,
            "joints.pin_rounds".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.joints.shortest_run_m.is_finite() && self.joints.shortest_run_m > 0.0,
            "joints.shortest_run_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.roads.fork_sin.is_finite() && (0.0..=1.0).contains(&self.roads.fork_sin),
            "roads.fork_sin".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.roads.side_road_tries > 0 && self.roads.side_road_tries <= 4096,
            "roads.side_road_tries".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.sliver_m.is_finite() && self.towns.geometry.sliver_m > 0.0,
            "towns.geometry.sliver_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.shared_m.is_finite() && self.towns.geometry.shared_m > 0.0,
            "towns.geometry.shared_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.meet_m.is_finite() && self.towns.geometry.meet_m > 0.0,
            "towns.geometry.meet_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.places > 0 && self.towns.geometry.places <= 4096,
            "towns.geometry.places".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.in_line_sin.is_finite()
                && (0.0..=1.0).contains(&self.towns.geometry.in_line_sin),
            "towns.geometry.in_line_sin".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.towns.geometry.avenue_slant_sin.is_finite()
                && (0.0..=1.0).contains(&self.towns.geometry.avenue_slant_sin),
            "towns.geometry.avenue_slant_sin".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.facing_past_m.is_finite()
                && self.parcels.geometry.facing_past_m > 0.0,
            "parcels.geometry.facing_past_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.parallel_cos.is_finite()
                && (0.0..=1.0).contains(&self.parcels.geometry.parallel_cos),
            "parcels.geometry.parallel_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.facing_cos.is_finite()
                && (0.0..=1.0).contains(&self.parcels.geometry.facing_cos),
            "parcels.geometry.facing_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.slant_sin.is_finite()
                && (0.0..=1.0).contains(&self.parcels.geometry.slant_sin),
            "parcels.geometry.slant_sin".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.turn_sin.is_finite()
                && (0.0..=1.0).contains(&self.parcels.geometry.turn_sin),
            "parcels.geometry.turn_sin".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.turn_widths.is_finite()
                && self.parcels.geometry.turn_widths > 0.0,
            "parcels.geometry.turn_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.in_line_cos.is_finite()
                && (0.0..=1.0).contains(&self.parcels.geometry.in_line_cos),
            "parcels.geometry.in_line_cos".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.ahead_spread.is_finite()
                && self.parcels.geometry.ahead_spread > 0.0,
            "parcels.geometry.ahead_spread".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.road_edge.is_finite() && self.parcels.geometry.road_edge > 0.0,
            "parcels.geometry.road_edge".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.open_edge.is_finite() && self.parcels.geometry.open_edge > 0.0,
            "parcels.geometry.open_edge".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.bow_lengths[0].is_finite()
                && self.parcels.geometry.bow_lengths[1].is_finite()
                && self.parcels.geometry.bow_lengths[0] >= 1.0
                && self.parcels.geometry.bow_lengths[0] <= self.parcels.geometry.bow_lengths[1],
            "parcels.geometry.bow_lengths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.bow_reach.is_finite() && self.parcels.geometry.bow_reach > 0.0,
            "parcels.geometry.bow_reach".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.alone_widths.is_finite()
                && self.parcels.geometry.alone_widths > 0.0,
            "parcels.geometry.alone_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.crowd_widths.is_finite()
                && self.parcels.geometry.crowd_widths > 0.0,
            "parcels.geometry.crowd_widths".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.own_tan.is_finite() && self.parcels.geometry.own_tan > 0.0,
            "parcels.geometry.own_tan".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.corner_tan.is_finite() && self.parcels.geometry.corner_tan > 0.0,
            "parcels.geometry.corner_tan".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.parcels.geometry.end_margin_m.is_finite()
                && self.parcels.geometry.end_margin_m > 0.0,
            "parcels.geometry.end_margin_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.street_props.edge_m.is_finite() && self.street_props.edge_m > 0.0,
            "street_props.edge_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.street_props.door_reach_m.is_finite() && self.street_props.door_reach_m > 0.0,
            "street_props.door_reach_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.street_props.door_look_m.is_finite() && self.street_props.door_look_m > 0.0,
            "street_props.door_look_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.street_props.owner_step_m.is_finite() && self.street_props.owner_step_m > 0.0,
            "street_props.owner_step_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
        check(
            self.street_props.site_room_m.is_finite() && self.street_props.site_room_m > 0.0,
            "street_props.site_room_m".into(),
            "Extracted policy must be finite and preserve its geometric/work domain",
        );
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
            ("river", self.fairness.river),
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
                && share(t.exit_window)
                && positive(t.centre_reach_m),
            "transit".into(),
            "transit needs a road speed, a time above its allowance, an exit window and a centre",
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
                && r.hub_offset_m.iter().all(|v| length(*v))
                // The main junction is the centre a journey is timed to.
                && libm::hypot(r.hub_offset_m[0], r.hub_offset_m[1]) <= t.centre_reach_m
                && (0.0..=1.0).contains(&r.cross_road_chance)
                && (0.0..=1.0).contains(&r.side_road_chance)
                && share(r.side_exit_window)
                && length(r.gate_margin_m)
                && (0.0..=90.0).contains(&r.turn_max_deg)
                && r.junction_reach_m >= 0.0
                && r.junction_reach_m.is_finite()
                && (0.0..=1.0).contains(&r.junction_chance)
                && positive(r.junction_apart_m)
                && r.waypoint_reach_m >= 0.0
                && r.waypoint_reach_m.is_finite()
                && share(r.main_street_reach)
                && share(r.through_reach)
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
        let towns = &self.towns;
        check(
            positive(towns.avenue_width_m)
                && (0.0..1.0).contains(&towns.growth_noise)
                && positive(towns.growth_patch_blocks)
                && towns.growth_patches_min >= 1
                && (0.0..90.0).contains(&towns.corner_min_deg)
                && range(towns.block_split)
                && towns.block_split[0] > 0.0
                && towns.block_split[1] < 1.0
                && (0.0..=20.0).contains(&towns.block_skew_deg)
                && towns.road_frontage.is_finite()
                && towns.road_frontage >= 1.0
                && length(towns.align_m)
                && length(towns.junction_clear_m)
                && towns.park_reach_blocks >= 1
                && length(towns.park_min_ha),
            "towns".into(),
            "towns need an avenue width, a growth noise below 1 in one or more patches of a positive size, a sharpest corner below 90°, a block split inside a row, a skew of 20° at most, a road frontage of 1 or more and a nonnegative reach to align junctions over",
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
                && positive_range(f.infill_cover)
                && f.max_woods > 0,
            "forests".into(),
            "forest sizes, outline and gaps must be finite, ordered and positive",
        );
        let w = &self.rivers;
        let steepest = contract::river::steepest_grade(self.terrain.slope_cutoff_deg);
        check(
            positive_range(w.width_m)
                && w.width_m[0] >= contract::river::MIN_WIDTH_CELLS * self.terrain.height_grid_m
                && (0.0..1.0).contains(&w.width_swing)
                && positive(w.bank_grade)
                && w.bank_grade <= steepest
                && positive(w.freeboard_m),
            "rivers.width_m".into(),
            "a river is at least three height samples wide, with a bank the slope cutoff admits and a surface below the land",
        );
        let m = &w.meander;
        // The line beside the water that a road follows stays a line only
        // where the tightest bend is wider than its distance from the middle.
        let beside = w.width_m[1] / 2.0 + w.road_gap_m;
        check(
            positive(w.point_step_m)
                && positive(w.point_turn_deg)
                && w.point_turn_deg < 90.0
                && positive_range(m.wavelength_m)
                && range(m.amplitude)
                && m.amplitude[0] >= 0.0
                && (0.0..1.0).contains(&m.overtone_gain)
                && range(m.overtone_ratio)
                && m.overtone_ratio[0] >= 1.0
                && m.sharpest_bend() * beside < 1.0,
            "rivers.meander".into(),
            "a meander is a positive wave with a weaker, shorter overtone, and bends no tighter than the water and a road beside it are wide",
        );
        check(
            [w.settlement_gap_m, w.forest_gap_m, w.road_gap_m]
                .iter()
                .all(|gap| gap.is_finite() && *gap >= w.bank_m())
                && w.junction_gap_m.is_finite()
                && w.junction_gap_m >= w.road_gap_m
                && w.side_margin_m.is_finite()
                && w.side_margin_m >= w.junction_gap_m,
            "rivers.settlement_gap_m".into(),
            "everything beside a river stands past its bank, a junction past the road beside it, and the map's side edges past that",
        );
        let b = &w.bridge;
        check(
            !b.deck.is_empty()
                && b.width_m.is_finite()
                && b.width_m >= r.country_road_width_m.max(r.dirt_track_width_m)
                // The ramp under a deck's end is the bank at its steepest,
                // and the height grid draws its top up to a sample's diagonal
                // farther out: past both, the deck is stepped onto from the
                // land's own height.
                && b.landing_m.is_finite()
                && b.landing_m
                    >= w.freeboard_m / steepest
                        + core::f64::consts::SQRT_2 * self.terrain.height_grid_m
                && b.deck_z.is_finite()
                && b.deck_z >= 0.0
                && positive(b.thickness_m)
                // A road's bend takes at most a width and a half of the run
                // either side of an authored point: none of it on the deck.
                && b.approach_m.is_finite()
                && b.approach_m >= 1.5 * r.country_road_width_m
                // A road beside the water runs outside that straight run, so
                // it comes onto a bridge's line without doubling back.
                && w.road_gap_m >= b.landing_m + b.approach_m
                && b.span_max_m.is_finite()
                && b.span_max_m >= w.width_m[1] + 2.0 * b.landing_m
                && (0.0..=60.0).contains(&b.skew_max_deg)
                && [SurfaceKind::CountryRoad, SurfaceKind::DirtTrack]
                    .iter()
                    .all(|kind| b.reuse_m.get(kind).is_some_and(|reach| length(*reach)))
                && length(b.worth_m),
            "rivers.bridge".into(),
            "a deck is as wide as the roads, ends where a ramp reaches it, spans the widest water, and is approached by a straight run no farther from the water than a road beside it",
        );
        let n = &self.retries;
        check(
            n.centre > 0 && n.site > 0 && n.forest > 0 && n.river > 0 && n.fit > 0 && n.infill > 0,
            "retries".into(),
            "every search needs at least one attempt",
        );
        let p = &self.parcels;
        check(
            !p.regional_families.is_empty()
                && p.regional_families.iter().all(|family| !family.is_empty())
                && !p.prop_kind.is_empty()
                && positive(p.street_width_m)
                && length(p.verge_m)
                && positive(p.street_step_m)
                && positive(p.lot_step_m)
                && length(p.run_on_m)
                && length(p.tail_min_m)
                && length(p.junction_clear_m),
            "parcels".into(),
            "parcels need a regional family, a prop type, a street width, positive steps, a nonnegative run-on and a nonnegative least tail",
        );
        for (field, message) in self.open_country.errors(self.wood_floor_m2()) {
            check(false, format!("open_country.{field}"), message);
        }
        let family = |family: &String| self.parcels.regional_families.contains(family);
        for (id, district) in &self.districts {
            let mix = &district.mix;
            check(
                (1..=2).contains(&mix.len()) && mix.iter().all(|(_, weight)| positive(*weight)),
                format!("districts.{id}.mix"),
                "a district is one building category, or one with a minor second, by positive weight",
            );
            check(
                district.ground_m.iter().all(|v| positive(*v)),
                format!("districts.{id}.ground_m"),
                "a district needs a positive length and depth of ground",
            );
            let streets = &district.streets;
            // A block must hold a street and its verges with ground to spare.
            let least = p.street_width_m + 2.0 * p.verge_m;
            check(
                matches!(streets.surface, SurfaceKind::Road | SurfaceKind::DirtTrack)
                    && streets.block_depth_m.is_finite()
                    && streets.block_length_m.is_finite()
                    && streets.block_depth_m > least
                    && streets.block_length_m > least
                    && (0.0..1.0).contains(&streets.cross_skip)
                    // A steeper swing would fold a street over its neighbour.
                    && streets.bend.is_none_or(|bend| {
                        length(bend.amplitude_m)
                            && positive(bend.wavelength_m)
                            && bend.amplitude_m * core::f64::consts::TAU / bend.wavelength_m <= 0.5
                    }),
                format!("districts.{id}.streets"),
                "streets are road or dirt_track, blocks wider than a street, cross_skip a chance below 1, and a bend gentle",
            );
            let lots = &district.lots;
            check(
                length(lots.front_m)
                    && length(lots.side_m)
                    && length(lots.rear_m)
                    && length(lots.apron_m)
                    && share(lots.coverage),
                format!("districts.{id}.lots"),
                "setbacks and aprons are nonnegative metres and coverage a share above 0",
            );
            let props = &district.props;
            let known = |kind: &String| self.street_props.bodies.contains_key(kind);
            check(
                (0.0..1.0).contains(&props.parking)
                    && (0.0..=1.0).contains(&props.site_chance)
                    && props
                        .verge
                        .iter()
                        .all(|row| known(&row.kind) && positive(row.spacing_m))
                    && props
                        .yard
                        .iter()
                        .all(|row| known(&row.kind) && row.count[0] <= row.count[1])
                    && props.gardens.as_ref().is_none_or(|gardens| {
                        positive(gardens.depth_m)
                            && gardens.depth_m <= lots.rear_m
                            && length(gardens.room_m)
                            && (0.0..=1.0).contains(&gardens.boundary_chance)
                            && gardens
                                .boundary
                                .iter()
                                .all(|(kind, weight)| known(kind) && positive(*weight))
                            && gardens
                                .pieces
                                .iter()
                                .all(|row| known(&row.kind) && row.count[0] <= row.count[1])
                    }),
                format!("districts.{id}.props"),
                "street furniture names bodies of street_props, by a parking share below 1, positive spacings and weights, ordered counts, a site chance and a garden no deeper than the rear setback",
            );
            let courts = &props.courts;
            // A group is tried along the walls or over the open court, each
            // at its own spacing.
            let table = |groups: &BTreeMap<String, f64>| {
                groups.iter().all(|(group, weight)| {
                    self.street_props.groups.get(group).is_some_and(|group| {
                        positive(if group.wall {
                            self.street_props.courts.wall_spacing_m
                        } else {
                            courts.spacing_m
                        })
                    }) && positive(*weight)
                })
            };
            let dressed = !courts.groups.is_empty() || !courts.families.is_empty();
            check(
                table(&courts.groups)
                    && courts
                        .families
                        .iter()
                        .all(|(name, groups)| family(name) && table(groups))
                    && (!dressed || courts.paved)
                    && length(courts.spacing_m),
                format!("districts.{id}.props.courts"),
                "a court's tables name groups of street_props by positive weight, each tried at a positive spacing for where it stands, its family tables name regional families, and a dressed court is paved",
            );
        }
        let s = &self.street_props;
        check(
            length(s.lane_margin_m)
                && length(s.kerb_gap_m)
                && length(s.wall_gap_m)
                && length(s.door_clear_m)
                && length(s.corner_clear_m)
                && length(s.slide_m)
                && positive(s.slide_step_m)
                && s.attempts > 0
                && s.squad_way_m >= s.wall_gap_m
                && s.group_ring_m >= s.squad_way_m
                && length(s.hull_way_margin_m),
            "street_props".into(),
            "street furniture needs nonnegative margins, a positive slide step, at least one attempt, a squad's way no narrower than the wall gap and a group's ring no narrower than a squad's way",
        );
        let courts = &s.courts;
        let known = |kind: &String| s.bodies.contains_key(kind);
        let split = courts.split;
        check(
            positive(courts.share)
                && courts.share <= 1.0
                && [split.yards, split.parking, split.paths, split.walls]
                    .iter()
                    .all(|n| *n >= 1)
                && length(courts.wall_spacing_m)
                && courts.yards.as_ref().is_none_or(|yards| {
                    positive(yards.gate_m)
                        && positive(yards.door_gate_m)
                        && length(yards.front_inset_m)
                        && yards.boundary.iter().all(|(name, kinds)| {
                            family(name)
                                && !kinds.is_empty()
                                && kinds
                                    .iter()
                                    .all(|(kind, weight)| known(kind) && positive(*weight))
                        })
                })
                && courts.parking.is_none_or(|parking| {
                    parking.bay_m.iter().all(|v| positive(*v))
                        && positive(parking.aisle_m)
                        && parking.bays[0] >= 1
                        && parking.bays[0] <= parking.bays[1]
                        && length(parking.every_m)
                })
                && courts.lawn.as_ref().is_none_or(|lawn| {
                    positive(lawn.path_m)
                        && lawn.lane_m >= lawn.path_m
                        && length(lawn.least_m2)
                        && lawn.streets >= 1
                        && positive(lawn.beside_spacing_m)
                        && lawn.beside.iter().all(|(group, weight)| {
                            s.groups.get(group).is_some_and(|group| {
                                group.size_m[0] <= lawn.beside_spacing_m
                            }) && positive(*weight)
                        })
                }),
            "street_props.courts".into(),
            "courts take a share in (0, 1] split one n-th at a time, n at least 1, try wall groups at a nonnegative spacing, bound yards by bodies for each regional family with two gates, cut car parks of positive bays, an aisle and an ordered count, and lay lawn paths with a lane no narrower than a path, at least one path from a street (the lane is one) and groups beside at a positive spacing no narrower than each",
        );
        for (kind, body) in &s.bodies {
            check(
                !kind.is_empty()
                    && body.half_extents_m.iter().all(|v| positive(*v))
                    && length(body.clear_m),
                format!("street_props.bodies.{kind}"),
                "a body is a positive box and a nonnegative clearance",
            );
        }
        for (name, group) in &s.groups {
            // The ground its fence keeps inside its edge: in to the panels'
            // inner face.
            let fence = group.fence.as_ref().map(|kind| s.bodies.get(kind));
            let inset = match fence {
                None => Some(0.0),
                Some(Some(body)) => Some(fence_inset(body) + body.half_extents_m[1]),
                Some(None) => None,
            };
            let boxes: Option<Vec<[f64; 4]>> = group
                .pieces
                .iter()
                .map(|piece| {
                    let body = s.bodies.get(&piece.kind)?;
                    let reach = piece.reach(body);
                    Some([
                        piece.at[0] - reach[0],
                        piece.at[1] - reach[1],
                        piece.at[0] + reach[0],
                        piece.at[1] + reach[1],
                    ])
                })
                .collect();
            let fits = match (inset, &boxes) {
                (Some(inset), Some(boxes)) => {
                    let half = group.size_m.map(|v| v / 2.0 - inset);
                    let apart = |a: &[f64; 4], b: &[f64; 4]| {
                        a[2] <= b[0] + 0.01
                            || b[2] <= a[0] + 0.01
                            || a[3] <= b[1] + 0.01
                            || b[3] <= a[1] + 0.01
                    };
                    boxes.iter().all(|b| {
                        b[0] >= -half[0] && b[1] >= -half[1] && b[2] <= half[0] && b[3] <= half[1]
                    }) && boxes
                        .iter()
                        .enumerate()
                        .all(|(i, a)| boxes[i + 1..].iter().all(|b| apart(a, b)))
                }
                _ => false,
            };
            // A fence's panels, as the fence routine lays them, run each
            // side to within a soldier's squeeze of its corners.
            let closed = fence.flatten().is_none_or(|body| {
                group.size_m.iter().all(|side| {
                    let run = side - 2.0 * fence_inset(body);
                    let panels = side_panels(run, body, &[]);
                    let covered: f64 = panels.iter().map(|[_, half]| 2.0 * half).sum();
                    !panels.is_empty() && run - covered <= CORNER_GAP_M
                })
            });
            check(
                group.size_m.iter().all(|v| positive(*v))
                    && !group.pieces.is_empty()
                    && closed
                    && group
                        .pieces
                        .iter()
                        .all(|piece| piece.at.iter().all(|v| v.is_finite())
                            && (piece.yaw_deg / 90.0).fract() == 0.0)
                    && fits
                    && group.gate_m.is_some() == group.fence.is_some()
                    && group
                        .gate_m
                        .is_none_or(|gate| positive(gate) && gate < group.size_m[0]),
                format!("street_props.groups.{name}"),
                "a group names bodies of street_props turned by quarter turns, inside its size and its fence and apart, and a fenced group whole panels to its corners and a gate narrower than its first side",
            );
        }
        let parking = &s.parking;
        check(
            s.bodies.contains_key(&parking.kind)
                && parking.run[0] >= 1
                && parking.run[0] <= parking.run[1]
                && length(parking.bumper_gap_m)
                && parking.run_gap_m.is_finite()
                && parking.run_gap_m > parking.bumper_gap_m
                && length(parking.both_sides_min_width_m),
            "street_props.parking".into(),
            "parking names a body, an ordered run of one car or more, a gap between runs wider than between bumpers, and a width that parks both sides",
        );
        let site = &s.site;
        check(
            s.bodies.contains_key(&site.cabin)
                && s.bodies.contains_key(&site.fence)
                && site.min_lot_m.iter().all(|v| positive(*v))
                && length(site.fence_inset_m)
                && positive(site.gate_m)
                && site
                    .stock
                    .iter()
                    .all(|row| s.bodies.contains_key(&row.kind) && row.count[0] <= row.count[1]),
            "street_props.site".into(),
            "a construction site names bodies for its cabin, fence and stock, a least parcel, a fence inset and a gate",
        );
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
            check(
                positive_range(class.built_share)
                    && class.built_share[1] <= 1.0
                    && class.parks.is_none_or(|parks| parks[0] <= parks[1])
                    && (0.0..0.5).contains(&class.open_blocks),
                at("built_share"),
                "built_share is an ordered range of shares, parks an ordered count and open_blocks a chance below a half",
            );
            check(
                positive_range(class.block.depth_m) && positive_range(class.block.length_m),
                at("block"),
                "a block's depth and length are ordered ranges of metres",
            );
            check(length(class.ribbon), at("ribbon"), "ribbon is nonnegative");
            check(
                class.side_roads.is_none_or(|rule| {
                    rule.count[0] <= rule.count[1]
                        && positive_range(rule.from)
                        && rule.from[1] < 1.0
                        && range(rule.turn_deg)
                        // Square to the road it leaves at most; no sharper
                        // than a road may turn.
                        && rule.turn_deg[0] >= self.roads.turn_max_deg
                        && rule.turn_deg[1] <= 180.0 - self.roads.turn_max_deg
                        && length(rule.apart_m)
                }),
                at("side_roads"),
                "side roads are an ordered count, leaving inside the settlement's ground at a turn no sharper than roads.turn_max_deg",
            );
            check(
                class.neighbourhood[0] >= 1 && class.neighbourhood[0] <= class.neighbourhood[1],
                at("neighbourhood"),
                "a neighbourhood is an ordered count of blocks from 1",
            );
            // A block cut to the class's least depth still holds one of its
            // kinds' parcels, behind the widest carriageway and its verge.
            let setback = self.setback_m();
            let shallowest = class
                .zones
                .iter()
                .flat_map(|zone| zone.districts.keys())
                .filter_map(|kind| self.districts.get(kind))
                .map(|district| district.ground_m[1])
                .fold(f64::INFINITY, f64::min);
            check(
                class.block.depth_m[0] >= shallowest + setback,
                at("block.depth_m"),
                "a block is at least as deep as the ground its shallowest district kind needs, behind a street's half width and verge",
            );
            let mut from = 0.0;
            for (index, zone) in class.zones.iter().enumerate() {
                check(
                    zone.to > from && zone.to <= 1.0,
                    at(&format!("zones[{index}]")),
                    "zones grow outward to 1",
                );
                let picks = |weights: &BTreeMap<String, f64>| {
                    !weights.is_empty()
                        && weights
                            .iter()
                            .all(|(d, w)| self.districts.contains_key(d) && positive(*w))
                };
                check(
                    picks(&zone.districts) && zone.roadside.as_ref().is_none_or(picks),
                    at(&format!("zones[{index}].districts")),
                    "a zone picks known districts with positive weights",
                );
                from = zone.to;
            }
            check(
                from == 1.0,
                at("zones"),
                "the last zone must reach the settlement's edge (to: 1)",
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
                self.classes.contains_key(&preset.centre.class),
                at("centre"),
                "the centre names a known class",
            );
            check(
                preset.siting.on_road >= 0.0
                    && preset.siting.near_main >= 0.0
                    && preset.siting.beside_river >= 0.0
                    && preset.siting.on_road + preset.siting.near_main + preset.siting.beside_river
                        <= 1.0,
                at("siting"),
                "siting chances are nonnegative and sum to at most 1",
            );
            check(
                (0.0..=1.0).contains(&preset.river_chance),
                at("river_chance"),
                "the share of maps with a river is a chance",
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
                    .flat_map(|class| &class.zones)
                    .flat_map(|zone| {
                        zone.districts
                            .keys()
                            .chain(zone.roadside.iter().flatten().map(|(d, _)| d))
                    });
                for district in districts {
                    let mix = self.districts.get(district).map(|d| &d.mix);
                    for (category, _) in mix.into_iter().flatten() {
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

/// A mix is written as an object and kept dominant first (then by name), so
/// the plan lists categories the same way whatever order the file used.
fn mix<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Mix, D::Error> {
    let wire = BTreeMap::<String, f64>::deserialize(decoder)?;
    let mut out = Vec::new();
    for (name, weight) in wire {
        let category: BuildingCategory = serde_json::from_value(serde_json::Value::String(name))
            .map_err(serde::de::Error::custom)?;
        out.push((category, weight));
    }
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(out)
}

/// Extracted generation policy. The initial values preserve the existing plan.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointPolicy {
    /// Ends this near each other are one place.
    pub meet_m: f64,
    /// An end joins a carriageway whose edge is this near it.
    pub near_m: f64,
    /// At a junction a wider road also ends on, two narrower ends within this of straight (30°) are one road through it; otherwise the wider road carries on down one of them (the carry pass).
    pub through_cos: f64,
    /// An end's last run is at least this much longer than the widest road's half width: the cut of a square end reaches half a width back, and a corner behind it is to stay round. A turn nearer an end than that is dropped, and no way's width changes that near the corner it was carried round.
    pub tail_spare_m: f64,
    /// A corner of unlike ways is mended over at most this many rounds: a way changes once a round.
    pub carry_rounds: usize,
    /// Two alike streets laid side by side whose ends run past each other by no more than this many widths are one street.
    pub beside_widths: f64,
    /// Two ways that each cross a third within this of where they crossed each other are still joined there.
    pub junction_m: f64,
    /// An end whose last run is no longer than this many of its widths may be swung to meet a road square, when it has no room to turn before it.
    pub swing_widths: f64,
    /// A branch within this of square to the road it joins (30°) is left as it comes; one that comes in at more of a slant is bent to meet it square.
    pub slant_cos: f64,
    /// A branch bent to meet a road square runs square for this many of its own widths before the road's edge.
    pub square_widths: f64,
    /// A road that stops within this many of its own widths past a road it crossed is cut back to that road: a stub, not a road of its own.
    pub stub_widths: f64,
    /// A road nearly alongside is not one an end runs into (20°).
    pub alongside_cos: f64,
    /// A road leaves the map square to its edge over twice its width.
    pub gate_widths: f64,
    /// A road cut back to a settlement stops this far past the last ground it runs along, and is walked back to it in steps this long.
    pub trim_margin_m: f64,
    /// How many times the ways of a lost joint are pinned and the rest closed again, before the plan's roads are left as they were laid.
    pub pin_rounds: usize,
    /// No end is cut back, and no gate laid, to leave a run shorter than this.
    pub shortest_run_m: f64,
}

/// Extracted generation policy. The initial values preserve the existing plan.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TownGeometry {
    /// A line must enter a piece of ground this far to cut it: nearer its edge it runs along that edge.
    pub sliver_m: f64,
    /// A road must run this far through a piece of ground to cut it, and two pieces must share this much edge to be neighbours.
    pub shared_m: f64,
    /// Two carriageways within this of each other meet.
    pub meet_m: f64,
    /// How many places either side of the one drawn a cut is tried at, to end clear of the junctions on the cuts it ends on.
    pub places: usize,
    /// Two cuts that end on a third from its two sides make a crossroads when they run within this of one line (the sine of 25 degrees).
    pub in_line_sin: f64,
    /// An avenue meets a road no farther off square than this (the sine of 22 degrees): at more of a slant it stops at its last corner before it.
    pub avenue_slant_sin: f64,
}

/// Extracted generation policy. The initial values preserve the existing plan.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreetGeometry {
    /// A street runs to an end that faces it when that end is no more than this far past the first carriageway it would cross.
    pub facing_past_m: f64,
    /// Two carriageways within this of parallel run the same way.
    pub parallel_cos: f64,
    /// Two streets that stop end to end run opposite ways within this (60 degrees).
    pub facing_cos: f64,
    /// A street comes to a carriageway on its own line no farther off square than this (the sine of 20 degrees).
    pub slant_sin: f64,
    /// One that would come to it farther off square, up to this (the sine of 28 degrees), turns at its last crossing to meet it square: a bend, where a sharper turn would be a hook. At more of a slant it does not meet it.
    pub turn_sin: f64,
    /// A street that turns to meet a carriageway square runs at least this many of its widths from the turn to the carriageway.
    pub turn_widths: f64,
    /// Two streets that meet a road from its two sides make a crossroads when they run opposite ways within this (25 degrees).
    pub in_line_cos: f64,
    /// A street that stops in the open looks this far to either side of its line for the carriageway it stops short of (20 degrees).
    pub ahead_spread: f64,
    /// How much an edge of a district counts for when the district picks the line of its streets: twice its length where a road runs along it, a quarter where no carriageway does.
    pub road_edge: f64,
    /// See the owning source for its geometric use.
    pub open_edge: f64,
    /// A bowed street's wave is this many times its district's length, or the preset's wavelength where that is longer: one bend or less from end to end, never a ripple.
    pub bow_lengths: [f64; 2],
    /// And it swings no farther off its line than this share of that length, so a short street bends as gently as a long one.
    pub bow_reach: f64,
    /// A street that is one run from a crossing to the road ahead is at least this many widths long: shorter, it is a connector between two streets that already lie side by side.
    pub alone_widths: f64,
    /// Two streets of one grid come to a carriageway no nearer each other than this many of their widths.
    pub crowd_widths: f64,
    /// A link runs to a node of its own grid that lies no farther off its line than this share of the way there.
    pub own_tan: f64,
    /// A street stopped in the open turns to another's end that lies no farther off its line than this share of the way to it (25 degrees).
    pub corner_tan: f64,
    /// A street that ends in the open stops this far inside its district.
    pub end_margin_m: f64,
}
