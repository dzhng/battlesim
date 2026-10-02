//! The parcel pass: each district of a generated layout gets its streets,
//! then parcels along them, then one template on each parcel. A template is
//! placed whole, by translation and rotation; a parcel is cut to it.
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

/// The one regional family the request's map is built in (M08): a draw of
/// its own stream, so every pass that stands buildings agrees on it.
pub(crate) fn family<'a>(request: &GenerationRequest, presets: &'a PresetDefinitions) -> &'a str {
    let families = &presets.parcels.regional_families;
    let pick = crate::layout::stream(request, "family").below(families.len() as u64) as usize;
    &families[pick]
}

/// Cut every district of `plan` into streets and parcels and stand templates
/// of `catalogue` on them. The result is the same plan with its streets and
/// aprons added to `surfaces`, its parcels in `lots` and one `buildings` row
/// per placed template, numbered into the plan's dense prop ids.
pub fn fill_districts(
    mut plan: MapPlan,
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
    let pass = Pass {
        request,
        presets,
        fits: catalogue.templates().iter().filter_map(Fit::new).collect(),
        family: family(request, presets),
    };

    // A street's whole width stays off a river's bank.
    let clearance = presets.rivers.bank_m() + presets.parcels.street_width_m / 2.0;
    let laid = {
        let mut network = streets::Network::new(&plan, clearance);
        let mut laid = Vec::new();
        for settlement in &plan.settlements {
            streets::lay(&pass, &mut network, settlement, &mut laid)?;
        }
        laid
    };
    // The streets' ends are closed against each other and the layout's
    // roads before any parcel is cut along them.
    plan.surfaces.extend(laid);
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
    plan.surfaces = crate::joints::close(core::mem::take(&mut plan.surfaces), plan.size, &blocks);
    let (lots, buildings, aprons) = {
        let network = streets::Network::new(&plan, clearance);
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
        (ground.plan_lots, ground.buildings, ground.aprons)
    };
    plan.surfaces.extend(aprons);
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
