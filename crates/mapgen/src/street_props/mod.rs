//! Street furniture (C46): the bodies a built town's streets, yards, courts,
//! gardens and open parcels are dressed with, placed after the parcel pass as ordinary props
//! of the catalog. A street fight then has cover, and a town reads as lived
//! in.
//!
//! Every body, whatever its kind, goes through one legality check
//! ([`Field::legal`]): on the map, clear of every carriageway and the lane
//! driven beside its middle, of buildings and the way to their doors, of the bodies
//! already placed (each with the room its row keeps), of water, bridges and
//! the measured open approaches. What differs between kinds is data
//! (`street_props` and each district's `props` in the presets): a new kind
//! is a row. Each search is bounded and walks its candidates in a stable
//! order, and each consumer draws from its own named stream, so the same
//! request places the same bodies on every target.
use crate::layout::geometry::{
    add, direction, distance, dot, scale, segment_bounds, segment_crossing, sub, Grid, Point,
};
use crate::layout::rng::Stream;
use crate::layout::water::Water;
use crate::layout::{
    approach_corridors, CountRow, DistrictPreset, Gardens, GenerationRequest, PresetDefinitions,
    PropBox, StreetProps, VergeSides,
};
use crate::parcels::space::Rect;
use crate::{Diagnostic, DiagnosticCode, MapPlan};
use contract::catalog::{Catalog, PropPlacement};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{AuthoredPropDefinition, SurfaceKind};
use contract::templates::TemplateGeometryCatalog;
use std::collections::{BTreeMap, BTreeSet};

mod courts;
mod field;
use field::{Candidate, Field, Piece};

/// A body is set this far past the line it must keep to, so rounding its
/// centre to a centimetre cannot put it over.
pub(crate) const SLACK_M: f64 = 0.05;

/// What a fence leaves of one side of the area it runs round.
#[derive(Clone, Copy)]
enum Side {
    Fenced,
    /// Fenced, with this wide an opening at its middle.
    Gate(f64),
    Open,
}

/// A line of points with the distance along it to each.
struct Line {
    points: Vec<Point>,
    along: Vec<f64>,
}

impl Line {
    fn new(points: Vec<Point>) -> Self {
        let mut along = vec![0.0];
        for pair in points.windows(2) {
            along.push(along[along.len() - 1] + distance(pair[0], pair[1]));
        }
        Self { points, along }
    }

    fn length(&self) -> f64 {
        self.along[self.along.len() - 1]
    }

    /// The point `s` metres along, and the unit vector the line runs by there.
    fn at(&self, s: f64) -> (Point, Point) {
        let next = self
            .along
            .partition_point(|at| *at <= s)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[next - 1], self.points[next]);
        let span = self.along[next] - self.along[next - 1];
        let share = ((s - self.along[next - 1]) / span).clamp(0.0, 1.0);
        (
            add(a, scale(sub(b, a), share)),
            scale(sub(b, a), 1.0 / span),
        )
    }
}

/// One carriageway: its rounded centreline, as the map's surface draws it.
struct Way {
    line: Line,
    half_width: f64,
    kind: SurfaceKind,
}

/// A building as street furniture meets it.
struct House<'a> {
    id: &'a str,
    /// Its parts, as indices into the field's walls.
    walls: std::ops::Range<usize>,
    /// The way each of its doors faces.
    doors: Vec<Point>,
}

/// One request's street furniture pass.
struct Pass<'a> {
    plan: &'a MapPlan,
    request: &'a GenerationRequest,
    presets: &'a PresetDefinitions,
    rule: &'a StreetProps,
    ways: Vec<Way>,
    /// Every district with its kind's preset: its setbacks and what it is
    /// dressed with.
    districts: Vec<(&'a crate::DistrictPlan, &'a DistrictPreset)>,
    district_grid: Grid,
    district_ids: BTreeMap<&'a str, usize>,
    houses: Vec<House<'a>>,
    /// Open ground kept round a court group: the widest hull and its margin.
    group_ring: f64,
    field: Field<'a>,
}

/// Place street furniture on a plan the parcel pass has built: parked cars
/// beside the streets, lamps, trees and small furniture on the verges, loose
/// stock in the yards and a construction site on an open parcel now and
/// then. `catalog` is the unit and prop catalog: its widest hull sets the
/// lane every body keeps clear, and every placed kind is one of its prop
/// types. The answer is the bodies to add to the plan's `props`, in the
/// order they were placed; none carries an id, so they take the ids after
/// the buildings' parts.
pub fn place_street_props(
    plan: &MapPlan,
    request: &GenerationRequest,
    templates: &TemplateGeometryCatalog,
    catalog: &Catalog,
    presets: &PresetDefinitions,
) -> Result<Vec<AuthoredPropDefinition>, Vec<Diagnostic>> {
    let mut pass = Pass::prepare(plan, request, templates, catalog, presets)?;
    pass.sites();
    pass.yards();
    pass.verges(false);
    pass.parking();
    pass.verges(true);
    Ok(pass.field.placed)
}

/// Dress the courts of the dense districts and the gardens behind the
/// houses of a finished plan, the last of a town's dressing: after the open
/// country's cover has certified the map's sight, among every body the plan
/// already holds, and `wood_clear_m` (the forests' trunk clearance) off every
/// forest, so no court or garden fells a tree the certificate counted. Both
/// stop at the request's authored-part limit, courts first: on the largest
/// maps the gardens take what the courts leave. The answer is the bodies to
/// add to the plan's `props`, as [`place_street_props`]'s are.
pub fn place_courts_and_gardens(
    plan: &MapPlan,
    request: &GenerationRequest,
    templates: &TemplateGeometryCatalog,
    catalog: &Catalog,
    presets: &PresetDefinitions,
    wood_clear_m: f64,
) -> Result<Vec<AuthoredPropDefinition>, Vec<Diagnostic>> {
    let mut pass = Pass::prepare(plan, request, templates, catalog, presets)?;
    pass.field.stand_plan_bodies(&plan.props);
    pass.field.keep_off_woods(&plan.forests, wood_clear_m);
    let parts = plan.props.len() + plan.buildings.iter().map(|b| b.parts.len()).sum::<usize>();
    let room = (request.limits.max_authored_parts as usize).saturating_sub(parts);
    let courts = pass.courts(room);
    pass.gardens(room - courts);
    Ok(pass.field.placed)
}

impl<'a> Pass<'a> {
    /// A pass over `plan` with every building standing: its kinds checked
    /// against the catalog, its lane sized to the widest hull.
    fn prepare(
        plan: &'a MapPlan,
        request: &'a GenerationRequest,
        templates: &'a TemplateGeometryCatalog,
        catalog: &Catalog,
        presets: &'a PresetDefinitions,
    ) -> Result<Self, Vec<Diagnostic>> {
        let rule = &presets.street_props;
        let unknown: Vec<Diagnostic> = rule
            .bodies
            .keys()
            .filter_map(|kind| {
                let refusal = match catalog.props().index(kind) {
                    None => "the catalog has no such prop type".to_string(),
                    Some(index) => catalog
                        .props()
                        .check_placement(index, PropPlacement::Ordinary)
                        .err()?,
                };
                Some(Diagnostic {
                    code: DiagnosticCode::InvalidPresets,
                    feature: Some(kind.clone()),
                    location: format!("$.presets.street_props.bodies.{kind}"),
                    message: refusal,
                })
            })
            .collect();
        if !unknown.is_empty() {
            return Err(unknown);
        }
        let widest_hull = catalog
            .indices()
            .filter_map(|unit| catalog.get(unit).hull())
            .map(|hull| 2.0 * hull.half_extents_m[1])
            .fold(0.0, f64::max);

        let mut pass = Pass::new(plan, request, presets, widest_hull);
        pass.stand_buildings(templates)?;
        Ok(pass)
    }

    fn new(
        plan: &'a MapPlan,
        request: &'a GenerationRequest,
        presets: &'a PresetDefinitions,
        widest_hull: f64,
    ) -> Self {
        let rule = &presets.street_props;
        let lane = widest_hull + rule.lane_margin_m;
        let mut ways = Vec::new();
        let mut pieces = Vec::new();
        let mut piece_grid = Grid::new(plan.size, 64.0);
        let mut widest: f64 = 0.0;
        for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
            // Strokes only: an apron is paved ground, not a way through.
            let GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            else {
                continue;
            };
            let line = Line::new(centerline.samples().to_vec());
            let half_width = width_m / 2.0;
            widest = widest.max(half_width);
            for (index, pair) in line.points.windows(2).enumerate() {
                piece_grid.insert(segment_bounds(pair[0], pair[1], 0.0), pieces.len() as u32);
                pieces.push(Piece {
                    a: pair[0],
                    b: pair[1],
                    half_width,
                    way: ways.len() as u32,
                    along: [line.along[index], line.along[index + 1]],
                });
            }
            ways.push(Way {
                line,
                half_width,
                kind: area.kind,
            });
        }
        let mut districts = Vec::new();
        let mut district_grid = Grid::new(plan.size, 64.0);
        let mut district_ids = BTreeMap::new();
        for district in plan.settlements.iter().flat_map(|s| &s.districts) {
            // A kind the presets lack was refused by the parcel pass.
            let Some(preset) = presets.districts.get(&district.kind) else {
                continue;
            };
            let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
            district_grid.insert([x0, y0, x1, y1], districts.len() as u32);
            district_ids.insert(district.id.as_str(), districts.len());
            districts.push((district, preset));
        }
        let deck_run = presets.rivers.bridge.approach_m;
        let decks = plan
            .bridges
            .iter()
            .map(|bridge| Rect {
                center: bridge.center,
                axis: direction(bridge.yaw),
                half: [bridge.half_extents[0] + deck_run, bridge.half_extents[1]],
            })
            .collect();
        let most_clear = rule
            .bodies
            .values()
            .map(|body| body.clear_m)
            .fold(0.0, f64::max);
        Self {
            plan,
            request,
            presets,
            rule,
            ways,
            districts,
            district_grid,
            district_ids,
            houses: Vec::new(),
            group_ring: widest_hull + rule.group_margin_m,
            field: Field {
                size: plan.size,
                rule,
                lane,
                pieces,
                piece_grid,
                widest,
                walls: Vec::new(),
                wall_grid: Grid::new(plan.size, 48.0),
                doors: Vec::new(),
                door_grid: Grid::new(plan.size, 48.0),
                bodies: Vec::new(),
                body_grid: Grid::new(plan.size, 32.0),
                most_clear,
                water: Water::new(&plan.rivers, plan.size),
                bank: presets.rivers.bank_m(),
                decks,
                corridors: approach_corridors(plan),
                woods: Vec::new(),
                wood_grid: Grid::new(plan.size, 64.0),
                wood_clear: 0.0,
                groups: 0,
                placed: Vec::new(),
            },
        }
    }

    fn stream(&self, name: &str) -> Stream {
        crate::layout::stream(self.request, &format!("street-props/{name}"))
    }

    fn body(&self, kind: &str) -> PropBox {
        // The presets were refused at load if a row named a body they lack.
        self.rule.bodies[kind]
    }

    /// The district the point stands in, as an index into `districts`.
    fn district_at(&self, p: Point) -> Option<usize> {
        let mut found = None;
        self.district_grid.any([p[0], p[1], p[0], p[1]], |item| {
            let inside = polygon_contains(&self.districts[item as usize].0.ring, p);
            if inside {
                found = Some(item as usize);
            }
            inside
        });
        found
    }

    /// Every building's walls, and each door's way to the street.
    fn stand_buildings(
        &mut self,
        templates: &'a TemplateGeometryCatalog,
    ) -> Result<(), Vec<Diagnostic>> {
        let plan = self.plan;
        for placement in &plan.buildings {
            let refuse = |code, location: &str, message: String| {
                vec![Diagnostic {
                    code,
                    feature: Some(placement.id.clone()),
                    location: location.into(),
                    message,
                }]
            };
            let template = templates
                .templates()
                .iter()
                .find(|template| template.id == placement.template_id)
                .ok_or_else(|| {
                    refuse(
                        DiagnosticCode::MissingTemplate,
                        "template_id",
                        format!("unknown physical template {}", placement.template_id),
                    )
                })?;
            let building = template
                .materialize(placement.frame)
                .map_err(|message| refuse(DiagnosticCode::InvalidPlacement, "frame", message))?;
            let first = self.field.walls.len();
            for part in &building.parts {
                let wall = Rect {
                    center: part.center,
                    axis: direction(part.yaw),
                    half: [part.half_extents[0], part.half_extents[1]],
                };
                self.field
                    .wall_grid
                    .insert(wall.bounds(), self.field.walls.len() as u32);
                self.field.walls.push(wall);
            }
            let mut doors = Vec::new();
            for entrance in building.entrances.iter().flatten() {
                let from = [entrance.position[0], entrance.position[1]];
                let out = entrance.normal;
                // To the middle of the first carriageway the door faces.
                let far = add(from, scale(out, self.rule.door_look_m));
                let mut reach: Option<f64> = None;
                self.field
                    .piece_grid
                    .any(segment_bounds(from, far, 0.0), |item| {
                        let piece = &self.field.pieces[item as usize];
                        if let Some((t, _)) = segment_crossing(from, far, piece.a, piece.b) {
                            let met = t * self.rule.door_look_m;
                            reach = Some(reach.map_or(met, |known| known.min(met)));
                        }
                        false
                    });
                let reach = reach.unwrap_or(self.rule.door_reach_m);
                let way = Rect {
                    center: add(from, scale(out, reach / 2.0)),
                    axis: out,
                    half: [reach / 2.0, self.rule.door_clear_m],
                };
                self.field.keep_clear(way);
                doors.push(out);
            }
            self.houses.push(House {
                id: &placement.id,
                walls: first..self.field.walls.len(),
                doors,
            });
        }
        Ok(())
    }

    /// The district a parcel or building id belongs to
    /// (`<district id>/lot-<n>`).
    fn district_of(&self, id: &str) -> Option<usize> {
        let (district, _) = id.rsplit_once("/lot-")?;
        self.district_ids.get(district).copied()
    }

    /// Construction sites: on parcels the parcel pass left open, by the
    /// district's chance, at most the rule's count to a settlement.
    fn sites(&mut self) {
        let built: BTreeSet<&str> = self
            .plan
            .buildings
            .iter()
            .map(|building| building.id.as_str())
            .collect();
        for settlement in &self.plan.settlements {
            let mut rng = self.stream(&format!("{}/sites", settlement.id));
            let prefix = format!("{}/", settlement.id);
            // Each open parcel draws once; the lowest draws under their
            // district's chance are tried first, so sites are spread over the
            // settlement and one that does not fit moves no other.
            let mut wanted: Vec<(f64, usize)> = Vec::new();
            for (index, lot) in self.plan.lots.iter().enumerate() {
                if !lot.id.starts_with(&prefix) || built.contains(lot.id.as_str()) {
                    continue;
                }
                let Some(district) = self.district_of(&lot.id) else {
                    continue;
                };
                let chance = self.districts[district].1.props.site_chance;
                if chance <= 0.0 {
                    continue;
                }
                let draw = rng.unit();
                if draw < chance {
                    wanted.push((draw / chance, index));
                }
            }
            wanted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let mut sites = 0;
            for (_, lot) in wanted {
                if sites >= self.rule.site.max_per_settlement {
                    break;
                }
                let ring = &self.plan.lots[lot].ring;
                if ring.len() == 4 && self.site(ring, &mut rng) {
                    sites += 1;
                }
            }
        }
    }

    /// One construction site on the parcel `ring` (its street side first):
    /// a cabin at the back, a fence round it with a gate on the street, and
    /// stock inside. `false`, with nothing placed, where the parcel is too
    /// small or the cabin has no legal ground.
    fn site(&mut self, ring: &[Point], rng: &mut Stream) -> bool {
        let all = self.rule;
        let rule = &all.site;
        let (width, depth) = (distance(ring[0], ring[1]), distance(ring[0], ring[3]));
        if width < rule.min_lot_m[0] || depth < rule.min_lot_m[1] {
            return false;
        }
        let along = scale(sub(ring[1], ring[0]), 1.0 / width);
        let inward = scale(sub(ring[3], ring[0]), 1.0 / depth);
        let origin = scale(add(ring[0], ring[1]), 0.5);
        // `x` metres along the parcel's front from its middle, `y` into it.
        let at = |x: f64, y: f64| add(origin, add(scale(along, x), scale(inward, y)));
        let group = self.field.group();
        let inset = rule.fence_inset_m;
        let room = inset + self.rule.site_room_m;

        let cabin = self.body(&rule.cabin);
        let corner = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let candidate = |body: &PropBox, center: Point, axis: Point| Candidate {
            group,
            ..Candidate::new(body, center, axis)
        };
        let c = candidate(
            &cabin,
            at(
                corner * (width / 2.0 - room - cabin.half_extents_m[0]),
                depth - room - cabin.half_extents_m[1],
            ),
            along,
        );
        if !self.field.legal(&c) {
            return false;
        }
        self.field.place(&rule.cabin, &cabin, &c);

        let corners = [
            at(-width / 2.0 + inset, inset),
            at(width / 2.0 - inset, inset),
            at(width / 2.0 - inset, depth - inset),
            at(-width / 2.0 + inset, depth - inset),
        ];
        let sides = [
            Side::Gate(rule.gate_m),
            Side::Fenced,
            Side::Fenced,
            Side::Fenced,
        ];
        self.fence_round(corners, &rule.fence, group, sides, usize::MAX);
        // The gate's way to the street is kept open, like a door's.
        self.field.keep_clear(Rect {
            center: at(0.0, (inset - self.rule.door_reach_m) / 2.0),
            axis: inward,
            half: [(inset + self.rule.door_reach_m) / 2.0, rule.gate_m / 2.0],
        });

        for row in &rule.stock {
            let body = self.body(&row.kind);
            let reach = body.half_extents_m[0].max(body.half_extents_m[1]);
            let (across, deep) = (width / 2.0 - room - reach, depth - 2.0 * room - 2.0 * reach);
            if across <= 0.0 || deep <= 0.0 {
                continue;
            }
            for _ in 0..rng.count(row.count) {
                for _ in 0..all.attempts {
                    let center = at(
                        rng.range([-across, across]),
                        room + reach + rng.unit() * deep,
                    );
                    let axis = if rng.chance(0.5) { along } else { inward };
                    let c = candidate(&body, center, axis);
                    if self.field.legal(&c) {
                        self.field.place(&row.kind, &body, &c);
                        break;
                    }
                }
            }
        }
        true
    }

    /// A fence of `kind` round `corners` (counter-clockwise), as
    /// [`Pass::fence_panels`] lays it: at most `most` panels, the sides in
    /// order; a panel with no legal ground is left out. The answer is how
    /// many stand.
    fn fence_round(
        &mut self,
        corners: [Point; 4],
        kind: &str,
        group: u32,
        sides: [Side; 4],
        most: usize,
    ) -> usize {
        let fence = self.body(kind);
        let mut placed = 0;
        for c in self.fence_panels(corners, kind, group, sides) {
            if placed >= most {
                break;
            }
            if self.field.legal(&c) {
                self.field.place(kind, &fence, &c);
                placed += 1;
            }
        }
        placed
    }

    /// The panels of a fence of `kind` round `corners` (counter-clockwise),
    /// in group `group`: whole panels along each side `sides` does not leave
    /// open, short of each corner by a panel's thickness so two sides meet
    /// without crossing, with a gate's width left open at the middle of a
    /// gated side.
    fn fence_panels(
        &self,
        corners: [Point; 4],
        kind: &str,
        group: u32,
        sides: [Side; 4],
    ) -> Vec<Candidate> {
        let fence = self.body(kind);
        let panel = 2.0 * fence.half_extents_m[0];
        let mut panels = Vec::new();
        for (side, open) in sides.into_iter().enumerate() {
            let gate = match open {
                Side::Open => continue,
                Side::Fenced => None,
                Side::Gate(width) => Some(width),
            };
            let (from, to) = (corners[side], corners[(side + 1) % 4]);
            let length = distance(from, to);
            let run = scale(sub(to, from), 1.0 / length);
            let count = libm::floor((length - 4.0 * fence.half_extents_m[1]) / panel);
            let start = (length - count * panel) / 2.0;
            for k in 0..count as usize {
                let middle = start + (k as f64 + 0.5) * panel;
                if gate.is_some_and(|gate| (middle - length / 2.0).abs() < (gate + panel) / 2.0) {
                    continue;
                }
                panels.push(Candidate {
                    group,
                    ..Candidate::new(&fence, add(from, scale(run, middle)), run)
                });
            }
        }
        panels
    }

    /// Each built parcel, in the buildings' order: its building (an index
    /// into `houses`), its ring and its district.
    fn built_lots(&self) -> Vec<(usize, &'a [Point], usize)> {
        let plan = self.plan;
        let lots: BTreeMap<&str, &[Point]> = plan
            .lots
            .iter()
            .map(|lot| (lot.id.as_str(), lot.ring.as_slice()))
            .collect();
        (0..self.houses.len())
            .filter_map(|house| {
                let id = self.houses[house].id;
                Some((house, *lots.get(id)?, self.district_of(id)?))
            })
            .collect()
    }

    /// Gardens: behind each building of a district that keeps them, on its
    /// own parcel, each parcel in the order its own stream draws, at most
    /// `room` bodies in all, so where the map's parts reach the request's
    /// limit the gardens left bare are spread over the map.
    fn gardens(&mut self, mut room: usize) {
        let mut order = Vec::new();
        for (house, lot, district) in self.built_lots() {
            let Some(rule) = &self.districts[district].1.props.gardens else {
                continue;
            };
            let mut rng = self.stream(&format!("{}/garden", self.houses[house].id));
            order.push((rng.unit(), lot, district, rule, rng));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, lot, district, rule, mut rng) in order {
            let most = room.min(rule.max_per_lot as usize);
            if most == 0 {
                break;
            }
            let front = self.districts[district].1.lots.front_m;
            room -= self.garden(lot, rule, front, most, &mut rng);
        }
    }

    /// One garden on the parcel `ring` (its street side first), at most
    /// `most` bodies: pieces in the strip `rule.depth_m` in from its rear
    /// edge, then by the rule's chance a boundary of one kind along its rear
    /// edge and its sides, from `front` in from the street, so the front
    /// garden stays open. The answer is how many bodies stand.
    fn garden(
        &mut self,
        ring: &[Point],
        rule: &Gardens,
        front: f64,
        most: usize,
        rng: &mut Stream,
    ) -> usize {
        let (width, depth) = (distance(ring[0], ring[1]), distance(ring[0], ring[3]));
        let along = scale(sub(ring[1], ring[0]), 1.0 / width);
        let inward = scale(sub(ring[3], ring[0]), 1.0 / depth);
        // `x` metres along the parcel's front from its first corner, `y` into it.
        let at = |x: f64, y: f64| add(ring[0], add(scale(along, x), scale(inward, y)));
        let mut placed = 0;
        for row in &rule.pieces {
            let body = self.body(&row.kind);
            let reach = body.half_extents_m[0].max(body.half_extents_m[1]);
            let across = [rule.room_m + reach, width - rule.room_m - reach];
            let deep = [depth - rule.depth_m + reach, depth - rule.room_m - reach];
            if across[0] > across[1] || deep[0] > deep[1] {
                continue;
            }
            for _ in 0..rng.count(row.count) {
                if placed >= most {
                    return placed;
                }
                for _ in 0..self.rule.attempts {
                    let center = at(rng.range(across), rng.range(deep));
                    let axis = if rng.chance(0.5) { along } else { inward };
                    let c = Candidate::new(&body, center, axis);
                    if self.field.legal(&c) {
                        self.field.place(&row.kind, &body, &c);
                        placed += 1;
                        break;
                    }
                }
            }
        }
        if rule.boundary.is_empty() || !rng.chance(rule.boundary_chance) {
            return placed;
        }
        let kind = rng
            .pick(&rule.boundary)
            .expect("a boundary names a kind")
            .as_str();
        // Inside the parcel by the panel's half thickness: a neighbour's run
        // along the same edge is then within the ground each keeps, and only
        // one of the two stands.
        let inset = self.body(kind).half_extents_m[1] + SLACK_M;
        // The rear edge first, then the sides; the front left open.
        let corners = [
            at(width - inset, depth - inset),
            at(inset, depth - inset),
            at(inset, front),
            at(width - inset, front),
        ];
        let sides = [Side::Fenced, Side::Fenced, Side::Open, Side::Fenced];
        let group = self.field.group();
        placed + self.fence_round(corners, kind, group, sides, most - placed)
    }

    /// Loose stock in the yards: beside each building of a district whose
    /// kind keeps any, against a wall with no door in it, on the building's
    /// own parcel.
    fn yards(&mut self) {
        for (house, lot, district) in self.built_lots() {
            let rows: &[CountRow] = &self.districts[district].1.props.yard;
            if rows.is_empty() {
                continue;
            }
            let id = self.houses[house].id;
            let mut rng = self.stream(&format!("{id}/yard"));
            for row in rows {
                let body = self.body(&row.kind);
                for _ in 0..rng.count(row.count) {
                    for _ in 0..self.rule.attempts {
                        let walls = self.houses[house].walls.clone();
                        let wall =
                            self.field.walls[walls.start + rng.below(walls.len() as u64) as usize];
                        // One of the wall's four faces: the way it looks,
                        // how far out it stands and how long it is.
                        let face = rng.below(4);
                        let turn = [-wall.axis[1], wall.axis[0]];
                        let (out, depth, half) = match face {
                            0 => (wall.axis, wall.half[0], wall.half[1]),
                            1 => (scale(wall.axis, -1.0), wall.half[0], wall.half[1]),
                            2 => (turn, wall.half[1], wall.half[0]),
                            _ => (scale(turn, -1.0), wall.half[1], wall.half[0]),
                        };
                        let run = [-out[1], out[0]];
                        let off = self.rule.wall_gap_m
                            + SLACK_M
                            + body.half_extents_m[1]
                            + rng.range([0.0, 1.5]);
                        let along = rng.range([-half, half]);
                        if self.houses[house]
                            .doors
                            .iter()
                            .any(|door| dot(*door, out) > 0.5)
                        {
                            continue;
                        }
                        let center =
                            add(wall.center, add(scale(out, depth + off), scale(run, along)));
                        let c = Candidate::new(&body, center, run);
                        if c.rect.inside(lot) && self.field.legal(&c) {
                            self.field.place(&row.kind, &body, &c);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// From a carriageway's middle to the nearest edge of a body beside it.
    fn kerb_line(&self, way: &Way) -> f64 {
        self.field.lane.max(way.half_width + self.rule.kerb_gap_m) + SLACK_M
    }

    /// The stretches of one side of a way that lie in a district, each
    /// `(from, to, district)` along the way.
    fn owners(&self, way: &Way, side: f64) -> Vec<(f64, f64, usize)> {
        let length = way.line.length();
        let steps = (libm::ceil(length / self.rule.owner_step_m) as usize).max(1);
        let step = length / steps as f64;
        let off = self.kerb_line(way) + 1.0;
        // Runs of steps, each `(first step, one past its last, district)`.
        let mut found: Vec<(usize, usize, usize)> = Vec::new();
        for index in 0..steps {
            let (p, run) = way.line.at((index as f64 + 0.5) * step);
            let p = add(p, scale([-run[1], run[0]], side * off));
            let Some(district) = self.district_at(p) else {
                continue;
            };
            match found.last_mut() {
                Some(last) if last.2 == district && last.1 == index => last.1 = index + 1,
                _ => found.push((index, index + 1, district)),
            }
        }
        found
            .into_iter()
            .map(|(from, to, district)| (from as f64 * step, to as f64 * step, district))
            .collect()
    }

    /// A body of `body`'s box beside `way` on `side`, its middle `s` metres
    /// along: parallel to the carriageway, its near edge on the kerb line.
    fn beside(&self, way: usize, side: f64, s: f64, body: &PropBox) -> Candidate {
        let line = &self.ways[way];
        let (p, run) = line.line.at(s);
        let off = self.kerb_line(line) + body.half_extents_m[1];
        Candidate {
            beside: Some((way as u32, s)),
            corner: self.rule.corner_clear_m,
            ..Candidate::new(body, add(p, scale([-run[1], run[0]], side * off)), run)
        }
    }

    /// Verge furniture: each district's rows along the carriageways that
    /// run through it or along its edge. The evenly spaced rows (lamps,
    /// trees) are placed before the cars, so a run of cars ends at one; the
    /// scattered rows after, in the room the cars left.
    fn verges(&mut self, scattered: bool) {
        let avenue = self.presets.towns.avenue_width_m / 2.0;
        for way in 0..self.ways.len() {
            let length = self.ways[way].line.length();
            for side in [1.0, -1.0] {
                let name = if side > 0.0 { "left" } else { "right" };
                let mut rng = self.stream(&format!("way-{way}/{name}/verge"));
                for (from, to, district) in self.owners(&self.ways[way], side) {
                    let props = &self.districts[district].1.props;
                    for row in &props.verge {
                        if (row.sides == VergeSides::Scatter) != scattered
                            || (row.avenue && self.ways[way].half_width < avenue)
                        {
                            continue;
                        }
                        let body = self.body(&row.kind);
                        let spots: Vec<f64> = match row.sides {
                            // One to the spacing over both sides: half that
                            // many to a side, the odd share by a draw.
                            VergeSides::Scatter => {
                                let count = (to - from) / (2.0 * row.spacing_m) + rng.unit();
                                (0..libm::floor(count) as usize)
                                    .map(|_| rng.range([from, to]))
                                    .collect()
                            }
                            // Counted from the way's start, so a row keeps
                            // its rhythm across the districts it runs through.
                            even => (0..)
                                .map(|k| (k, (k as f64 + 0.5) * row.spacing_m))
                                .take_while(|(_, s)| *s < to.min(length))
                                .filter(|(k, s)| {
                                    *s >= from
                                        && (even == VergeSides::Both
                                            || (k % 2 == 0) == (side > 0.0))
                                })
                                .map(|(_, s)| s)
                                .collect(),
                        };
                        for s in spots {
                            self.settle(way, side, s, [from, to], &row.kind, &body);
                        }
                    }
                }
            }
        }
    }

    /// Stand a body beside `way` at `s`, or at the nearest legal place along
    /// the way within the rule's slide, inside `within`.
    fn settle(
        &mut self,
        way: usize,
        side: f64,
        s: f64,
        within: [f64; 2],
        kind: &str,
        body: &PropBox,
    ) -> bool {
        let steps = libm::floor(self.rule.slide_m / self.rule.slide_step_m) as i32;
        for step in 0..=2 * steps {
            // 0, +1, −1, +2, −2, …
            let off = (step + 1) / 2 * if step % 2 == 1 { 1 } else { -1 };
            let at = s + off as f64 * self.rule.slide_step_m;
            let half = body.half_extents_m[0];
            if at - half < within[0] || at + half > within[1] {
                continue;
            }
            let c = self.beside(way, side, at, body);
            if self.field.legal(&c) {
                self.field.place(kind, body, &c);
                return true;
            }
        }
        false
    }

    /// Parked cars: beside the paved streets of each district that parks,
    /// in runs with gaps between. A narrow street parks along one side,
    /// drawn for the whole street; an avenue along both.
    fn parking(&mut self) {
        let all = self.rule;
        let rule = &all.parking;
        let car = self.body(&rule.kind);
        let length = 2.0 * car.half_extents_m[0];
        let pitch = length + rule.bumper_gap_m;
        for way in 0..self.ways.len() {
            if self.ways[way].kind != SurfaceKind::Road {
                continue;
            }
            let both = 2.0 * self.ways[way].half_width >= rule.both_sides_min_width_m;
            let only = if self.stream(&format!("way-{way}/side")).chance(0.5) {
                1.0
            } else {
                -1.0
            };
            for side in [1.0, -1.0] {
                if !both && side != only {
                    continue;
                }
                let name = if side > 0.0 { "left" } else { "right" };
                let mut rng = self.stream(&format!("way-{way}/{name}/parking"));
                // Where the next run may start: a run's gap is kept across
                // the districts one side of a street runs through.
                let mut next: f64 = 0.0;
                for (from, to, district) in self.owners(&self.ways[way], side) {
                    let share = self.districts[district].1.props.parking;
                    if share <= 0.0 {
                        continue;
                    }
                    // The gap after a run of `cars`, so that runs cover the
                    // district's share of the kerb.
                    let gap = |cars: u32, rng: &mut Stream| {
                        let open = cars as f64 * pitch * (1.0 - share) / share;
                        rule.run_gap_m.max(open * rng.range([0.6, 1.4]))
                    };
                    let lead = rng.unit();
                    let mut s = next.max(from + lead * gap(rule.run[0], &mut rng));
                    while s + length <= to {
                        let cars = rng.count(rule.run);
                        let group = self.field.group();
                        let mut cursor = s;
                        for _ in 0..cars {
                            // The first legal place from the bumper ahead.
                            let steps = libm::floor(all.slide_m / all.slide_step_m);
                            let spot = (0..=steps as usize)
                                .map(|step| cursor + step as f64 * all.slide_step_m)
                                .take_while(|start| start + length <= to)
                                .find_map(|start| {
                                    let mut c = self.beside(way, side, start + length / 2.0, &car);
                                    c.group = group;
                                    self.field.legal(&c).then_some((start, c))
                                });
                            let Some((start, c)) = spot else {
                                break;
                            };
                            self.field.place(&rule.kind, &car, &c);
                            cursor = start + pitch;
                        }
                        s = cursor.max(s + pitch) + gap(cars, &mut rng);
                        next = s;
                    }
                }
            }
        }
    }
}
