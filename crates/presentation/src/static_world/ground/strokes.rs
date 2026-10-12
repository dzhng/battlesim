//! The paved strokes as the ground draws them. A stroke of a kind the biome
//! draws as another through built ground (`roads.<kind>.town`, a country
//! road through a town as a street) is that kind along each run with built
//! ground either side of it: its own stretches stop at the run, round as a
//! stroke's stretches meet, and the run is laid as the other kind on the
//! same line and as wide, cut square a carry past each end, under the
//! stroke's own surface. No edge of the road moves.
use super::biome::{Biome, Town};
use super::js;
use super::{chunks, AREA_KINDS, CUT_A, CUT_B, SURFACE_STROKE};

/// Built ground is looked for at points this far apart along a stretch, and
/// a change between two of them is placed this closely.
const BUILT_STEP_M: f64 = 4.0;
const BUILT_PLACED_M: f64 = 0.25;

/// `strokes` as drawn, where `built(x, y)` says a point is a settlement's
/// own ground. `None` where nothing is drawn differently.
pub fn drawn(strokes: &[f32], biome: &Biome, built: impl Fn(f64, f64) -> bool) -> Option<Vec<f32>> {
    // Per paved kind, the kind it is drawn as through built ground, how
    // (`Town`), and how far that kind's surface runs on under its ends.
    let through = AREA_KINDS.map(|kind| {
        let town = biome.road(kind).town.as_ref()?;
        let as_ = AREA_KINDS.iter().position(|k| *k == town.kind);
        Some((
            as_.map_or(-1.0, |k| k as f64),
            town,
            biome.road(&town.kind).join_m,
        ))
    });
    let stride = SURFACE_STROKE;
    let mut out: Vec<f32> = vec![];
    let mut changed = false;
    let mut o = 0;
    while o < strokes.len() {
        // A stroke's stretches are exported in order, each starting where
        // the last ended.
        let mut end = o + stride;
        while end < strokes.len()
            && strokes[end] == strokes[end - stride + 2]
            && strokes[end + 1] == strokes[end - stride + 3]
            && strokes[end + 5] == strokes[o + 5]
        {
            end += stride;
        }
        let stroke = &strokes[o..end];
        let tag = stroke[5] as f64;
        let rule = through
            .get(tag as usize)
            .and_then(|r| r.filter(|r| r.0 != tag));
        let runs = rule.map_or(vec![], |r| built_runs(stroke, r.1, &built));
        o = end;
        let Some((as_, _, carry)) = rule.filter(|_| !runs.is_empty()) else {
            out.extend_from_slice(stroke);
            continue;
        };
        changed = true;
        let total = chunks::<SURFACE_STROKE>(stroke)
            .iter()
            .fold(0.0, |t, s| t + length(s));
        // The stroke as itself between the runs, then each run with its carry.
        let (mut between, mut carried): (Vec<[f64; 2]>, Vec<[f64; 2]>) = (vec![], vec![]);
        let mut at = 0.0;
        for &[start, stop] in &runs {
            between.push([at, start]);
            at = stop;
            let span = [js::max(0.0, start - carry), js::min(total, stop + carry)];
            match carried.last_mut() {
                Some(last) if span[0] <= last[1] => last[1] = span[1],
                _ => carried.push(span),
            }
        }
        between.push([at, total]);
        lay(&mut out, stroke, &between, tag, false);
        lay(&mut out, stroke, &carried, as_, true);
    }
    changed.then_some(out)
}

fn length(s: &[f32]) -> f64 {
    js::hypot(s[2] as f64 - s[0] as f64, s[3] as f64 - s[1] as f64)
}

/// The stroke's stretches within `spans` of its length, as `kind`: a part
/// that ends with its span short of the stroke's own end is cut square
/// there (`square`) or left round.
fn lay(out: &mut Vec<f32>, stroke: &[f32], spans: &[[f64; 2]], kind: f64, square: bool) {
    let total = chunks::<SURFACE_STROKE>(stroke)
        .iter()
        .fold(0.0, |t, s| t + length(s));
    let mut from = 0.0;
    for s in chunks::<SURFACE_STROKE>(stroke) {
        let [ax, ay, bx, by] = [s[0], s[1], s[2], s[3]].map(f64::from);
        let cuts = s[6] as u32;
        let length = js::hypot(bx - ax, by - ay);
        let along = |a: f64, b: f64, at: f64| (a + ((b - a) * (at - from)) / length) as f32;
        let point = |at: f64| match at {
            _ if at == from => [s[0], s[1]],
            _ if at == from + length => [s[2], s[3]],
            _ => [along(ax, bx, at), along(ay, by, at)],
        };
        for &[first, last] in spans {
            let (start, stop) = (js::max(first, from), js::min(last, from + length));
            if stop <= start {
                continue;
            }
            let bit = |on: bool, bit: u32| if on { bit } else { 0 };
            let cut = bit(start == from, cuts & CUT_A)
                | bit(stop == from + length, cuts & CUT_B)
                | bit(square && start == first && first > 0.0, CUT_A)
                | bit(square && stop == last && last < total, CUT_B);
            out.extend(point(start));
            out.extend(point(stop));
            out.extend([s[4], kind as f32, cut as f32]);
        }
        from += length;
    }
}

/// The runs of one stroke with built ground either side of them, as
/// `[start, stop]` distances along the stroke; two runs nearer than the
/// rule's gap are one.
fn built_runs(stroke: &[f32], rule: &Town, built: &impl Fn(f64, f64) -> bool) -> Vec<[f64; 2]> {
    let mut runs: Vec<[f64; 2]> = vec![];
    let mut was = false;
    let mut before = 0.0;
    for s in chunks::<SURFACE_STROKE>(stroke) {
        let [ax, ay, bx, by, half] = [s[0], s[1], s[2], s[3], s[4]].map(f64::from);
        let length = js::hypot(bx - ax, by - ay);
        if length == 0.0 {
            continue;
        }
        let (ux, uy) = ((bx - ax) / length, (by - ay) / length);
        let aside = half + rule.beside_m;
        let built_at = |along: f64| {
            let (x, y) = (ax + ux * along, ay + uy * along);
            built(x - uy * aside, y + ux * aside) && built(x + uy * aside, y - ux * aside)
        };
        let steps = (length / BUILT_STEP_M).ceil();
        let mut step = 0.0;
        while step <= steps {
            let at = (length * step) / steps;
            let is = built_at(at);
            if is != was {
                // Between this point and the last; at a stretch's start, there.
                let previous = (length * (step - 1.0)) / steps;
                let (mut low, mut high) = (if step == 0.0 { at } else { previous }, at);
                while high - low > BUILT_PLACED_M {
                    let middle = (low + high) / 2.0;
                    if built_at(middle) == was {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                let turn = before + (low + high) / 2.0;
                match runs.last_mut() {
                    Some(last) if !is => last[1] = turn,
                    Some(last) if turn - last[1] < rule.gap_m => last[1] = f64::INFINITY,
                    _ => runs.push([turn, f64::INFINITY]),
                }
                was = is;
            }
            step += 1.0;
        }
        before += length;
    }
    if was {
        runs.last_mut().unwrap()[1] = before;
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::super::biome::{tests::summer, Biome};
    use super::super::{CUT_A, CUT_B, SURFACE_STROKE};
    use super::drawn;

    const COUNTRY: f32 = 1.0;
    const STREET: f32 = 0.0;

    /// A country road is a street through built ground, looked for 5 m
    /// beside it, and across a gap in it of under 40 m; the street's surface
    /// is carried 6 m on under the road's.
    fn as_street() -> Biome {
        summer(|b| {
            let mut country = b["roads"]["default"].clone();
            country["join_m"] = 2.into();
            country["town"] = serde_json::json!({ "kind": "road", "beside_m": 5, "gap_m": 40 });
            b["roads"]["country_road"] = country;
            b["roads"]["road"]["join_m"] = 6.into();
        })
        .unwrap()
    }

    /// One country road along y = 50, 8 m wide, cut square at both ends.
    const THROUGH: [f32; 7] = [0.0, 50.0, 300.0, 50.0, 4.0, COUNTRY, (CUT_A | CUT_B) as f32];

    /// Houses either side of the road from x = 100 to 200.
    fn built_beside(x: f64, _: f64) -> bool {
        (100.0..=200.0).contains(&x)
    }

    /// Each drawn stretch's `[from x, to x, kind]`, for a road along x.
    fn pieces(drawn: &[f32]) -> Vec<[f32; 3]> {
        drawn
            .as_chunks::<SURFACE_STROKE>()
            .0
            .iter()
            .map(|s| [s[0].round(), s[2].round(), s[5]])
            .collect()
    }

    #[test]
    fn a_country_road_is_a_street_between_built_ground_and_itself_beyond() {
        let drawn = drawn(&THROUGH, &as_street(), built_beside).unwrap();
        // The road stops at the houses and starts again past them; the
        // street runs its carry on under it, either end.
        assert_eq!(
            pieces(&drawn),
            [
                [0.0, 100.0, COUNTRY],
                [200.0, 300.0, COUNTRY],
                [94.0, 206.0, STREET]
            ]
        );
        // The street ends square; the road ends round over it, and keeps its
        // own square ends. All on the road's own line, as wide.
        let cuts: Vec<f32> = drawn
            .as_chunks::<SURFACE_STROKE>()
            .0
            .iter()
            .map(|s| s[6])
            .collect();
        assert_eq!(cuts, [CUT_A as f32, CUT_B as f32, (CUT_A | CUT_B) as f32]);
        for s in drawn.as_chunks::<SURFACE_STROKE>().0 {
            assert_eq!([s[1], s[3], s[4]], [50.0, 50.0, 4.0]);
        }
        assert_eq!((drawn[0], drawn[SURFACE_STROKE + 2]), (0.0, 300.0));
    }

    #[test]
    fn a_street_runs_on_across_a_short_gap_and_ends_at_a_long_one() {
        let yards = |x: f64, _: f64| {
            [(60.0, 100.0), (130.0, 170.0), (230.0, 260.0)]
                .iter()
                .any(|&(a, b)| x >= a && x <= b)
        };
        let drawn = pieces(&drawn(&THROUGH, &as_street(), yards).unwrap());
        let of = |kind: f32| {
            drawn
                .iter()
                .filter(|p| p[2] == kind)
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(of(STREET), [[54.0, 176.0, STREET], [224.0, 266.0, STREET]]);
        assert_eq!(
            of(COUNTRY),
            [
                [0.0, 60.0, COUNTRY],
                [170.0, 230.0, COUNTRY],
                [260.0, 300.0, COUNTRY]
            ]
        );
    }

    #[test]
    fn a_streets_run_carries_on_from_one_stretch_of_its_road_to_the_next() {
        // The same road in two stretches that meet at x = 150, inside the town.
        let bent = [
            [0.0, 50.0, 150.0, 50.0, 4.0, COUNTRY, CUT_A as f32],
            [150.0, 50.0, 300.0, 50.0, 4.0, COUNTRY, CUT_B as f32],
        ]
        .concat();
        let drawn = drawn(&bent, &as_street(), built_beside).unwrap();
        assert_eq!(
            pieces(&drawn),
            [
                [0.0, 100.0, COUNTRY],
                [200.0, 300.0, COUNTRY],
                [94.0, 150.0, STREET],
                [150.0, 206.0, STREET]
            ]
        );
        // The street is not cut where its two stretches meet.
        let cuts: Vec<f32> = drawn
            .as_chunks::<SURFACE_STROKE>()
            .0
            .iter()
            .map(|s| s[6])
            .collect();
        assert_eq!(cuts, [CUT_A, CUT_B, CUT_A, CUT_B].map(|c| c as f32));
    }

    #[test]
    fn a_road_built_along_one_side_only_or_never_stays_itself() {
        assert!(drawn(&THROUGH, &as_street(), |x, y| built_beside(x, y)
            && y > 50.0)
        .is_none());
        assert!(drawn(&THROUGH, &as_street(), |_, _| false).is_none());
        // A kind drawn as itself: the street test map's roads, streets by name.
        let mut itself = as_street();
        itself
            .roads
            .get_mut("country_road")
            .unwrap()
            .town
            .as_mut()
            .unwrap()
            .kind = "country_road".into();
        assert!(drawn(&THROUGH, &itself, built_beside).is_none());
    }

    /// How deep `(x, y)` lies in the deepest of `strokes`: its half width
    /// less the distance to the stretch, and past a cut end the distance to
    /// that square end instead (`terrain/strokes.ts`' `strokeInside`).
    fn inside(strokes: &[f32], x: f64, y: f64) -> f64 {
        let mut deepest = f64::NEG_INFINITY;
        for s in strokes.as_chunks::<SURFACE_STROKE>().0 {
            let [ax, ay, bx, by, half, _, cuts]: [f64; 7] = std::array::from_fn(|k| s[k] as f64);
            let (dx, dy) = (bx - ax, by - ay);
            let length = dx.hypot(dy);
            let along = ((x - ax) * dx + (y - ay) * dy) / length;
            let aside = ((x - ax) * dy - (y - ay) * dx).abs() / length;
            let t = along.clamp(0.0, length);
            let round = half - (along - t).hypot(aside);
            let cut = |bit: u32| cuts as u32 & bit != 0;
            let past = f64::max(
                if cut(CUT_A) {
                    -along
                } else {
                    f64::NEG_INFINITY
                },
                if cut(CUT_B) {
                    along - length
                } else {
                    f64::NEG_INFINITY
                },
            );
            let depth = if past <= 0.0 {
                round.min(-past)
            } else {
                -(aside - half).max(0.0).hypot(past)
            };
            deepest = deepest.max(depth);
        }
        deepest
    }

    #[test]
    fn drawing_a_road_as_a_street_moves_no_edge_of_it() {
        let drawn = drawn(&THROUGH, &as_street(), built_beside).unwrap();
        // An edge is where the depth is 0: within a metre of it nothing may
        // move. (Deeper in, a street's square end is nearer than the road's
        // edge was.) Across the road at its ends, at each change of kind, and
        // between.
        let near = |depth: f64| depth.min(1.0);
        for x in [
            -3.0, 0.0, 2.0, 60.0, 98.0, 100.0, 102.0, 150.0, 199.5, 200.5, 298.0, 300.0, 304.0,
        ] {
            for y in [40.0, 45.9, 46.1, 50.0, 53.9, 54.1, 60.0] {
                let (now, was) = (near(inside(&drawn, x, y)), near(inside(&THROUGH, x, y)));
                assert!((now - was).abs() < 1e-4, "({x}, {y}): {now} against {was}");
            }
        }
    }
}
