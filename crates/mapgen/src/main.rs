//! File preparation boundary; generation, lowering and diagnostics live in the library.
use contract::maps::{CatalogueSelection, MapIdentity, MapSources, SourceReceipt};
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{self, PresetDefinitions};
use mapgen::{CompileOutcome, GenerateOutcome};
use std::path::Path;

const USAGE: &str = "usage:
  mapgen request <type> <size> <seed> <presets.json> <catalogue.json> <generated-battle.json>
  mapgen lower <request.json> <catalogue.json> [output-directory]
  mapgen generate <request.json> <presets.json> <catalogue.json> <rules.json> <catalog.json> [plan.json]
  mapgen generate-map <request.json> <presets.json> <catalogue.json> <rules.json> <catalog.json> [output-directory]
  mapgen inspect <plan.json> <presets.json> <catalogue.json> <picture.svg> [crop]
  mapgen catalogue <catalogue.json>

<catalogue.json> is a list of physical template descriptors; `catalogue` prints its
canonical form, whose hash a request pins. <rules.json> is the existing battle rules
(fixtures/game.json); <catalog.json> explicitly resolves its unit and prop catalog
(fixtures/catalog.json). Together they supply generation's physical inputs. Street
furniture is placed as those prop types, clear of
the lane its widest hull drives. [output-directory] receives a saved map
(fixtures/README.md): map.json with each building as its template and frame,
SOURCES.json naming <catalogue.json>'s file as the map's library, and sites.json.
`request` prints the generation request for a map type (open, mixed, metro), size
(small, medium, large, xl) and seed, pinned to this generator, the presets' revision and
the catalogue's hash, under the limits of <generated-battle.json> (fixtures/generated-battle.json):
the request the game makes for the same choice. [crop] is a settlement or district id of
the plan, bridge-<n> for its nth bridge from 0, or x,y,width,height in metres.";

/// Print the compiler's outcome and, on success, save the map in the saved
/// form (each building as its template and frame) beside receipts for
/// exactly the input bytes that made it. `library` is the catalogue's file:
/// its name is what the saved map's sources tell a loader to fetch.
fn finish(
    outcome: CompileOutcome,
    inputs: &[(&str, &str)],
    library: &std::ffi::OsStr,
    directory: Option<&std::ffi::OsString>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let (CompileOutcome::Ok { result }, Some(path)) = (&outcome, directory) {
        let directory = Path::new(path);
        let library = Path::new(library)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("the catalogue's file has no name to save as the map's library")?;
        std::fs::create_dir_all(directory)?;
        std::fs::write(
            directory.join("map.json"),
            serde_json::to_vec(&result.map.saved())?,
        )?;
        // What the encounter planner reads beside the map.
        std::fs::write(
            directory.join("sites.json"),
            serde_json::to_vec(&result.sites)?,
        )?;
        std::fs::write(
            directory.join("SOURCES.json"),
            serde_json::to_vec(&MapSources {
                identity: MapIdentity::Generated {
                    generation: result.identity.clone(),
                },
                catalogue: CatalogueSelection {
                    library: library.into(),
                    template_ids: None,
                },
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

/// Explicit existing mechanics and resolved catalog files form the same
/// rules record the browser preparation worker already carries.
fn generation_rules(rules: &str, catalog: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut rules: serde_json::Value = serde_json::from_str(rules)?;
    let view: serde_json::Value = serde_json::from_str(catalog)?;
    rules["catalog"] = view
        .get("documents")
        .ok_or("the catalog file has no documents")?
        .clone();
    Ok(serde_json::to_string(&rules)?)
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let command = arguments.first().and_then(|command| command.to_str());
    let read = |index: usize| std::fs::read_to_string(&arguments[index]);
    match (command, arguments.len()) {
        (Some("request"), 7) => {
            let text = |index: usize| arguments[index].to_string_lossy().into_owned();
            let presets: serde_json::Value = serde_json::from_str(&read(4)?)?;
            let catalogue = TemplateGeometryCatalog::new(serde_json::from_str(&read(5)?)?)?;
            let game: serde_json::Value = serde_json::from_str(&read(6)?)?;
            let request: layout::GenerationRequest = serde_json::from_value(serde_json::json!({
                "generator_version": layout::GENERATOR_VERSION,
                "preset_revision": presets["revision"],
                "seed": text(3),
                "template_catalog_hash": catalogue.hash(),
                "type": text(1),
                "size": text(2),
                "limits": game["limits"],
            }))?;
            println!("{}", serde_json::to_string(&request)?);
            Ok(true)
        }
        (Some("lower"), 3 | 4) => {
            let (request, catalogue) = (read(1)?, read(2)?);
            finish(
                mapgen::compile(&request, &catalogue),
                &[("request", &request), ("catalogue", &catalogue)],
                &arguments[2],
                arguments.get(3),
            )
        }
        (Some("generate"), 6 | 7) => {
            let outcome = mapgen::generate_plan(
                &read(1)?,
                &read(2)?,
                &read(3)?,
                &generation_rules(&read(4)?, &read(5)?)?,
            );
            if let (GenerateOutcome::Ok { plan }, Some(path)) = (&outcome, arguments.get(6)) {
                std::fs::write(path, serde_json::to_vec(plan)?)?;
            }
            println!("{}", serde_json::to_string(&outcome)?);
            Ok(matches!(outcome, GenerateOutcome::Ok { .. }))
        }
        (Some("generate-map"), 6 | 7) => {
            let (request, presets, catalogue, rules, catalog) =
                (read(1)?, read(2)?, read(3)?, read(4)?, read(5)?);
            finish(
                mapgen::generate_map(
                    &request,
                    &presets,
                    &catalogue,
                    &generation_rules(&rules, &catalog)?,
                ),
                &[
                    ("request", &request),
                    ("presets", &presets),
                    ("catalogue", &catalogue),
                    ("rules", &rules),
                    ("catalog", &catalog),
                ],
                &arguments[3],
                arguments.get(6),
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
