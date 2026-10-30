//! Road columns, legal stations and equal travel into the objective zone.
use super::*;

impl Planner<'_> {
    /// The fastest route navigation plans for `m` from `from` into the
    /// capture zone about `centre`: its waypoints after `from`, the last
    /// inside the zone. None when the planner finds no route, or one that
    /// stops short of the zone.
    pub(super) fn route_into_zone(&self, from: V2, centre: V2, m: &Mobility) -> Option<Vec<V2>> {
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
    pub(super) fn drive(&self, from: V2, centre: V2) -> Option<Route> {
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
    pub(super) fn edge_roads(&self, edge: Half) -> Vec<Road> {
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
    pub(super) fn stations(
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
    pub(super) fn legal(
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
    pub(super) fn column_on(
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
    pub(super) fn column(
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
    pub(super) fn shapes(&self, column: &Column) -> Vec<Footprint> {
        column
            .rows
            .iter()
            .zip(&column.stations)
            .map(|(&row, s)| footprint(self.unit_type(row), self.rules, s.at, s.yaw))
            .collect()
    }

    /// Bring the two columns' drives within the recipe's difference: the
    /// side with the longer drive moves up its road.
    pub(super) fn level(
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
}
