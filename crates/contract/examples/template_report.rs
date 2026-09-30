//! Small physical catalogue/materialization report. Inputs are a descriptor
//! JSON file and a placement-frame JSON string; no world or art is constructed.
use contract::templates::{BuildingTemplateDescriptor, PlacementFrame, TemplateGeometryCatalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: template_report <descriptor.json> <frame-json>")?;
    let frame = args.next().ok_or("placement-frame JSON is required")?;
    let descriptor: BuildingTemplateDescriptor =
        serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let frame: PlacementFrame = serde_json::from_str(&frame)?;
    let catalogue = TemplateGeometryCatalog::new(vec![descriptor])?;
    let materialized = catalogue.templates()[0].materialize(frame)?;
    println!(
        "{}",
        serde_json::to_string(
            &serde_json::json!({"catalogue":catalogue,"materialized":materialized})
        )?
    );
    Ok(())
}
