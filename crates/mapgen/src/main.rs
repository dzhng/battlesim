//! File preparation boundary; physical lowering and diagnostics live in the library.
use mapgen::CompileOutcome;
use std::path::Path;

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !(arguments.len() == 3 || arguments.len() == 4) || arguments[0] != "lower" {
        return Err(
            "usage: mapgen lower <request.json> <catalogue.json> [output-directory]".into(),
        );
    }
    let request = std::fs::read_to_string(&arguments[1])?;
    let catalogue = std::fs::read_to_string(&arguments[2])?;
    let outcome = mapgen::compile(&request, &catalogue);
    if let CompileOutcome::Ok { result } = &outcome {
        if let Some(path) = arguments.get(3) {
            let directory = Path::new(path);
            std::fs::create_dir_all(directory)?;
            std::fs::write(directory.join("map.json"), serde_json::to_vec(&result.map)?)?;
            std::fs::write(
                directory.join("SOURCES.json"),
                serde_json::to_vec(&result.identity)?,
            )?;
        }
    }
    println!("{}", serde_json::to_string(&outcome)?);
    Ok(matches!(outcome, CompileOutcome::Ok { .. }))
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
