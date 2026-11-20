//! A wood in the sight raster: the fog cells its canopy covers, and those
//! it covers with a metre to spare, which may witness.
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
        let mut trunks = 0;
        for p in contract::forest::trunk_positions(i, f, &rule, &self.queries) {
            trunks += 1;
            let r = rule.canopy_radius_m;
            let (x0, y0, x1, y1) = self.bounds(p, r);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !self.queries.work.spend(1) {
                        return Err("foliage raster exceeds ground-sight operation envelope".into());
                    }
                    let mid = self.middle(x, y);
                    if distance(mid, p) <= r {
                        let k = y * self.nx + x;
                        if !self.occupied[k] {
                            if let Some(changes) = changed.as_mut() {
                                changes.push((k, false));
                            }
                            self.occupied[k] = true;
                        }
                        // foliage_depth samples every metre and clips the
                        // forest's canopy-expanded bounds. Certify a whole
                        // final step inside both owners of that foliage.
                        if distance(mid, p) <= r - FOLIAGE_SAMPLE_M
                            && mid[0] > bounds[0] - r + FOLIAGE_SAMPLE_M
                            && mid[0] < bounds[2] + r - FOLIAGE_SAMPLE_M
                            && mid[1] > bounds[1] - r + FOLIAGE_SAMPLE_M
                            && mid[1] < bounds[3] + r - FOLIAGE_SAMPLE_M
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
        }
        self.queries.work.check()?;
        Ok(trunks)
    }
}
