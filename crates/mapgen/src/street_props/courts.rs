//! Courts: a dense district's yards, car parks and lawn, dressed through the
//! one legality check. Each yard is bounded by one kind of its region's
//! boundary, with a gate before each door and one at its rear; each car
//! park takes a car in each bay either side of its aisle; amenity groups (a
//! row of bays against a wall, a playground, a fenced basketball court)
//! stand in the yards and on the lawn, each placed whole or not at all.
//!
//! A group asks for its own ground and a ring of open ground round its open
//! sides as wide as the widest hull drives through (`hull_way`), clear of
//! every wall, carriageway and body, and kept clear of every body placed
//! after it, so a vehicle still finds ground to stand on and pass among
//! them. A wall group's back stands a squad's walk (`wall_walk_m`) off a
//! building wall that runs its whole length. Rings may share ground: each
//! stays open, so no group closes a way a squad or a vehicle had. A yard's
//! boundary keeps a vehicle's way to its building and a hull-wide gate in
//! each side, so it closes none either. A fenced group keeps a gate on its
//! front, so its inside is reached on foot.
use super::field::Candidate;
use super::{Frame, Pass, Side, SLACK_M};
use crate::layout::geometry::{add, area, direction, distance, dot, scale, sub, Point};
use crate::layout::rng::Stream;
use crate::layout::{CourtParking, Courts, Group, Yards};
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

/// The names of `table` in an order drawn by weight: each in turn the
/// heavier the likelier to come first (Efraimidis–Spirakis), so where the
/// first does not fit the next is tried.
fn drawn<'k>(table: &BTreeMap<&'k str, f64>, rng: &mut Stream) -> Vec<&'k str> {
    let mut keyed: Vec<(f64, &str)> = table
        .iter()
        .map(|(name, weight)| (-libm::log(1.0 - rng.unit()) / weight, *name))
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|(_, name)| name).collect()
}

/// `items` in an order `rng` draws, so a court whose share of parts runs
/// out is dressed here and there rather than from one side.
fn shuffled<T>(items: Vec<T>, rng: &mut Stream) -> Vec<T> {
    let mut keyed: Vec<(f64, T)> = items.into_iter().map(|item| (rng.unit(), item)).collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|(_, item)| item).collect()
}

/// One dense district as its courts are dressed: its rule, its tables, its
/// yards and car parks.
struct District<'a> {
    ring: &'a [Point],
    rule: &'a Courts,
    tables: Tables<'a>,
    yards: Vec<&'a CourtPlan>,
    parks: Vec<&'a CourtPlan>,
    rng: Stream,
}

impl<'a> Pass<'a> {
    /// Every dense district whose presets pave its courts, each in the order
    /// its own stream draws, at most the rule's court share of `room` bodies
    /// in all. Each district may take its area's share of what the districts
    /// before it left, so where parts run short every district is dressed
    /// thinly rather than some fully and the rest not at all. In each, the
    /// yards are bounded first, with at most a third of its share, then the
    /// car parks filled with at most half what is left, then its groups
    /// stood, those against a wall with at most a quarter of what is left:
    /// where parts run short, each kind of structure gets some.
    /// The answer is how many bodies stand.
    pub(super) fn courts(&mut self, room: usize) -> usize {
        let family = crate::parcels::family(self.request, self.presets);
        let plan = self.plan;
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
            for (group, weight) in rule
                .groups
                .iter()
                .chain(rule.families.get(family).into_iter().flatten())
            {
                let table = if self.rule.groups[group].wall {
                    &mut tables.wall
                } else {
                    &mut tables.open
                };
                *table.entry(group.as_str()).or_insert(0.0) += weight;
            }
            let pieces = courts.remove(district.id.as_str()).unwrap_or_default();
            let (yards, parks) = pieces.into_iter().partition(|c| c.kind == CourtKind::Yard);
            let mut rng = self.stream(&format!("{}/courts", district.id));
            order.push((
                rng.unit(),
                District {
                    ring: &district.ring,
                    rule,
                    tables,
                    yards,
                    parks,
                    rng,
                },
            ));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let budget = libm::floor(room as f64 * self.rule.court_share) as usize;
        let mut area_left: f64 = order.iter().map(|o| area(o.1.ring)).sum();
        let mut placed = 0;
        for (_, mut district) in order {
            let area = area(district.ring);
            let left = budget - placed;
            let share = (libm::floor(left as f64 * area / area_left) as usize).min(left);
            area_left -= area;
            placed += self.district_courts(&mut district, share);
        }
        placed
    }

    /// One district's courts, at most `room` bodies. The answer is how many
    /// stand.
    fn district_courts(&mut self, d: &mut District, room: usize) -> usize {
        let family = crate::parcels::family(self.request, self.presets);
        let rng = &mut d.rng;
        let mut placed = 0;
        let yards = shuffled(d.yards.clone(), rng);
        if let Some(rule) = &d.rule.yards {
            if let Some(kinds) = rule.boundary.get(family) {
                for yard in &yards {
                    let kind = rng.pick(kinds).expect("a boundary names a kind").as_str();
                    placed += self.bound(yard, rule, kind, room / 3 - placed);
                }
            }
        }
        if let Some(rule) = &d.rule.parking {
            let most = placed + (room - placed) / 2;
            for park in &d.parks {
                placed += self.park(park, rule, most - placed);
            }
        }
        let walls = (room - placed) / 4;
        let mut wall_placed = 0;
        for yard in &yards {
            let room = walls - wall_placed;
            wall_placed += self.walls(&yard.ring, d.rule, &d.tables.wall, room, rng);
        }
        placed += wall_placed;
        for yard in &yards {
            placed += self.middle(&yard.ring, &[], d.rule, &d.tables.open, room - placed, rng);
        }
        let paved: Vec<&[Point]> = yards
            .iter()
            .chain(&d.parks)
            .map(|court| court.ring.as_slice())
            .collect();
        placed += self.middle(d.ring, &paved, d.rule, &d.tables.open, room - placed, rng);
        placed
    }

    /// The boundary of `yard`, of `kind`, at most `room` bodies: along its
    /// street edge (`rule.front_inset_m` in, or less where its building
    /// stands nearer, a wall gap off it), its sides and its rear, inside the
    /// parcel by half the panel's thickness, so a neighbour's run along the
    /// same edge is within the ground each keeps and only one of the two
    /// stands. A gate `rule.gate_m` wide stands where each door's line out
    /// meets the boundary and in the middle of each side and the rear, each
    /// with its way through kept open. A side the building stands too near
    /// for a vehicle's way (`hull_way`) is left open. The answer is how many
    /// bodies stand.
    fn bound(&mut self, yard: &CourtPlan, rule: &Yards, kind: &str, room: usize) -> usize {
        let ring = &yard.ring;
        if ring.len() != 4 || room == 0 {
            return 0;
        }
        let Some(house) = self.house_on(yard) else {
            return 0;
        };
        let frame = Frame::new(ring);
        let (width, depth) = (frame.width, frame.depth);
        let thick = self.body(kind).half_extents_m[1];
        let inset = thick + SLACK_M;
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
        // A side the building stands too near for a vehicle to drive between
        // them is left open, so the boundary closes no way a vehicle or a
        // squad had between two buildings: the building's own wall bounds
        // the yard there, and a door opens on the street.
        let narrow: [bool; 4] = core::array::from_fn(|side| {
            let (a, normal) = (corners[side], sides[side].2);
            parts
                .iter()
                .map(|p| dot(sub(a, *p), normal))
                .fold(f64::INFINITY, f64::min)
                < self.hull_way + thick
        });
        let mut gates: [Vec<f64>; 4] = Default::default();
        let mut ways = Vec::new();
        for &(door, out) in &self.houses[house].doors {
            let side = (0..4)
                .max_by(|a, b| dot(sides[*a].2, out).total_cmp(&dot(sides[*b].2, out)))
                .expect("four sides");
            let (middle, run, normal) = sides[side];
            let off = dot(sub(door, middle), run);
            gates[side].push(off);
            let gate = add(middle, scale(run, off));
            // From the door out through the gate.
            let reach = dot(sub(gate, door), normal).max(0.0) + rule.gate_m;
            ways.push(Rect {
                center: add(door, scale(normal, reach / 2.0)),
                axis: normal,
                half: [reach / 2.0, rule.gate_m / 2.0],
            });
        }
        // The rear gate's way runs into the yard and through its own wall
        // only: a neighbour backing onto it closes it with its rear wall, so
        // a gate opens on a lawn, never into the next yard.
        let (rear, _, normal) = sides[2];
        gates[2].push(0.0);
        ways.push(Rect {
            center: add(rear, scale(normal, (2.0 * thick - rule.gate_m) / 2.0)),
            axis: normal,
            half: [(rule.gate_m + 2.0 * thick) / 2.0, rule.gate_m / 2.0],
        });
        // A gate in the middle of each side, its way running on into the
        // neighbour's yard, so two walled yards side by side keep a way
        // between them.
        for side in [1, 3] {
            let (middle, _, normal) = sides[side];
            gates[side].push(0.0);
            ways.push(Rect {
                center: middle,
                axis: normal,
                half: [rule.gate_m, rule.gate_m / 2.0],
            });
        }
        for way in ways {
            self.field.keep_clear(way);
        }
        let sides: [Side; 4] = core::array::from_fn(|side| {
            if narrow[side] {
                Side::Open
            } else {
                Side::Gates {
                    width: rule.gate_m,
                    at: gates[side].clone(),
                }
            }
        });
        let group = self.field.group();
        self.fence_round(corners, kind, group, sides, room)
    }

    /// The building standing on `yard`'s parcel, as an index into `houses`.
    fn house_on(&self, yard: &CourtPlan) -> Option<usize> {
        self.house_ids.get(yard.id.strip_suffix("/yard")?).copied()
    }

    /// Keep every car park's aisle open, from the far end out to the
    /// carriageway's edge at its mouth, from the first body placed.
    pub(super) fn keep_aisles(&mut self) {
        let verge = self.presets.parcels.verge_m;
        for park in self
            .plan
            .courts
            .iter()
            .filter(|c| c.kind == CourtKind::Parking)
        {
            let Some(rule) = self
                .district_ids
                .get(park.district.as_str())
                .and_then(|d| self.districts[*d].1.props.courts.parking)
            else {
                continue;
            };
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
        let ring = &park.ring;
        if ring.len() != 4 || room == 0 {
            return 0;
        }
        let frame = Frame::new(ring);
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
    /// `rule.wall_spacing_m` long, the stretches in a drawn order, at drawn
    /// places along it, each group in an order drawn by weight until one
    /// stands. The answer
    /// is how many bodies stand.
    fn walls(
        &mut self,
        ring: &[Point],
        rule: &Courts,
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
        let walk = self.rule.wall_walk_m - self.rule.wall_gap_m;
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
        for wall in walls {
            let wall = self.field.walls[wall];
            let turn = [-wall.axis[1], wall.axis[0]];
            // Each face: the way it looks, how far out it stands and half
            // its length.
            let faces = [
                (wall.axis, wall.half[0], wall.half[1]),
                (scale(wall.axis, -1.0), wall.half[0], wall.half[1]),
                (turn, wall.half[1], wall.half[0]),
                (scale(turn, -1.0), wall.half[1], wall.half[0]),
            ];
            for (out, depth, half) in faces {
                let middle = add(wall.center, scale(out, depth));
                if !polygon_contains(ring, add(middle, scale(out, gap + 1.0))) {
                    continue;
                }
                let count = libm::floor(2.0 * half / rule.wall_spacing_m).max(1.0);
                for k in 0..count as usize {
                    stretches.push((middle, out, half, 2.0 * half / count, k));
                }
            }
        }
        let mut placed = 0;
        for (middle, out, half, stretch, k) in shuffled(stretches, rng) {
            if placed == room {
                break;
            }
            // Along the face, with the court on its left: a group's `x`.
            let run = [-out[1], out[0]];
            let from = -half + k as f64 * stretch;
            'groups: for name in drawn(table, rng) {
                let group = &self.rule.groups[name];
                let [gx, gy] = group.size_m.map(|v| v / 2.0);
                if gx > half {
                    continue;
                }
                for _ in 0..self.rule.attempts {
                    let s = (from + rng.unit() * stretch).clamp(-half + gx, half - gx);
                    let centre = add(middle, add(scale(out, gap + walk + gy), scale(run, s)));
                    let at = Stand {
                        centre,
                        axis: run,
                        behind: walk,
                    };
                    if let Some(stood) = self.stand_group(group, at, ring, &[], room - placed) {
                        placed += stood;
                        break 'groups;
                    }
                }
            }
        }
        placed
    }

    /// Open groups over `ring`, off every one of `paved`, at most `room`
    /// bodies: for each square of a grid `rule.spacing_m` apart, the squares
    /// in a drawn order, groups in an order drawn by weight, each tried at
    /// drawn places in that square until one stands, lined up with the ring's longest edge a quarter
    /// turn at a time. The answer is how many bodies stand.
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
        let mut placed = 0;
        for (column, row) in shuffled(squares, rng) {
            if placed == room {
                break;
            }
            'groups: for name in drawn(table, rng) {
                let group = &self.rule.groups[name];
                for _ in 0..self.rule.attempts {
                    let centre = [
                        (column as f64 + rng.unit()) * spacing,
                        (row as f64 + rng.unit()) * spacing,
                    ];
                    let at = Stand {
                        centre,
                        axis: turns[rng.below(4) as usize],
                        behind: self.hull_way,
                    };
                    if let Some(stood) = self.stand_group(group, at, ring, paved, room - placed) {
                        placed += stood;
                        break 'groups;
                    }
                }
            }
        }
        placed
    }

    /// Stand `group` as `at` puts it, wholly inside `ring` and off every one
    /// of `paved`, with the ground round its front and ends kept open
    /// a vehicle's way (`hull_way`) wide and `at.behind` behind it. All of its bodies, at most
    /// `room`, or none. The answer is how many stand.
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
        let ring_m = self.hull_way;
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
            let inset = self.body(fence).half_extents_m[1] + SLACK_M;
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
