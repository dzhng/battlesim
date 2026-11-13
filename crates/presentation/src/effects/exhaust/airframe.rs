//! Where a jet's effects come from on its model: its nozzles, its wingtips and
//! its wings' planforms, in its model's own frame (+X its nose, +Y its left,
//! +Z up). Only its engine kind is authored by hand; the rest is read from its
//! source art by `bun run --cwd web jet-airframes`
//! (`packages/scene-assets/src/jetAirframes.ts`), because the baked art keeps
//! no node for them. So every jet gets every effect from its own data, and a
//! new jet gets them by running that tool.
//!
//! The wing-top condensation sheet rises from points one shared rule
//! ([`SheetRule`]) lays over each wing's planform: the same shares of every
//! wing's span and chord, a point every `spacing_m` along its span, so a
//! bigger wing carries more. Its rows run from the root's leading edge back
//! along the span (the low-pressure band behind a swept leading edge), and
//! one more crosses the inboard wing behind it: fitted once to the sheet a
//! hand placed on the F-22 and shared by every jet.
use serde::Deserialize;

use crate::world::Vec3;

/// One jet appearance: its engine kind (`engines`), and where its nozzles,
/// wingtips and wings are.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Airframe {
    pub engine: String,
    pub nozzles: Vec<Nozzle>,
    /// Its left wingtip, then its right.
    pub wingtips: Vec<Point>,
    /// Its left wing, then its right.
    pub wings: Vec<Wing>,
}

/// A nozzle's exit: its middle and radius.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nozzle {
    pub at: Vec3,
    pub radius_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub at: Vec3,
}

/// A wing's planform: its sections from its root (where it meets the body)
/// out to its tip.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wing {
    pub stations: Vec<WingStation>,
}

/// One section of a wing, `span_y` across the jet: where its leading and
/// trailing edges are, and how high its upper surface rises.
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct WingStation {
    pub span_y: f64,
    pub leading_x: f64,
    pub trailing_x: f64,
    pub top_z: f64,
}

/// Where on every wing the condensation sheet rises: between these shares of
/// its span from root to tip (below 0 reaching on into the root's fairing), a
/// point every `spacing_m` (at least one), in one row per `rows` entry: its
/// share of the chord from leading to trailing edge at the row's inner end,
/// then at its outer end, `lift_m` over the upper surface.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SheetRule {
    pub span: [f64; 2],
    pub rows: Vec<[f64; 2]>,
    pub spacing_m: f64,
    pub lift_m: f64,
}

/// How far past a wing's root (in span) or ahead of its leading edge (in
/// chord) a sheet rule's shares may reach.
const REACH: f64 = 0.2;

/// The paths of signed numbers under an airframe: model-frame places.
pub(crate) const SIGNED: [&str; 6] = [
    ".at[0]",
    ".at[1]",
    ".at[2]",
    ".span_y",
    ".leading_x",
    ".trailing_x",
];

impl SheetRule {
    pub(super) fn check(&self) -> Result<(), String> {
        // Chord shares may reach a little ahead of the leading edge (onto a
        // root extension) and span shares a little inboard of the root.
        let reach = |lo: f64| move |s: &f64| (lo..=1.0).contains(s);
        if !self.span.iter().all(reach(-REACH))
            || self.span[0] > self.span[1]
            || self.rows.is_empty()
            || !self.rows.iter().flatten().all(reach(-REACH))
            || self.spacing_m <= 0.0
        {
            return Err(format!(
                "wing_sheet_rule: span a rising pair of shares, rows one or more [inner, outer] chord shares (each in [-{REACH}, 1]), spacing_m positive"
            ));
        }
        Ok(())
    }
}

impl Wing {
    /// Its section `share` of the way from its root to its tip, between the
    /// stations either side.
    fn at(&self, share: f64) -> WingStation {
        let s = &self.stations;
        let (root, tip) = (s[0].span_y, s[s.len() - 1].span_y);
        let y = root + (tip - root) * share;
        let out = |p: &WingStation| (p.span_y - root).abs();
        let k = (0..s.len() - 2)
            .find(|&k| out(&s[k + 1]) >= (y - root).abs())
            .unwrap_or(s.len() - 2);
        let (a, b) = (s[k], s[k + 1]);
        let f = if b.span_y == a.span_y {
            0.0
        } else {
            (y - a.span_y) / (b.span_y - a.span_y)
        };
        let mix = |p: f64, q: f64| p + (q - p) * f;
        WingStation {
            span_y: y,
            leading_x: mix(a.leading_x, b.leading_x),
            trailing_x: mix(a.trailing_x, b.trailing_x),
            top_z: mix(a.top_z, b.top_z),
        }
    }
}

impl Airframe {
    pub(super) fn check(&self, name: &str) -> Result<(), String> {
        let fail = |what: &str| Err(format!("airframes.{name}: {what}"));
        if self.nozzles.is_empty() || self.nozzles.iter().any(|n| n.radius_m <= 0.0) {
            return fail("needs nozzles of positive radius");
        }
        if self.wingtips.len() != 2 || self.wings.len() != 2 {
            return fail("needs two wingtips and two wings");
        }
        for w in &self.wings {
            let s = &w.stations;
            let outward = s.windows(2).all(|p| {
                p[1].span_y.abs() > p[0].span_y.abs()
                    && p[1].span_y.signum() == p[0].span_y.signum()
            });
            if s.len() < 2 || !outward || s.iter().any(|p| p.leading_x < p.trailing_x) {
                return fail(
                    "each wing needs two or more stations running out from its root, its leading edge ahead of its trailing edge",
                );
            }
        }
        Ok(())
    }

    /// The points its wing-top condensation sheet rises from, by `rule`:
    /// each wing's, row by row.
    pub fn sheet(&self, rule: &SheetRule) -> Vec<Vec3> {
        let mut out = Vec::new();
        for wing in &self.wings {
            let s = &wing.stations;
            let reach = (s[s.len() - 1].span_y - s[0].span_y).abs() * (rule.span[1] - rule.span[0]);
            let n = if reach > 0.0 {
                libm::ceil(reach / rule.spacing_m - 1e-9) as usize + 1
            } else {
                1
            };
            for &[inner, outer] in &rule.rows {
                for i in 0..n {
                    let along = if n > 1 {
                        i as f64 / (n - 1) as f64
                    } else {
                        0.0
                    };
                    let p = wing.at(rule.span[0] + (rule.span[1] - rule.span[0]) * along);
                    let c = inner + (outer - inner) * along;
                    out.push([
                        p.leading_x + (p.trailing_x - p.leading_x) * c,
                        p.span_y,
                        p.top_z + rule.lift_m,
                    ]);
                }
            }
        }
        out
    }
}
