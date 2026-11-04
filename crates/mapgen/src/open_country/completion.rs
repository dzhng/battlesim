//! Complete physical sight coverage after authored bodies are final.
use super::*;

/// Complete-map certificate and bounded furnishing of any remaining cells.
/// Bodies are final before this runs; only forests can be appended here.
pub(crate) fn cover(
    mut plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
    physics: &contract::generation_physics::GenerationPhysics,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let fail = |message: String| {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some("ground_sight".into()),
            location: "$.presets.open_country".into(),
            message: format!(
                "{message} ({} {}, seed {})",
                request.map_type.name(),
                request.size.name(),
                request.seed.value()
            ),
        }]
    };
    let (map, _) = crate::materialize(
        &crate::CompileRequest::generated(request, plan.clone()),
        catalogue,
    )?;
    let cell = presets.open_country.sight.cell_m;
    let mut coverage = coverage::Coverage::new(&map, physics, cell).map_err(fail)?;
    let (added, kept) = {
        let mut country = Country::new(&plan, request, presets);
        country.proof_fill = true;
        country.work = Some(coverage.work());
        let initial = measure(&plan, presets);
        country.tally.copses = [initial.copses.top, initial.copses.bottom];
        country.tally.tree_line_m = [initial.tree_line_m.top, initial.tree_line_m.bottom];
        let mut rng = country.stream("ground_sight");
        let step = (cell / 2.).min(map.fog_cell_m);
        let span = libm::floor(coverage.reach / step) as i64;
        let mut offsets: Vec<_> = (-span..=span)
            .flat_map(|j| (-span..=span).map(move |i| [i as f64 * step, j as f64 * step]))
            .filter(|p| libm::hypot(p[0], p[1]) <= coverage.reach)
            .collect();
        offsets.sort_by(|a, b| {
            libm::hypot(a[0], a[1])
                .total_cmp(&libm::hypot(b[0], b[1]))
                .then(a[0].total_cmp(&b[0]))
                .then(a[1].total_cmp(&b[1]))
        });
        let nx = libm::ceil(plan.size[0] / cell) as usize;
        let ny = libm::ceil(plan.size[1] / cell) as usize;
        for y in 0..ny {
            for x in 0..nx {
                let low = [x as f64 * cell, y as f64 * cell];
                let high = [
                    (low[0] + cell).min(plan.size[0]),
                    (low[1] + cell).min(plan.size[1]),
                ];
                fill_cell(
                    &mut country,
                    &mut coverage,
                    &mut rng,
                    &offsets,
                    (map.forests.len(), map.fog_cell_m),
                    low,
                    high,
                )
                .map_err(|message| fail(format!("{message} in clipped cell [{x},{y}]")))?;
            }
        }
        // Whole-cell furnishing can need more features in one half. Earn
        // the existing per-kind fairness again by adding only real features
        // in the lesser half, using the same placement and proof owners.
        for kind in &presets.open_country.sight.fill {
            let (least, attempts) = match kind {
                FillKind::Copse => (
                    presets.open_country.fairness.copses,
                    presets.open_country.copses.attempts,
                ),
                FillKind::TreeLine => (
                    presets.open_country.fairness.tree_line_m,
                    presets.open_country.tree_lines.attempts,
                ),
            };
            let mut rng = country.stream(match kind {
                FillKind::Copse => "coverage_balance/copses",
                FillKind::TreeLine => "coverage_balance/tree_lines",
            });
            for _ in 0..presets.open_country.placed_max {
                let held = match kind {
                    FillKind::Copse => country.tally.copses,
                    FillKind::TreeLine => country.tally.tree_line_m,
                };
                if even(presets.open_country.fairness.rel, least, held[0], held[1]) {
                    break;
                }
                let half = usize::from(held[1] < held[0]);
                let mut stood = false;
                for _ in 0..attempts {
                    let p = country.anywhere(half, &mut rng);
                    if country
                        .try_feature(*kind, p, &mut rng, &mut coverage, map.forests.len(), None)
                        .map_err(fail)?
                    {
                        stood = true;
                        break;
                    }
                }
                if !stood {
                    return Err(fail(format!(
                        "no physical {kind:?} can restore country fairness"
                    )));
                }
            }
        }
        coverage.check().map_err(fail)?;
        (country.forests, country.kept)
    };
    plan.forests.extend(added);
    retain_approaches(&mut plan, presets, kept);
    let country = measure(&plan, presets);
    if !(country.homes.fair
        && country.tree_line_m.fair
        && country.copses.fair
        && country.trees.fair
        && country.cover.fair)
    {
        return Err(fail(format!("uneven country halves: {country:?}")));
    }
    let metrics = crate::layout::measure(&plan, presets);
    if !(metrics.main_approach_top && metrics.main_approach_bottom) {
        return Err(fail(
            "physical furnishing closed a mandatory main firing lane".into(),
        ));
    }
    Ok(plan)
}

/// A coarse location rectangle can straddle two different physical witnesses.
/// Partition only when it cannot be certified whole; every child still earns
/// the same continuous certificate. The fog-cell floor and shared work budget
/// bound the search, including all failed proposals.
fn fill_cell(
    country: &mut Country<'_>,
    coverage: &mut coverage::Coverage<'_>,
    rng: &mut Stream,
    offsets: &[Point],
    bounds: (usize, f64),
    low: Point,
    high: Point,
) -> Result<(), String> {
    let (forest_start, smallest) = bounds;
    let covered = coverage.covered(low, high);
    coverage.check()?;
    if covered {
        return Ok(());
    }
    let at = scale(add(low, high), 0.5);
    for main_only in [false, true] {
        country.main_only = main_only;
        for offset in offsets {
            let p = add(at, *offset);
            if !coverage.admits(p, low, high) {
                coverage.check()?;
                continue;
            }
            for kind in &country.presets.open_country.sight.fill {
                if country.try_feature(*kind, p, rng, coverage, forest_start, Some((low, high)))? {
                    return Ok(());
                }
            }
            coverage.check()?;
        }
    }
    if (high[0] - low[0]).min(high[1] - low[1]) <= smallest {
        return Err(format!(
            "no physically certified sight feature covers rectangle {low:?}..{high:?}"
        ));
    }
    for (a, b) in [
        (low, at),
        ([at[0], low[1]], [high[0], at[1]]),
        ([low[0], at[1]], [at[0], high[1]]),
        (at, high),
    ] {
        fill_cell(country, coverage, rng, offsets, bounds, a, b)?;
    }
    Ok(())
}
