//! The land past a map's edge, where nothing is simulated: the backdrop's
//! relief. Its features are the battle's own (`contract::map::Relief`),
//! sampled every `cell_m` and triangulated by the simulation's rule
//! (`tables::cell_height`), so the backdrop's ground (`mesh::backdrop`), the
//! trees it stands (`scenery`) and every height query read one surface. It
//! lies wholly past the map and the cells touching it, so the map's own
//! ground meets a level backdrop. A battle's ground has none: past its edge
//! the land stays level.
use super::tables::cell_height;
use contract::map::{relief_height, Relief};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackdropRelief {
    /// The spacing of the samples, metres.
    pub cell_m: f64,
    pub features: Vec<Relief>,
}

/// `[min x, min y, max x, max y]`.
type Box2 = [f64; 4];

impl BackdropRelief {
    /// `text` (JSON) as the relief past the map `[min x, min y, max x, max
    /// y]`, or why it can't be (`check`).
    pub fn admit(text: &str, map: Box2) -> Result<Self, String> {
        let relief: Self =
            serde_json::from_str(text).map_err(|e| format!("backdrop relief: {e}"))?;
        relief.check(map)?;
        Ok(relief)
    }

    /// Whether every feature is finite and bounded, and stays clear of `map`
    /// and of the cells touching it, which stay level.
    pub fn check(&self, map: Box2) -> Result<(), String> {
        if self.features.is_empty() {
            return Ok(());
        }
        let c = self.cell_m;
        if !(c.is_finite() && c > 0.0) {
            return Err("backdrop relief: cell_m must be a positive length".into());
        }
        let near = [map[0] - c, map[1] - c, map[2] + c, map[3] + c];
        for (i, f) in self.features.iter().enumerate() {
            let (rect, top) = f.footprint();
            let Some(b) =
                rect.filter(|r| r.iter().all(|v| v.is_finite()) && r[2] >= 0.0 && r[3] >= 0.0)
            else {
                return Err(format!("backdrop relief: feature {i} has no finite bound"));
            };
            if !top.is_finite() {
                return Err(format!("backdrop relief: feature {i} has no finite height"));
            }
            let [x0, y0, x1, y1] = [b[0], b[1], b[0] + b[2], b[1] + b[3]];
            if x0 < near[2] && x1 > near[0] && y0 < near[3] && y1 > near[1] {
                return Err(format!(
                    "backdrop relief: feature {i} reaches within a cell ({c} m) of the map"
                ));
            }
        }
        Ok(())
    }

    /// Whether the land past the map is level.
    pub fn is_level(&self) -> bool {
        self.features.is_empty()
    }

    /// The box holding every feature, out to whole cells; none when level.
    pub fn bounds(&self) -> Option<Box2> {
        let c = self.cell_m;
        self.features
            .iter()
            .filter_map(|f| f.footprint().0)
            .map(|r| [r[0], r[1], r[0] + r[2], r[1] + r[3]])
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            })
            .map(|b| {
                [
                    (b[0] / c).floor() * c,
                    (b[1] / c).floor() * c,
                    (b[2] / c).ceil() * c,
                    (b[3] / c).ceil() * c,
                ]
            })
    }

    /// The features' height at sample `(i, j)`, which stands at
    /// `(i, j) · cell_m`.
    pub fn sample(&self, i: i64, j: i64) -> f64 {
        let c = self.cell_m;
        relief_height(&self.features, i as f64 * c, j as f64 * c)
    }

    /// The land's height at `(x, y)` over the backdrop's level: the samples at
    /// the corners of the cell it lies in, across the triangle it lies in.
    pub fn height(&self, x: f64, y: f64) -> f64 {
        if self.is_level() {
            return 0.0;
        }
        let (fx, fy) = (x / self.cell_m, y / self.cell_m);
        let (i, j) = (fx.floor(), fy.floor());
        let (i0, j0) = (i as i64, j as i64);
        cell_height(
            |di, dj| self.sample(i0 + di as i64, j0 + dj as i64),
            fx - i,
            fy - j,
        )
    }
}
