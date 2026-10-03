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
    fn sampled_sight_reports_empty_and_clipped_edge_samples_truthfully() {
        let directory =
            std::env::temp_dir().join(format!("map-workbench-sight-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let geometry = json!({"size":[100.0,100.0],"fog_cell_m":20.0,"height_grid_m":5.0,"slope_cutoff_deg":35.0});
        let mut artifact = json!({"inputs":inputs(),"choice":{"type":"open","size":"small","seed":"1"},"map":geometry,"plan":geometry,"sites":{"settlements":[],"approaches":[]}});
        std::fs::write(directory.join("artifact.json"), artifact.to_string()).unwrap();
        let empty = run(json!({"operation":"sight","artifactDir":directory}));
        assert_eq!(empty["status"], "unavailable", "{empty}");
        assert_eq!(empty["samples"], 0);
        assert!(empty.get("median").is_none());
        let mut defaults: Value =
            serde_json::from_str(artifact["inputs"]["defaults"].as_str().unwrap()).unwrap();
        defaults["analysis"]["step_m"] = json!(50.0);
        artifact["inputs"]["defaults"] = json!(defaults.to_string());
        std::fs::write(directory.join("artifact.json"), artifact.to_string()).unwrap();
        let clipped = run(json!({"operation":"sight","artifactDir":directory}));
        assert_eq!(clipped["status"], "measured", "{clipped}");
        assert_eq!(clipped["samples"], 4);
        assert_eq!(clipped["median"], 1.0);
        assert_eq!(clipped["belowTarget"], 0);
        let _ = std::fs::remove_dir_all(directory);
    }
    #[test]
    fn compiler_work_allowances_are_contained_and_do_not_lower_saved_admission() {
        let mut captured = inputs();
        let mut defaults: Value =
            serde_json::from_str(captured["defaults"].as_str().unwrap()).unwrap();
        defaults["limits"]["max_authored_parts"] =
            json!(contract::maps::MapAdmission::CATALOGUE.max_authored_parts + 1);
        captured["defaults"] = json!(defaults.to_string());
        let over = run(json!({"operation":"validate","inputs":captured}));
        assert_eq!(over["status"], "invalid", "{}", over["diagnostics"]);
        assert!(over["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["location"] == "$.defaults.limits.max_authored_parts"));
        defaults["limits"]["max_authored_parts"] = json!(1);
        captured["defaults"] = json!(defaults.to_string());
        let lowered = run(json!({"operation":"validate","inputs":captured}));
        assert_eq!(lowered["status"], "valid", "{}", lowered["diagnostics"]);
        let fallback = sim::maps::load("market-town").unwrap();
        assert!(
            fallback.definition.props.len() > 1,
            "The released fallback admits more geometry than the edited generator work allowance"
        );
        assert!(!sim::maps::Catalogue::shipped()
            .encounter("market-town", "assault")
            .unwrap()
            .units
            .is_empty());
    }
    #[test]
    fn control_inventory_labels_percentage_ranges_and_relative_weights_truthfully() {
        let result = run(json!({"operation":"validate","inputs":inputs()}));
        assert_eq!(result["status"], "valid", "{}", result["diagnostics"]);
        let fields = result["fields"].as_array().unwrap();
        let field = |id: &str| fields.iter().find(|f| f["id"] == id).unwrap();
        assert_eq!(field("presets.types.open.forest_share.0")["unit"], "%");
        assert_eq!(
            field("presets.districts.apartments.mix.urban_apartment")["unit"],
            "weight"
        );
        assert!(field("presets.fairness.town.rel")["description"]
            .as_str()
            .unwrap()
            .contains("combined"));
        assert!(field("presets.joints.meet_m")["description"]
            .as_str()
            .unwrap()
            .contains("Ends"));
        assert_eq!(
            field("defaults.admission.max_generated_attempts")["editable"],
            false
        );
        assert_eq!(field("presets.terrain.height_grid_m")["editable"], false);
    }
    #[test]
    fn a_compile_refusal_keeps_the_requested_seed_inputs_and_stage() {
        let mut captured = inputs();
        let mut defaults: Value =
            serde_json::from_str(captured["defaults"].as_str().unwrap()).unwrap();
        defaults["limits"]["max_authored_parts"] = json!(1);
        captured["defaults"] = json!(defaults.to_string());
        let directory =
            std::env::temp_dir().join(format!("map-workbench-refused-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let request = json!({"operation":"generate","inputs":captured,"choice":{"type":"open","size":"small","seed":"1"},"artifactDir":directory});
        let result = run(request);
        assert_eq!(result["status"], "refused", "{}", result["diagnostics"]);
        assert_eq!(result["stage"], "compile");
        assert_eq!(result["choice"]["seed"], "1");
        let receipt: Value =
            serde_json::from_slice(&std::fs::read(directory.join("request.json")).unwrap())
                .unwrap();
        assert_eq!(receipt["inputs"], captured);
        assert_eq!(
            receipt["receipts"]["defaults"],
            contract::identity::bytes_hash(captured["defaults"].as_str().unwrap().as_bytes())
        );
        let unavailable = run(json!({"operation":"sight","artifactDir":directory}));
        assert_eq!(unavailable["status"], "unavailable");
        assert_eq!(unavailable["samples"], 0);
        let _ = std::fs::remove_dir_all(directory);
    }
    #[test]
    fn report_retains_the_exact_plan_and_inspects_without_regenerating() {
        let directory =
            std::env::temp_dir().join(format!("map-workbench-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let request = json!({"operation":"generate","inputs":inputs(),"choice":{"type":"open","size":"small","seed":"1"},"artifactDir":directory});
        let result = run(request);
        assert_eq!(result["status"], "ok", "{}", result["diagnostics"]);
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
