//! File preparation boundary; generation, lowering and diagnostics live in the library.
use contract::maps::{CatalogueSelection, MapIdentity, MapSources, SourceReceipt};
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{self, PresetDefinitions};
use mapgen::{CompileOutcome, GenerateOutcome};
use std::path::Path;

const USAGE: &str = "usage:
  mapgen lower <request.json> <catalogue.json> [output-directory]
  mapgen generate <request.json> <presets.json> <catalogue.json> [plan.json]
  mapgen generate-map <request.json> <presets.json> <catalogue.json> [output-directory]
  mapgen inspect <plan.json> <presets.json> <catalogue.json> <picture.svg> [crop]
  mapgen catalogue <catalogue.json>

<catalogue.json> is a list of physical template descriptors; `catalogue` prints its
canonical form, whose hash a request pins. [crop] is a settlement or district id of
the plan, or x,y,width,height in metres.";

/// Print the compiler's outcome and, on success, save the map beside
/// receipts for exactly the input bytes that made it.
fn finish(
    outcome: CompileOutcome,
    inputs: &[(&str, &str)],
    directory: Option<&std::ffi::OsString>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let (CompileOutcome::Ok { result }, Some(path)) = (&outcome, directory) {
        let directory = Path::new(path);
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("map.json"), serde_json::to_vec(&result.map)?)?;
        std::fs::write(
            directory.join("SOURCES.json"),
            serde_json::to_vec(&MapSources {
                identity: MapIdentity::Generated {
                    generation: result.identity.clone(),
                },
                catalogue: CatalogueSelection { template_ids: None },
                inputs: inputs
                    .iter()
                    .map(|(label, bytes)| SourceReceipt::Supplied {
                        label: (*label).into(),
                        sha256: contract::identity::bytes_hash(bytes.as_bytes()),
                    })
                    .collect(),
            })?,
        )?;
    }
    println!("{}", serde_json::to_string(&outcome)?);
    Ok(matches!(outcome, CompileOutcome::Ok { .. }))
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let command = arguments.first().and_then(|command| command.to_str());
    let read = |index: usize| std::fs::read_to_string(&arguments[index]);
    match (command, arguments.len()) {
        (Some("lower"), 3 | 4) => {
            let (request, catalogue) = (read(1)?, read(2)?);
            finish(
                mapgen::compile(&request, &catalogue),
                &[("request", &request), ("catalogue", &catalogue)],
                arguments.get(3),
            )
        }
        (Some("generate"), 4 | 5) => {
            let outcome = mapgen::generate_plan(&read(1)?, &read(2)?, &read(3)?);
            if let (GenerateOutcome::Ok { plan }, Some(path)) = (&outcome, arguments.get(4)) {
                std::fs::write(path, serde_json::to_vec(plan)?)?;
            }
            println!("{}", serde_json::to_string(&outcome)?);
            Ok(matches!(outcome, GenerateOutcome::Ok { .. }))
        }
        (Some("generate-map"), 4 | 5) => {
            let (request, presets, catalogue) = (read(1)?, read(2)?, read(3)?);
            finish(
                mapgen::generate_map(&request, &presets, &catalogue),
                &[
                    ("request", &request),
                    ("presets", &presets),
                    ("catalogue", &catalogue),
                ],
                arguments.get(4),
            )
        }
        (Some("inspect"), 5 | 6) => {
            let plan: mapgen::MapPlan = serde_json::from_str(&read(1)?)?;
            let presets =
                PresetDefinitions::from_json(&read(2)?).map_err(|errors| format!("{errors:?}"))?;
            let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&read(3)?)?)?;
            let title = Path::new(&arguments[1])
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let crop = arguments.get(5).and_then(|crop| crop.to_str());
            let metrics = layout::measure(&plan, &presets);
            let picture = mapgen::inspect::svg(&plan, &catalogue, &title, &metrics, crop)?;
            std::fs::write(&arguments[4], picture)?;
            println!("{}", serde_json::to_string(&metrics)?);
            Ok(true)
        }
        (Some("catalogue"), 2) => {
            let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&read(1)?)?)?;
            println!("{}", catalogue.canonical_json()?);
            Ok(true)
        }
        _ => Err(USAGE.into()),
    }
}

fn main() {
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
