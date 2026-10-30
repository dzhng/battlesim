//! Every build of the simulation reads a JSON number as the float its text
//! names. serde_json's default reader misreads about one 17-digit number in
//! fifteen by one ULP; its `float_roundtrip` reader is exact. Cargo unifies a
//! crate's features per binary, so if the exact reader were enabled by one
//! crate rather than the workspace, the browser's simulation and native's
//! would read the same map apart, and their digests would part silently.
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;

/// Numbers that serde_json's default reader takes one ULP off: each is the
/// shortest text of its float, as a map or a browser writes it.
const MISREAD: [f64; 9] = [
    382.362_159_065_303_33,
    234.546_898_712_818_75,
    439.201_187_209_355_57,
    117.308_594_539_535_35,
    126.980_008_052_538_47,
    100.383_095_134_661_73,
    133.862_685_523_984_23,
    21.674_101_813_272_806,
    0.393_729_273_678_719_24,
];

fn scenario() -> serde_json::Value {
    let [a, b, c, d, e, f, g, h, yaw] = MISREAD;
    json!({
        "map": {
            "size": [800.0, 600.0], "fog_cell_m": 8.0, "height_grid_m": 4.0, "slope_cutoff_deg": 35.0,
            "props": [{ "kind": "bench", "center": [e, f], "yaw": yaw, "half_extents": [0.9, 0.3, 0.45] }],
        },
        "rules": sim::fixtures::test_rules(),
        "units": [
            { "side": "blue", "kind": "test_tank", "position": [a, b], "yaw": yaw, "engagement": "return_fire_only" },
            { "side": "red", "kind": "test_rifle", "position": [c, d], "engagement": "return_fire_only" },
        ],
        "scripts": [{ "tick": 1, "side": "blue", "order":
            { "kind": "move", "units": [0], "gesture": 1, "goal": [g, h], "route": "shortest" } }],
    })
}

#[test]
fn a_battle_read_from_text_holds_every_number_and_has_a_paired_browser_record() {
    let scenario = scenario();
    let text = scenario.to_string();
    let setup: ScenarioDefinition = serde_json::from_str(&text).unwrap();
    let written = serde_json::to_string(&setup).unwrap();
    for number in MISREAD {
        let shortest = format!("{number}");
        assert!(
            written.contains(&shortest),
            "{shortest} was read as another float"
        );
    }
    let mut battle = Battle::new(&setup, 1);
    let mut rows = Vec::new();
    for tick in [0, 1, 90] {
        while battle.tick() < tick {
            battle.step();
        }
        rows.push(json!({ "tick": tick, "digest": format!("{:016x}", battle.digest()) }));
    }
    let record = json!({ "scenario": scenario, "seed": 1, "rows": rows });
    if crate::common::bless_parity("exact-numbers.json", &record) {
        return;
    }
    assert_eq!(
        serde_json::to_string(&record).unwrap(),
        include_str!("../../../fixtures/parity/exact-numbers.json").trim()
    );
}
