//! Courts: a dense district's yards, car parks and lawn, dressed through the
//! one legality check. Each yard is bounded by its district's one kind of
//! its region's boundary wherever a squad passes between boundary and
//! building, with a squad's gate before each door and at its rear and one
//! gate a vehicle drives through where a side leaves it room; each car park
//! takes a car in each bay either side of its aisle; groups are planted
//! beside the lawn's footpaths, facing them (a lane's edges are left open);
//! amenity groups (a row of bays against a wall, a playground, a fenced
//! basketball court) stand in the yards and on the lawn, each placed whole
//! or not at all.
//!
//! A court's inside is infantry ground: a vehicle is owed a way in, not a
//! way everywhere. A group asks for its own ground and a ring a squad
//! passes abreast (`group_ring_m`) round its open sides, clear of every
//! wall, carriageway and body, and kept clear of every body placed after
//! it, so a squad still passes among them; a wall group's back stands a
//! squad's walk (`squad_way_m`) off a building wall that runs its whole
//! length. Rings may share ground: each stays open. The vehicle's ways in
//! are kept by others: each car park's aisle, each yard's vehicle gate, and
//! each lawn's lane, the lawn either side of it kept open the widest hull's
//! way (`hull_way`). A fenced group keeps a gate on its front, so its
//! inside is reached on foot.
use super::field::{faces, Candidate};
use super::{fence_inset, Frame, Pass, Side, SLACK_M};
use crate::layout::geometry::{add, area, direction, distance, dot, scale, sub, Point};
use crate::layout::rng::{in_drawn_order, Stream};
use crate::layout::{CourtParking, Courts, Group, Lawn, Yards};
use crate::parcels::space::Rect;
use crate::{CourtKind, CourtPlan};
use contract::ground::polygon_contains;
use std::collections::BTreeMap;

/// A district's groups by weight: those against a building's wall, in its
/// yards, and those in the open, in its yards and on its lawn.
#[derive(Default)]
struct Tables<'a> {
    wall: BTreeMap<&'a str, f64>,
    open: BTreeMap<&'a str, f64>,
}

/// One dense district as its courts are dressed: its rule, its tables, its
/// yards and car parks.
struct District<'a> {
    ring: &'a [Point],
    rule: &'a Courts,
    tables: Tables<'a>,
    yards: Vec<&'a CourtPlan>,
    parks: Vec<&'a CourtPlan>,
    paths: Vec<&'a CourtPlan>,
    rng: Stream,
}

impl<'a> Pass<'a> {
    /// Every dense district whose presets pave its courts, each in the order
    /// its own stream draws, at most the courts' share of `room` bodies in
    /// all. Each district may take its area's share of what the districts
    /// before it left, so where parts run short every district is dressed
    /// thinly rather than some fully and the rest not at all; within one,
    /// its parts are split as `district_courts` says. The answer is how
    /// many bodies stand.
    pub(super) fn courts(&mut self, room: usize) -> usize {
        let plan = self.plan;
        let family = plan.regional_family.as_deref();
        let mut courts: BTreeMap<&str, Vec<&CourtPlan>> = BTreeMap::new();
        for court in &plan.courts {
            courts
                .entry(court.district.as_str())
                .or_default()
                .push(court);
        }
        let mut order = Vec::new();
        for (district, preset) in &self.districts {
            let rule = &preset.props.courts;
            if !rule.paved {
                continue;
            }
            let mut tables = Tables::default();
            for (group, weight) in rule.groups.iter().chain(
                family
                    .and_then(|f| rule.families.get(f))
                    .into_iter()
                    .flatten(),
            ) {
                let table = if self.rule.groups[group].wall {
                    &mut tables.wall
                } else {
                    &mut tables.open
                };
                *table.entry(group.as_str()).or_insert(0.0) += weight;
            }
            let pieces = courts.remove(district.id.as_str()).unwrap_or_default();
            let of = |kinds: &[CourtKind]| -> Vec<&CourtPlan> {
                pieces
                    .iter()
                    .copied()
                    .filter(|c| kinds.contains(&c.kind))
                    .collect()
            };
            let (yards, parks) = (of(&[CourtKind::Yard]), of(&[CourtKind::Parking]));
            let paths = of(&[CourtKind::Path, CourtKind::Lane]);
            let mut rng = self.stream(&format!("{}/courts", district.id));
            order.push((
                rng.unit(),
                District {
                    ring: &district.ring,
                    rule,
                    tables,
                    yards,
                    parks,
                    paths,
                    rng,
                },
            ));
        }
        let order = in_drawn_order(order);
        let budget = libm::floor(room as f64 * self.rule.courts.share) as usize;
        let mut area_left: f64 = order.iter().map(|d| area(d.ring)).sum();
        let mut placed = 0;
        for mut district in order {
            let area = area(district.ring);
            let left = budget - placed;
            let share = (libm::floor(left as f64 * area / area_left) as usize).min(left);
            area_left -= area;
            placed += self.district_courts(&mut district, share);
        }
        placed
    }

    /// One district's courts, at most `room` bodies: its yards bounded
    /// first, then its car parks filled, then its lawn's paths planted,
    /// then its groups stood, those against a wall first, each kind of
    /// structure with at most its split of what the ones before it left.
    /// The answer is how many stand.
    fn district_courts(&mut self, d: &mut District, room: usize) -> usize {
        let shared = &self.rule.courts;
        let split = shared.split;
        let rng = &mut d.rng;
        let mut placed = 0;
        // In a drawn order, so a court whose share of parts runs out is
        // dressed here and there rather than from one side.
        let yards = rng.shuffled(d.yards.clone());
        let boundary = shared.yards.as_ref().and_then(|rule| {
            let family = self.plan.regional_family.as_deref()?;
            Some((rule, rule.boundary.get(family)?))
        });
        if let Some((rule, kinds)) = boundary {
            // One kind for the district: its yards' runs meet and continue
            // each other, so a block's boundary reads as one.
            let kind = rng.pick(kinds).expect("a boundary names a kind").as_str();
            let most = placed + (room - placed) / split.yards;
            for yard in &yards {
                placed += self.bound(yard, rule, kind, most - placed);
            }
        }
        if let Some(rule) = &shared.parking {
            let most = placed + (room - placed) / split.parking;
            for park in &d.parks {
                placed += self.park(park, rule, most - placed);
            }
        }
        let paved: Vec<&[Point]> = yards
            .iter()
            .chain(&d.parks)
            .chain(&d.paths)
            .map(|court| court.ring.as_slice())
            .collect();
        if let Some(lawn) = &shared.lawn {
            let beside: BTreeMap<&str, f64> =
                lawn.beside.iter().map(|(k, w)| (k.as_str(), *w)).collect();
            let most = placed + (room - placed) / split.paths;
            for path in rng.shuffled(d.paths.clone()) {
                let room = most - placed;
                placed += self.plant_path(path, d.ring, &paved, lawn, &beside, room, rng);
            }
        }
        let most = placed + (room - placed) / split.walls;
        for yard in &yards {
            placed += self.walls(&yard.ring, &d.tables.wall, most - placed, rng);
        }
        for yard in &yards {
            placed += self.middle(&yard.ring, &[], d.rule, &d.tables.open, room - placed, rng);
        }
        placed += self.middle(d.ring, &paved, d.rule, &d.tables.open, room - placed, rng);
        placed
    }

    /// The width of ground `path` keeps open: its own, or a lane's, the
    /// lawn's `lane_m` and at least a vehicle's way (`hull_way`).
    fn kept_wide(&self, path: &CourtPlan) -> f64 {
        let width = Frame::new(&path.ring).width;
        if path.kind != CourtKind::Lane {
            return width;
        }
        let lane = self
            .rule
            .courts
            .lawn
            .as_ref()
            .map_or(0.0, |lawn| lawn.lane_m);
        lane.max(self.hull_way).max(width)
    }

    /// Keep every lawn path open, and the lawn either side of a lane as
    /// `kept_wide` says, from the first body placed.
    pub(super) fn keep_paths(&mut self) {
        let plan = self.plan;
        for path in plan
            .courts
            .iter()
            .filter(|c| matches!(c.kind, CourtKind::Path | CourtKind::Lane))
        {
            let frame = Frame::new(&path.ring);
            let wide = self.kept_wide(path);
            self.field.keep_clear(Rect {
                center: frame.at(frame.width / 2.0, frame.depth / 2.0),
                axis: frame.inward,
                half: [frame.depth / 2.0, wide / 2.0],
            });
        }
    }

    /// Groups of `table` planted along both sides of footpath `path`,
    /// facing it, at most `room` bodies (none along a lane): one tried to
    /// each stretch `lawn.beside_spacing_m` long, at drawn places along it,
    /// as [`Pass::try_groups`] tries them, on the lawn of `ring`, off every
    /// one of `paved` and the ground the path keeps. The answer is how many
    /// bodies stand.
    #[allow(clippy::too_many_arguments)]
    fn plant_path(
        &mut self,
        path: &CourtPlan,
        ring: &[Point],
        paved: &[&[Point]],
        lawn: &Lawn,
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
        // A lane's edges are left open: a vehicle turns off it onto the lawn.
        if table.is_empty() || room == 0 || path.kind == CourtKind::Lane {
            return 0;
        }
        let frame = Frame::new(&path.ring);
        // Half the open ground the path keeps, and the slack.
        let kept = self.kept_wide(path) / 2.0 + SLACK_M;
        let count = libm::floor(frame.depth / lawn.beside_spacing_m) as usize;
        let mut placed = 0;
        for k in 0..count {
            for side in [-1.0, 1.0] {
                if placed == room {
                    return placed;
                }
                // Out from the path, and along it with the path on the
                // group's front.
                let out = scale(frame.along, side);
                let axis = [out[1], -out[0]];
                let spacing = lawn.beside_spacing_m;
                let behind = self.rule.squad_way_m;
                let at = |group: &Group, rng: &mut Stream| {
                    let [gx, gy] = group.size_m.map(|v| v / 2.0);
                    if 2.0 * gx > spacing {
                        return None;
                    }
                    let s = ((k as f64 + rng.unit()) * spacing)
                        .clamp(k as f64 * spacing + gx, (k + 1) as f64 * spacing - gx);
                    Some(Stand {
                        centre: add(frame.at(frame.width / 2.0, s), scale(out, kept + gy)),
                        axis,
                        behind,
                    })
                };
                placed += self.try_groups(table, (ring, paved), room - placed, rng, at);
            }
        }
        placed
    }

    /// The boundary of `yard`, of `kind`, at most `room` bodies: along its
    /// street edge (`rule.front_inset_m` in, or less where its building
    /// stands nearer, a wall gap off it), its sides and its rear, on the
    /// parcel, so a neighbour's run along the same edge is a run beside it
    /// (`Field::beside_run`) and only one of the two stands. A side is
    /// bounded only where a squad passes between it and the building
    /// (`squad_way_m`); nearer, the building's own wall bounds the yard. A gate `rule.door_gate_m` wide stands where each door's
    /// line out meets the boundary and in the middle of the rear; one
    /// `rule.gate_m` wide, the yard's way in for a vehicle, in the middle of
    /// the first of its street side, rear and sides that leaves a vehicle's
    /// way (`hull_way`) to the building, in place of the rear's where that
    /// is the rear. Each gate's way through is kept open. The answer is how
    /// many bodies stand.
    fn bound(&mut self, yard: &CourtPlan, rule: &Yards, kind: &str, room: usize) -> usize {
        let ring = &yard.ring;
        if room == 0 {
            return 0;
        }
        let Some(house) = self.house_on(yard) else {
            return 0;
        };
        let frame = Frame::new(ring);
        let (width, depth) = (frame.width, frame.depth);
        let fence = self.body(kind);
        let thick = fence.half_extents_m[1];
        let inset = fence_inset(&fence);
        let parts: Vec<Point> = self.houses[house]
            .walls
            .clone()
            .flat_map(|wall| self.field.walls[wall].corners())
            .collect();
        let building = parts
            .iter()
            .map(|p| dot(sub(*p, ring[0]), frame.inward))
            .fold(f64::INFINITY, f64::min);
        let front = rule
            .front_inset_m
            .min(building - self.rule.wall_gap_m - 2.0 * thick - SLACK_M)
            .max(inset);
        let corners = [
            frame.at(inset, front),
            frame.at(width - inset, front),
            frame.at(width - inset, depth - inset),
            frame.at(inset, depth - inset),
        ];
        // Each side: its middle, the way it runs and the way out of the yard.
        let sides: [(Point, Point, Point); 4] = core::array::from_fn(|side| {
            let (a, b) = (corners[side], corners[(side + 1) % 4]);
            let run = scale(sub(b, a), 1.0 / distance(a, b));
            (scale(add(a, b), 0.5), run, [run[1], -run[0]])
        });
        // The open ground between each side and the building.
        let room_to: [f64; 4] = core::array::from_fn(|side| {
            let (a, normal) = (corners[side], sides[side].2);
            parts
                .iter()
                .map(|p| dot(sub(a, *p), normal))
                .fold(f64::INFINITY, f64::min)
                - thick
        });
        let vehicle = [0, 2, 1, 3]
            .into_iter()
            .find(|side| room_to[*side] >= self.hull_way);
        let mut gates: [Vec<(f64, f64)>; 4] = Default::default();
        let mut ways = Vec::new();
        for &(door, out) in &self.houses[house].doors {
            let side = (0..4)
                .max_by(|a, b| dot(sides[*a].2, out).total_cmp(&dot(sides[*b].2, out)))
                .expect("four sides");
            let (middle, run, normal) = sides[side];
            let off = dot(sub(door, middle), run);
            gates[side].push((off, rule.door_gate_m));
            let gate = add(middle, scale(run, off));
            // From the door out through the gate.
            let reach = dot(sub(gate, door), normal).max(0.0) + rule.door_gate_m;
            ways.push(Rect {
                center: add(door, scale(normal, reach / 2.0)),
                axis: normal,
                half: [reach / 2.0, rule.door_gate_m / 2.0],
            });
        }
        // A gate in the middle of a side: its way runs into the yard and
        // through its own boundary only, so a neighbour backing onto it
        // closes it with its own run, and a gate opens on a lawn or a
        // street, never into the next yard.
        let middles = vehicle
            .map(|side| (side, rule.gate_m))
            .into_iter()
            .chain((vehicle != Some(2)).then_some((2, rule.door_gate_m)));
        for (side, gate) in middles {
            let (middle, _, normal) = sides[side];
            gates[side].push((0.0, gate));
            ways.push(Rect {
                center: add(middle, scale(normal, (2.0 * thick - gate) / 2.0)),
                axis: normal,
                half: [(gate + 2.0 * thick) / 2.0, gate / 2.0],
            });
        }
        for way in ways {
            self.field.keep_clear(way);
        }
        let sides: [Side; 4] = core::array::from_fn(|side| {
            if room_to[side] < self.rule.squad_way_m {
                Side::Open
            } else {
                Side::Gates(gates[side].clone())
            }
        });
        let group = self.field.group();
        self.fence_round(corners, kind, group, sides, room)
    }

    /// The building standing on `yard`'s parcel, as an index into `houses`.
    fn house_on(&self, yard: &CourtPlan) -> Option<usize> {
        self.house_ids.get(yard.parcel()?).copied()
    }

    /// Keep every car park's aisle open, from the far end out to the
    /// carriageway's edge at its mouth, from the first body placed.
    pub(super) fn keep_aisles(&mut self) {
        let verge = self.presets.parcels.verge_m;
        let Some(rule) = self.rule.courts.parking else {
            return;
        };
        for park in self
            .plan
            .courts
            .iter()
            .filter(|c| c.kind == CourtKind::Parking)
        {
            let frame = Frame::new(&park.ring);
            self.field.keep_clear(Rect {
                center: frame.at(frame.width / 2.0, (frame.depth - verge) / 2.0),
                axis: frame.inward,
                half: [(frame.depth + verge) / 2.0, rule.aisle_m / 2.0],
            });
        }
    }

    /// The cars of `park`, at most `room`: one in each bay of the two rows
    /// either side of its aisle, nose to the aisle (which `keep_aisles`
    /// keeps open). The answer is how many stand.
    fn park(&mut self, park: &CourtPlan, rule: &CourtParking, room: usize) -> usize {
        if room == 0 {
            return 0;
        }
        let frame = Frame::new(&park.ring);
        let kind = self.rule.parking.kind.clone();
        let car = self.body(&kind);
        let group = self.field.group();
        let bays = libm::floor(frame.depth / rule.bay_m[0] + 1e-6) as usize;
        let mut placed = 0;
        for x in [rule.bay_m[1] / 2.0, frame.width - rule.bay_m[1] / 2.0] {
            for bay in 0..bays {
                if placed == room {
                    return placed;
                }
                let y = (bay as f64 + 0.5) * rule.bay_m[0];
                let c = Candidate {
                    group,
                    ..Candidate::new(&car, frame.at(x, y), frame.along)
                };
                if self.field.legal(&c) {
                    self.field.place(&kind, &car, &c);
                    placed += 1;
                }
            }
        }
        placed
    }

    /// Wall groups along every face of a building wall that looks into
    /// `ring`, at most `room` bodies: one tried to each stretch
    /// `courts.wall_spacing_m` long, the stretches in a drawn order, at
    /// drawn places along it, as [`Pass::try_groups`] tries them. The
    /// answer is how many bodies stand.
    fn walls(
        &mut self,
        ring: &[Point],
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
        if table.is_empty() || room == 0 {
            return 0;
        }
        // A group's ground starts a wall gap off the wall, the group itself
        // a squad's walk off it.
        let gap = self.rule.wall_gap_m + SLACK_M;
        let walk = self.rule.squad_way_m - self.rule.wall_gap_m;
        let mut walls = Vec::new();
        self.field
            .wall_grid
            .any(contract::ground::limits(ring, 0.0), |item| {
                walls.push(item as usize);
                false
            });
        walls.sort_unstable();
        walls.dedup();
        // Each stretch: the face's middle, the way it looks, half its
        // length, and where along it the stretch starts.
        let mut stretches = Vec::new();
        let spacing = self.rule.courts.wall_spacing_m;
        for wall in walls {
            let wall = self.field.walls[wall];
            for (out, depth, half) in faces(&wall) {
                let middle = add(wall.center, scale(out, depth));
                if !polygon_contains(ring, add(middle, scale(out, gap + 1.0))) {
                    continue;
                }
                let count = libm::floor(2.0 * half / spacing).max(1.0);
                for k in 0..count as usize {
                    stretches.push((middle, out, half, 2.0 * half / count, k));
                }
            }
        }
        let mut placed = 0;
        for (middle, out, half, stretch, k) in rng.shuffled(stretches) {
            if placed == room {
                break;
            }
            // Along the face, with the court on its left: a group's `x`.
            let run = [-out[1], out[0]];
            let from = -half + k as f64 * stretch;
            let at = |group: &Group, rng: &mut Stream| {
                let [gx, gy] = group.size_m.map(|v| v / 2.0);
                if gx > half {
                    return None;
                }
                let s = (from + rng.unit() * stretch).clamp(-half + gx, half - gx);
                Some(Stand {
                    centre: add(middle, add(scale(out, gap + walk + gy), scale(run, s))),
                    axis: run,
                    behind: walk,
                })
            };
            placed += self.try_groups(table, (ring, &[]), room - placed, rng, at);
        }
        placed
    }

    /// Open groups over `ring`, off every one of `paved`, at most `room`
    /// bodies: for each square of a grid `rule.spacing_m` apart, the squares
    /// in a drawn order, as [`Pass::try_groups`] tries them at drawn places
    /// in that square, lined up with the ring's longest edge a quarter turn
    /// at a time. The answer is how many bodies stand.
    #[allow(clippy::too_many_arguments)]
    fn middle(
        &mut self,
        ring: &[Point],
        paved: &[&[Point]],
        rule: &Courts,
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
        if table.is_empty() || room == 0 {
            return 0;
        }
        let spacing = rule.spacing_m;
        let edge = (0..ring.len())
            .map(|i| (ring[i], ring[(i + 1) % ring.len()]))
            .max_by(|a, b| distance(a.0, a.1).total_cmp(&distance(b.0, b.1)))
            .expect("a court has edges");
        let along = scale(sub(edge.1, edge.0), 1.0 / distance(edge.0, edge.1));
        let turns = [
            along,
            [-along[1], along[0]],
            scale(along, -1.0),
            [along[1], -along[0]],
        ];
        let [x0, y0, x1, y1] = contract::ground::limits(ring, 0.0);
        let cells = |low: f64, high: f64| {
            libm::floor(low / spacing) as i64..=libm::floor(high / spacing) as i64
        };
        let squares: Vec<(i64, i64)> = cells(y0, y1)
            .flat_map(|row| cells(x0, x1).map(move |column| (column, row)))
            .collect();
        let behind = self.rule.group_ring_m;
        let mut placed = 0;
        for (column, row) in rng.shuffled(squares) {
            if placed == room {
                break;
            }
            let at = |_: &Group, rng: &mut Stream| {
                let centre = [
                    (column as f64 + rng.unit()) * spacing,
                    (row as f64 + rng.unit()) * spacing,
                ];
                Some(Stand {
                    centre,
                    axis: turns[rng.below(4) as usize],
                    behind,
                })
            };
            placed += self.try_groups(table, (ring, paved), room - placed, rng, at);
        }
        placed
    }

    /// One group of `table` stood inside `ring` and off every one of
    /// `paved`, at most `room` bodies: the groups in an order drawn by
    /// weight, each tried at up to `attempts` places `at` draws (none where
    /// it says the group does not fit) until one stands. The answer is how
    /// many bodies stand.
    fn try_groups(
        &mut self,
        table: &BTreeMap<&str, f64>,
        (ring, paved): (&[Point], &[&[Point]]),
        room: usize,
        rng: &mut Stream,
        mut at: impl FnMut(&Group, &mut Stream) -> Option<Stand>,
    ) -> usize {
        let rule = self.rule;
        for name in rng.drawn(table) {
            let group = &rule.groups[*name];
            for _ in 0..rule.attempts {
                let Some(place) = at(group, rng) else {
                    break;
                };
                if let Some(stood) = self.stand_group(group, place, ring, paved, room) {
                    return stood;
                }
            }
        }
        0
    }

    /// Stand `group` as `at` puts it, wholly inside `ring` and off every one
    /// of `paved`, with the ground round its front and ends kept open a
    /// group's ring (`group_ring_m`) wide and `at.behind` behind it. All of
    /// its bodies, at most `room`, or none. The answer is how many stand.
    fn stand_group(
        &mut self,
        group: &Group,
        at: Stand,
        ring: &[Point],
        paved: &[&[Point]],
        room: usize,
    ) -> Option<usize> {
        let Stand {
            centre,
            axis,
            behind,
        } = at;
        let ring_m = self.rule.group_ring_m;
        let half = group.size_m.map(|v| v / 2.0);
        let across = [-axis[1], axis[0]];
        let at = |[x, y]: [f64; 2]| add(centre, add(scale(axis, x), scale(across, y)));
        let outline = Rect {
            center: centre,
            axis,
            half,
        };
        if !outline.corners().iter().all(|p| polygon_contains(ring, *p))
            || paved.iter().any(|court| outline.touches(court))
        {
            return None;
        }
        let yaw = libm::atan2(axis[1], axis[0]);
        // The front is `y` least, the back `y` most.
        let ground = Candidate {
            rect: Rect {
                center: add(centre, scale(across, (behind - ring_m) / 2.0)),
                axis,
                half: [half[0] + ring_m, half[1] + (ring_m + behind) / 2.0],
            },
            yaw,
            clear: 0.0,
            group: 0,
            beside: None,
            corner: 0.0,
        };
        if !self.field.open(&ground) {
            return None;
        }
        let id = self.field.group();
        let mut wanted: Vec<(&str, Candidate)> = group
            .pieces
            .iter()
            .map(|piece| {
                let body = self.body(&piece.kind);
                let heading = direction(yaw + piece.yaw_deg.to_radians());
                let c = Candidate::new(&body, at(piece.at), heading);
                (piece.kind.as_str(), Candidate { group: id, ..c })
            })
            .collect();
        if let (Some(fence), Some(gate)) = (&group.fence, group.gate_m) {
            let inset = fence_inset(&self.body(fence));
            let [hx, hy] = [half[0] - inset, half[1] - inset];
            let corners = [at([-hx, -hy]), at([hx, -hy]), at([hx, hy]), at([-hx, hy])];
            let sides = [Side::gate(gate), Side::Fenced, Side::Fenced, Side::Fenced];
            for c in self.fence_panels(corners, fence, id, sides) {
                wanted.push((fence.as_str(), c));
            }
        }
        if wanted.len() > room || !wanted.iter().all(|(_, c)| self.field.legal(c)) {
            return None;
        }
        for (kind, c) in &wanted {
            let body = self.body(kind);
            self.field.place(kind, &body, c);
        }
        // No body placed later stands in the ground kept round it.
        self.field.keep_clear(ground.rect);
        Some(wanted.len())
    }
}

/// Where a group is asked to stand: its middle, the way its `x` runs, and
/// the open ground kept behind it.
#[derive(Clone, Copy)]
struct Stand {
    centre: Point,
    axis: Point,
    behind: f64,
}
