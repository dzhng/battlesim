//! The physical catalogue the parcel pass builds towns from. Its rows are the
//! city source sets' descriptors (`assets/source/city/`).
use contract::templates::{
    BuildingCategory, BuildingTemplateDescriptor, PlacementFrame, TemplateGeometryCatalog,
};
use mapgen::layout::PresetDefinitions;

const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");

fn catalogue() -> TemplateGeometryCatalog {
    TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap()).unwrap()
}

/// The regional families a map may be built in (M08).
fn families() -> Vec<String> {
    PresetDefinitions::from_json(PRESETS)
        .unwrap()
        .parcels
        .regional_families
}

/// Width and depth of a template's footprint in its own frame.
fn footprint(template: &BuildingTemplateDescriptor) -> [f64; 2] {
    [0, 1].map(|axis| {
        let reach = |sign: f64| {
            template
                .parts
                .iter()
                .map(|part| sign * part.center[axis] + part.half_extents[axis])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        reach(1.0) + reach(-1.0)
    })
}

#[test]
fn every_template_is_of_a_shipping_family() {
    // Loading the catalogue already refuses an incomplete building.
    let catalogue = catalogue();
    let families = families();
    for template in catalogue.templates() {
        assert!(
            families.contains(&template.regional_family),
            "{}: {} is not among {families:?}",
            template.id,
            template.regional_family
        );
    }
    // Its identity is its geometry: the list's order and spacing do not move it.
    let mut reordered: Vec<BuildingTemplateDescriptor> = serde_json::from_str(TEMPLATES).unwrap();
    reordered.reverse();
    assert_eq!(
        TemplateGeometryCatalog::new(reordered).unwrap().hash(),
        catalogue.hash()
    );
}

/// A map draws one family, so each family on its own must offer every
/// category at the scale its districts and countryside cut lots for.
#[test]
fn every_family_has_several_variants_of_every_category_at_its_accepted_scale() {
    for family in families() {
        category_scales(&family);
    }
}

fn category_scales(family: &str) {
    use BuildingCategory::*;
    let catalogue = catalogue();
    // (category, floors, longest footprint side in metres)
    let scales = [
        (Farmstead, 1..=2, 20.0..=45.0),
        (DetachedHome, 1..=2, 8.0..=14.0),
        (AttachedHome, 2..=3, 7.0..=32.0),
        (UrbanApartment, 4..=8, 20.0..=64.0),
        (Highrise, 9..=40, 24.0..=64.0),
        (Industry, 1..=2, 24.0..=96.0),
    ];
    for (category, floors, longest) in scales {
        let variants: Vec<_> = catalogue
            .templates()
            .iter()
            .filter(|template| template.category == category && template.regional_family == family)
            .collect();
        assert!(
            variants.len() >= 3,
            "{family} {category:?}: {}",
            variants.len()
        );
        for template in variants {
            let count = template.floor_heights_m.len();
            assert!(floors.contains(&count), "{}: {count} floors", template.id);
            let [width, depth] = footprint(template);
            assert!(
                longest.contains(&width.max(depth)),
                "{}: {width} × {depth} m",
                template.id
            );
            // A storey is between 2.5 and 12 m (a warehouse is one tall floor).
            let storey = template.height_m() / count as f64;
            assert!((2.5..=12.0).contains(&storey), "{}: {storey}", template.id);
        }
    }
    // M07: Open stops at six floors, so apartments must offer both sides of it.
    let apartment_floors = |low: usize, high: usize| {
        catalogue.templates().iter().any(|template| {
            template.category == UrbanApartment
                && template.regional_family == family
                && (low..=high).contains(&template.floor_heights_m.len())
        })
    };
    assert!(apartment_floors(4, 6) && apartment_floors(7, 8), "{family}");
}

/// The parcel pass turns a template's first entrance toward the street, so
/// every door must open on that one side, with nothing of the building in
/// front of it.
#[test]
fn every_entrance_opens_on_the_street_side_with_a_clear_way_out() {
    for template in catalogue().templates() {
        let placed = template
            .materialize(PlacementFrame {
                translation: [0.0; 3],
                yaw: 0.0,
            })
            .unwrap();
        let entrances = placed.entrances;
        let street = entrances[0].normal;
        for entrance in &entrances {
            assert_eq!(entrance.normal, street, "{}", template.id);
            // Walk straight out for 60 m: no part of the building is met.
            for step in 1..=120 {
                let reach = 0.5 * f64::from(step);
                let point = [
                    entrance.position[0] + street[0] * reach,
                    entrance.position[1] + street[1] * reach,
                ];
                let blocked = placed.parts.iter().any(|part| {
                    (point[0] - part.center[0]).abs() < part.half_extents[0]
                        && (point[1] - part.center[1]).abs() < part.half_extents[1]
                });
                assert!(!blocked, "{} {}", template.id, entrance.id);
            }
        }
    }
}
