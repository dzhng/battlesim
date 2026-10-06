//! Public skirmish sites reserved before parcels, trees and street furniture.
use contract::encounter::{EntrySite, ObjectiveSite, ObjectiveSiteKind, SkirmishSites};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize};
use contract::ground::GroundShape;
use contract::ids::Side;
use contract::map::SurfaceArea;
use crate::{Diagnostic, DiagnosticCode};

pub(crate) fn reserve(request: &GenerationRequest, surfaces: &[SurfaceArea], hub: [f64; 2], towns: &[&[[f64; 2]]])
    -> Result<Option<SkirmishSites>, Vec<Diagnostic>> {
    if request.profile == GenerationProfile::Standard { return Ok(None); }
    let extent = request.extent_m();
    let fail = |message: &str| vec![Diagnostic { code: DiagnosticCode::GenerationFailed,
        feature: Some("skirmish_sites".into()), location: "$.profile".into(), message: message.into() }];
    let edge = |side: Side, y: f64| -> Result<EntrySite, Vec<Diagnostic>> {
        let road = surfaces.iter().filter_map(|s| match &s.shape {
            GroundShape::Stroke { centerline, .. } => Some(centerline.samples()), _ => None,
        }).flat_map(|p| p.windows(2)).find_map(|pair| {
            let (a,b) = (pair[0],pair[1]);
            if a[1] == y { Some((a,b)) } else if b[1] == y { Some((b,a)) } else { None }
        }).ok_or_else(|| fail("opposing road-edge entry is missing"))?;
        let delta = [road.1[0]-road.0[0], road.1[1]-road.0[1]];
        let length = libm::hypot(delta[0], delta[1]);
        Ok(EntrySite { side, center: [road.0[0]+delta[0]*8.0/length, road.0[1]+delta[1]*8.0/length],
            yaw: libm::atan2(delta[1],delta[0]) })
    };
    let entries = [edge(Side::Blue, extent)?, edge(Side::Red, 0.0)?];
    let mut objectives = vec![ObjectiveSite { id: "objective-0".into(), center: hub,
        radius_m: 50.0, kind: ObjectiveSiteKind::Junction, counterpart: None }];
    let central = if matches!(request.size, MapSize::Large | MapSize::Xl) { 3 } else { 1 };
    for sign in [-1.0, 1.0].into_iter().take(central-1) {
        objectives.push(ObjectiveSite { id: format!("objective-{}", objectives.len()),
            center: [hub[0]+sign*extent*0.2, extent/2.0], radius_m: 50.0,
            kind: ObjectiveSiteKind::Field, counterpart: None });
    }
    let pairs = match request.size { MapSize::Small => 1, MapSize::Medium => 2, _ => 2 };
    let mut rng = crate::layout::stream(request, "objectives");
    for pair in 0..pairs {
        let first = objectives.len();
        let paired = (0..200).find_map(|_| {
            // Keep each counterpart pair in the same lateral corridor.  The
            // entry roads may have different segment origins, so deriving the
            // two points independently from those origins can produce wildly
            // different X coordinates and therefore asymmetric travel routes.
            // A shared hub-relative X with mirrored Y bands gives mapgen a
            // stable geometric approximation; sim remains authoritative on
            // actual navigable route lengths.
            let lateral = if pairs == 1 {
                // The compact three-site layout still needs a small lateral
                // offset to clear the central settlement ring.
                (if pair == 0 { -1.0 } else { 1.0 }) * extent * 0.10
            } else {
                let magnitude = match request.map_type {
                    contract::generation::MapType::Open => 0.16,
                    contract::generation::MapType::Mixed => 0.27,
                    contract::generation::MapType::Metro => 0.34,
                };
                (if pair == 0 { -1.0 } else { 1.0 }) * extent * magnitude
            };
            let x = hub[0] + lateral + (rng.unit()-0.5)*40.0;
            // Keep counterparts close to the centreline, where the two sides
            // share more of the same transit network and route lengths remain
            // comparable even on maps whose outer terrain is irregular.
            let low = extent*(0.38+rng.unit()*0.04);
            let high = extent-low;
            let candidates = [[x, low], [x, high]];
            candidates.iter().all(|&p| p.iter().all(|v| *v>=70.0 && *v<=extent-70.0)
                && !towns.iter().any(|ring| contract::ground::polygon_contains(ring,p)
                    || contract::ground::edges(ring).any(|(a,b)| contract::ground::segment_distance(*a,*b,p)<70.0))
                && objectives.iter().all(|o| distance(o.center,p)>=150.0)
                && entries.iter().all(|e| distance(e.center,p)>=150.0))
                .then_some(candidates)
        }).ok_or_else(|| fail("no reserved rural objective pair fits after 200 candidates"))?;
        for (half,center) in paired.into_iter().enumerate() {
            objectives.push(ObjectiveSite { id: format!("objective-{}", objectives.len()),center,
                radius_m: 50.0, kind: ObjectiveSiteKind::Field,
                counterpart: Some(format!("objective-{}",first+1-half)) });
        }
    }
    let sites = SkirmishSites { entries, objectives };
    for (i,o) in sites.objectives.iter().enumerate() {
        if sites.objectives[..i].iter().any(|p| distance(o.center,p.center)<150.0)
            || sites.entries.iter().any(|e| distance(o.center,e.center)<150.0) {
            return Err(fail("objective spacing is below 150 metres"));
        }
    }
    Ok(Some(sites))
}
fn distance(a: [f64;2], b: [f64;2]) -> f64 { libm::hypot(a[0]-b[0],a[1]-b[1]) }
