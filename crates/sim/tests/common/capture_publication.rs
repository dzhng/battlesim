//! Re-record publication bytes from a capture's deterministic battle while
//! preserving its camera records and proving every recorded battle digest.
use serde_json::{json, Value};
use sim::{
    battle::Battle,
    fixtures::{self, CatalogSet},
    publication::{self, Publisher},
};

pub fn refresh(capture: &mut Value) {
    let workload = &capture["workload"];
    let mut rules = fixtures::rules();
    rules["catalog"] = json!(fixtures::catalog_documents(CatalogSet::Capture));
    let scenario = sim::maps::scenario(
        workload["map"].as_str().unwrap(),
        workload["encounter"].as_str().unwrap(),
        serde_json::from_value(rules).unwrap(),
    )
    .unwrap();
    let mut battle = Battle::new(&scenario, workload["seed"].as_u64().unwrap());
    let mut publisher = Publisher::new();
    capture["layout"] = json!(publication::layout_json(&battle));
    for sample in capture["samples"].as_array_mut().unwrap() {
        let tick = sample["tick"].as_u64().unwrap();
        while battle.tick() < tick {
            battle.step();
        }
        assert_eq!(
            sample["digest"],
            format!("{:016x}", battle.digest()),
            "capture digest at {tick}"
        );
        sample["publication"] = json!(publisher
            .publish(&battle, contract::ids::Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect::<Vec<_>>());
    }
}
