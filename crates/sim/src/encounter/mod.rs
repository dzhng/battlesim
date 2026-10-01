//! The encounter planner: one recipe, placed legally on any compiled map.
//!
//! ```text
//! plan_encounter(map queries, rules, recipe, encounter seed)
//!   → EncounterDefinition | diagnostics
//! ```
//!
//! The two sides start at the top and the bottom of the map. Each side's
//! column stands on the road that meets its edge and drives soonest to the
//! objective, and the two drives are held within the recipe's difference:
//! the side with the longer drive starts further up its road. The objective
//! is a settlement's centre. The defender's garrison rows stand at the doors
//! of buildings round it, each squad whole within its building's seats, and
//! its overwatch rows at the settlement's edge, looking down the ways in:
//! the road the attacker drives in by, and the measured open approaches on
//! the attacker's side.
//!
//! Every place is asked of the same world and navigation a battle runs on
//! ([`legality`]): no rule here says what blocks what. Each search tries a
//! bounded list of candidates in a stable order, so the same inputs give
//! the same result on every target, and a placement that cannot be made
//! legally is a diagnostic, never a battle. The encounter seed chooses among
//! the buildings a garrison may take; nothing else is drawn.
//!
//! The result is the scenario's encounter half
//! ([`contract::encounter::EncounterSetup`]): the same units, opponent
//! policy, objective and scripted orders an authored scenario carries
//! ([`crate::village::scenario`]).
use std::sync::Arc;

use contract::catalog::{TypeIndex, UnitType};
use contract::command::{MoveDirection, Order, RoutePolicy};
use contract::encounter::{
    bearing, heading, side_name, Deployment, EncounterDefinition, EncounterDiagnostic,
    EncounterDiagnosticCode as Code, EncounterOutcome, EncounterRecipe, EncounterSetup,
    EncounterSites, GarrisonPost, Half, Objective, OverwatchPost, Placement, Post, Requirement,
    SettlementPreference,
};
use contract::identity::Seed;
use contract::ids::{Side, UnitId};
use contract::map::MapDefinition;
use contract::scenario::{EncounterRules, Opponent, Rules, ScriptedOrder, UnitSetup};

use crate::math::{v2, v3, V2};
use crate::navigation::{self, Leg, Mobility, NavBase, NavGrid, Plan, RoadNet};
use crate::rng::Rng;
use crate::world::WorldGeometry;

pub mod legality;

use legality::{apart, distance, footprint, stands, Footprint, Illegal};

/// A road counts as meeting the map's edge when an end lies this near it.
const EDGE_REACH_M: f64 = 1.0;
/// The encounter seed's stream for the order garrison buildings are tried in.
const GARRISON_STREAM: u64 = 0x6761_7272_6973_6f6e;
/// An overwatch post's sight down its avenue is judged at this many points.
const SIGHT_STEPS: u32 = 8;
/// The gesture token of the defender column's scripted advance.
const ADVANCE_GESTURE: u64 = 2_000_000;

/// The world and navigation a map is planned on: what a battle on it builds
/// for itself. Preparation that already holds a battle's world hands that
/// one to [`MapQueries`] instead of building this.
pub struct PreparedMap {
    pub world: WorldGeometry,
    pub grid: NavGrid,
    pub roads: RoadNet,
}

impl PreparedMap {
    pub fn new(map: &MapDefinition, rules: &Rules) -> Self {
        let world = WorldGeometry::new(map, rules);
        let roads = RoadNet::build(&world);
        let base = NavBase::build(&world, world.props(), rules.physics.soldier_radius_m);
        PreparedMap {
            grid: NavGrid::new(Arc::new(base)),
            roads,
            world,
        }
    }

    pub fn queries<'a>(
        &'a self,
        map: &'a MapDefinition,
        sites: &'a EncounterSites,
    ) -> MapQueries<'a> {
        MapQueries {
            map,
            sites,
            world: &self.world,
            grid: &self.grid,
            roads: &self.roads,
        }
    }
}

/// Everything the planner may ask of a map: the compiled map and its sites,
/// and the simulation's terrain, bodies, water, navigation and garrison
/// seats over it.
#[derive(Clone, Copy)]
pub struct MapQueries<'a> {
    pub map: &'a MapDefinition,
    pub sites: &'a EncounterSites,
    pub world: &'a WorldGeometry,
    pub grid: &'a NavGrid,
    pub roads: &'a RoadNet,
}

/// The seats a building offers one squad: its perimeter slots, as
/// [`crate::garrison`] seats a squad at them. Zero for a body no squad can
/// hold. The one place the planner asks capacity, whatever seats become.
pub fn garrison_seats(q: &MapQueries, rules: &Rules, owner: u32) -> usize {
    q.world
        .prop(owner)
        .filter(|p| p.body.garrison)
        .map_or(0, |p| crate::garrison::slots(q.world, p, rules).len())
}

fn diagnostic(
    code: Code,
    feature: impl Into<Option<String>>,
    message: String,
) -> EncounterDiagnostic {
    let feature = feature.into();
    EncounterDiagnostic {
        code,
        location: feature
            .as_ref()
            .map_or("$".into(), |f| format!("$.recipe.{f}")),
        feature,
        message,
    }
}

fn cm(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn cm2(p: V2) -> V2 {
    v2(cm(p.x), cm(p.y))
}

fn micro(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn milli(v: f64) -> f64 {
    (v * 1e3).round() / 1e3
}

fn xy(p: V2) -> [f64; 2] {
    [p.x, p.y]
}

fn at(p: [f64; 2]) -> V2 {
    v2(p[0], p[1])
}

/// One roster row as the catalog resolves it. Its unit id is its place in
/// `rows`.
struct Row {
    side: Side,
    /// Its place in its side's roster.
    index: usize,
    kind: TypeIndex,
    post: Post,
    mobility: Mobility,
}

/// A road from a map edge inward: its centreline's samples and the length
/// along it to each.
struct Road {
    points: Vec<V2>,
    along: Vec<f64>,
}

impl Road {
    fn new(points: Vec<V2>) -> Self {
        let mut along = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                total += distance(points[i - 1], *p);
            }
            along.push(total);
        }
        Road { points, along }
    }

    /// The point `metres` along the road from the edge, and the road's
    /// direction there; none past its end.
    fn station(&self, metres: f64) -> Option<(V2, V2)> {
        let i = self.along.partition_point(|&a| a < metres).max(1);
        let (a, b) = (*self.points.get(i - 1)?, *self.points.get(i)?);
        let length = self.along[i] - self.along[i - 1];
        if length <= 0.0 {
            return None;
        }
        let t = (metres - self.along[i - 1]) / length;
        Some((a + (b - a) * t, (b - a) * (1.0 / length)))
    }
}

struct Route {
    /// From the start to the goal.
    points: Vec<V2>,
    seconds: f64,
    metres: f64,
}

#[derive(Clone, Copy)]
struct Station {
    at: V2,
    yaw: f64,
}

/// One side's column on one road.
struct Column {
    side: Side,
    /// Its rows, leader first.
    rows: Vec<usize>,
    road: Road,
    advance: f64,
    stations: Vec<Station>,
    /// The pace unit's drive from the leader's place to the objective.
    route: Route,
}

/// Why a column does not stand at one place along a road.
enum Fault {
    /// The road is shorter than the column reaches.
    RoadEnds,
    Row(usize, Illegal),
}

struct Planner<'a> {
    q: MapQueries<'a>,
    rules: &'a Rules,
    recipe: &'a EncounterRecipe,
    seed: Seed,
    rows: Vec<Row>,
    pace: Mobility,
    attempts: u32,
}

/// A legal encounter for `recipe` on the map `q` answers for, or why there
/// is none.
pub fn plan_encounter(
    q: &MapQueries,
    rules: &Rules,
    recipe: &EncounterRecipe,
    encounter_seed: Seed,
) -> Result<EncounterDefinition, Vec<EncounterDiagnostic>> {
    recipe
        .validate()
        .map_err(|e| vec![diagnostic(Code::InvalidRecipe, None, e)])?;
    q.sites.validate().map_err(|e| {
        vec![EncounterDiagnostic {
            code: Code::InvalidSites,
            feature: None,
            location: "$.sites".into(),
            message: e,
        }]
    })?;
    let mut planner = Planner::new(*q, rules, recipe, encounter_seed)?;
    let order = planner.objective_order();
    if order.is_empty() {
        return Err(vec![diagnostic(
            Code::NoObjective,
            Some("objective.settlement".to_string()),
            "the map's sites hold no settlement to put an objective on".into(),
        )]);
    }
    let mut refusals = Vec::new();
    let tried = order.len().min(recipe.attempts.objectives as usize);
    for &settlement in &order[..tried] {
        planner.attempts += 1;
        match planner.plan_on(settlement) {
            Ok(definition) => return Ok(definition),
            Err(mut why) => refusals.append(&mut why),
        }
    }
    let mut diagnostics = vec![diagnostic(
        Code::NoObjective,
        Some("objective.settlement".to_string()),
        format!(
            "none of the {tried} settlements tried (of {}) takes the encounter",
            order.len()
        ),
    )];
    diagnostics.append(&mut refusals);
    Err(diagnostics)
}

impl<'a> Planner<'a> {
    fn new(
        q: MapQueries<'a>,
        rules: &'a Rules,
        recipe: &'a EncounterRecipe,
        seed: Seed,
    ) -> Result<Self, Vec<EncounterDiagnostic>> {
        let mut problems = Vec::new();
        let mut rows = Vec::new();
        for side in Side::ALL {
            for (index, row) in recipe.forces.side(side).iter().enumerate() {
                let feature = format!("forces.{}[{index}]", side_name(side));
                let Some(kind) = rules.catalog.index(&row.kind) else {
                    problems.push(diagnostic(
                        Code::InvalidRecipe,
                        feature,
                        format!("no unit type {:?} in the catalog", row.kind),
                    ));
                    continue;
                };
                let t = rules.catalog.get(kind);
                if row.post == Post::Garrison && t.hull().is_some() {
                    problems.push(diagnostic(
                        Code::InvalidRecipe,
                        feature,
                        format!(
                            "{:?} is a vehicle: only a squad garrisons a building",
                            row.kind
                        ),
                    ));
                    continue;
                }
                rows.push(Row {
                    side,
                    index,
                    kind,
                    post: row.post,
                    mobility: crate::units::mobility(t, rules),
                });
            }
        }
        let pace = rules.catalog.index(&recipe.deployment.pace);
        if pace.is_none() {
            problems.push(diagnostic(
                Code::InvalidRecipe,
                Some("deployment.pace".to_string()),
                format!("no unit type {:?} in the catalog", recipe.deployment.pace),
            ));
        }
        match pace {
            Some(pace) if problems.is_empty() => Ok(Planner {
                q,
                rules,
                recipe,
                seed,
                rows,
                pace: crate::units::mobility(rules.catalog.get(pace), rules),
                attempts: 0,
            }),
            _ => Err(problems),
        }
    }

    fn unit_type(&self, row: usize) -> &UnitType {
        self.rules.catalog.get(self.rows[row].kind)
    }

    fn feature(&self, row: usize) -> String {
        let r = &self.rows[row];
        format!("forces.{}[{}]", side_name(r.side), r.index)
    }

    fn kind_name(&self, row: usize) -> &str {
        let r = &self.rows[row];
        &self.recipe.forces.side(r.side)[r.index].kind
    }

    /// The rows of `side` at `post`, in roster order.
    fn posted(&self, side: Side, post: Post) -> Vec<usize> {
        (0..self.rows.len())
            .filter(|&r| self.rows[r].side == side && self.rows[r].post == post)
            .collect()
    }

    fn half_of(&self, p: [f64; 2]) -> Half {
        if p[1] >= self.q.world.depth() / 2.0 {
            Half::Top
        } else {
            Half::Bottom
        }
    }

    /// Whether an open approach lies on the attacker's side of its
    /// settlement: its bearing points toward the edge the attacker starts on.
    fn faces_attacker(&self, approach: &contract::encounter::Approach) -> bool {
        let north = heading(approach_bearing(approach.from_rad, approach.to_rad))[1];
        match self.recipe.attacker_edge {
            Half::Bottom => north < 0.0,
            Half::Top => north > 0.0,
        }
    }

    /// The settlements in the order they are tried as the objective: the
    /// recipe's preference first, then those with a measured open approach
    /// on the attacker's side, then the largest, then plan order.
    fn objective_order(&self) -> Vec<usize> {
        let settlements = &self.q.sites.settlements;
        let attacker = self.recipe.attacker_edge;
        let preferred = |i: usize| match self.recipe.objective.settlement {
            SettlementPreference::Main => i == 0,
            SettlementPreference::AttackerHalf => self.half_of(settlements[i].center) == attacker,
            SettlementPreference::DefenderHalf => self.half_of(settlements[i].center) != attacker,
        };
        let open = |i: usize| {
            self.q
                .sites
                .approaches
                .iter()
                .any(|a| a.settlement == i && self.faces_attacker(a))
        };
        let mut order: Vec<usize> = (0..settlements.len()).collect();
        order.sort_by(|&a, &b| {
            preferred(b)
                .cmp(&preferred(a))
                .then(open(b).cmp(&open(a)))
                .then(
                    settlements[b]
                        .area_m2()
                        .total_cmp(&settlements[a].area_m2()),
                )
                .then(a.cmp(&b))
        });
        order
    }

    /// The fastest route navigation plans for `m` from `from` into the
    /// capture zone about `centre`: its waypoints after `from`, the last
    /// inside the zone. None when the planner finds no route, or one that
    /// stops short of the zone.
    fn route_into_zone(&self, from: V2, centre: V2, m: &Mobility) -> Option<Vec<V2>> {
        let leg = Leg {
            from,
            goal: centre,
            m,
            policy: RoutePolicy::Fastest,
            avoid: &[],
        };
        let (plan, _) = navigation::plan(self.q.grid, self.q.roads, leg, &self.rules.navigation);
        let Plan::Route(points) = plan else {
            return None;
        };
        points
            .last()
            .is_some_and(|&end| distance(end, centre) <= self.recipe.objective.zone_radius_m)
            .then_some(points)
    }

    /// The pace unit's drive from `from` into the zone about `centre`.
    fn drive(&self, from: V2, centre: V2) -> Option<Route> {
        let points = self.route_into_zone(from, centre, &self.pace)?;
        let seconds = self.q.grid.route_time(from, &points, &self.pace);
        let mut metres = 0.0;
        let mut last = from;
        for &p in &points {
            metres += distance(last, p);
            last = p;
        }
        let mut all = vec![from];
        all.extend(points);
        seconds.is_finite().then_some(Route {
            points: all,
            seconds,
            metres,
        })
    }

    /// The roads with an end on `edge`, each from that end inward, in the
    /// map's surface order.
    fn edge_roads(&self, edge: Half) -> Vec<Road> {
        let depth = self.q.world.depth();
        let on_edge = |p: &[f64; 2]| match edge {
            Half::Bottom => p[1] <= EDGE_REACH_M,
            Half::Top => p[1] >= depth - EDGE_REACH_M,
        };
        self.q
            .world
            .road_strokes()
            .filter_map(|(samples, _, _)| {
                let (first, last) = (samples.first()?, samples.last()?);
                let points: Vec<V2> = if on_edge(first) {
                    samples.iter().map(|&p| at(p)).collect()
                } else if on_edge(last) {
                    samples.iter().rev().map(|&p| at(p)).collect()
                } else {
                    return None;
                };
                (points.len() >= 2).then(|| Road::new(points))
            })
            .collect()
    }

    /// Where `rows` stand in column on `road`, moved `advance` up it from
    /// the recipe's inset: the tail nearest the edge, each row a spacing
    /// ahead of the next, heading along the road. Every station is legal
    /// ground for its unit, clear of the stations before it and of `others`.
    fn stations(
        &self,
        rows: &[usize],
        road: &Road,
        advance: f64,
        others: &[Footprint],
    ) -> Result<Vec<Station>, Fault> {
        let d = &self.recipe.deployment;
        let mut taken: Vec<Footprint> = others.to_vec();
        let mut stations = Vec::with_capacity(rows.len());
        for (k, &row) in rows.iter().enumerate() {
            let metres = d.edge_inset_m + advance + (rows.len() - 1 - k) as f64 * d.spacing_m;
            let (p, direction) = road.station(metres).ok_or(Fault::RoadEnds)?;
            let station = Station {
                at: cm2(p),
                yaw: micro(bearing(xy(direction))),
            };
            let shape = self
                .legal(row, station, &taken)
                .map_err(|why| Fault::Row(row, why))?;
            taken.push(shape);
            stations.push(station);
        }
        Ok(stations)
    }

    /// The row's footprint at `station` when the map lets it stand there
    /// clear of `taken`.
    fn legal(
        &self,
        row: usize,
        station: Station,
        taken: &[Footprint],
    ) -> Result<Footprint, Illegal> {
        let t = self.unit_type(row);
        stands(
            &self.q,
            self.rules,
            t,
            &self.rows[row].mobility,
            station.at,
            station.yaw,
        )?;
        let shape = footprint(t, self.rules, station.at, station.yaw);
        if taken
            .iter()
            .all(|other| apart(&shape, other, self.recipe.clearance_m))
        {
            Ok(shape)
        } else {
            Err(Illegal::Unit)
        }
    }

    /// The first legal place for the column on `road` at or beyond
    /// `advance`, a step at a time, with the pace unit's drive from its
    /// leader into the zone about `centre`.
    fn column_on(
        &mut self,
        side: Side,
        rows: &[usize],
        road: Road,
        mut advance: f64,
        centre: V2,
        others: &[Footprint],
    ) -> Result<Column, EncounterDiagnostic> {
        let d = &self.recipe.deployment;
        let feature = format!("forces.{}", side_name(side));
        let mut last = None;
        while advance <= d.max_advance_m {
            self.attempts += 1;
            match self.stations(rows, &road, advance, others) {
                Ok(stations) => {
                    let Some(route) = self.drive(stations[0].at, centre) else {
                        return Err(diagnostic(
                            Code::UnreachableObjective,
                            feature,
                            format!(
                                "a {} has no route from the head of the column at ({}, {}) into the objective zone",
                                d.pace, stations[0].at.x, stations[0].at.y
                            ),
                        ));
                    };
                    return Ok(Column {
                        side,
                        rows: rows.to_vec(),
                        road,
                        advance,
                        stations,
                        route,
                    });
                }
                Err(Fault::RoadEnds) => {
                    last = None;
                    break;
                }
                Err(Fault::Row(row, why)) => last = Some((row, why)),
            }
            advance += d.step_m;
        }
        Err(match last {
            Some((row, why)) => diagnostic(
                Code::NoDeployment,
                self.feature(row),
                format!(
                    "no legal place for the column within {} m up its road: for the {} at the last place tried, {}",
                    d.max_advance_m,
                    self.kind_name(row),
                    why.describe()
                ),
            ),
            None => diagnostic(
                Code::NoDeployment,
                feature,
                "the road from the edge is shorter than the column".into(),
            ),
        })
    }

    /// `side`'s column on the edge road its pace unit drives soonest from.
    fn column(
        &mut self,
        side: Side,
        centre: V2,
        others: &[Footprint],
    ) -> Result<Option<Column>, Vec<EncounterDiagnostic>> {
        let rows = self.posted(side, Post::Column);
        if rows.is_empty() {
            return Ok(None);
        }
        let edge = self.recipe.edge(side);
        let roads = self.edge_roads(edge);
        if roads.is_empty() {
            return Err(vec![diagnostic(
                Code::NoEdgeRoad,
                format!("forces.{}", side_name(side)),
                format!("no road meets the {edge:?} edge of the map"),
            )]);
        }
        let mut best: Option<Column> = None;
        let mut refusals = Vec::new();
        for road in roads.into_iter().take(self.recipe.attempts.roads as usize) {
            match self.column_on(side, &rows, road, 0.0, centre, others) {
                Ok(column) => {
                    if best
                        .as_ref()
                        .is_none_or(|b| column.route.seconds < b.route.seconds)
                    {
                        best = Some(column);
                    }
                }
                Err(why) => refusals.push(why),
            }
        }
        best.map(Some).ok_or(refusals)
    }

    /// The footprints of a column's units.
    fn shapes(&self, column: &Column) -> Vec<Footprint> {
        column
            .rows
            .iter()
            .zip(&column.stations)
            .map(|(&row, s)| footprint(self.unit_type(row), self.rules, s.at, s.yaw))
            .collect()
    }

    /// Bring the two columns' drives within the recipe's difference: the
    /// side with the longer drive moves up its road.
    fn level(
        &mut self,
        mut attacker: Column,
        mut defender: Column,
        centre: V2,
    ) -> Result<(Column, Column), Vec<EncounterDiagnostic>> {
        let d = &self.recipe.deployment;
        let rounds = (d.max_advance_m / d.step_m).ceil() as usize + 1;
        for _ in 0..rounds {
            let lead = attacker.route.seconds - defender.route.seconds;
            if lead.abs() <= d.max_route_difference_s {
                return Ok((attacker, defender));
            }
            let (slow, fast) = if lead > 0.0 {
                (&mut attacker, &defender)
            } else {
                (&mut defender, &attacker)
            };
            let speed = slow.route.metres / slow.route.seconds;
            let steps = ((lead.abs() * speed) / d.step_m).round().max(1.0);
            let advance = slow.advance + steps * d.step_m;
            if advance > d.max_advance_m {
                break;
            }
            let others = self.shapes(fast);
            let road = Road {
                points: std::mem::take(&mut slow.road.points),
                along: std::mem::take(&mut slow.road.along),
            };
            let (side, rows) = (slow.side, slow.rows.clone());
            *slow = self
                .column_on(side, &rows, road, advance, centre, &others)
                .map_err(|why| vec![why])?;
        }
        let lead = attacker.route.seconds - defender.route.seconds;
        if lead.abs() <= d.max_route_difference_s {
            return Ok((attacker, defender));
        }
        Err(vec![diagnostic(
            Code::UnfairDeployment,
            Some("deployment.max_route_difference_s".to_string()),
            format!(
                "a {} drives {:.0} s from the {} column and {:.0} s from the {} column, and moving the farther one up to {} m up its road does not bring them within {} s",
                d.pace,
                attacker.route.seconds,
                side_name(attacker.side),
                defender.route.seconds,
                side_name(defender.side),
                d.max_advance_m,
                d.max_route_difference_s
            ),
        )])
    }

    /// The whole encounter with its objective on `settlement`.
    fn plan_on(
        &mut self,
        settlement: usize,
    ) -> Result<EncounterDefinition, Vec<EncounterDiagnostic>> {
        let recipe = self.recipe;
        let site = &self.q.sites.settlements[settlement];
        let centre = at(site.center);
        let (attacker, defender) = (recipe.attacker, recipe.defender());

        // The measured open approaches on the attacker's side.
        let approaches: Vec<usize> = (0..self.q.sites.approaches.len())
            .filter(|&a| {
                let approach = &self.q.sites.approaches[a];
                approach.settlement == settlement && self.faces_attacker(approach)
            })
            .collect();
        if approaches.is_empty() && recipe.objective.open_approach == Requirement::Required {
            return Err(vec![diagnostic(
                Code::NoOpenApproach,
                Some("objective.open_approach".to_string()),
                format!(
                    "{} has no measured open approach toward the {:?} edge",
                    site.id, recipe.attacker_edge
                ),
            )]);
        }

        // The columns, each on its own edge, then levelled.
        let attack = self.column(attacker, centre, &[])?;
        let others = attack.as_ref().map_or(Vec::new(), |c| self.shapes(c));
        let defence = self.column(defender, centre, &others)?;
        let (attack, defence) = match (attack, defence) {
            (Some(a), Some(d)) => {
                let (a, d) = self.level(a, d, centre)?;
                (Some(a), Some(d))
            }
            other => other,
        };

        let mut stations: Vec<Option<Station>> = vec![None; self.rows.len()];
        let mut taken: Vec<Footprint> = Vec::new();
        let mut problems = Vec::new();
        for column in [&attack, &defence].into_iter().flatten() {
            taken.extend(self.shapes(column));
            let mut checked: Vec<TypeIndex> = Vec::new();
            for (&row, &station) in column.rows.iter().zip(&column.stations) {
                stations[row] = Some(station);
                // Each unit type of the column drives or walks into the
                // zone from where its first unit stands.
                let kind = self.rows[row].kind;
                if checked.contains(&kind) {
                    continue;
                }
                checked.push(kind);
                if self
                    .route_into_zone(station.at, centre, &self.rows[row].mobility)
                    .is_none()
                {
                    problems.push(self.unreachable(row, station));
                }
            }
        }
        if !problems.is_empty() {
            return Err(problems);
        }

        let avenues = self.avenues(settlement, &approaches, attack.as_ref());
        let front = self.attack_point(settlement, attack.as_ref()) - centre;
        let garrisons = self.garrisons(settlement, defender, front, &mut stations, &mut taken)?;
        let overwatch =
            self.overwatch(settlement, defender, &avenues, &mut stations, &mut taken)?;

        let units: Vec<UnitSetup> = stations
            .iter()
            .enumerate()
            .map(|(row, station)| {
                let station = station.expect("every row has a post, and every post was placed");
                let r = &self.rows[row];
                let authored = &recipe.forces.side(r.side)[r.index];
                UnitSetup {
                    side: r.side,
                    kind: authored.kind.clone(),
                    position: xy(station.at),
                    yaw: station.yaw,
                    engagement: authored.engagement,
                    condition: None,
                    stock: None,
                }
            })
            .collect();
        let mut scripts = Vec::new();
        if let Some(column) = defence
            .as_ref()
            .filter(|_| recipe.deployment.defender_advances)
        {
            scripts.push(ScriptedOrder {
                tick: 0,
                side: defender,
                order: Order::Move {
                    units: column.rows.iter().map(|&r| UnitId(r as u32)).collect(),
                    gesture: ADVANCE_GESTURE,
                    goal: xy(centre),
                    route: RoutePolicy::Fastest,
                    direction: MoveDirection::Forward,
                    facing: None,
                },
                queued: false,
            });
        }
        let deployment = |column: &Column| Deployment {
            side: column.side,
            edge: recipe.edge(column.side),
            units: column.rows.iter().map(|&r| r as u32).collect(),
            head: xy(column.stations[0].at),
            yaw: column.stations[0].yaw,
            advance_m: column.advance,
            route_s: milli(column.route.seconds),
            route_m: cm(column.route.metres),
            route: column.route.points.iter().map(|&p| xy(cm2(p))).collect(),
        };
        Ok(EncounterDefinition {
            recipe_hash: contract::identity::json_hash(recipe).expect("a recipe serializes"),
            encounter_seed: self.seed,
            setup: EncounterSetup {
                units,
                scripts,
                opponent: Opponent {
                    side: defender,
                    garrisons: garrisons.iter().map(|g| [g.unit, g.building]).collect(),
                    at_attack_range_m: recipe.defender.at_attack_range_m,
                    tank_retreat_hp_fraction: recipe.defender.tank_retreat_hp_fraction,
                    tank_fallback: xy(centre),
                    infantry_retreat_survivor_fraction: recipe
                        .defender
                        .infantry_retreat_survivor_fraction,
                    infantry_fallback: xy(centre),
                },
                encounter: EncounterRules {
                    attacker,
                    success_zone_center: xy(centre),
                    success_zone_radius_m: recipe.objective.zone_radius_m,
                    hold_s: recipe.objective.hold_s,
                    max_assessment_s: recipe.objective.max_assessment_s,
                },
            },
            placement: Placement {
                objective: Objective {
                    settlement: site.id.clone(),
                    center: xy(centre),
                    radius_m: recipe.objective.zone_radius_m,
                },
                deployments: [&attack, &defence]
                    .into_iter()
                    .flatten()
                    .map(deployment)
                    .collect(),
                garrisons,
                overwatch,
                attempts: self.attempts,
            },
        })
    }

    fn unreachable(&self, row: usize, station: Station) -> EncounterDiagnostic {
        diagnostic(
            Code::UnreachableObjective,
            self.feature(row),
            format!(
                "the {} at ({}, {}) has no route into the objective zone",
                self.kind_name(row),
                station.at.x,
                station.at.y
            ),
        )
    }

    /// A building for each of the defender's garrison rows: one within
    /// reach of the objective, in one of the settlement's districts, with a
    /// seat for every soldier, apart from the others chosen, and with legal
    /// ground at a door from which the squad can walk into the zone. The
    /// buildings on the side of the centre the attack comes from (`front`)
    /// are tried first, and the encounter seed orders them within each side.
    fn garrisons(
        &mut self,
        settlement: usize,
        defender: Side,
        front: V2,
        stations: &mut [Option<Station>],
        taken: &mut Vec<Footprint>,
    ) -> Result<Vec<GarrisonPost>, Vec<EncounterDiagnostic>> {
        let rows = self.posted(defender, Post::Garrison);
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let recipe = self.recipe;
        let site = &self.q.sites.settlements[settlement];
        let centre = at(site.center);
        struct Candidate {
            owner: u32,
            centre: V2,
            district: usize,
            seats: usize,
        }
        let mut candidates: Vec<Candidate> = self
            .q
            .map
            .buildings
            .iter()
            .filter_map(|b| {
                let [x, y, _] = b.geometry.frame.translation;
                let middle = v2(x, y);
                if distance(middle, centre) > recipe.garrison.reach_m {
                    return None;
                }
                let district = site
                    .districts
                    .iter()
                    .position(|d| contract::ground::polygon_contains(&d.ring, [x, y]))?;
                let seats = garrison_seats(&self.q, self.rules, b.owner);
                (seats > 0).then_some(Candidate {
                    owner: b.owner,
                    centre: middle,
                    district,
                    seats,
                })
            })
            .collect();
        candidates.sort_by_key(|c| c.owner);
        let mut rng = Rng::new(self.seed.value() ^ GARRISON_STREAM);
        for i in (1..candidates.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            candidates.swap(i, j);
        }
        // The buildings on the attacker's side of the centre come first.
        candidates.sort_by_key(|c| (c.centre - centre).dot(front) < 0.0);

        let mut chosen: Vec<usize> = Vec::new();
        let mut posts = Vec::new();
        for row in rows {
            let soldiers = self.unit_type(row).squad_size();
            let (mut tried, mut short, mut crowded, mut no_ground, mut cut_off) = (0, 0, 0, 0, 0);
            let mut found = None;
            for (c, candidate) in candidates.iter().enumerate() {
                if tried == recipe.attempts.buildings {
                    break;
                }
                if chosen.contains(&c) {
                    continue;
                }
                tried += 1;
                self.attempts += 1;
                if candidate.seats < soldiers {
                    short += 1;
                    continue;
                }
                if chosen.iter().any(|&o| {
                    distance(candidates[o].centre, candidate.centre) < recipe.garrison.apart_m
                }) {
                    crowded += 1;
                    continue;
                }
                let building = self
                    .q
                    .world
                    .building(candidate.owner)
                    .expect("a candidate is one of the map's buildings");
                let door = self.doors(building).into_iter().find_map(|station| {
                    self.legal(row, station, taken.as_slice())
                        .ok()
                        .map(|shape| (station, shape))
                });
                let Some((station, shape)) = door else {
                    no_ground += 1;
                    continue;
                };
                if self
                    .route_into_zone(station.at, centre, &self.rows[row].mobility)
                    .is_none()
                {
                    cut_off += 1;
                    continue;
                }
                found = Some((c, station, shape));
                break;
            }
            let Some((c, station, shape)) = found else {
                return Err(vec![diagnostic(
                    Code::NoGarrisonBuilding,
                    self.feature(row),
                    format!(
                        "no building of {} within {} m of its centre takes the {} of {soldiers}: {} stand in reach, {tried} tried, {short} with too few seats, {crowded} within {} m of one already held, {no_ground} with no legal ground at a door, {cut_off} with no route into the zone",
                        site.id,
                        recipe.garrison.reach_m,
                        self.kind_name(row),
                        candidates.len(),
                        recipe.garrison.apart_m,
                    ),
                )]);
            };
            chosen.push(c);
            stations[row] = Some(station);
            taken.push(shape);
            posts.push(GarrisonPost {
                unit: row as u32,
                building: candidates[c].owner,
                district: site.districts[candidates[c].district].id.clone(),
                soldiers: soldiers as u32,
                seats: candidates[c].seats as u32,
            });
        }
        Ok(posts)
    }

    /// Where a squad may wait at a building: outside each entrance, or
    /// outside the middle of each exposed face when it names no entrance,
    /// facing away from the wall.
    fn doors(&self, building: &contract::map::BuildingDefinition) -> Vec<Station> {
        let out = self.recipe.garrison.door_standoff_m;
        let station = |p: V2, normal: [f64; 2]| Station {
            at: cm2(p + at(normal) * out),
            yaw: micro(bearing(normal)),
        };
        match &building.geometry.entrances {
            Some(entrances) if !entrances.is_empty() => entrances
                .iter()
                .map(|e| station(v2(e.position[0], e.position[1]), e.normal))
                .collect(),
            _ => building
                .geometry
                .edges
                .iter()
                .filter(|e| e.exposed)
                .map(|e| station((at(e.span[0]) + at(e.span[1])) * 0.5, e.normal))
                .collect(),
        }
    }

    /// Where the attack comes at the settlement from: the point of its edge
    /// the attacker's column drives in across; failing that the head of the
    /// column, or the middle of the attacker's map edge.
    fn attack_point(&self, settlement: usize, attack: Option<&Column>) -> V2 {
        let site = &self.q.sites.settlements[settlement];
        let road = attack.and_then(|column| road_entry(&site.outline, &column.route.points));
        match (road, attack) {
            (Some((entry, _)), _) => entry,
            (None, Some(column)) => column.stations[0].at,
            (None, None) => match self.recipe.attacker_edge {
                Half::Bottom => v2(site.center[0], 0.0),
                Half::Top => v2(site.center[0], self.q.world.depth()),
            },
        }
    }

    /// The ways into the settlement the defender watches, the likeliest
    /// first: the road the attacker's column drives in by, where it crosses
    /// the settlement's edge, then the open approaches on the attacker's
    /// side, nearest that road first. With neither, the bearing the
    /// attacker's edge lies on.
    fn avenues(
        &self,
        settlement: usize,
        approaches: &[usize],
        attack: Option<&Column>,
    ) -> Vec<Avenue> {
        let site = &self.q.sites.settlements[settlement];
        let centre = at(site.center);
        let road = attack.and_then(|column| road_entry(&site.outline, &column.route.points));
        let toward = self.attack_point(settlement, attack);
        let attack_bearing = bearing(xy(toward - centre));
        let mut avenues = Vec::new();
        if let Some((entry, outward)) = road {
            avenues.push(Avenue {
                from: entry,
                yaw: micro(bearing(xy(outward))),
                approach: None,
                road: true,
            });
        }
        let mut open: Vec<(usize, f64)> = approaches
            .iter()
            .map(|&a| {
                let approach = &self.q.sites.approaches[a];
                (a, approach_bearing(approach.from_rad, approach.to_rad))
            })
            .collect();
        open.sort_by(|a, b| {
            turn(a.1 - attack_bearing)
                .abs()
                .total_cmp(&turn(b.1 - attack_bearing).abs())
                .then(a.0.cmp(&b.0))
        });
        avenues.extend(open.into_iter().map(|(a, yaw)| Avenue {
            from: centre,
            yaw: micro(turn(yaw)),
            approach: Some(a),
            road: false,
        }));
        if avenues.is_empty() {
            avenues.push(Avenue {
                from: centre,
                yaw: micro(attack_bearing),
                approach: None,
                road: false,
            });
        }
        avenues
    }

    /// A post for each of the defender's overwatch rows, the avenues taken
    /// in turn: at the settlement's edge on its avenue, on legal ground
    /// clear of the others with a route back into the zone, and of the
    /// places tried the one that sees farthest down the avenue. A row whose
    /// avenue offers no such ground within its attempts takes the next.
    fn overwatch(
        &mut self,
        settlement: usize,
        defender: Side,
        avenues: &[Avenue],
        stations: &mut [Option<Station>],
        taken: &mut Vec<Footprint>,
    ) -> Result<Vec<OverwatchPost>, Vec<EncounterDiagnostic>> {
        let recipe = self.recipe;
        let site = &self.q.sites.settlements[settlement];
        let centre = at(site.center);
        // The next place across each avenue: on an approach's own line
        // first, then out to either side in turn; beside a road, never on it.
        let mut next_slot: Vec<u32> = avenues.iter().map(|a| u32::from(a.road)).collect();
        let mut posts = Vec::new();
        for (n, row) in self
            .posted(defender, Post::Overwatch)
            .into_iter()
            .enumerate()
        {
            let m = self.rows[row].mobility;
            let mut found = None;
            let mut why = Illegal::OffMap;
            for k in 0..avenues.len() {
                let lane = (n + k) % avenues.len();
                let avenue = &avenues[lane];
                let direction = at(heading(avenue.yaw));
                let across = v2(-direction.y, direction.x);
                // An approach's line starts at the centre. A road's entry is
                // on the edge already, so its neighbours' lines start far
                // enough back to cross the whole settlement.
                let behind = if avenue.road {
                    site.outline
                        .iter()
                        .map(|&p| distance(avenue.from, at(p)))
                        .fold(0.0, f64::max)
                } else {
                    0.0
                };
                // The legal place that sees farthest down the avenue; the
                // first of those that see equally far.
                let mut best: Option<(u32, Station, Footprint)> = None;
                for _ in 0..recipe.attempts.posts {
                    self.attempts += 1;
                    let slot = next_slot[lane];
                    next_slot[lane] += 1;
                    let sign = if slot % 2 == 1 { 1.0 } else { -1.0 };
                    // The line of this place: parallel to the avenue, from
                    // behind the settlement out through its edge.
                    let from = avenue.from
                        + across * (sign * slot.div_ceil(2) as f64 * recipe.overwatch.apart_m)
                        - direction * behind;
                    let Some(edge) = ring_exit(&site.outline, from, direction) else {
                        continue;
                    };
                    let wanted = from + direction * (edge + recipe.overwatch.standoff_m);
                    let Some(p) = self.q.grid.snap(wanted, &m, recipe.overwatch.standoff_m) else {
                        why = Illegal::NoRoom;
                        continue;
                    };
                    let station = Station {
                        at: cm2(p),
                        yaw: avenue.yaw,
                    };
                    let shape = match self.legal(row, station, taken.as_slice()) {
                        Ok(shape) => shape,
                        Err(illegal) => {
                            why = illegal;
                            continue;
                        }
                    };
                    if self.route_into_zone(station.at, centre, &m).is_none() {
                        why = Illegal::NoRoom;
                        continue;
                    }
                    let sight = self.sight_down(row, station, direction);
                    if best.as_ref().is_none_or(|(seen, _, _)| sight > *seen) {
                        best = Some((sight, station, shape));
                    }
                    if sight == SIGHT_STEPS {
                        break;
                    }
                }
                if let Some((_, station, shape)) = best {
                    found = Some((station, shape, avenue.approach));
                    break;
                }
            }
            let Some((station, shape, approach)) = found else {
                return Err(vec![diagnostic(
                    Code::NoOverwatchPost,
                    self.feature(row),
                    format!(
                        "no post for the {} at the edge of {} within {} places on each of its {} avenues: at the last, {}",
                        self.kind_name(row),
                        site.id,
                        recipe.attempts.posts,
                        avenues.len(),
                        why.describe()
                    ),
                )]);
            };
            stations[row] = Some(station);
            taken.push(shape);
            posts.push(OverwatchPost {
                unit: row as u32,
                at: xy(station.at),
                yaw: station.yaw,
                approach,
            });
        }
        Ok(posts)
    }

    /// How far the unit's eye at `station` sees down `direction`, in
    /// eighths of `overwatch.sight_m` (or of the way to the map's edge, when
    /// that is nearer): the farthest of those points at which it sees a
    /// soldier's middle.
    fn sight_down(&self, row: usize, station: Station, direction: V2) -> u32 {
        let world = self.q.world;
        let reach = |from: f64, step: f64, limit: f64| {
            if step > 0.0 {
                (limit - from) / step
            } else if step < 0.0 {
                -from / step
            } else {
                f64::INFINITY
            }
        };
        let to_edge = reach(station.at.x, direction.x, world.width()).min(reach(
            station.at.y,
            direction.y,
            world.depth(),
        ));
        let far = self.recipe.overwatch.sight_m.min(to_edge - 1.0);
        let physics = &self.rules.physics;
        let eye = self
            .unit_type(row)
            .hull()
            .map_or(physics.infantry_eye_m, |h| h.eye_m);
        let Some(here) = world.height_at(station.at.x, station.at.y) else {
            return 0;
        };
        if far <= 0.0 {
            return SIGHT_STEPS;
        }
        (1..=SIGHT_STEPS)
            .rev()
            .find(|&k| {
                let target = station.at + direction * (far * f64::from(k) / f64::from(SIGHT_STEPS));
                world.height_at(target.x, target.y).is_some_and(|there| {
                    world.sight_clear(
                        v3(station.at.x, station.at.y, here + eye),
                        v3(target.x, target.y, there + physics.infantry_center_m),
                    )
                })
            })
            .unwrap_or(0)
    }
}

/// One way into the objective's settlement that the defender watches.
struct Avenue {
    /// Where its line starts: the settlement's centre for an open approach,
    /// the point of the settlement's edge a road crosses for the road.
    from: V2,
    /// The bearing it is watched down.
    yaw: f64,
    /// The measured approach it is (`EncounterSites::approaches`).
    approach: Option<usize>,
    /// The attacker's road: posts stand beside it, at the edge.
    road: bool,
}

/// Where a route that ends inside `ring` last comes in across it, and the
/// direction back out along the route there.
fn road_entry(ring: &[[f64; 2]], route: &[V2]) -> Option<(V2, V2)> {
    for pair in route.windows(2).rev() {
        let (a, b) = (pair[0], pair[1]);
        let step = b - a;
        let length = distance(a, b);
        if length == 0.0 {
            continue;
        }
        let mut last: Option<f64> = None;
        for (c, d) in contract::ground::edges(ring) {
            let (c, d) = (at(*c), at(*d));
            let side = d - c;
            let cross = step.cross(side);
            if cross == 0.0 {
                continue;
            }
            let t = (c - a).cross(side) / cross;
            let u = (c - a).cross(step) / cross;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                last = Some(last.map_or(t, |l| l.max(t)));
            }
        }
        if let Some(t) = last {
            return Some((a + step * t, step * (-1.0 / length)));
        }
    }
    None
}

/// The bearing down the middle of an approach's arc.
fn approach_bearing(from_rad: f64, to_rad: f64) -> f64 {
    let to = if to_rad < from_rad {
        to_rad + std::f64::consts::TAU
    } else {
        to_rad
    };
    (from_rad + to) / 2.0
}

/// An angle as the least turn from zero, in (-π, π].
fn turn(mut angle: f64) -> f64 {
    use std::f64::consts::{PI, TAU};
    while angle > PI {
        angle -= TAU;
    }
    while angle <= -PI {
        angle += TAU;
    }
    angle
}

/// How far along `direction` from `from` the ray last crosses `ring`; none
/// when it never does.
fn ring_exit(ring: &[[f64; 2]], from: V2, direction: V2) -> Option<f64> {
    let mut last: Option<f64> = None;
    for (a, b) in contract::ground::edges(ring) {
        let (a, b) = (at(*a), at(*b));
        let side = b - a;
        let cross = direction.cross(side);
        if cross == 0.0 {
            continue;
        }
        let t = (a - from).cross(side) / cross;
        let u = (a - from).cross(direction) / cross;
        if t >= 0.0 && (0.0..=1.0).contains(&u) {
            last = Some(last.map_or(t, |l| l.max(t)));
        }
    }
    last
}

/// [`plan_encounter`] over JSON, as the native tools and the Wasm boundary
/// call it: the compiled map, its sites, the rules (a fixture with its
/// catalog), one recipe and the encounter seed as canonical decimal text.
/// The answer is an [`EncounterOutcome`], the same bytes on every target.
pub fn plan_encounter_json(
    map_json: &str,
    sites_json: &str,
    rules_json: &str,
    recipe_json: &str,
    encounter_seed: &str,
) -> String {
    let outcome = match plan_from_json(
        map_json,
        sites_json,
        rules_json,
        recipe_json,
        encounter_seed,
    ) {
        Ok(encounter) => EncounterOutcome::Ok {
            encounter: Box::new(encounter),
        },
        Err(diagnostics) => EncounterOutcome::Error { diagnostics },
    };
    serde_json::to_string(&outcome).expect("an encounter outcome serializes")
}

fn plan_from_json(
    map_json: &str,
    sites_json: &str,
    rules_json: &str,
    recipe_json: &str,
    encounter_seed: &str,
) -> Result<EncounterDefinition, Vec<EncounterDiagnostic>> {
    fn refused(code: Code, location: &str, message: String) -> Vec<EncounterDiagnostic> {
        vec![EncounterDiagnostic {
            code,
            feature: None,
            location: location.into(),
            message,
        }]
    }
    fn read<T: serde::de::DeserializeOwned>(
        text: &str,
        code: Code,
        location: &str,
    ) -> Result<T, Vec<EncounterDiagnostic>> {
        serde_json::from_str(text).map_err(|e| refused(code, location, e.to_string()))
    }
    let seed: Seed = serde_json::from_value(serde_json::Value::String(encounter_seed.into()))
        .map_err(|e| refused(Code::InvalidRequest, "$.encounter_seed", e.to_string()))?;
    let recipe: EncounterRecipe = read(recipe_json, Code::InvalidRecipe, "$.recipe")?;
    let sites: EncounterSites = read(sites_json, Code::InvalidSites, "$.sites")?;
    let rules: Rules = read(rules_json, Code::InvalidRequest, "$.rules")?;
    let map: MapDefinition = read(map_json, Code::InvalidRequest, "$.map")?;
    // The world refuses a map it cannot hold by panicking: ask first.
    if let Some(error) = contract::map::validate_header(
        map.size,
        map.fog_cell_m,
        map.height_grid_m,
        map.slope_cutoff_deg,
    )
    .first()
    {
        return Err(refused(
            Code::InvalidRequest,
            "$.map",
            format!("{}: {}", error.field, error.message),
        ));
    }
    map.authored_props()
        .map_err(|e| refused(Code::InvalidRequest, "$.map", e))?;
    let prepared = PreparedMap::new(&map, &rules);
    plan_encounter(&prepared.queries(&map, &sites), &rules, &recipe, seed)
}
