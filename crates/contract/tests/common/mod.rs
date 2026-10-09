//! The one catalogue of buildings the contract tests place.
use contract::templates::BuildingTemplateDescriptor;

/// The map generator's template library, the one catalogue of buildings.
pub const LIBRARY: &str = include_str!("../../../../fixtures/prototype-building-templates.json");

/// The library's template `id`.
pub fn template(id: &str) -> BuildingTemplateDescriptor {
    serde_json::from_str::<Vec<BuildingTemplateDescriptor>>(LIBRARY)
        .unwrap()
        .into_iter()
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("no template {id:?}"))
}
