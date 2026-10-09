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
use contract::generation_physics::GenerationPhysics;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{AuthoredPropDefinition, SurfaceKind};
use contract::templates::TemplateGeometryCatalog;
use std::collections::{BTreeMap, BTreeSet};

mod courts;
mod field;
use field::{faces, Candidate, Field, Piece};

/// A body is set this far past the line it must keep to, so rounding its
/// centre to a centimetre cannot put it over.
const SLACK_M: f64 = 0.05;

/// How far in from the edge of the ground it bounds a fence of `fence`
/// stands, its middle: half its thickness and the slack, so the whole panel
/// stays on that ground.
pub(crate) fn fence_inset(fence: &PropBox) -> f64 {
    fence.half_extents_m[1] + SLACK_M
}

/// The panels of a fence of `fence` along one side `length` long, between
/// two corners, each as its middle along the side and half its length:
/// short of each corner by a panel's thickness so two sides meet without
/// crossing, with a gate `width` wide left open at each `(at, width)`, `at`
/// metres from the side's middle. A fixed panel is laid whole, centred along
/// the side, and one that would stand in a gate is left out; a cut-to-fit
/// one is cut to the stretches between the gates, each in as few equal
/// panels as its box allows.
pub(crate) fn side_panels(length: f64, fence: &PropBox, gates: &[(f64, f64)]) -> Vec<[f64; 2]> {
    let panel = 2.0 * fence.half_extents_m[0];
    let thick = fence.half_extents_m[1];
    let mut panels = Vec::new();
    if !fence.cut_to_fit {
        let count = libm::floor((length - 4.0 * thick) / panel);
        let start = (length - count * panel) / 2.0;
        for k in 0..count as usize {
            let middle = start + (k as f64 + 0.5) * panel;
            let off = middle - length / 2.0;
            if !gates
                .iter()
                .any(|(at, width)| (off - at).abs() < (width + panel) / 2.0)
            {
                panels.push([middle, fence.half_extents_m[0]]);
            }
        }
        return panels;
    }
    // The stretches between the corners and the gates, in order.
    let mut cuts: Vec<[f64; 2]> = gates
        .iter()
        .map(|(at, width)| {
            [
                length / 2.0 + at - width / 2.0,
                length / 2.0 + at + width / 2.0,
            ]
        })
        .collect();
    cuts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut start = 2.0 * thick;
    for [gate_from, gate_to] in cuts.into_iter().chain([[length - 2.0 * thick; 2]]) {
        let stretch = gate_from.min(length - 2.0 * thick) - start;
        if stretch >= 4.0 * thick {
            let count = libm::ceil(stretch / panel);
            let each = stretch / count;
            for k in 0..count as usize {
                panels.push([start + (k as f64 + 0.5) * each, each / 2.0]);
            }
        }
        start = start.max(gate_to);
    }
    panels
}

/// A cut-to-fit panel refused its ground is cut in two while each half would
/// be at least this long.
const SPLIT_M: f64 = 1.0;

/// What a fence leaves of one side of the area it runs round.
#[derive(Clone)]
enum Side {
    Fenced,
    /// Fenced, with an opening at each `(at, width)`: `width` wide, centred
    /// `at` metres along the side from its middle.
    Gates(Vec<(f64, f64)>),
    Open,
}

impl Side {
    /// Fenced with one opening `width` wide at its middle.
    fn gate(width: f64) -> Self {
        Side::Gates(vec![(0.0, width)])
    }
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

/// A parcel's or car park's own frame (its ring's first edge the street
/// side): `x` metres along that edge from its first corner, `y` into it.
struct Frame {
    origin: Point,
    along: Point,
    inward: Point,
    width: f64,
    depth: f64,
}

impl Frame {
    fn new(ring: &[Point]) -> Self {
        let (width, depth) = (distance(ring[0], ring[1]), distance(ring[0], ring[3]));
        Self {
            origin: ring[0],
            along: scale(sub(ring[1], ring[0]), 1.0 / width),
            inward: scale(sub(ring[3], ring[0]), 1.0 / depth),
            width,
            depth,
        }
    }

    fn at(&self, x: f64, y: f64) -> Point {
        add(
            self.origin,
            add(scale(self.along, x), scale(self.inward, y)),
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
    /// Each of its doors: where it is, and the way it faces.
    doors: Vec<(Point, Point)>,
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
    /// Open ground a vehicle drives through: the widest hull and its
    /// margin. Kept either side of a lawn's lane, and the room a yard's
    /// vehicle gate needs to its building.
    hull_way: f64,
    /// Each kind abandoned in a district's roads, and the way it leaves open
    /// beside it: as wide as the widest hull that cannot shove it and the
    /// vehicle margin, a hull's length past it either way.
    left_open: BTreeMap<&'a str, [f64; 2]>,
    /// Each building's place in `houses`, by its id.
    house_ids: BTreeMap<&'a str, usize>,
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
    pass.abandoned();
    pass.parking();
    pass.verges(true);
    Ok(pass.field.placed)
}

/// Dress the courts of the dense districts and the gardens behind the
/// houses of a finished plan, the last of a town's dressing: after the open
/// country's cover has certified the map's sight, among every body the plan
/// already holds, and the forests' trunk clearance (of `physics`, whose
/// catalog the bodies are) off every forest, so no court or garden fells a
/// tree the certificate counted. Both
/// stop at the request's authored-part limit, courts first: on the largest
/// maps the gardens take what the courts leave. The answer is the bodies to
/// add to the plan's `props`, as [`place_street_props`]'s are.
pub fn place_courts_and_gardens(
    plan: &MapPlan,
    request: &GenerationRequest,
    templates: &TemplateGeometryCatalog,
    physics: &GenerationPhysics,
    presets: &PresetDefinitions,
) -> Result<Vec<AuthoredPropDefinition>, Vec<Diagnostic>> {
    let mut pass = Pass::prepare(plan, request, templates, &physics.catalog, presets)?;
    pass.field.stand_plan_bodies(&plan.props);
    pass.field
        .keep_off_woods(&plan.forests, physics.forests.rule.trunk_clearance_m);
    let parts = plan.props.len() + plan.buildings.iter().map(|b| b.parts.len()).sum::<usize>();
    let room = (request.limits.max_authored_parts as usize).saturating_sub(parts);
    let courts = pass.courts(room);
    pass.gardens(room - courts);
    Ok(pass.field.placed)
}

/// The width of `catalog`'s widest hull.
fn widest_hull(catalog: &Catalog) -> f64 {
    catalog
        .indices()
        .filter_map(|unit| catalog.get(unit).hull())
        .map(|hull| 2.0 * hull.half_extents_m[1])
        .fold(0.0, f64::max)
}

/// The way a body of prop `kind` abandoned in the road leaves open: the
/// widest of `catalog`'s hulls that cannot shove it, with the presets'
/// vehicle margin, and the longest such hull's length; none where every
/// hull shoves it.
fn left_open(catalog: &Catalog, presets: &PresetDefinitions, kind: &str) -> [f64; 2] {
    let props = catalog.props();
    let weight = props
        .get(props.index(kind).expect("a presets body is a catalog prop"))
        .body
        .weight_class;
    let [wide, long] = catalog
        .indices()
        .filter_map(|unit| catalog.get(unit).hull())
        .filter(|hull| !hull.push_class.pushes(weight))
        .fold([0.0_f64, 0.0_f64], |[w, l], hull| {
            [
                w.max(2.0 * hull.half_extents_m[1]),
                l.max(2.0 * hull.half_extents_m[0]),
            ]
        });
    if wide > 0.0 {
        [wide + presets.street_props.hull_way_margin_m, long]
    } else {
        [0.0, 0.0]
    }
}

/// Open ground a vehicle drives through: `catalog`'s widest hull and the
/// presets' margin. A lawn's lane keeps it open either side, and a yard's
/// vehicle gate needs it to the building.
pub fn hull_way(catalog: &Catalog, presets: &PresetDefinitions) -> f64 {
    widest_hull(catalog) + presets.street_props.hull_way_margin_m
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
            .iter()
            .filter_map(|(kind, body)| {
                let props = catalog.props();
                let refusal = match props.index(kind) {
                    None => "the catalog has no such prop type".to_string(),
                    // A panel cut short draws its module repeated, not its
                    // whole art squeezed.
                    Some(index) if body.cut_to_fit && !props.get(index).appearance.modular => {
                        "a body cut to fit needs a modular appearance in the catalog".to_string()
                    }
                    Some(index) => props
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
        let mut pass = Pass::new(plan, request, presets, catalog);
        pass.stand_buildings(templates)?;
        pass.keep_aisles();
        pass.keep_paths();
        Ok(pass)
    }

    fn new(
        plan: &'a MapPlan,
        request: &'a GenerationRequest,
        presets: &'a PresetDefinitions,
        catalog: &Catalog,
    ) -> Self {
        let rule = &presets.street_props;
        let lane = widest_hull(catalog) + rule.lane_margin_m;
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
            let preset = &presets.districts[&district.kind];
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
            hull_way: hull_way(catalog, presets),
            left_open: presets
                .districts
                .values()
                .flat_map(|d| &d.props.abandoned)
                .map(|row| (row.kind.as_str(), left_open(catalog, presets, &row.kind)))
                .collect(),
            house_ids: BTreeMap::new(),
            field: Field {
                objectives: crate::skirmish::objective_clearances(plan.skirmish.as_ref())
                    .map(|(center, clearance)| Rect {
                        center,
                        axis: [1.0, 0.0],
                        half: [clearance; 2],
                    })
                    .collect(),
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
                runs: Vec::new(),
                run_grid: Grid::new(plan.size, 32.0),
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
            let template = templates
                .templates()
                .iter()
                .find(|template| template.id == placement.template_id)
                .expect("a placed template is the catalogue's: the generator chose it there");
            let building = template.materialize(placement.frame).map_err(|message| {
                vec![Diagnostic {
                    code: DiagnosticCode::InvalidPlacement,
                    feature: Some(placement.id.clone()),
                    location: "frame".into(),
                    message,
                }]
            })?;
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
                doors.push((from, out));
            }
            self.house_ids.insert(&placement.id, self.houses.len());
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
            Side::gate(rule.gate_m),
            Side::Fenced,
            Side::Fenced,
            Side::Fenced,
        ];
        self.fence_round(corners, &rule.fence, group, sides, usize::MAX);
        // The gate's way to the street is kept open, like a door's: from the
        // site's room inside the fence, through the gate, to the street.
        self.field.keep_clear(Rect {
            center: at(0.0, (room - self.rule.door_reach_m) / 2.0),
            axis: inward,
            half: [(room + self.rule.door_reach_m) / 2.0, rule.gate_m / 2.0],
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
    /// order; a panel with no legal ground, or running beside another
    /// fence nearer than a squad's way (that fence bounds the ground
    /// already), is left out, or, of a cut-to-fit kind, cut in two and each
    /// half tried in turn, down to `SPLIT_M`, so one lamp or doorway opens
    /// a gap its own width rather than a whole panel's. The answer is how
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
        let mut panels = self.fence_panels(corners, kind, group, sides);
        panels.reverse();
        while let Some(c) = panels.pop() {
            if placed >= most {
                break;
            }
            if self.field.legal(&c) && !self.field.beside_run(&c, self.rule.squad_way_m) {
                self.field.place_run(kind, &fence, &c);
                placed += 1;
            } else if fence.cut_to_fit && c.rect.half[0] >= SPLIT_M {
                let half = c.rect.half[0] / 2.0;
                let body = PropBox {
                    half_extents_m: [half, c.rect.half[1], fence.half_extents_m[2]],
                    ..fence
                };
                // The nearer half first, as the side runs.
                for side in [1.0, -1.0] {
                    let middle = add(c.rect.center, scale(c.rect.axis, side * half));
                    panels.push(Candidate {
                        group,
                        ..Candidate::new(&body, middle, c.rect.axis)
                    });
                }
            }
        }
        placed
    }

    /// The panels of a fence of `kind` round `corners` (counter-clockwise),
    /// in group `group`, along each side `sides` does not leave open, as
    /// [`side_panels`] lays them.
    fn fence_panels(
        &self,
        corners: [Point; 4],
        kind: &str,
        group: u32,
        sides: [Side; 4],
    ) -> Vec<Candidate> {
        let fence = self.body(kind);
        let mut panels = Vec::new();
        for (side, open) in sides.into_iter().enumerate() {
            let gates = match open {
                Side::Open => continue,
                Side::Fenced => Vec::new(),
                Side::Gates(gates) => gates,
            };
            let (from, to) = (corners[side], corners[(side + 1) % 4]);
            let length = distance(from, to);
            let run = scale(sub(to, from), 1.0 / length);
            for [middle, half] in side_panels(length, &fence, &gates) {
                let body = PropBox {
                    half_extents_m: [half, fence.half_extents_m[1], fence.half_extents_m[2]],
                    ..fence
                };
                panels.push(Candidate {
                    group,
                    ..Candidate::new(&body, add(from, scale(run, middle)), run)
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
        let frame = Frame::new(ring);
        let (width, depth, along, inward) = (frame.width, frame.depth, frame.along, frame.inward);
        let at = |x: f64, y: f64| frame.at(x, y);
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
            .pick(rule.boundary.iter().map(|(kind, weight)| (kind, *weight)))
            .expect("a boundary names a kind")
            .as_str();
        // On the parcel: a neighbour's run along the same edge is a run
        // beside this one (`Field::beside_run`), and only one of the two
        // stands.
        let inset = fence_inset(&self.body(kind));
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
                        let (out, depth, half) = faces(&wall)[rng.below(4) as usize];
                        let run = [-out[1], out[0]];
                        let off = self.rule.wall_gap_m
                            + SLACK_M
                            + body.half_extents_m[1]
                            + rng.range([0.0, 1.5]);
                        let along = rng.range([-half, half]);
                        if self.houses[house]
                            .doors
                            .iter()
                            .any(|(_, door)| dot(*door, out) > 0.5)
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
    /// along: parallel to the carriageway, its near edge `setback` metres
    /// past the kerb line.
    fn beside(&self, way: usize, side: f64, s: f64, body: &PropBox, setback: f64) -> Candidate {
        let line = &self.ways[way];
        let (p, run) = line.line.at(s);
        let off = self.kerb_line(line) + setback + body.half_extents_m[1];
        Candidate {
            beside: Some((way as u32, s)),
            corner: self.rule.corner_clear_m,
            ..Candidate::new(body, add(p, scale([-run[1], run[0]], side * off)), run)
        }
    }

    /// Verge furniture: each district's rows along the carriageways that
    /// run through it or along its edge. The evenly spaced rows (lamps,
    /// trees) stand in the walk behind the parked cars' line where the
    /// district parks, so they never take the kerb's room; the scattered
    /// rows come after the cars, in the room they left.
    fn verges(&mut self, scattered: bool) {
        let avenue = self.presets.towns.avenue_width_m / 2.0;
        let car = self.body(&self.rule.parking.kind);
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
                        // Behind a parked car and the room either keeps.
                        let setback = if !scattered && props.parking > 0.0 {
                            2.0 * car.half_extents_m[1] + car.clear_m.max(body.clear_m)
                        } else {
                            0.0
                        };
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
                            self.settle(way, side, s, setback, [from, to], &row.kind);
                        }
                    }
                }
            }
        }
    }

    /// Stand a `kind` beside `way` at `s`, `setback` past the kerb line, or
    /// at the nearest legal place along the way within the rule's slide,
    /// inside `within`.
    fn settle(
        &mut self,
        way: usize,
        side: f64,
        s: f64,
        setback: f64,
        within: [f64; 2],
        kind: &str,
    ) -> bool {
        let body = &self.body(kind);
        let steps = libm::floor(self.rule.slide_m / self.rule.slide_step_m) as i32;
        for step in 0..=2 * steps {
            // 0, +1, −1, +2, −2, …
            let off = (step + 1) / 2 * if step % 2 == 1 { 1 } else { -1 };
            let at = s + off as f64 * self.rule.slide_step_m;
            let half = body.half_extents_m[0];
            if at - half < within[0] || at + half > within[1] {
                continue;
            }
            let c = self.beside(way, side, at, body, setback);
            if self.field.legal(&c) {
                self.field.place(kind, body, &c);
                return true;
            }
        }
        false
    }

    /// Bodies abandoned in the road, before the kerbs are parked: askew in
    /// one half of each street, the same half its whole length, each keeping
    /// open beside it the way a hull that cannot shove it needs.
    fn abandoned(&mut self) {
        let [least, most] = self.rule.abandoned_skew_deg.map(f64::to_radians);
        for way in 0..self.ways.len() {
            if !matches!(
                self.ways[way].kind,
                SurfaceKind::Road | SurfaceKind::CountryRoad
            ) {
                continue;
            }
            let side = if self
                .stream(&format!("way-{way}/abandoned-side"))
                .chance(0.5)
            {
                1.0
            } else {
                -1.0
            };
            let mut rng = self.stream(&format!("way-{way}/abandoned"));
            for (from, to, district) in self.owners(&self.ways[way], side) {
                let props = &self.districts[district].1.props;
                for row in &props.abandoned {
                    let count = (to - from) / row.spacing_m + rng.unit();
                    for _ in 0..libm::floor(count) as usize {
                        // A few places along the stretch, each slid to room.
                        for _ in 0..self.rule.attempts {
                            let s = rng.range([from, to]);
                            let turn = if rng.chance(0.5) { 1.0 } else { -1.0 };
                            let skew = turn * rng.range([least, most]);
                            if self.strand(way, side, s, skew, [from, to], &row.kind) {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Stand a `kind` in `way`'s half on `side`, `skew` askew of the street,
    /// its outer edge on its kerb line, at `s` or the nearest place along the
    /// way within the rule's slide where the way it leaves open is clear;
    /// then keep that way clear of every body placed after it.
    fn strand(
        &mut self,
        way: usize,
        side: f64,
        s: f64,
        skew: f64,
        within: [f64; 2],
        kind: &str,
    ) -> bool {
        let body = &self.body(kind);
        let [long, wide] = [body.half_extents_m[0], body.half_extents_m[1]];
        // How far the askew box reaches across the street, and along it.
        let across = long * libm::fabs(libm::sin(skew)) + wide * libm::cos(skew);
        let ahead = long * libm::cos(skew) + wide * libm::fabs(libm::sin(skew));
        // Its outer edge on its own kerb line, where parked cars begin, but
        // never across the middle nor so far out that less than half of it
        // stands in the road. The way it leaves open runs from its inner edge
        // across the road and, where it must, into the far kerb's parking:
        // no car parks opposite one abandoned.
        let [open, hull] = self.left_open[kind];
        let kerb = self.kerb_line(&self.ways[way]);
        let inner = (kerb - 2.0 * across).clamp(0.0, self.ways[way].half_width - wide);
        let off = inner + across;
        let steps = libm::floor(self.rule.slide_m / self.rule.slide_step_m) as i32;
        for step in 0..=2 * steps {
            let at = s
                + ((step + 1) / 2 * if step % 2 == 1 { 1 } else { -1 }) as f64
                    * self.rule.slide_step_m;
            if at - long < within[0] || at + long > within[1] {
                continue;
            }
            let (p, run) = self.ways[way].line.at(at);
            let normal = [-run[1], run[0]];
            let heading = libm::atan2(run[1], run[0]) + skew;
            let c = Candidate {
                beside: Some((way as u32, at)),
                corner: self.rule.abandoned_corner_m,
                in_road: true,
                ..Candidate::new(
                    body,
                    add(p, scale(normal, side * off)),
                    [libm::cos(heading), libm::sin(heading)],
                )
            };
            let way = Rect {
                center: add(p, scale(normal, side * (inner - open / 2.0))),
                axis: run,
                half: [ahead + hull, open / 2.0],
            };
            if self.field.legal(&c) && self.field.clear_of_bodies(&way) {
                self.field.place(kind, body, &c);
                self.field.keep_clear(way);
                return true;
            }
        }
        false
    }

    /// Parked cars: beside the paved carriageways of each district that
    /// parks, a country road through a town as much as its streets, in runs
    /// with gaps between. A narrow street parks along one side, drawn for the
    /// whole street; an avenue along both.
    fn parking(&mut self) {
        let all = self.rule;
        let rule = &all.parking;
        let car = self.body(&rule.kind);
        let length = 2.0 * car.half_extents_m[0];
        let pitch = length + rule.bumper_gap_m;
        for way in 0..self.ways.len() {
            if !matches!(
                self.ways[way].kind,
                SurfaceKind::Road | SurfaceKind::CountryRoad
            ) {
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
                                    let mut c =
                                        self.beside(way, side, start + length / 2.0, &car, 0.0);
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
