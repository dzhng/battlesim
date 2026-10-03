use serde_json::{json, Value};
use std::io::Read;

#[path = "common/workbench.rs"]
mod workbench;
fn run(request: Value) -> Value {
    serde_json::from_value(request).map_err(|e| e.to_string()).and_then(workbench::run).unwrap_or_else(|error| json!({"status":"invalid","diagnostics":[{"code":"invalid_request","feature":null,"location":"$","message":error}],"fields":[]}))
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("stdin");
    let outcome = serde_json::from_str(&input).map(run).unwrap_or_else(|error| json!({"status":"invalid","diagnostics":[{"code":"invalid_request","feature":null,"location":"$","message":error.to_string()}],"fields":[]}));
    println!("{outcome}");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> Value {
        json!({"presets":include_str!("../../../fixtures/map-presets.json"),"defaults":include_str!("../../../fixtures/generated-battle.json"),"templates":include_str!("../../../fixtures/prototype-building-templates.json"),"rules":include_str!("../../../fixtures/game.json"),"catalog":include_str!("../../../fixtures/catalog.json"),"recipes":include_str!("../../../fixtures/encounters.json")})
    }
    #[test]
    fn report_retains_the_exact_plan_and_inspects_without_regenerating() {
        let directory =
            std::env::temp_dir().join(format!("map-workbench-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let request = json!({"operation":"generate","inputs":inputs(),"choice":{"type":"open","size":"small","seed":"1"},"artifactDir":directory});
        let result = run(request);
        assert_eq!(result["status"], "ok", "{result}");
        let inspection = run(json!({"operation":"inspect","artifactDir":directory}));
        assert_eq!(inspection["svg"], result["svg"]);
        let _ = std::fs::remove_dir_all(directory);
    }
    #[test]
    fn a_nonpositive_sample_spacing_is_invalid_before_generation() {
        let request = json!({"operation":"validate","inputs":{
            "presets":include_str!("../../../fixtures/map-presets.json"),
            "defaults":"{\"analysis\":{\"step_m\":0,\"min_median_open\":0.5}}",
            "templates":include_str!("../../../fixtures/prototype-building-templates.json"),
            "rules":include_str!("../../../fixtures/game.json"),
            "catalog":include_str!("../../../fixtures/catalog.json"),
            "recipes":include_str!("../../../fixtures/encounters.json")
        }});
        let result = run(request);
        assert_eq!(result["status"], "invalid");
        assert!(result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["location"] == "$.defaults.analysis.step_m"));
    }
}
