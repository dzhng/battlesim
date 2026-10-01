//! The generator through its file boundary: the same commands a person or a
//! build step runs.
use std::path::PathBuf;
use std::process::Command;

const PRESETS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/map-presets.json"
);
const CATALOGUE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/prototype-building-templates.json"
);

fn catalogue() -> contract::templates::TemplateGeometryCatalog {
    let descriptors = std::fs::read_to_string(CATALOGUE).unwrap();
    contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(&descriptors).unwrap())
        .unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("mapgen-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn mapgen(arguments: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mapgen"))
        .args(arguments)
        .output()
        .unwrap()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Corpus {
    cases: Vec<Record>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Record {
    name: String,
    command: String,
    request_json: String,
    /// The outcome runs to megabytes; its hash holds Wasm to every byte.
    native_sha256: String,
    exit_code: i32,
}

/// The native half of the native/Wasm proof: `web/tests/mapLayout.test.ts`
/// holds the Wasm exports to these same outcome hashes.
#[test]
fn native_cli_replays_the_frozen_generation_records() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/parity/map-layout/paired-records.json"
    );
    let mut corpus: Corpus = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    // `BLESS_PARITY=1` rewrites the native half for a named generator or
    // preset change; the web test then holds Wasm to it.
    let bless = std::env::var_os("BLESS_PARITY").is_some();
    let directory = scratch("layout-replay");
    let request = directory.join("request.json");
    for record in &mut corpus.cases {
        std::fs::write(&request, &record.request_json).unwrap();
        let process = mapgen(&[
            record.command.as_ref(),
            request.as_os_str(),
            PRESETS.as_ref(),
            CATALOGUE.as_ref(),
        ]);
        let stdout = String::from_utf8(process.stdout).unwrap();
        let outcome = stdout.strip_suffix('\n').unwrap();
        let hash = contract::identity::bytes_hash(outcome.as_bytes());
        if bless {
            record.exit_code = process.status.code().unwrap();
            record.native_sha256 = hash;
            continue;
        }
        assert_eq!(
            process.status.code(),
            Some(record.exit_code),
            "{}",
            record.name
        );
        assert_eq!(
            hash, record.native_sha256,
            "{}: {outcome:.300}",
            record.name
        );
        let status = if record.exit_code == 0 { "ok" } else { "error" };
        assert!(outcome.starts_with(&format!("{{\"status\":\"{status}\"")));
    }
    std::fs::remove_dir_all(directory).unwrap();
    if bless {
        std::fs::write(file, serde_json::to_string_pretty(&corpus).unwrap() + "\n").unwrap();
    }
}

fn request(directory: &std::path::Path) -> PathBuf {
    let path = directory.join("request.json");
    let request = serde_json::json!({
        "generator_version": mapgen::layout::GENERATOR_VERSION,
        "preset_revision": "layout-presets-4",
        "seed": "11",
        "template_catalog_hash": catalogue().hash(),
        "type": "mixed",
        "size": "small",
        "limits": {"max_authored_parts": 20_000, "max_bay_positions": 200_000, "max_ground_points": 200_000},
    });
    std::fs::write(&path, request.to_string()).unwrap();
    path
}

#[test]
fn a_generated_plan_file_compiles_and_draws() {
    let directory = scratch("layout-plan");
    let request_path = request(&directory);
    let plan_path = directory.join("plan.json");
    let process = mapgen(&[
        "generate".as_ref(),
        request_path.as_os_str(),
        PRESETS.as_ref(),
        CATALOGUE.as_ref(),
        plan_path.as_os_str(),
    ]);
    assert!(process.status.success());
    let plan: mapgen::MapPlan =
        serde_json::from_str(&std::fs::read_to_string(&plan_path).unwrap()).unwrap();
    // The saved plan is the printed plan, and reads back to the same bytes.
    let saved = std::fs::read_to_string(&plan_path).unwrap();
    assert_eq!(
        String::from_utf8(process.stdout).unwrap(),
        format!("{{\"status\":\"ok\",\"plan\":{saved}}}\n")
    );
    assert_eq!(serde_json::to_string(&plan).unwrap(), saved);

    assert!(!plan.buildings.is_empty() && plan.lots.len() >= plan.buildings.len());

    let picture = directory.join("plan.svg");
    let inspect = |crop: Option<&str>| {
        let mut arguments = vec![
            "inspect".as_ref(),
            plan_path.as_os_str(),
            PRESETS.as_ref(),
            CATALOGUE.as_ref(),
            picture.as_os_str(),
        ];
        arguments.extend(crop.map(std::ffi::OsStr::new));
        let process = mapgen(&arguments);
        assert!(
            process.status.success(),
            "{}",
            String::from_utf8_lossy(&process.stderr)
        );
        let svg = std::fs::read_to_string(&picture).unwrap();
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        (svg, process.stdout)
    };
    // Every layer a reviewer needs is drawn on the whole map: districts,
    // roads and streets, aprons, forests, approaches, every building's
    // parts, and the scale.
    let (svg, stdout) = inspect(None);
    let districts: usize = plan.settlements.iter().map(|s| s.districts.len()).sum();
    let parts: usize = plan.buildings.iter().map(|b| b.parts.len()).sum();
    let drawn = svg.matches("<path").count();
    let layers =
        districts + plan.surfaces.len() + plan.forests.len() + plan.approaches.len() + parts;
    assert!(drawn >= layers, "{drawn} paths for {layers} features");
    assert!(svg.contains("1 km") && svg.contains("detached home"));
    let metrics: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(metrics["roads"]["unconnected_settlements"], 0);
    assert_eq!(metrics["roads"]["unconnected_street_km"], 0.0);
    assert!(metrics["roads"]["street_km"].as_f64().unwrap() > 10.0);

    // One district, close enough to read: its parcels and an entrance tick
    // for each door are drawn, and far fewer buildings than the whole map.
    let district = &plan.settlements[0].districts[0];
    let (crop, _) = inspect(Some(&district.id));
    let inside = plan
        .lots
        .iter()
        .filter(|lot| lot.id.starts_with(&format!("{}/", district.id)))
        .count();
    assert!(inside > 0 && crop.matches("#5c5346").count() >= inside);
    assert!(crop.contains("#ffd43b"), "no entrance is drawn");
    assert!(!svg.contains("#ffd43b") && !svg.contains("#5c5346"));
    assert!(svg.contains("this view is 6000 m wide"));
    assert!(!crop.contains("this view is 6000 m wide") && !crop.contains("1 km"));
    // A crop that names nothing in the plan is refused.
    let process = mapgen(&[
        "inspect".as_ref(),
        plan_path.as_os_str(),
        PRESETS.as_ref(),
        CATALOGUE.as_ref(),
        picture.as_os_str(),
        "settlement-999".as_ref(),
    ]);
    assert_eq!(process.status.code(), Some(2));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn generate_map_saves_a_map_the_battle_loader_resolves() {
    let directory = scratch("layout-map");
    let request_path = request(&directory);
    let saved = directory.join("saved");
    let process = mapgen(&[
        "generate-map".as_ref(),
        request_path.as_os_str(),
        PRESETS.as_ref(),
        CATALOGUE.as_ref(),
        saved.as_os_str(),
    ]);
    assert!(
        process.status.success(),
        "{}",
        String::from_utf8_lossy(&process.stdout)
    );
    let outcome: serde_json::Value = serde_json::from_slice(&process.stdout).unwrap();
    let map = std::fs::read_to_string(saved.join("map.json")).unwrap();
    let sources = std::fs::read_to_string(saved.join("SOURCES.json")).unwrap();
    // The battle loader resolves it against the catalogue the CLI's
    // `catalogue` command prints for the same descriptor list.
    let library = mapgen(&["catalogue".as_ref(), CATALOGUE.as_ref()]);
    assert!(library.status.success());
    let library = String::from_utf8(library.stdout).unwrap();
    assert_eq!(library.trim_end(), catalogue().canonical_json().unwrap());
    let resolved = contract::maps::resolve(
        &map,
        &sources,
        &library,
        contract::maps::MapAdmission {
            max_authored_parts: 20_000,
            max_bay_positions: 200_000,
        },
    )
    .unwrap();
    assert_eq!(resolved.definition.size, [6_000.0, 6_000.0]);
    assert!(!resolved.definition.surfaces.is_empty() && !resolved.definition.forests.is_empty());
    // A generated map has its town: streets, and buildings on them.
    let definition = &resolved.definition;
    assert!(definition.buildings.len() > 1_000);
    let streets = definition.surfaces.iter();
    assert!(
        streets
            .filter(|area| area.kind == contract::map::SurfaceKind::Road)
            .count()
            > 100
    );
    assert_eq!(
        serde_json::to_value(&resolved.identity).unwrap()["generation"],
        outcome["result"]["identity"]
    );
    assert_eq!(outcome["result"]["identity"]["seed"], "11");
    // The receipts name every input the map came from, presets included.
    let sources: serde_json::Value = serde_json::from_str(&sources).unwrap();
    let labels: Vec<&str> = sources["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| input["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["request", "presets", "catalogue"]);
    std::fs::remove_dir_all(directory).unwrap();
}
