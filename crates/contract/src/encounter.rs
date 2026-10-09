//! Encounter placement on a compiled map: what a recipe asks for, where the
//! map offers it, and the legal placement a planner returns. A recipe is
//! intent (a roster, each row's post, an objective preference), never
//! coordinates; the result is the scenario's encounter half, with what a
//! reviewer needs to see why it stands where it does.
//!
//! The planner is the simulation's (`sim::encounter`): legality is asked of
//! the terrain, bodies, water, navigation and garrison seats a battle runs on.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::identity::Seed;
use crate::ids::Side;
use crate::scenario::{EncounterRules, Opponent, ScriptedOrder, UnitSetup};

/// The half of the playable area north (`Top`) or south of its midline, and
/// the map edge that bounds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Half {
    Top,
    Bottom,
}

impl Half {
    pub fn opposite(self) -> Half {
        match self {
            Half::Top => Half::Bottom,
            Half::Bottom => Half::Top,
        }
    }
}

/// Open ground beside a settlement, with no settlement, forest or water on
/// it. Along every bearing from `from_rad` to `to_rad` (counter-clockwise
/// from +X) a corridor `front_m` wide about the line from the settlement's
/// centre is open for `depth_m`, from the last of the settlement's own
/// ground inside the corridor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approach {
    /// Index into the settlements.
    pub settlement: usize,
    pub half: Half,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub from_rad: f64,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub to_rad: f64,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub depth_m: f64,
    /// The corridor's width.
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub front_m: f64,
}

/// Where a map's settlements and measured open approaches are: what the
/// layout knows that the compiled map does not carry. Generation hands it
/// out beside the map; the planner reads nothing else of a plan.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterSites {
    /// The main settlement first.
    pub settlements: Vec<SettlementSite>,
    pub approaches: Vec<Approach>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skirmish: Option<SkirmishSites>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkirmishSites {
    pub entries: [EntrySite; 2],
    pub objectives: Vec<ObjectiveSite>,
    /// Reserved alternatives; simulation admission publishes the selected objectives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<ObjectiveSite>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntrySite {
    pub side: crate::ids::Side,
    pub center: [f64; 2],
    pub yaw: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveSite {
    pub id: String,
    pub center: [f64; 2],
    pub radius_m: f64,
    pub kind: ObjectiveSiteKind,
    pub counterpart: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveSiteKind {
    Junction,
    Field,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementSite {
    pub id: String,
    /// Where its main streets meet: a point of its roads.
    #[serde(deserialize_with = "crate::numbers::array")]
    pub center: [f64; 2],
    /// The edge of its built ground: a simple ring.
    #[serde(deserialize_with = "crate::numbers::points")]
    pub outline: Vec<[f64; 2]>,
    /// Its built blocks, nearest `center` first.
    pub districts: Vec<DistrictSite>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistrictSite {
    pub id: String,
    #[serde(deserialize_with = "crate::numbers::points")]
    pub ring: Vec<[f64; 2]>,
    #[serde(deserialize_with = "crate::numbers::scalar")]
    pub area_m2: f64,
}

impl SettlementSite {
    pub fn area_m2(&self) -> f64 {
        self.districts.iter().map(|d| d.area_m2).sum()
    }
}

impl EncounterSites {
    /// Sites a planner can index: finite geometry, and every approach names
    /// a settlement.
    pub fn validate(&self) -> Result<(), String> {
        let finite = |points: &[[f64; 2]]| points.iter().flatten().all(|v| v.is_finite());
        for (i, s) in self.settlements.iter().enumerate() {
            if s.id.is_empty() || !finite(&[s.center]) || !finite(&s.outline) {
                return Err(format!("settlements[{i}] needs an id and finite geometry"));
            }
            if s.outline.len() < 3 {
                return Err(format!("settlements[{i}].outline is not a ring"));
            }
            for (j, d) in s.districts.iter().enumerate() {
                if d.id.is_empty() || d.ring.len() < 3 || !finite(&d.ring) || !d.area_m2.is_finite()
                {
                    return Err(format!(
                        "settlements[{i}].districts[{j}] needs an id and a finite ring"
                    ));
                }
            }
        }
        for (i, a) in self.approaches.iter().enumerate() {
            if a.settlement >= self.settlements.len() {
                return Err(format!("approaches[{i}] names no settlement"));
            }
            if ![a.from_rad, a.to_rad, a.depth_m, a.front_m]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err(format!("approaches[{i}] is not finite"));
            }
        }
        if let Some(sites) = &self.skirmish {
            sites.validate()?;
        }

        Ok(())
    }
}

impl SkirmishSites {
    pub fn all_reserved_objectives(&self) -> impl Iterator<Item = &ObjectiveSite> {
        self.objectives.iter().chain(&self.candidates)
    }
    pub fn validate(&self) -> Result<(), String> {
        let finite = |points: &[[f64; 2]]| points.iter().flatten().all(|v| v.is_finite());
        if self.entries[0].side != crate::ids::Side::Blue
            || self.entries[1].side != crate::ids::Side::Red
            || self
                .entries
                .iter()
                .any(|e| !finite(&[e.center]) || !e.yaw.is_finite())
        {
            return Err("skirmish entries require finite blue/red geometry".into());
        }
        if !matches!(self.objectives.len(), 3 | 5 | 7) {
            return Err("skirmish requires three, five or seven objectives".into());
        }
        if self.candidates.len() > 24 {
            return Err("skirmish reserves at most 24 alternatives".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for o in self.all_reserved_objectives() {
            if o.id.is_empty() || !ids.insert(&o.id) {
                return Err("skirmish objective identity is empty or duplicated".into());
            }
            if !finite(&[o.center]) || !o.radius_m.is_finite() || o.radius_m <= 0.0 {
                return Err(format!(
                    "objective {} requires finite positive geometry",
                    o.id
                ));
            }
        }
        for o in self.all_reserved_objectives() {
            if let Some(id) = &o.counterpart {
                if !self
                    .all_reserved_objectives()
                    .any(|p| &p.id == id && p.id != o.id && p.counterpart.as_ref() == Some(&o.id))
                {
                    return Err(format!("objective {} has no reciprocal counterpart", o.id));
                }
            }
        }
        let central = self
            .objectives
            .iter()
            .filter(|o| o.counterpart.is_none())
            .count();
        if central != if self.objectives.len() == 7 { 3 } else { 1 } {
            return Err(
                "skirmish requires one central objective, or three for seven-site maps".into(),
            );
        }
        Ok(())
    }
}

/// Where a roster row starts the battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Post {
    /// In its side's column, on the road in from its side's map edge.
    Column,
    /// A squad at the door of a building of the objective settlement, which
    /// it is ordered into as the battle starts: the whole squad within the
    /// building's seats, one squad to a building.
    Garrison,
    /// At the objective settlement's edge on a way in the attacker may
    /// take, facing down it: the road the attacker's column drives in by
    /// first, then the open approaches on the attacker's side.
    Overwatch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterRow {
    /// A unit type's catalog id.
    pub kind: String,
    pub post: Post,
    /// Its starting fire policy; fire at will when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engagement: Option<crate::command::Engagement>,
}

/// Which settlement the objective is put on first; the rest follow, largest
/// first, while attempts last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementPreference {
    /// The map's main settlement.
    Main,
    /// The largest settlement whose centre lies in the attacker's half.
    AttackerHalf,
    /// The largest settlement whose centre lies in the defender's half.
    DefenderHalf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    Required,
    Preferred,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveRecipe {
    pub settlement: SettlementPreference,
    /// Whether the objective settlement must have a measured open approach
    /// on the attacker's side (one whose bearing points toward the
    /// attacker's edge): open ground the attacker may cross at range.
    pub open_approach: Requirement,
    /// The capture zone, about the settlement's centre.
    pub zone_radius_m: f64,
    pub hold_s: f64,
    pub max_assessment_s: f64,
}

/// The two columns. Each stands on the road that meets its side's edge and
/// drives soonest to the objective: its tail `edge_inset_m` in from the
/// edge, each unit `spacing_m` ahead of the next, the first row leading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRecipe {
    pub edge_inset_m: f64,
    pub spacing_m: f64,
    /// A column whose ground is not legal, or whose drive is longer than
    /// the other's, moves up its road this far at a time.
    pub step_m: f64,
    /// The farthest a column moves up its road from `edge_inset_m`.
    pub max_advance_m: f64,
    /// The unit type whose drive from the head of each column to the
    /// objective is the measure of a fair start.
    pub pace: String,
    /// The two drives differ by no more than this.
    pub max_route_difference_s: f64,
    /// Whether the defender's column is ordered to the objective as the
    /// battle starts (its policy holds what it garrisons and orders nothing
    /// else forward).
    pub defender_advances: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GarrisonRecipe {
    /// A garrisoned building stands within this of the objective's centre.
    pub reach_m: f64,
    /// Garrisoned buildings stand at least this far apart.
    pub apart_m: f64,
    /// The squad starts this far out from the building's door.
    pub door_standoff_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverwatchRecipe {
    /// A post stands this far beyond the settlement's last ground.
    pub standoff_m: f64,
    /// Posts on one avenue stand this far apart, across it; a post beside
    /// a road stands this far from its centreline.
    pub apart_m: f64,
    /// How far down its avenue a post looks: of the places tried, it takes
    /// the one that sees farthest, up to this.
    pub sight_m: f64,
}

/// The opponent policy's numbers (`scenario::Opponent`); its fallback points
/// are the planner's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefenderRecipe {
    pub at_attack_range_m: f64,
    pub tank_retreat_hp_fraction: f64,
    pub infantry_retreat_survivor_fraction: f64,
}

/// The most candidates each search tries before the planner gives the
/// placement up with a diagnostic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptLimits {
    /// Settlements tried as the objective.
    pub objectives: u32,
    /// Edge roads tried for each column.
    pub roads: u32,
    /// Buildings tried for each garrison row.
    pub buildings: u32,
    /// Places tried for each overwatch row.
    pub posts: u32,
}

/// One encounter's intent: who attacks from which edge, each side's roster
/// and where its rows start, and what the objective should be. No
/// coordinates: the planner finds them on the map it is given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterRecipe {
    /// The side that must take the objective. The other is the opponent
    /// policy's.
    pub attacker: Side,
    /// The map edge the attacker's column starts on; the defender's starts
    /// on the other.
    pub attacker_edge: Half,
    /// Blue's rows, then red's: a unit's id is its row's place in that order.
    pub forces: Forces,
    pub objective: ObjectiveRecipe,
    pub deployment: DeploymentRecipe,
    pub garrison: GarrisonRecipe,
    pub overwatch: OverwatchRecipe,
    pub defender: DefenderRecipe,
    /// Unit footprints keep this far from each other.
    pub clearance_m: f64,
    pub attempts: AttemptLimits,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forces {
    pub blue: Vec<RosterRow>,
    pub red: Vec<RosterRow>,
}

impl Forces {
    pub fn side(&self, side: Side) -> &[RosterRow] {
        match side {
            Side::Blue => &self.blue,
            Side::Red => &self.red,
        }
    }
}

impl EncounterRecipe {
    pub fn defender(&self) -> Side {
        match self.attacker {
            Side::Blue => Side::Red,
            Side::Red => Side::Blue,
        }
    }

    /// The edge `side`'s column starts on.
    pub fn edge(&self, side: Side) -> Half {
        if side == self.attacker {
            self.attacker_edge
        } else {
            self.attacker_edge.opposite()
        }
    }

    /// What can be judged without a map or a catalog: numbers in range, a
    /// roster on each side, and posts a side can take. Each message names
    /// the field.
    pub fn validate(&self) -> Result<(), String> {
        let positive = |name: &str, v: f64| {
            (v.is_finite() && v > 0.0)
                .then_some(())
                .ok_or(format!("{name} must be finite and positive"))
        };
        let distance = |name: &str, v: f64| {
            (v.is_finite() && v >= 0.0)
                .then_some(())
                .ok_or(format!("{name} must be finite and not negative"))
        };
        let fraction = |name: &str, v: f64| {
            (0.0..=1.0)
                .contains(&v)
                .then_some(())
                .ok_or(format!("{name} must be within 0..=1"))
        };
        positive("objective.zone_radius_m", self.objective.zone_radius_m)?;
        positive("objective.hold_s", self.objective.hold_s)?;
        positive(
            "objective.max_assessment_s",
            self.objective.max_assessment_s,
        )?;
        distance("deployment.edge_inset_m", self.deployment.edge_inset_m)?;
        positive("deployment.spacing_m", self.deployment.spacing_m)?;
        positive("deployment.step_m", self.deployment.step_m)?;
        distance("deployment.max_advance_m", self.deployment.max_advance_m)?;
        distance(
            "deployment.max_route_difference_s",
            self.deployment.max_route_difference_s,
        )?;
        if self.deployment.pace.is_empty() {
            return Err("deployment.pace must name a unit type".into());
        }
        positive("garrison.reach_m", self.garrison.reach_m)?;
        distance("garrison.apart_m", self.garrison.apart_m)?;
        positive("garrison.door_standoff_m", self.garrison.door_standoff_m)?;
        distance("overwatch.standoff_m", self.overwatch.standoff_m)?;
        positive("overwatch.apart_m", self.overwatch.apart_m)?;
        distance("overwatch.sight_m", self.overwatch.sight_m)?;
        positive(
            "defender.at_attack_range_m",
            self.defender.at_attack_range_m,
        )?;
        fraction(
            "defender.tank_retreat_hp_fraction",
            self.defender.tank_retreat_hp_fraction,
        )?;
        fraction(
            "defender.infantry_retreat_survivor_fraction",
            self.defender.infantry_retreat_survivor_fraction,
        )?;
        distance("clearance_m", self.clearance_m)?;
        for (name, v) in [
            ("attempts.objectives", self.attempts.objectives),
            ("attempts.roads", self.attempts.roads),
            ("attempts.buildings", self.attempts.buildings),
            ("attempts.posts", self.attempts.posts),
        ] {
            if v == 0 {
                return Err(format!("{name} must be positive: nothing would be tried"));
            }
        }
        for side in Side::ALL {
            let name = side_name(side);
            let rows = self.forces.side(side);
            if rows.is_empty() {
                return Err(format!("forces.{name} has no rows"));
            }
            for (i, row) in rows.iter().enumerate() {
                if row.kind.is_empty() {
                    return Err(format!("forces.{name}[{i}].kind must name a unit type"));
                }
                // The attacker comes from its edge: holding the objective
                // from the start is the defender's.
                if side == self.attacker && row.post != Post::Column {
                    return Err(format!(
                        "forces.{name}[{i}].post must be column: the attacker starts at its edge"
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::Blue => "blue",
        Side::Red => "red",
    }
}

/// The recipes file (`fixtures/encounters.json`): named recipes under one
/// revision, which a prepared encounter records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterRecipes {
    pub revision: String,
    pub recipes: BTreeMap<String, EncounterRecipe>,
}

impl EncounterRecipes {
    /// The file's recipes, each valid; the error names the recipe and field.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let file: EncounterRecipes = serde_json::from_str(text).map_err(|e| e.to_string())?;
        crate::identity::validate_version_identifier(&file.revision)
            .map_err(|_| "revision must be nonempty".to_string())?;
        if file.recipes.is_empty() {
            return Err("recipes is empty".into());
        }
        for (id, recipe) in &file.recipes {
            recipe.validate().map_err(|e| format!("recipes.{id}.{e}"))?;
        }
        Ok(file)
    }
}

/// The scenario's encounter half: with a map and rules it is the
/// `ScenarioDefinition` a battle runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterSetup {
    pub units: Vec<UnitSetup>,
    pub scripts: Vec<ScriptedOrder>,
    pub opponent: Opponent,
    pub encounter: EncounterRules,
}

impl EncounterSetup {
    /// The scenario a battle runs: this encounter on the map it was planned
    /// for, under the rules it was planned with.
    pub fn scenario(
        self,
        map: crate::map::MapDefinition,
        rules: crate::scenario::Rules,
    ) -> crate::scenario::ScenarioDefinition {
        crate::scenario::ScenarioDefinition {
            skirmish: None,
            map,
            rules,
            units: self.units,
            events: Vec::new(),
            scripts: self.scripts,
            opponent: Some(self.opponent),
            encounter: Some(self.encounter),
        }
    }
}

/// Where one side's column stands and what its drive to the objective is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    pub side: Side,
    pub edge: Half,
    /// The column's units, leader first.
    pub units: Vec<u32>,
    /// The leader's place and heading.
    pub head: [f64; 2],
    pub yaw: f64,
    /// How far the column was moved up its road from the recipe's inset.
    pub advance_m: f64,
    /// The pace unit's drive from `head` to the objective.
    pub route_s: f64,
    pub route_m: f64,
    pub route: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    /// The settlement it stands on (`EncounterSites`).
    pub settlement: String,
    pub center: [f64; 2],
    pub radius_m: f64,
}

/// A squad and the building it is ordered into.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GarrisonPost {
    pub unit: u32,
    /// The building's owner prop id in the compiled map.
    pub building: u32,
    /// The district the building stands in.
    pub district: String,
    pub soldiers: u32,
    /// The seats the building offers a squad.
    pub seats: u32,
}

/// An overwatch post and the approach it covers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverwatchPost {
    pub unit: u32,
    pub at: [f64; 2],
    /// The bearing it watches down.
    pub yaw: f64,
    /// The measured open approach it covers (an index into
    /// `EncounterSites::approaches`); none when it covers the road the
    /// attacker's column drives in by.
    pub approach: Option<usize>,
}

/// Why the encounter stands where it does: what an overlay draws and a
/// report tabulates. Nothing here is read by the battle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub objective: Objective,
    /// The attacker's column, then the defender's if it has one.
    pub deployments: Vec<Deployment>,
    pub garrisons: Vec<GarrisonPost>,
    pub overwatch: Vec<OverwatchPost>,
    /// Candidates tried, over every search.
    pub attempts: u32,
}

/// A legal encounter on one compiled map, and the inputs that made it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterDefinition {
    /// The recipe's content hash (`identity::json_hash`).
    pub recipe_hash: String,
    pub encounter_seed: Seed,
    pub setup: EncounterSetup,
    pub placement: Placement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterDiagnosticCode {
    /// The recipe names a unit type the catalog lacks, posts a row where
    /// its type cannot stand, or holds a number out of range.
    InvalidRecipe,
    /// The sites do not describe this map.
    InvalidSites,
    /// No settlement can be the objective; the reasons follow, one per
    /// settlement tried.
    NoObjective,
    /// The settlement has no measured open approach on the attacker's side.
    NoOpenApproach,
    /// No road meets the side's map edge.
    NoEdgeRoad,
    /// A column has no legal ground on its road: bodies, water or the other
    /// units fill every place tried.
    NoDeployment,
    /// A unit has no route from where it would start to the objective.
    UnreachableObjective,
    /// The two columns' drives to the objective cannot be brought within
    /// the recipe's difference.
    UnfairDeployment,
    /// No building within reach of the objective admits the squad whole
    /// with legal ground at its door.
    NoGarrisonBuilding,
    /// No legal ground for an overwatch post.
    NoOverwatchPost,
}

/// Why a placement was refused. `feature` is the recipe field or site the
/// refusal is about; `location` is a JSON path into the planner's inputs
/// (`$.recipe.…`, `$.sites`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterDiagnostic {
    pub code: EncounterDiagnosticCode,
    pub feature: Option<String>,
    pub location: String,
    pub message: String,
}

/// The unit vector of a bearing, counter-clockwise from +X. One software
/// evaluator, so placements agree to the bit on every target.
pub fn heading(yaw: f64) -> [f64; 2] {
    let (s, c) = libm::sincos(yaw);
    [c, s]
}

/// The bearing of a direction, counter-clockwise from +X.
pub fn bearing(direction: [f64; 2]) -> f64 {
    libm::atan2(direction[1], direction[0])
}
