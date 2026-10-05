//! Courts: the paved pieces of a dense district's block. Each built parcel
//! is paved as its own yard, the building standing on it; car parks are cut
//! from the ground the parcels leave beside the carriageways; the rest of
//! the district, off its carriageways, stays grass: its lawn.
use super::lots::Ground;
use super::Pass;
use crate::{CourtKind, CourtPlan, Diagnostic, MapPlan};
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::BTreeSet;

/// The courts of every district of `plan` whose presets pave them, with the
/// paving each is laid as. `ground` holds the parcels and buildings the pass
/// cut and stood; the car parks are kept on it as parcels are.
pub fn lay(
    pass: &Pass,
    plan: &MapPlan,
    ground: &mut Ground,
) -> Result<(Vec<CourtPlan>, Vec<SurfaceArea>), Vec<Diagnostic>> {
    let built: BTreeSet<String> = ground.buildings.iter().map(|b| b.id.clone()).collect();
    let mut courts = Vec::new();
    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
        let rule = &pass.district(district)?.props.courts;
        if !rule.paved {
            continue;
        }
        let prefix = format!("{}/", district.id);
        for lot in &ground.plan_lots {
            if lot.id.starts_with(&prefix) && built.contains(&lot.id) {
                courts.push(CourtPlan {
                    id: format!("{}/yard", lot.id),
                    district: district.id.clone(),
                    kind: CourtKind::Yard,
                    ring: lot.ring.clone(),
                });
            }
        }
        if let Some(parking) = &rule.parking {
            for (n, ring) in ground
                .parking(pass, district, parking)
                .into_iter()
                .enumerate()
            {
                courts.push(CourtPlan {
                    id: format!("{}/parking-{n}", district.id),
                    district: district.id.clone(),
                    kind: CourtKind::Parking,
                    ring: ring.to_vec(),
                });
            }
        }
    }
    let paving = courts
        .iter()
        .filter_map(|court| {
            Some(SurfaceArea {
                kind: SurfaceKind::Paving,
                shape: GroundShape::polygon(court.ring.clone()).ok()?,
            })
        })
        .collect();
    Ok((courts, paving))
}
