//! A side's unit another player owns is its ally's: hidden by the world, it
//! shows through in the ally's colour, while the player's own show in the
//! side's. Who is an ally comes from each unit's published owner and the
//! player the UI commands as; a battle without owners has no allies.
use contract::ids::Side;
use presentation::fielded;
use presentation::presentation::{Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::{self, Publisher};
use std::path::PathBuf;

const KIND: &str = "test_tank";
const MINE_AT: [f64; 2] = [300.0, 200.0];
const ALLY_AT: [f64; 2] = [340.0, 200.0];
const OWN: [f64; 4] = [0.2, 0.4, 0.6, 0.5];
const ALLY: [f64; 4] = [0.8, 0.6, 0.4, 0.5];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Blue's presentation of two tanks, one blue-1's and one blue-2's.
fn presented() -> Presentation {
    let root = root();
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let mut rules = sim::fixtures::test_rules();
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        KIND,
        json!({"roster": {"factions":["us","eastern"],"category":"veh","family_name":"Test tank","variant":"Test"}}),
    );
    let setup = serde_json::from_value(json!({
        "map": { "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": rules,
        "units": [
            { "side": "blue", "kind": KIND, "position": MINE_AT, "owner": "blue-1" },
            { "side": "blue", "kind": KIND, "position": ALLY_AT, "owner": "blue-2" },
        ],
        "players": [
            {"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},
            {"id":"blue-2","side":"blue","faction":"us","controller":"active_ai","human_capable":false},
            {"id":"red-1","side":"red","faction":"eastern","controller":"passive_ai"}
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let battle = Battle::new(&setup, 1);
    let f = fielded::fielded(&root, battle.rules(), &[KIND.to_owned()]).unwrap();
    let physics = &game["physics"];
    let world = WorldInput {
        battle: Some(BattleInput {
            units: f.units,
            effects: f.effects,
            side: "blue".into(),
            tick_hz: game["tick_hz"].as_f64().unwrap(),
            soldier: SoldierBody {
                radius_m: physics["soldier_radius_m"].as_f64().unwrap(),
                height_m: physics["soldier_height_m"].as_f64().unwrap(),
            },
            xray: Xray {
                own: OWN,
                selected: [1.0, 0.6, 0.1, 0.6],
                ally: ALLY,
            },
            blocked: [0.0; 4],
        }),
        sun_azimuth: 0.0,
        sun_elevation: 0.7,
        buildings: None,
    };
    let mut p = Presentation::new(
        Some(&publication::layout_json(&battle)),
        battle.rules().tick_hz as f64,
    )
    .unwrap();
    p.load_world(world, Default::default(), None).unwrap();
    let words: Vec<u32> = Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|f| f.to_bits())
        .collect();
    assert!(p.push_publication(&words, 0.0).unwrap());
    p
}

/// The x-ray colour the frame packs for the model drawn at `at`, as rgb
/// bytes.
fn xray_at(p: &mut Presentation, ui: &Ui, at: [f64; 2]) -> [u32; 3] {
    let camera = Camera {
        eye: [320.0, -100.0, 300.0],
        target: [320.0, 200.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [1920, 1080],
    };
    let f = p.frame(0.0, &camera, ui, 0).unwrap();
    let i = f
        .instances
        .iter()
        .find(|i| {
            (i.placement[0] as f64 - at[0]).abs() < 1.0
                && (i.placement[1] as f64 - at[1]).abs() < 1.0
        })
        .unwrap_or_else(|| panic!("a model drawn at {at:?}"));
    let packed = i.data[3] as u32;
    [packed >> 16, (packed >> 8) & 255, packed & 255]
}

fn bytes(c: [f64; 4]) -> [u32; 3] {
    [c[0], c[1], c[2]].map(|v| (v * 255.0).round() as u32)
}

#[test]
fn an_allys_unit_shows_through_the_world_in_the_allys_colour() {
    let mut p = presented();
    let ui = Ui {
        xray: true,
        player: Some(0),
        ..Default::default()
    };
    assert_eq!(xray_at(&mut p, &ui, MINE_AT), bytes(OWN), "your own tank");
    assert_eq!(
        xray_at(&mut p, &ui, ALLY_AT),
        bytes(ALLY),
        "your ally's tank"
    );
    // Selecting an ally's unit (to read its panel) leaves it the ally's.
    let selected = Ui {
        selected: &[1],
        ..ui
    };
    assert_eq!(xray_at(&mut p, &selected, ALLY_AT), bytes(ALLY));
    // As the side's ally sees it, the colours swap.
    let ally = Ui {
        xray: true,
        player: Some(1),
        ..Default::default()
    };
    assert_eq!(xray_at(&mut p, &ally, MINE_AT), bytes(ALLY));
    assert_eq!(xray_at(&mut p, &ally, ALLY_AT), bytes(OWN));
}
