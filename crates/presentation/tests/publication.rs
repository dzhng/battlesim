use presentation::presentation::Presentation;
use serde_json::Value;
use sim::battle::Battle;
use sim::publication::{self, Publisher};

fn words(record: &[f32]) -> Vec<u32> {
    record.iter().map(|word| word.to_bits()).collect()
}

#[test]
fn presentation_decodes_a_two_by_two_skirmish_publication() {
    let record: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parity/skirmish-purchases.json"
    ))
    .unwrap();
    let mut scenario = record["scenario"].clone();
    let players = scenario["players"].as_array_mut().unwrap();
    let mut blue = players[0].clone();
    blue["id"] = "blue-2".into();
    let mut red = players[1].clone();
    red["id"] = "red-2".into();
    players.extend([blue, red]);
    let entries = scenario["skirmish"]["sites"]["entries"]
        .as_array_mut()
        .unwrap();
    // A second entry per side (D52), 200 m along the edge: entries stand at
    // least 150 m apart.
    for side in 0..2 {
        let mut second = entries[side].clone();
        second["center"][0] = (second["center"][0].as_f64().unwrap() + 200.0).into();
        entries.push(second);
    }
    let scenario: contract::scenario::ScenarioDefinition =
        serde_json::from_value(scenario).unwrap();
    let battle = Battle::new(&scenario, 1);
    let layout = publication::layout_json(&battle);
    let mut publisher = Publisher::new();
    let record = words(
        publisher
            .publish(&battle, contract::ids::Side::Blue)
            .unwrap(),
    );
    let mut presentation = Presentation::new(Some(&layout), battle.rules().tick_hz as f64).unwrap();
    assert!(presentation.push_publication(&record, 0.0).unwrap());
    assert_eq!(
        presentation
            .observation()
            .unwrap()
            .skirmish
            .as_ref()
            .unwrap()
            .players
            .len(),
        2
    );
}
