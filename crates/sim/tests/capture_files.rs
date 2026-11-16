//! The capture files (`fixtures/captures`) film battles that exist: each
//! scene's map is a capture map, and each shot follows one of the filming
//! side's own units. The battles themselves follow the game, so nothing pins
//! what happens in them.
use std::path::PathBuf;

use serde_json::Value;
use sim::fixtures;

/// Every capture file films only capture maps, and every shot follows a unit
/// its scene's encounter fields (a unit's id is its index in `units`). The
/// web validator checks the rest of the file, but loads no encounter.
#[test]
fn every_capture_films_a_capture_map_and_follows_its_own_units() {
    let read = |path: PathBuf| -> Value {
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(fixtures::dir().join("captures"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no capture files");
    for file in files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let capture = read(file);
        for (i, scene) in capture["scenes"].as_array().unwrap().iter().enumerate() {
            let map = scene["map"].as_str().unwrap();
            let encounter = scene["encounter"].as_str().unwrap();
            let maps = fixtures::dir().join("maps").join(map);
            let category = read(maps.join("meta.json"))["category"].clone();
            assert_eq!(
                category, "capture",
                "{name} scenes[{i}].map: {map} is a {category} map, not a capture map"
            );
            let encounter_json = read(maps.join("encounters").join(format!("{encounter}.json")));
            let units = encounter_json["units"].as_array().unwrap();
            for (s, shot) in scene["reel"]["shots"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                if let Some(follow) = shot["follow"].as_u64() {
                    let at = format!("{name} scenes[{i}].reel.shots[{s}].follow");
                    let unit = units.get(follow as usize).unwrap_or_else(|| {
                        panic!(
                            "{at}: unit {follow}, but {map}'s encounter {encounter} fields {} units",
                            units.len()
                        )
                    });
                    // The camera follows the filming side's own units: the
                    // client sees nothing else's true position.
                    assert_eq!(
                        unit["side"], "blue",
                        "{at}: unit {follow} is not one of blue's"
                    );
                }
            }
        }
    }
}
