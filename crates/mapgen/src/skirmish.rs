//! Public skirmish sites reserved before parcels, trees and street furniture.
use crate::{Diagnostic, DiagnosticCode};
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize};
use contract::ground::GroundShape;
use contract::ids::Side;
use contract::map::SurfaceArea;

pub(crate) fn reserve(
    request: &GenerationRequest,
    surfaces: &[SurfaceArea],
    hub: [f64; 2],
    towns: &[&[[f64; 2]]],
) -> Result<Option<SkirmishSites>, Vec<Diagnostic>> {
    if request.profile == GenerationProfile::Standard {
        return Ok(None);
    }
    let extent = request.extent_m();
    let fail = |message: &str| {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some("skirmish_sites".into()),
            location: "$.profile".into(),
            message: message.into(),
        }]
    };
    let edge = |side: Side, y: f64| -> Result<EntrySite, Vec<Diagnostic>> {
        let road = surfaces
            .iter()
            .filter_map(|s| match &s.shape {
                GroundShape::Stroke { centerline, .. } => Some(centerline.samples()),
                _ => None,
            })
            .flat_map(|p| p.windows(2))
            .find_map(|pair| {
                let (a, b) = (pair[0], pair[1]);
                if a[1] == y {
                    Some((a, b))
                } else if b[1] == y {
                    Some((b, a))
                } else {
                    None
                }
            })
            .ok_or_else(|| fail("opposing road-edge entry is missing"))?;
        let delta = [road.1[0] - road.0[0], road.1[1] - road.0[1]];
        let length = libm::hypot(delta[0], delta[1]);
        Ok(EntrySite {
            side,
            center: [
                road.0[0] + delta[0] * 8.0 / length,
                road.0[1] + delta[1] * 8.0 / length,
            ],
            yaw: libm::atan2(delta[1], delta[0]),
        })
    };
    let entries = [edge(Side::Blue, extent)?, edge(Side::Red, 0.0)?];
    let mut objectives = vec![ObjectiveSite {
        id: "objective-0".into(),
        center: hub,
        radius_m: 50.0,
        kind: ObjectiveSiteKind::Junction,
        counterpart: None,
    }];
    let central = if matches!(request.size, MapSize::Large | MapSize::Xl) {
        3
    } else {
        1
    };
    for sign in [-1.0, 1.0].into_iter().take(central - 1) {
        objectives.push(ObjectiveSite {
            id: format!("objective-{}", objectives.len()),
            center: [hub[0] + sign * extent * 0.2, extent / 2.0],
            radius_m: 50.0,
            kind: ObjectiveSiteKind::Field,
            counterpart: None,
        });
    }
    let pairs = match request.size {
        MapSize::Small => 1,
        MapSize::Medium => 2,
        _ => 2,
    };
    let mut rng = crate::layout::stream(request, "objectives");
    let mut candidates = Vec::new();
    if central == 3 {
        for index in 0..6 {
            if let Some(center) = (0..200).find_map(|_| {
                let p = [extent * (0.12 + rng.unit() * 0.76), extent / 2.0];
                (!towns.iter().any(|ring| {
                    contract::ground::polygon_contains(ring, p)
                        || contract::ground::edges(ring)
                            .any(|(a, b)| contract::ground::segment_distance(*a, *b, p) < 70.0)
                }) && objectives.iter().all(|o| distance(o.center, p) >= 150.0))
                .then_some(p)
            }) {
                candidates.push(ObjectiveSite {
                    id: format!("central-{index}"),
                    center,
                    radius_m: 50.0,
                    kind: ObjectiveSiteKind::Field,
                    counterpart: None,
                });
            }
        }
    }
    // Independent locations in opposing approach bands. Actual routes, not
    // distance or reflected coordinates, decide which counterparts are used.
    for pair in 0..(pairs + (24 - candidates.len()) / 2) {
        let mut points = Vec::new();
        for high in [false, true] {
            let point = (0..200).find_map(|_| {
                let p = [
                    extent * (0.12 + rng.unit() * 0.76),
                    extent
                        * (if high {
                            0.56 + rng.unit() * 0.22
                        } else {
                            0.22 + rng.unit() * 0.22
                        }),
                ];
                (p.iter().all(|v| *v >= 70.0 && *v <= extent - 70.0)
                    && !towns.iter().any(|ring| {
                        contract::ground::polygon_contains(ring, p)
                            || contract::ground::edges(ring)
                                .any(|(a, b)| contract::ground::segment_distance(*a, *b, p) < 70.0)
                    })
                    && objectives.iter().all(|o| distance(o.center, p) >= 150.0)
                    && entries.iter().all(|e| distance(e.center, p) >= 150.0)
                    && points.iter().all(|q| distance(*q, p) >= 150.0))
                .then_some(p)
            });
            let Some(point) = point else {
                break;
            };
            points.push(point);
        }
        if points.len() != 2 {
            if pair < pairs {
                return Err(fail(
                    "no reserved rural objective pair fits after 200 candidates",
                ));
            }
            break;
        }
        let pair_sites: Vec<_> = points
            .into_iter()
            .enumerate()
            .map(|(half, center)| ObjectiveSite {
                id: format!("field-{pair}-{half}"),
                center,
                radius_m: 50.0,
                kind: ObjectiveSiteKind::Field,
                counterpart: Some(format!("field-{pair}-{}", 1 - half)),
            })
            .collect();
        if pair < pairs {
            objectives.extend(pair_sites);
        } else {
            candidates.extend(pair_sites);
        }
    }
    let sites = SkirmishSites {
        entries,
        objectives,
        candidates,
    };
    for (i, o) in sites.objectives.iter().enumerate() {
        if sites.objectives[..i]
            .iter()
            .any(|p| distance(o.center, p.center) < 150.0)
            || sites
                .entries
                .iter()
                .any(|e| distance(o.center, e.center) < 150.0)
        {
            return Err(fail("objective spacing is below 150 metres"));
        }
    }
    Ok(Some(sites))
}
/// Ground kept beyond a reserved objective's clearance, so what stands round
/// it does not crowd its edge.
const CLEARANCE_MARGIN_M: f64 = 10.0;
/// The most of the primary junction's capture circle that is kept clear: it
/// stands in the main town's square, and clearing all of it would hollow the
/// town out. The capture radius itself stays the objective's.
const JUNCTION_CLEARANCE_M: f64 = 28.0;

/// The ground every generation pass keeps clear about each reserved objective
/// of `sites` (forests, parcels, the open country and street furniture
/// alike): its centre and how far from it nothing stands.
pub(crate) fn objective_clearances(
    sites: Option<&SkirmishSites>,
) -> impl Iterator<Item = ([f64; 2], f64)> + '_ {
    sites
        .into_iter()
        .flat_map(SkirmishSites::all_reserved_objectives)
        .map(|o| {
            let kept = match o.kind {
                ObjectiveSiteKind::Junction => o.radius_m.min(JUNCTION_CLEARANCE_M),
                _ => o.radius_m,
            };
            (o.center, kept + CLEARANCE_MARGIN_M)
        })
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}
