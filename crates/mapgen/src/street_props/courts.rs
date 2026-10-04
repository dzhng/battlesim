//! Court amenities: the groups a dense district's court is dressed with (a
//! row of parking bays, a playground, a fenced basketball court), each placed
//! whole or not at all through the one legality check. Wall groups stand
//! with their backs to the buildings' walls; open groups then take the
//! middle.
//!
//! A group asks for its own ground and a ring round its open sides the
//! widest hull of the catalog drives through (`group_ring`): clear of every
//! wall, carriageway and body, and kept clear of every body placed after it.
//! An open group's ring runs all round it. A wall group's back stands a
//! squad's walk off a wall that runs its whole length, kept open as well, so
//! a squad passes behind it and no vehicle does; its ring runs round its
//! other three sides to that wall. Rings may share ground: each stays open.
//! A group so set apart from everything else that stops a mover cannot
//! close a way that was open before it, for a soldier or for any vehicle:
//! whatever a mover reached before, it reaches round the group. A fenced
//! group keeps a gate on its front, so its inside is reached on foot.
use super::field::Candidate;
use super::{Pass, Side, SLACK_M};
use crate::layout::geometry::{add, area, direction, distance, scale, sub, Point};
use crate::layout::rng::Stream;
use crate::layout::{Courts, Group};
use crate::parcels::space::Rect;
use crate::CourtPlan;
use contract::ground::polygon_contains;
use std::collections::BTreeMap;

/// A court's groups by weight, wall and open groups apart.
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

impl<'a> Pass<'a> {
    /// Every court of a district whose presets dress one, each court in the
    /// order its own stream draws, at most the rule's court share of `room`
    /// bodies in all. Each court may take its area's share of what the
    /// courts before it left, so where parts run short every court is
    /// dressed thinly rather than some fully and the rest not at all. A
    /// court draws from its district's shared groups and the map family's:
    /// its wall groups first, with at most a quarter of its share, so the middle,
    /// where an empty court shows, keeps the rest. The answer is how many
    /// bodies stand.
    pub(super) fn courts(&mut self, room: usize) -> usize {
        let family = crate::parcels::family(self.request, self.presets);
        let mut order = Vec::new();
        for court in &self.plan.courts {
            let Some(&district) = self.district_ids.get(court.district.as_str()) else {
                continue;
            };
            let rule = &self.districts[district].1.props.courts;
            let mut tables = Tables {
                wall: BTreeMap::new(),
                open: BTreeMap::new(),
            };
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
            if tables.wall.is_empty() && tables.open.is_empty() {
                continue;
            }
            let mut rng = self.stream(&format!("{}/court", court.id));
            order.push((rng.unit(), court, rule, tables, rng));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let budget = libm::floor(room as f64 * self.rule.court_share) as usize;
        let mut area_left: f64 = order.iter().map(|o| area(&o.1.ring)).sum();
        let mut placed = 0;
        for (_, court, rule, tables, mut rng) in order {
            let area = area(&court.ring);
            let left = budget - placed;
            let share = (libm::floor(left as f64 * area / area_left) as usize).min(left);
            area_left -= area;
            let walls = self.walls(court, rule, &tables.wall, share / 4, &mut rng);
            placed += walls + self.middle(court, rule, &tables.open, share - walls, &mut rng);
        }
        placed
    }

    /// Wall groups along every face of a building wall that looks into the
    /// court, at most `room` bodies: one tried to each stretch
    /// `rule.wall_spacing_m` long, the stretches in a drawn order, at drawn
    /// places along it, each group in an order drawn by weight until one
    /// stands. The answer is how many bodies stand.
    fn walls(
        &mut self,
        court: &CourtPlan,
        rule: &Courts,
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
        if table.is_empty() {
            return 0;
        }
        let ring = &court.ring;
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
                    if let Some(stood) =
                        self.stand_group(group, centre, run, ring, walk, room - placed)
                    {
                        placed += stood;
                        break 'groups;
                    }
                }
            }
        }
        placed
    }

    /// Open groups over the court, at most `room` bodies: for each square of
    /// a grid `rule.spacing_m` apart, the squares in a drawn order, groups in
    /// an order drawn by weight, each tried at drawn places in that square
    /// until one stands, lined up with the court's longest edge a quarter
    /// turn at a time. The answer is how many bodies stand.
    fn middle(
        &mut self,
        court: &CourtPlan,
        rule: &Courts,
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
        if table.is_empty() {
            return 0;
        }
        let spacing = rule.spacing_m;
        let ring = &court.ring;
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
                    let axis = turns[rng.below(4) as usize];
                    if let Some(stood) =
                        self.stand_group(group, centre, axis, ring, self.group_ring, room - placed)
                    {
                        placed += stood;
                        break 'groups;
                    }
                }
            }
        }
        placed
    }

    /// Stand `group` at `centre`, its `x` along `axis`, wholly inside `ring`,
    /// with the ground round its front and ends a vehicle drives kept open,
    /// and `behind` metres of ground kept open behind it: the same ring where
    /// it stands in the open, a squad's walk where it backs onto a wall. All
    /// of its bodies, at most `room`, or none. The answer is how many stand.
    fn stand_group(
        &mut self,
        group: &Group,
        centre: Point,
        axis: Point,
        ring: &[Point],
        behind: f64,
        room: usize,
    ) -> Option<usize> {
        let half = group.size_m.map(|v| v / 2.0);
        let across = [-axis[1], axis[0]];
        let at = |[x, y]: [f64; 2]| add(centre, add(scale(axis, x), scale(across, y)));
        let outline = Rect {
            center: centre,
            axis,
            half,
        };
        if !outline.corners().iter().all(|p| polygon_contains(ring, *p)) {
            return None;
        }
        let ring_m = self.group_ring;
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
            let sides = [Side::Gate(gate), Side::Fenced, Side::Fenced, Side::Fenced];
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
