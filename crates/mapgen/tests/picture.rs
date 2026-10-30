//! The lobby's map picture: an admitted map and its sites, drawn top-down
//! with Blue's edge on the left (D23, D39).
use contract::encounter::SkirmishSites;
use contract::ids::Side;
use contract::map::MapDefinition;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const PHYSICAL_RULES: &str =
    include_str!("../../../fixtures/parity/map-layout/physical-rules.json");

/// Three Blue players and four Red ones: three entries a side (D52).
fn players() -> Vec<contract::players::Player> {
    let seat = |side: &str, faction: &str, n: usize| {
        serde_json::json!({"id": format!("{side}-{n}"), "side": side,
            "faction": faction, "controller": "active_ai"})
    };
    let mut seats: Vec<_> = (1..=3).map(|n| seat("blue", "us", n)).collect();
    seats.extend((1..=4).map(|n| seat("red", "eastern", n)));
    serde_json::from_value(serde_json::Value::Array(seats)).unwrap()
}

/// A Large mixed skirmish map with three entries a side, and the sites the
/// simulation admitted on it.
fn admitted_map() -> (MapDefinition, SkirmishSites) {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let presets = mapgen::layout::PresetDefinitions::from_json(PRESETS).unwrap();
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    for seed in 1..20u64 {
        let request = serde_json::json!({
            "generator_version": mapgen::layout::GENERATOR_VERSION,
            "preset_revision": presets.revision, "seed": seed.to_string(),
            "template_catalog_hash": catalogue.hash(), "type": "mixed", "size": "large",
            "profile": "skirmish", "entries_per_side": 3,
            "limits": { "max_authored_parts": 1000000,
                "max_bay_positions": 100000000, "max_ground_points": 10000000 }
        });
        let mapgen::CompileOutcome::Ok { result } =
            mapgen::generate_map(&request.to_string(), PRESETS, TEMPLATES, PHYSICAL_RULES)
        else {
            continue;
        };
        let prepared = sim::encounter::PreparedMap::new(&result.map, &rules);
        let Ok(admission) = sim::map_analysis::admit_skirmish(
            &prepared.queries(&result.map, &result.sites),
            &rules,
            &players(),
        ) else {
            continue;
        };
        return (result.map.clone(), admission.sites.skirmish.unwrap());
    }
    panic!("no Large three-entry map was admitted in 19 seeds");
}

/// Every mark of `kind` (`entry` or `objective`) the picture drew: its
/// `data-<kind>` value and where it stands in the picture.
fn marks(svg: &str, kind: &str) -> Vec<(String, [f64; 2])> {
    let key = format!("data-{kind}=\"");
    svg.match_indices(&key)
        .map(|(at, _)| {
            let rest = &svg[at + key.len()..];
            let value = &rest[..rest.find('"').unwrap()];
            let translate = &rest[rest.find("translate(").unwrap() + "translate(".len()..];
            let numbers: Vec<f64> = translate[..translate.find(')').unwrap()]
                .split([' ', ','])
                .filter(|n| !n.is_empty())
                .map(|n| n.parse().unwrap())
                .collect();
            (value.to_string(), [numbers[0], numbers[1]])
        })
        .collect()
}

#[test]
fn the_picture_draws_every_admitted_entry_and_objective_with_blue_on_the_left() {
    let (map, sites) = admitted_map();
    let svg = mapgen::inspect::skirmish_picture(&map, &sites);
    let [width, height] = map.size;
    // Turned a quarter: the map's north edge (Blue's) is the picture's left.
    let at = |p: [f64; 2]| [height - p[1], width - p[0]];
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1.0 && (a[1] - b[1]).abs() < 1.0;

    let entries = marks(&svg, "entry");
    assert_eq!(entries.len(), sites.entries.len());
    for entry in &sites.entries {
        let side = if entry.side == Side::Blue {
            "blue"
        } else {
            "red"
        };
        assert!(
            entries
                .iter()
                .any(|(drawn, p)| drawn == side && near(*p, at(entry.center))),
            "{side} entry at {:?} is not drawn",
            entry.center
        );
        let left = at(entry.center)[0] < height / 2.0;
        assert_eq!(
            left,
            entry.side == Side::Blue,
            "{side} entry on the wrong edge"
        );
    }

    let objectives = marks(&svg, "objective");
    assert_eq!(objectives.len(), sites.objectives.len());
    for objective in &sites.objectives {
        assert!(
            objectives
                .iter()
                .any(|(id, p)| *id == objective.id && near(*p, at(objective.center))),
            "objective {} is not drawn",
            objective.id
        );
    }
    // Reserved alternatives were not chosen; the player never meets them.
    for candidate in &sites.candidates {
        assert!(!objectives.iter().any(|(id, _)| *id == candidate.id));
    }
}
