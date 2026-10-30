//! Who wades, as the effects read a side's publications: every unit the side
//! sees standing in water (its own, and an enemy it has in sight), a hull at
//! its centre or each soldier, and no squad riding a carrier, though the
//! carrier wades; and a round landing in the water splashes where one on the
//! bank raises dust. Real battles with swamps
//! laid over their ground, read as a page reads them, beside the ground's one
//! `StandingWater`.
use contract::ids::Side;
use contract::scenario::ScenarioDefinition;
use presentation::effects::publication::{EffectFeed, EffectPublication};
use presentation::fielded;
use presentation::seed::side_key;
use presentation::static_world::ground::biome::Biome;
use presentation::static_world::ground::{self, Ground, Tuning};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::reader::{Host, Observation, Reader};
use sim::publication::{self, Publisher};
use sim::world::WorldGeometry;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `scenario` with a swamp 0.6 m deep over each of `rects` (x0, y0, x1, y1).
fn flooded(scenario: &ScenarioDefinition, rects: &[[f64; 4]]) -> ScenarioDefinition {
    let mut v = serde_json::to_value(scenario).unwrap();
    v["map"]["swamps"] = rects
        .iter()
        .map(|&[x0, y0, x1, y1]| {
            json!({ "shape": { "kind": "polygon", "ring": [[x0, y0], [x1, y0], [x1, y1], [x0, y1]] },
                "depth_m": 0.6 })
        })
        .collect();
    serde_json::from_value(v).unwrap()
}

/// The presentation's ground of `scenario`'s map.
fn ground_of(scenario: &ScenarioDefinition) -> Ground {
    let rules = &scenario.rules;
    let world = WorldGeometry::new(&scenario.map, rules);
    let layout = sim::world::export::layout_json(&rules.catalog);
    let source = ground::source(&world, &scenario.map.extents(), &layout).unwrap();
    let summer = std::fs::read_to_string(root().join("fixtures/biomes/summer.json")).unwrap();
    let biome = Biome::load(&summer, None).unwrap();
    let tuning: Tuning = presentation::settings::PresentationSettings::from_value(
        &serde_json::from_slice(&std::fs::read(root().join("fixtures/presentation.json")).unwrap())
            .unwrap(),
    )
    .unwrap()
    .tuning();
    Ground::new(source, biome, tuning).unwrap()
}

/// Blue's observation and its effects' publication after each of `ticks`
/// ticks of `scenario`.
fn watch(
    scenario: &ScenarioDefinition,
    ticks: u64,
    mut each: impl FnMut(&Observation, &EffectPublication),
) {
    let ground = ground_of(scenario);
    let water = ground.standing_water();
    let kinds: Vec<String> = scenario.units.iter().map(|u| u.kind.clone()).collect();
    let f = fielded::fielded(&root(), &scenario.rules, &kinds).unwrap();
    let mut battle = Battle::new(scenario, 21);
    let mut reader = Reader::new(&publication::layout_json(&battle)).unwrap();
    let mut publisher = Publisher::new();
    let mut feed = EffectFeed::new("blue");
    for _ in 0..ticks {
        battle.step();
        let words: Vec<u32> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|w| w.to_bits())
            .collect();
        let o = reader.decode(&words).unwrap().unwrap();
        each(
            &o,
            &feed.publication(&o, &f.units, &f.effects, Some(&water)),
        );
    }
}

#[test]
fn a_squad_riding_in_an_apc_makes_no_wake_and_the_apc_does() {
    let rules = presentation::static_world::files::rules(&root(), "transport").unwrap();
    let board = sim::maps::scenario("transport", "board", rules).unwrap();
    // The squads board where they stand, in a marsh; the red squad waits
    // in another, far off.
    let scenario = flooded(
        &board,
        &[[150.0, 200.0, 280.0, 300.0], [470.0, 220.0, 530.0, 280.0]],
    );
    let (mut rode, mut seen) = (0, 0);
    watch(&scenario, 880, |o, p| {
        let wading = |id: f64| {
            let key = side_key(id, true);
            p.shooters
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.wading.len())
        };
        for u in &o.own {
            match u.hold.as_ref().map(|h| &h.host) {
                Some(Host::Carrier { id }) if &*u.hold.as_ref().unwrap().phase == "inside" => {
                    rode += 1;
                    assert_eq!(wading(u.id), Some(0), "rider {} at tick {}", u.id, o.tick);
                    assert_eq!(wading(*id), Some(1), "its carrier {id} wades");
                }
                Some(_) => {}
                None => assert!(
                    wading(u.id).unwrap() >= 1,
                    "unit {} stands in the marsh at tick {}",
                    u.id,
                    o.tick
                ),
            }
        }
        // The red squad stands in water too: seen, each of its men wades.
        for e in &o.identified {
            let key = side_key(e.id, false);
            let s = p.shooters.iter().find(|s| s.key == key).unwrap();
            assert_eq!(s.wading.len(), e.members.len(), "enemy {}", e.id);
            seen += 1;
        }
    });
    assert!(rode > 0, "the squads boarded");
    assert!(seen > 0, "the red squad was seen");
}

#[test]
fn a_round_landing_in_the_water_splashes_and_one_on_the_bank_raises_dust() {
    let scenario: ScenarioDefinition = serde_json::from_value(json!({
        "map": { "size": [200, 120], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
            "swamps": [{ "shape": { "kind": "polygon",
                "ring": [[130, 70], [190, 70], [190, 110], [130, 110]] }, "depth_m": 0.6 }] },
        "rules": sim::fixtures::test_game(),
        "units": [
            { "side": "blue", "kind": "test_rifle", "position": [60.0, 60.0] },
            { "side": "red", "kind": "test_rifle", "position": [150.0, 75.0] }
        ],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let water = ground_of(&scenario);
    let water = water.standing_water();
    let (mut splashed, mut dusty) = (0, 0);
    watch(&scenario, 30 * 30, |_, p| {
        for s in &p.segments {
            let [x, y, _] = s.path[s.path.len() - 1];
            let wet = water.at(x, y).is_some();
            match &*s.hit {
                "water" => {
                    assert!(wet, "a splash on dry ground at ({x}, {y})");
                    splashed += 1;
                }
                "ground" => {
                    assert!(!wet, "dust off the water at ({x}, {y})");
                    dusty += 1;
                }
                _ => {}
            }
        }
    });
    assert!(splashed > 0, "some rounds fell in the water");
    assert!(dusty > 0, "some fell on the bank");
}

#[test]
fn a_tank_on_a_bridge_over_the_river_does_not_wade() {
    let scenario: ScenarioDefinition = serde_json::from_value(json!({
        "map": serde_json::from_str::<Value>(
            &std::fs::read_to_string(root().join("fixtures/maps/river/map.json")).unwrap()
        ).unwrap(),
        "rules": sim::fixtures::test_game(),
        "units": [{ "side": "blue", "kind": "test_tank", "position": [60.0, 240.0],
            "engagement": "return_fire_only" }],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let water = ground_of(&scenario);
    assert!(
        water.standing_water().at(60.0, 240.0).is_some(),
        "the river runs under it"
    );
    let mut seen = 0;
    watch(&scenario, 60, |o, p| {
        let tank = &o.own[0];
        let s = p
            .shooters
            .iter()
            .find(|s| s.key == side_key(tank.id, true))
            .unwrap();
        assert!(s.wading.is_empty(), "on the deck at {:?}", tank.position);
        seen += 1;
    });
    assert!(seen > 0);
}

#[test]
fn a_soldier_crossing_a_puddle_makes_rings() {
    // The river lab with a metalled road across its north, where puddles lie.
    let mut map: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/maps/river/map.json")).unwrap(),
    )
    .unwrap();
    map["surfaces"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "road",
        "shape": { "kind": "stroke", "points": [[100, 440], [700, 440]], "width_m": 8 } }));
    let bare: ScenarioDefinition = serde_json::from_value(json!({
        "map": map, "rules": sim::fixtures::test_game(), "units": [], "events": [], "scripts": [],
    }))
    .unwrap();
    let ground = ground_of(&bare);
    let puddle = ground
        .puddles
        .iter()
        .filter(|p| (p.centre[1] - 440.0).abs() < 2.0 && p.radius_m > 0.8)
        .max_by(|a, b| a.radius_m.total_cmp(&b.radius_m))
        .expect("a puddle on the road");
    let [px, py] = puddle.centre;
    // A rifle squad walks along the road, through it.
    let mut v = serde_json::to_value(&bare).unwrap();
    v["units"] = json!([{ "side": "blue", "kind": "test_rifle", "position": [px - 25.0, py],
        "engagement": "return_fire_only" }]);
    v["scripts"] = json!([{ "tick": 1, "side": "blue", "order": { "kind": "move", "units": [0],
        "gesture": 1, "goal": [px + 25.0, py], "route": "shortest" } }]);
    let scenario: ScenarioDefinition = serde_json::from_value(v).unwrap();
    let f = fielded::fielded(&root(), &scenario.rules, &["test_rifle".to_owned()]).unwrap();
    let presentation =
        presentation::effects::tables::EffectPresentation::from_value(&f.effects.presentation)
            .unwrap();
    let mut frame = presentation::effects::frame::EffectFrame::new(30.0, presentation);
    let (mut waded, mut made) = (0, vec![]);
    watch(&scenario, 30 * 30, |o, p| {
        waded += p.shooters.iter().map(|s| s.wading.len()).sum::<usize>();
        frame.note(p);
        let t = o.tick / 30.0;
        made.extend(frame.wakes(t - 1.0 / 30.0, t).copied());
    });
    assert!(waded > 0, "the squad waded through the puddle");
    let near = |w: &&presentation::effects::frame::WakeImpulse| {
        (w.xy[0] - px).hypot(w.xy[1] - py) <= puddle.radius_m + 0.5
    };
    let there: Vec<_> = made.iter().filter(near).collect();
    assert!(!there.is_empty(), "a wake in the puddle");
    assert!(
        there
            .iter()
            .all(|w| w.kind == presentation::effects::frame::WakeKind::Rings),
        "rings, never a V, in a puddle: {there:?}"
    );
}
