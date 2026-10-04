//! The parcel pass: each district of a generated layout gets its streets,
//! then parcels along them, then one template on each parcel. A template is
//! placed whole, by translation and rotation; a parcel is cut to it.
pub(crate) mod courts;
pub(crate) mod lots;
pub(crate) mod space;
pub(crate) mod streets;

use crate::layout::rng::Stream;
use crate::layout::{measure, DistrictPreset, GenerationRequest, PresetDefinitions};
use crate::{Diagnostic, DiagnosticCode, DistrictPlan, MapPlan};
use contract::templates::TemplateGeometryCatalog;
use lots::{Choices, Fit, Ground};

/// One request's fixed inputs, shared by the pass's steps.
pub(crate) struct Pass<'a> {
    request: &'a GenerationRequest,
    pub presets: &'a PresetDefinitions,
    /// Every template of the catalogue a parcel may select.
    fits: Vec<Fit<'a>>,
    /// The one regional family this map is built in (M08).
    family: &'a str,
}

impl Pass<'_> {
    fn stream(&self, name: &str) -> Stream {
        crate::layout::stream(self.request, name)
    }

    /// A step could not do what the presets ask. Names the feature, where
    /// its numbers come from, and the request; never tries another seed.
    fn fail(&self, feature: &str, location: &str, message: &str) -> Vec<Diagnostic> {
        vec![Diagnostic {
            code: DiagnosticCode::GenerationFailed,
            feature: Some(feature.into()),
            location: location.into(),
            message: format!(
                "{message} ({} {}, seed {})",
                self.request.map_type.name(),
                self.request.size.name(),
                self.request.seed.value()
            ),
        }]
    }

    /// How wide a street of `kind` is: a lane's width where it is a dirt
    /// track.
    fn street_width(&self, kind: contract::map::SurfaceKind) -> f64 {
        match kind {
            contract::map::SurfaceKind::DirtTrack => self.presets.roads.dirt_track_width_m,
            _ => self.presets.parcels.street_width_m,
        }
    }

    fn district(&self, district: &DistrictPlan) -> Result<&DistrictPreset, Vec<Diagnostic>> {
        self.presets.districts.get(&district.kind).ok_or_else(|| {
            vec![Diagnostic {
                code: DiagnosticCode::InvalidPresets,
                feature: Some(district.id.clone()),
                location: "$.presets.districts".into(),
                message: format!("the presets have no district kind {:?}", district.kind),
            }]
        })
    }

    fn choices(&self, district: &DistrictPlan) -> Result<Choices<'_>, Vec<Diagnostic>> {
        Choices::new(&self.fits, district, self.family).map_err(|error| vec![error])
    }
}

/// The one regional family the request's map is built in (M08): the region
/// it asks for, or else a draw of its own stream, so every pass that stands
/// buildings agrees on it. `admit_region` has refused an unlisted region.
pub(crate) fn family<'a>(request: &GenerationRequest, presets: &'a PresetDefinitions) -> &'a str {
    let families = &presets.parcels.regional_families;
    match &request.region {
        Some(region) => families
            .iter()
            .find(|family| *family == region)
            .expect("generate admits only a listed region"),
        None => {
            &families
                [crate::layout::stream(request, "family").below(families.len() as u64) as usize]
        }
    }
}

/// A request that asks for a region the presets do not list is refused:
/// nothing else is drawn in its place.
pub(crate) fn admit_region(
    request: &GenerationRequest,
    presets: &PresetDefinitions,
) -> Result<(), Vec<Diagnostic>> {
    match &request.region {
        Some(region) if !presets.parcels.regional_families.contains(region) => {
            Err(vec![Diagnostic {
                code: DiagnosticCode::InvalidRequest,
                feature: None,
                location: "$.region".into(),
                message: format!(
                    "region {region:?} is not one of the presets' regional families {:?}",
                    presets.parcels.regional_families
                ),
            }])
        }
        _ => Ok(()),
    }
}

/// Cut every district of `plan` into streets and parcels and stand templates
/// of `catalogue` on them. The result is the same plan with its streets,
/// aprons and courts added to `surfaces`, its parcels in `lots`, its courts
/// in `courts` and one `buildings` row per placed template, numbered into
/// the plan's dense prop ids.
pub fn fill_districts(
    plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
) -> Result<MapPlan, Vec<Diagnostic>> {
    if request.template_catalog_hash != catalogue.hash() {
        return Err(vec![Diagnostic {
            code: DiagnosticCode::InvalidCatalogue,
            feature: None,
            location: "$.template_catalog_hash".into(),
            message: "requested physical catalogue hash differs from the supplied catalogue".into(),
        }]);
    }
    // An avenue that stops short of a road it may not meet is cut back a
    // row of lots, like a street. Where that takes the only frontage a
    // district has (a hamlet's one lane, mostly), the avenues keep the
    // length the layout gave them.
    fill(plan.clone(), request, catalogue, presets, true)
        .or_else(|_| fill(plan, request, catalogue, presets, false))
}

/// `fill_districts`, cutting back the layout's avenues that stop short of a
/// carriageway where `trim_avenues`.
fn fill(
    mut plan: MapPlan,
    request: &GenerationRequest,
    catalogue: &TemplateGeometryCatalog,
    presets: &PresetDefinitions,
    trim_avenues: bool,
) -> Result<MapPlan, Vec<Diagnostic>> {
    let pass = Pass {
        request,
        presets,
        fits: catalogue.templates().iter().filter_map(Fit::new).collect(),
        family: family(request, presets),
    };

    // A street's whole width stays off a river's bank.
    let clearance = presets.rivers.bank_m() + presets.parcels.street_width_m / 2.0;
    let laid = {
        let mut network = streets::Network::new(&plan, clearance, &presets.parcels.geometry);
        let mut laid = Vec::new();
        for settlement in &plan.settlements {
            streets::lay(&pass, &mut network, settlement, &mut laid)?;
        }
        streets::run_on(&pass, &mut network, &plan, &mut laid)?;
        laid
    };
    // The streets' ends are closed against each other and the layout's
    // roads before any parcel is cut along them.
    let planned = if trim_avenues { 0 } else { plan.surfaces.len() };
    plan.surfaces.extend(laid);
    streets::trim_tails(&pass, &mut plan.surfaces, planned, plan.size);
    let blocks: Vec<&[[f64; 2]]> = plan
        .settlements
        .iter()
        .flat_map(|settlement| {
            settlement
                .districts
                .iter()
                .map(|district| &district.ring[..])
        })
        .collect();
    plan.surfaces = crate::joints::close(
        &presets.joints,
        core::mem::take(&mut plan.surfaces),
        plan.size,
        &blocks,
    );
    let (lots, buildings, aprons, courts, paving) = {
        let network = streets::Network::new(&plan, clearance, &presets.parcels.geometry);
        let mut ground = Ground::new(&plan, &network);
        for district in plan.settlements.iter().flat_map(|s| &s.districts) {
            if ground.fill(&pass, district)? == 0 {
                return Err(pass.fail(
                    &district.id,
                    &format!("$.presets.districts.{}", district.kind),
                    "no parcel of its templates and setbacks fits along its streets",
                ));
            }
        }
        let (courts, paving) = courts::lay(&pass, &network, &plan, &ground.plan_lots)?;
        (
            ground.plan_lots,
            ground.buildings,
            ground.aprons,
            courts,
            paving,
        )
    };
    plan.surfaces.extend(aprons);
    plan.surfaces.extend(paving);
    plan.courts = courts;
    plan.lots = lots;
    plan.buildings = buildings;

    // Held to what the finished plan measures as, like the layout before it.
    let metrics = measure(&plan, presets);
    if metrics.roads.unbridged > 0 {
        return Err(pass.fail(
            "streets",
            "$.presets.parcels",
            &format!("{} runs of street enter the water", metrics.roads.unbridged),
        ));
    }
    if metrics.roads.unconnected_street_km > 0.0 {
        return Err(pass.fail(
            "streets",
            "$.presets.parcels",
            &format!(
                "{:.2} km of streets have no way to the centre",
                metrics.roads.unconnected_street_km
            ),
        ));
    }
    if metrics.ground_points > request.limits.max_ground_points {
        return Err(vec![Diagnostic {
            code: DiagnosticCode::ComplexityLimit,
            feature: None,
            location: "$.limits.max_ground_points".into(),
            message: format!(
                "the generated plan has {} ground points",
                metrics.ground_points
            ),
        }]);
    }
    Ok(plan)
}
