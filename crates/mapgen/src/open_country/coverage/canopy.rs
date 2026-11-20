//! A wood in the sight raster: the fog cells its canopy covers, and those
//! it covers with a metre to spare, which may witness. The canopy is the
//! simulation's (`contract::forest`): the wood's own ground under its stands.
use super::*;

impl Coverage<'_> {
    pub(super) fn stamp_forest(
        &mut self,
        i: usize,
        f: &Forest,
        mut changed: Option<&mut Vec<(usize, bool)>>,
    ) -> Result<usize, String> {
        let rule = self.physics.forests.rule;
        let bounds = f.shape.limits();
        let step = rule.trunk_spacing_m;
        if bounds
            .iter()
            .any(|v| !(*v + step).is_finite() || *v + step <= *v)
        {
            return Err("forest lattice cannot advance at the generated bounds".into());
        }
        let width = libm::ceil((bounds[2] - bounds[0]) / step) + 1.;
        let height = libm::ceil((bounds[3] - bounds[1]) / step) + 1.;
        let edges = match &f.shape {
            GroundShape::Polygon { ring } => ring.len(),
            GroundShape::Stroke { centerline, .. } => centerline.samples().len() * 2,
        };
        let forecast = width * height * (edges + 4) as f64;
        if !forecast.is_finite()
            || forecast > self.queries.work.0.get() as f64
            || !self.queries.work.spend(forecast as u64)
        {
            return Err("forest candidates exceed the ground-sight operation envelope".into());
        }
        let positions = contract::forest::trunk_positions(i, f, &rule, &self.queries);
        // The simulation's canopy (`foliage_cell`): the wood's own ground
        // within a spacing of a tree whose crown meets another's. Partners
        // are sought in this wood alone, which can only claim less.
        let pair = 2. * rule.canopy_radius_m;
        let key = |p: Point| {
            (
                libm::floor(p[0] / pair) as i64,
                libm::floor(p[1] / pair) as i64,
            )
        };
        let mut buckets: std::collections::HashMap<(i64, i64), Vec<Point>> = Default::default();
        for p in &positions {
            buckets.entry(key(*p)).or_default().push(*p);
        }
        let mut stand = Vec::new();
        for p in &positions {
            let (bx, by) = key(*p);
            let mut met = false;
            for q in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (bx + dx, by + dy))) {
                for o in buckets.get(&q).into_iter().flatten() {
                    if !self.queries.work.spend(1) {
                        return Err("stand search exceeds ground-sight operation envelope".into());
                    }
                    met |= o != p && contract::forest::crowns_meet(*p, *o, &rule);
                }
            }
            if met {
                stand.push(*p);
            }
        }
        let reach = rule.trunk_spacing_m;
        // Keep a metre inside the wood's edge: the simulation reads the same
        // outline, but the certificate never rests on a raster edge.
        let inside = |m: Point| {
            [[0., 0.], [1., 0.], [-1., 0.], [0., 1.], [0., -1.]]
                .iter()
                .all(|d| {
                    f.shape.contains(
                        [
                            m[0] + d[0] * FOLIAGE_SAMPLE_M,
                            m[1] + d[1] * FOLIAGE_SAMPLE_M,
                        ],
                        0.,
                    )
                })
        };
        for p in stand {
            let (x0, y0, x1, y1) = self.bounds(p, reach);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !self.queries.work.spend(1) {
                        return Err("foliage raster exceeds ground-sight operation envelope".into());
                    }
                    let mid = self.middle(x, y);
                    if !contract::forest::under_stand(p, mid, &rule) || !f.shape.contains(mid, 0.) {
                        continue;
                    }
                    let k = y * self.nx + x;
                    if !self.occupied[k] {
                        if let Some(changes) = changed.as_mut() {
                            changes.push((k, false));
                        }
                        self.occupied[k] = true;
                    }
                    if distance(mid, p) <= reach - FOLIAGE_SAMPLE_M
                        && inside(mid)
                        && !self.physical[k]
                    {
                        if let Some(changes) = changed.as_mut() {
                            changes.push((k, true));
                        }
                        self.physical[k] = true;
                    }
                }
            }
        }
        self.queries.work.check()?;
        Ok(positions.len())
    }
}
