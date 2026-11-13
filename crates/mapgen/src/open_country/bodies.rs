//! Loose bodies in the open country: one body placed, and the clusters of
//! low cover in the fields.
use super::*;

impl<'a> Country<'a> {
    pub(super) fn body(&mut self, kind: &str, half: [f64; 3], at: Point, yaw: f64, radius: f64) {
        let at = round_cm(at);
        self.props.push(AuthoredPropDefinition {
            id: None,
            geometry: PropDefinition {
                kind: kind.into(),
                center: at,
                yaw: libm::round(yaw * 1e6) / 1e6,
                half_extents: half,
                base_z: None,
                wreck_of: None,
            },
        });
        self.bodies.push((at, radius));
        self.tally.cover[self.half(at)] += 1.0;
    }

    /// A few bodies of one kind about `p`.
    pub(super) fn cover(&mut self, p: Point, rng: &mut Stream) -> bool {
        let rules = self.rules;
        let f = &rules.field_cover;
        let row = rng
            .pick(f.bodies.iter().map(|row| (row, row.weight)))
            .expect("field cover has bodies");
        let radius = libm::hypot(row.half_extents_m[0], row.half_extents_m[1]);
        let ask = Ask {
            radius: radius + f.gap_m,
            blocks_sight: false,
            road_gap: rules.clear.road_m,
            own_from: usize::MAX,
        };
        let heading = rng.range([0.0, TAU]);
        // The cluster is found whole before any of it stands, so its own
        // bodies are judged against each other here and not as strangers.
        let mut cluster: Vec<(Point, f64)> = Vec::new();
        for index in 0..rng.count(row.count) {
            let (at, yaw) = if row.stacked {
                // Side by side across the heading.
                let step = 2.0 * row.half_extents_m[1] + 0.1;
                let across = direction(heading + TAU / 4.0);
                (add(p, scale(across, step * f64::from(index))), heading)
            } else if index == 0 {
                (p, heading)
            } else {
                let away = rng.range([0.0, f.spread_m]);
                (
                    add(p, scale(direction(rng.range([0.0, TAU])), away)),
                    rng.range([0.0, TAU]),
                )
            };
            let apart = row.stacked
                || cluster
                    .iter()
                    .all(|(other, _)| distance(*other, at) >= 2.0 * radius + f.gap_m);
            let free = self
                .groves
                .iter()
                .all(|(grove, reach)| distance(*grove, at) >= radius + reach);
            if apart && free && self.open(at, ask) {
                cluster.push((at, yaw));
            } else if index == 0 {
                return false;
            }
        }
        for (at, yaw) in cluster {
            self.body(&row.kind, row.half_extents_m, at, yaw, radius);
        }
        true
    }
}
