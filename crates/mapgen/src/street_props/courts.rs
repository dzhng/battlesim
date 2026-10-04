//! Court amenities: the groups a dense district's court is dressed with (a
//! playground, a fenced basketball court, a row of garages), each placed
//! whole or not at all through the one legality check.
//!
//! A group asks for its own ground and a ring round it the widest hull of
//! the catalog drives through (`group_ring`): clear of every wall, door way,
//! carriageway and body, and kept clear of every body placed after it.
//! A group so set apart from everything else that stops a mover cannot close
//! a way that was open before it, for a soldier or for any vehicle: whatever
//! a mover reached before, it reaches round the group. A fenced group keeps
//! a gate, so its inside is reached on foot.
use super::field::Candidate;
use super::{Pass, Side, SLACK_M};
use crate::layout::geometry::{add, direction, distance, scale, sub, Point};
use crate::layout::rng::Stream;
use crate::layout::Group;
use crate::parcels::space::Rect;
use crate::CourtPlan;
use contract::ground::polygon_contains;
use std::collections::BTreeMap;

impl<'a> Pass<'a> {
    /// Every court of a district whose presets dress one, each court in the
    /// order its own stream draws, at most `room` bodies in all. A court
    /// draws from its district's shared groups and the map family's. The
    /// answer is how many bodies stand.
    pub(super) fn courts(&mut self, room: usize) -> usize {
        let family = crate::parcels::family(self.request, self.presets);
        let mut order = Vec::new();
        for court in &self.plan.courts {
            let Some(&district) = self.district_ids.get(court.district.as_str()) else {
                continue;
            };
            let rule = &self.districts[district].1.props.courts;
            let mut table: BTreeMap<&str, f64> = BTreeMap::new();
            for (group, weight) in rule
                .groups
                .iter()
                .chain(rule.families.get(family).into_iter().flatten())
            {
                *table.entry(group.as_str()).or_insert(0.0) += weight;
            }
            if table.is_empty() {
                continue;
            }
            let mut rng = self.stream(&format!("{}/court", court.id));
            order.push((rng.unit(), court, rule.spacing_m, table, rng));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut placed = 0;
        for (_, court, spacing, table, mut rng) in order {
            placed += self.court(court, spacing, &table, room - placed, &mut rng);
        }
        placed
    }

    /// One court: a group drawn for each square of a grid `spacing` apart
    /// over it, tried at drawn places in that square, lined up with the
    /// court's longest edge a quarter turn at a time. The answer is how many
    /// bodies stand.
    fn court(
        &mut self,
        court: &CourtPlan,
        spacing: f64,
        table: &BTreeMap<&str, f64>,
        room: usize,
        rng: &mut Stream,
    ) -> usize {
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
        let mut placed = 0;
        for row in cells(y0, y1) {
            for column in cells(x0, x1) {
                let name = rng.pick(table).expect("a court's table names a group");
                let group = &self.rule.groups[*name];
                for _ in 0..self.rule.attempts {
                    let centre = [
                        (column as f64 + rng.unit()) * spacing,
                        (row as f64 + rng.unit()) * spacing,
                    ];
                    let axis = turns[rng.below(4) as usize];
                    if let Some(stood) = self.stand_group(group, centre, axis, ring, room - placed)
                    {
                        placed += stood;
                        break;
                    }
                }
            }
        }
        placed
    }

    /// Stand `group` at `centre`, its `x` along `axis`, wholly inside `ring`,
    /// with the ground round it a vehicle drives kept open: all of its
    /// bodies, at most `room`, or none. The answer is how many stand.
    fn stand_group(
        &mut self,
        group: &Group,
        centre: Point,
        axis: Point,
        ring: &[Point],
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
        let ground = Candidate {
            rect: Rect {
                half: [half[0] + ring_m, half[1] + ring_m],
                ..outline
            },
            yaw,
            clear: 0.0,
            group: 0,
            beside: None,
            corner: 0.0,
        };
        if !self.field.legal(&ground) {
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
