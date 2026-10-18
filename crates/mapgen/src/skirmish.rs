//! Public skirmish sites reserved before parcels, trees and street furniture.
use crate::layout::water::Water;
use crate::{Diagnostic, DiagnosticCode};
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize};
use contract::ground::GroundShape;
use contract::ids::Side;
use contract::map::{SurfaceArea, SurfaceKind};

pub(crate) fn reserve(
    request: &GenerationRequest,
    surfaces: &mut Vec<SurfaceArea>,
    water: &Water<'_>,
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
    let centre_entries = [edge(Side::Blue, extent)?, edge(Side::Red, 0.0)?];
    let mut entries = centre_entries.to_vec();
    if request.entries_per_side > 1 {
        let extra = add_entry_arms(request, surfaces, water, extent, &entries, fail)?;
        entries.extend(extra);
    }
    let mut ordered = Vec::with_capacity(entries.len());
    for side in [Side::Blue, Side::Red] {
        let mut side_entries: Vec<_> = entries.iter().filter(|entry| entry.side == side).collect();
        let centre_exit = centre_entries
            .iter()
            .find(|entry| entry.side == side)
            .expect("centre entry");
        let centre_index = side_entries
            .iter()
            .position(|entry| entry.center == centre_exit.center && entry.yaw == centre_exit.yaw)
            .expect("centre exit remains in entries");
        let centre = side_entries.remove(centre_index);
        ordered.push(centre.clone());
        let mut flank: Vec<_> = side_entries.into_iter().cloned().collect();
        flank.sort_by(|a, b| a.center[0].total_cmp(&b.center[0]));
        ordered.extend(flank);
    }
    entries = ordered;
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

/// Add the lateral shared entry arms without touching the one-entry layout.
/// Each stroke ends on an existing carriageway centreline, so the generated
/// road graph remains connected. Candidate exits are sampled from the named
/// `entries` stream and rejected when they approach water or another arm.
fn add_entry_arms(
    request: &GenerationRequest,
    surfaces: &mut Vec<SurfaceArea>,
    water: &Water<'_>,
    extent: f64,
    existing: &[EntrySite],
    fail: impl Fn(&str) -> Vec<Diagnostic> + Copy,
) -> Result<Vec<EntrySite>, Vec<Diagnostic>> {
    let wanted = request.entries_per_side - 1;
    let bands: &[f64] = match wanted {
        1 => &[0.22, 0.30, 0.38, 0.62, 0.70, 0.78],
        2 => &[0.18, 0.24, 0.30, 0.70, 0.76, 0.82],
        _ => unreachable!(),
    };
    let mut rng = crate::layout::stream(request, "entries");
    let road_points: Vec<[f64; 2]> = surfaces
        .iter()
        .filter(|s| s.kind.is_road())
        .filter_map(|s| match &s.shape {
            GroundShape::Stroke { centerline, .. } => Some(centerline.samples().to_vec()),
            GroundShape::Polygon { .. } => None,
        })
        .flatten()
        .filter(|p| p[1] > 30.0 && p[1] < extent - 30.0)
        .collect();
    if road_points.is_empty() {
        return Err(fail("entry arms have no interior road to join"));
    }
    let mut added = Vec::new();
    let mut side_entries = [
        existing
            .iter()
            .filter(|e| e.side == Side::Blue)
            .cloned()
            .collect::<Vec<_>>(),
        existing
            .iter()
            .filter(|e| e.side == Side::Red)
            .cloned()
            .collect::<Vec<_>>(),
    ];
    type ArmSegment = (Side, [f64; 2], [f64; 2], f64, f64);
    struct ArmCandidate {
        pair: Vec<ArmSegment>,
    }
    let centre_x = [side_entries[0][0].center[0], side_entries[1][0].center[0]];
    let mut candidates = Vec::new();
    for band in bands.iter().copied() {
        let fraction = (band + (rng.unit() - 0.5) * 0.04).clamp(0.12, 0.88);
        let mut pair = Vec::new();
        for (index, side) in [Side::Blue, Side::Red].into_iter().enumerate() {
            let y = if side == Side::Blue { extent } else { 0.0 };
            let inward = if side == Side::Blue { -1.0 } else { 1.0 };
            let start = [extent * fraction, y];
            if side_entries[index]
                .iter()
                .any(|e| distance(e.center, start) < 150.0)
                || water.gap(start, 90.0) < 90.0
            {
                pair.clear();
                break;
            }
            let Some(target) = road_points
                .iter()
                .copied()
                .filter(|p| (p[1] - y) * inward > 100.0 && (p[0] - start[0]).abs() < extent * 0.45)
                .min_by(|a, b| distance(start, *a).total_cmp(&distance(start, *b)))
            else {
                pair.clear();
                break;
            };
            if water.segment_gap(start, target, 90.0) < 30.0 {
                pair.clear();
                break;
            }
            let delta = [target[0] - start[0], target[1] - start[1]];
            pair.push((side, start, target, libm::atan2(delta[1], delta[0]), inward));
        }
        if pair.len() == 2 {
            candidates.push(ArmCandidate { pair });
        }
    }
    let is_left = |candidate: &ArmCandidate| {
        candidate
            .pair
            .iter()
            .enumerate()
            .all(|(index, (_, start, _, _, _))| start[0] < centre_x[index])
    };
    let is_right = |candidate: &ArmCandidate| {
        candidate
            .pair
            .iter()
            .enumerate()
            .all(|(index, (_, start, _, _, _))| start[0] > centre_x[index])
    };
    let selected: Vec<&ArmCandidate> = if wanted == 2 {
        let mut left = candidates.iter().filter(|candidate| is_left(candidate));
        let right: Vec<_> = candidates
            .iter()
            .filter(|candidate| is_right(candidate))
            .collect();
        left.find_map(|left| {
            right
                .iter()
                .copied()
                .find(|right| distance(left.pair[0].1, right.pair[0].1) >= 150.0)
                .map(|right| vec![left, right])
        })
        .ok_or_else(|| fail("no mirrored, opposite-side entry arm pair fits"))?
    } else {
        let left_width = centre_x.iter().sum::<f64>() / 2.0;
        let prefer_left = left_width > extent - left_width;
        let preferred = candidates.iter().find(|candidate| {
            if prefer_left {
                is_left(candidate)
            } else {
                is_right(candidate)
            }
        });
        let fallback = candidates.iter().find(|candidate| {
            if prefer_left {
                is_right(candidate)
            } else {
                is_left(candidate)
            }
        });
        vec![preferred
            .or(fallback)
            .ok_or_else(|| fail("no mirrored, spaced, river-safe entry arm fits"))?]
    };
    for candidate in selected {
        for (side, start, target, yaw, inward) in &candidate.pair {
            let shape = GroundShape::stroke(vec![*start, *target], 12.0)
                .map_err(|_| fail("entry arm geometry is not representable"))?;
            surfaces.push(SurfaceArea {
                kind: SurfaceKind::CountryRoad,
                shape,
            });
            let entry = EntrySite {
                side: *side,
                center: [start[0], start[1] + inward * 8.0],
                yaw: *yaw,
            };
            let index = usize::from(*side == Side::Red);
            side_entries[index].push(entry.clone());
            added.push(entry);
        }
    }
    Ok(added)
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
