//! Control metadata follows the owning schemas; the generator remains the validity authority.
use serde_json::{json, Value};

const PRESETS: &str = include_str!("../../src/layout/presets.rs");
const COUNTRY: &str = include_str!("../../src/open_country/rules.rs");

fn schema_help(schema: &str, owner: &str, key: &str) -> Option<String> {
    let start = schema.find(&format!("pub struct {owner} {{"))?;
    let mut type_docs = Vec::new();
    for line in schema[..start].lines().rev() {
        let line = line.trim();
        if let Some(doc) = line.strip_prefix("///") {
            type_docs.push(doc.trim().to_string());
        } else if !line.starts_with("#[") && !line.is_empty() {
            break;
        }
    }
    type_docs.reverse();
    let mut docs = Vec::new();
    for line in schema[start..].lines().skip(1) {
        let line = line.trim();
        if line == "}" {
            break;
        }
        if let Some(doc) = line.strip_prefix("///") {
            docs.push(doc.trim().to_string());
        } else if line.starts_with(&format!("pub {key}:")) {
            return Some(if docs.is_empty() {
                type_docs.join(" ")
            } else {
                docs.join(" ")
            });
        } else if line.starts_with("pub ") {
            docs.clear();
        }
    }
    None
}
fn help(document: &str, path: &[String], key: &str, group: &str) -> String {
    let parts: Vec<_> = path.iter().map(String::as_str).collect();
    let head = parts.first().copied().unwrap_or("");
    if document == "defaults" {
        return match (head,key){
            ("analysis","step_m")=>"Spacing of the square grid used to sample eligible open ground. Every sample uses the simulation's infantry sight rays; changing spacing changes the report's sample, not map generation.",
            ("analysis","min_median_open")=>"Minimum median share of bearings that see to infantry range for the sampled openness report. This is an assessment target; a report below it does not reject map generation or prohibit saving valid settings.",
            ("limits","max_authored_parts")=>"Maximum authored bodies and materialized building parts admitted by the compiler. Procedural forest trunks have their own physical owner. Lowering this allowance can refuse a generated seed and never lowers released saved-map admission.",
            ("limits","max_bay_positions")=>"Maximum materialized facade bay positions admitted by the compiler. Lowering it may refuse a generated seed; released catalogue admission stays fixed.",
            ("limits","max_ground_points")=>"Maximum polygon vertices and rounded stroke samples across surfaces, forests and rivers. Layout, parcel and compiler checks name refusals when their generated geometry exceeds this work allowance.",
            ("admission",_)=>"Read-only ordinary Play recovery policy. Explicit seed inspection and replay remain exact.",
            ("camera",_)=>"Read-only initial battle camera framing; it changes no generated geometry.",
            _=>"Read-only battle defaults.",
        }.into();
    }
    if head == "revision" {
        return "Tool-managed preset revision. Exact input receipts and physical map identity are recorded separately.".into();
    }
    if head == "terrain" {
        return "Read-only physical map header. Grid, ground slope and rendering margin keep their existing owners.".into();
    }
    if head == "fairness" || (head == "open_country" && parts.get(1) == Some(&"fairness")) {
        return match key{
            "rel"=>"Relative permitted imbalance between top and bottom halves, scaled by their combined measured amount. The permitted difference is the larger of this relative term and the absolute allowance.",
            "abs"=>"Absolute permitted imbalance as a share of the whole playable map area, or its side length for river length. It combines with the relative term by taking the larger allowance.",
            _=>"Absolute permitted top/bottom difference in this furnishing kind's measured count or metres. The larger of this value and the relative allowance admits the split.",
        }.into();
    }
    if head == "approach" {
        return match key{
            "depth_m"=>"Required open ground beyond the main settlement's own edge in each map half. Finished geometry must demonstrate a corridor this deep.",
            "front_m"=>"Required width of the open approach corridor in each map half. Finished geometry must demonstrate this front across the required depth.",
            "reserve_front_m"=>"Width kept clear during construction to leave the required measured approach front. This steers placement; front_m is the final admission requirement.",
            "reserve_margin_m"=>"Extra depth kept clear beyond the required approach depth during construction, leaving room for later placement and measurement.",
            "bearing_candidates"=>"Finite number of approach bearings tried in each half. Exhausting the search refuses the requested seed; it never switches seeds.",
            _=>"Open approach policy measured from the finished plan.",
        }.into();
    }
    if head == "transit" {
        return match key{
            "road_kmh"=>"Road speed assumed by the generator's journey measurement. It assesses road layout; it does not change a battle unit's movement rules.",
            "allowance_s"=>"Time added to measured road driving for planning, turns and slower stretches before judging the transit limit.",
            "max_s"=>"Maximum measured journey time from each deployment edge to the central road junction. A finished plan exceeding it is refused.",
            "centre_reach_m"=>"Maximum distance between the central road junction and the map midpoint admitted by the finished-plan measurement.",
            "exit_window"=>"Centered share of the top and bottom edges within which the main roads leave the playable map.",
            _=>"Transit admission policy assessed on the rounded road network.",
        }.into();
    }
    if key == "urban_share_max" {
        return "Ceiling on settlement area as a share of the playable map for this type and size. Finished-plan measurement refuses a plan above it.".into();
    }
    if key == "share_tolerance" {
        return "Additional allowance on either side of the requested forest-share range when admitting finished geometry.".into();
    }
    let (schema, owner) = match head {
        "joints" => (PRESETS, "JointPolicy"),
        "roads" => (PRESETS, "Roads"),
        "sites" => (PRESETS, "Sites"),
        "forests" => (PRESETS, "Forests"),
        "towns" => (
            PRESETS,
            if parts.get(1) == Some(&"geometry") {
                "TownGeometry"
            } else {
                "Towns"
            },
        ),
        "parcels" => (
            PRESETS,
            if parts.get(1) == Some(&"geometry") {
                "StreetGeometry"
            } else {
                "Parcels"
            },
        ),
        "rivers" => (
            PRESETS,
            if parts.contains(&"bridge") {
                "BridgeRule"
            } else if parts.contains(&"meander") {
                "Meander"
            } else {
                "Rivers"
            },
        ),
        "crossings" => (PRESETS, "Crossings"),
        "retries" => (PRESETS, "Retries"),
        "street_props" => (
            PRESETS,
            if parts.contains(&"yards") {
                "Yards"
            } else if parts.contains(&"courts") && parts.contains(&"parking") {
                "CourtParking"
            } else if parts.contains(&"lawn") {
                "Lawn"
            } else if parts.contains(&"split") {
                "CourtSplit"
            } else if parts.contains(&"courts") {
                "CourtRule"
            } else if parts.contains(&"parking") {
                "Parking"
            } else if parts.contains(&"site") {
                "ConstructionSite"
            } else if parts.contains(&"bodies") {
                "PropBox"
            } else if parts.contains(&"pieces") {
                "GroupPiece"
            } else if parts.contains(&"groups") {
                "Group"
            } else {
                "StreetProps"
            },
        ),
        "districts" => (
            PRESETS,
            if parts.contains(&"bend") {
                "Bend"
            } else if parts.contains(&"streets") {
                "StreetPattern"
            } else if parts.contains(&"lots") {
                "LotRule"
            } else if parts.contains(&"verge") {
                "VergeRow"
            } else if parts.contains(&"gardens") {
                "Gardens"
            } else if parts.contains(&"courts") {
                "Courts"
            } else if parts.contains(&"props") {
                "DistrictProps"
            } else {
                "DistrictPreset"
            },
        ),
        "classes" => (
            PRESETS,
            if parts.contains(&"outline") {
                "OutlineShape"
            } else if parts.contains(&"block") {
                "BlockRule"
            } else if parts.contains(&"side_roads") {
                "SideRoads"
            } else if parts.contains(&"zones") {
                "Zone"
            } else {
                "SettlementClass"
            },
        ),
        "types" => (
            PRESETS,
            if parts.contains(&"sizes") {
                "SizePreset"
            } else if parts.contains(&"siting") {
                "Siting"
            } else {
                "TypePreset"
            },
        ),
        "open_country" => (
            COUNTRY,
            match parts.get(1).copied() {
                Some("clear") => "Clearances",
                Some("sight") => "SightRule",
                Some("homesteads") => {
                    if parts.contains(&"yard") {
                        "Yard"
                    } else if parts.contains(&"groups") {
                        "Group"
                    } else {
                        "Homesteads"
                    }
                }
                Some("tree_lines") => "TreeLines",
                Some("copses") => "Copses",
                Some("lone_trees") => "LoneTrees",
                Some("field_cover") => "FieldCover",
                _ => "Rules",
            },
        ),
        _ => (PRESETS, "PresetDefinitions"),
    };
    let described = schema_help(schema, owner, key)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| {
            format!(
                "{} controls {} construction wherever this rule group applies.",
                key.replace('_', " "),
                group
            )
        });
    described
        .replace("[`carry`]", "the carry pass")
        .replace('`', "")
}

pub fn describe(presets: &str, defaults: &str) -> Vec<Value> {
    fn visit(value: &Value, path: &mut Vec<String>, document: &str, out: &mut Vec<Value>) {
        match value {
            Value::Object(rows) => {
                for (key, value) in rows {
                    path.push(key.clone());
                    visit(value, path, document, out);
                    path.pop();
                }
            }
            Value::Array(rows) => {
                for (index, value) in rows.iter().enumerate() {
                    path.push(index.to_string());
                    visit(value, path, document, out);
                    path.pop();
                }
            }
            Value::Null => {}
            _ => {
                let head = path.first().map(String::as_str).unwrap_or("");
                let key = path.last().map(String::as_str).unwrap_or("");
                let numeric = value.is_number() || value.is_boolean();
                let role = if document == "defaults" {
                    match head {
                        "limits" => "work",
                        "analysis" => "analysis",
                        _ => "identity",
                    }
                } else if head == "revision" {
                    "identity"
                } else if head == "terrain" {
                    "physical"
                } else if head == "retries"
                    || key.contains("attempt")
                    || key.contains("candidates")
                    || key.ends_with("tries")
                    || key.ends_with("rounds")
                    || key == "placed_max"
                    || key == "walk_m"
                    || key == "owner_step_m"
                    || key == "places"
                {
                    "work"
                } else if head == "fairness"
                    || head == "transit"
                    || head == "approach"
                    || key == "urban_share_max"
                    || key == "share_tolerance"
                    || (head == "open_country" && path.get(1).is_some_and(|p| p == "fairness"))
                {
                    "validation"
                } else {
                    "construction"
                };
                let editable = numeric
                    && if document == "defaults" {
                        head == "limits" || head == "analysis"
                    } else {
                        head != "revision" && head != "terrain"
                    };
                let group = if document == "presets"
                    && (matches!(head, "districts" | "classes" | "types")
                        || (head == "open_country" && path.len() > 2))
                {
                    path.iter().take(2).cloned().collect::<Vec<_>>().join(".")
                } else {
                    head.to_string()
                };
                let id = format!("{document}.{}", path.join("."));
                let weights = path
                    .iter()
                    .any(|part| part == "mix" || part == "weight" || part == "weights")
                    || (head == "classes"
                        && path.iter().any(|p| p == "zones")
                        && path.iter().any(|p| p == "districts" || p == "roadside"));
                let semantic_key = if key.parse::<usize>().is_ok() {
                    path.iter()
                        .rev()
                        .find(|part| part.parse::<usize>().is_err())
                        .map(String::as_str)
                        .unwrap_or(key)
                } else {
                    key
                };
                let percent = semantic_key.contains("chance")
                    || semantic_key.contains("share")
                    || semantic_key == "min_median_open"
                    || matches!(
                        semantic_key,
                        "cross_skip"
                            | "coverage"
                            | "open_blocks"
                            | "growth_noise"
                            | "bend_amplitude"
                    )
                    || (path.iter().any(|p| p == "siting"))
                    || (path.iter().any(|p| p == "fairness")
                        && matches!(semantic_key, "rel" | "abs"));
                let unit = if weights {
                    "weight"
                } else if percent {
                    "%"
                } else if semantic_key.ends_with("_m2") {
                    "m²"
                } else if semantic_key.ends_with("_m") {
                    "m"
                } else if semantic_key.ends_with("_s") {
                    "s"
                } else if semantic_key.ends_with("_kmh") {
                    "km/h"
                } else if semantic_key.ends_with("_cos") {
                    "cosine"
                } else if semantic_key.ends_with("_sin") {
                    "sine"
                } else if semantic_key.ends_with("_tan") {
                    "tangent ratio"
                } else if semantic_key.ends_with("_deg") {
                    "degrees"
                } else {
                    "stored value"
                };
                let label = if semantic_key == "half_extents_m" {
                    format!(
                        "half {}",
                        match key {
                            "0" => "length",
                            "1" => "width",
                            _ => "height",
                        }
                    )
                } else if semantic_key == "hub_offset_m" {
                    format!(
                        "hub {} offset",
                        if key == "0" {
                            "east/west"
                        } else {
                            "north/south"
                        }
                    )
                } else if semantic_key == "ground_m" {
                    format!(
                        "minimum parcel {}",
                        if key == "0" { "frontage" } else { "depth" }
                    )
                } else if weights {
                    format!("{} relative weight", semantic_key.replace('_', " "))
                } else if key == "0" || key == "1" {
                    format!(
                        "{} {}",
                        semantic_key.replace('_', " "),
                        if key == "0" { "minimum" } else { "maximum" }
                    )
                } else {
                    key.replace('_', " ")
                };

                out.push(json!({"id":id,"document":document,"path":path,"group":group,"label":label,"description":help(document,path,if weights && head=="classes" {if path.iter().any(|p|p=="roadside"){"roadside"}else{"districts"}}else if weights {"mix"}else{semantic_key},&group),"unit":unit,"role":role,"editable":editable}));
            }
        }
    }
    let mut out = Vec::new();
    for (document, text) in [("presets", presets), ("defaults", defaults)] {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            visit(&value, &mut Vec::new(), document, &mut out);
        }
    }
    out
}
