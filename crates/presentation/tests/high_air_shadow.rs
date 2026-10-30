//! A high-air aircraft casts no sun shadow: from its height the sun would
//! throw it hundreds of metres off, where nobody ties it to its aircraft.
//! Instead a soft silhouette of it lies on the ground straight beneath it,
//! turned with its heading. A low-air aircraft keeps its sun-cast shadow and
//! shows none beneath it. A high-air model is drawn in its own pass, above
//! the world's cards where a client has them. The rule follows the unit's
//! altitude layer, never what kind of aircraft it is.
use contract::ids::Side;
use presentation::effects::frame::SHAPE_NAMES;
use presentation::effects::high_air_shadow::{high_air_ground_shadow, HighAirShadowStyle};
use presentation::fielded;
use presentation::presentation::{Presentation, Ui};
use presentation::records::{HIGH_PASS, NONE, VIEW_PASS};
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, WorldInput, Xray};
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::{self, Publisher};
use std::f64::consts::FRAC_PI_2;
use std::path::PathBuf;

/// `test_jet` flies in the high-air layer, `test_heli` in the low-air one.
const KINDS: [&str; 2] = ["test_jet", "test_heli"];
const JET_AT: [f64; 2] = [200.0, 200.0];
const HELI_AT: [f64; 2] = [400.0, 200.0];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Blue's presentation of a jet and a helicopter over open ground, framed
/// from high above both.
fn framed() -> Presentation {
    let root = root();
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let setup = serde_json::from_value(json!({
        "map": { "size": [600, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": sim::fixtures::test_game(),
        "units": [
            { "side": "blue", "kind": KINDS[0], "position": JET_AT, "yaw": 0.6 },
            { "side": "blue", "kind": KINDS[1], "position": HELI_AT },
        ],
        "events": [],
        "scripts": [],
    }))
    .unwrap();
    let battle = Battle::new(&setup, 1);
    let kinds: Vec<String> = KINDS.map(String::from).to_vec();
    let f = fielded::fielded(&root, battle.rules(), &kinds).unwrap();
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
                own: [0.0; 4],
                selected: [0.0; 4],
                ally: [0.0; 4],
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
    let mut publisher = Publisher::new();
    let words: Vec<u32> = publisher
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|f| f.to_bits())
        .collect();
    assert!(p.push_publication(&words, 0.0).unwrap());
    let camera = Camera {
        eye: [300.0, -300.0, 900.0],
        target: [300.0, 200.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 1.0,
        size: [1920, 1080],
    };
    p.frame(0.0, &camera, &Ui::default(), 0).unwrap();
    p
}

/// Each drawn model's placement (x, y, z, yaw), the tier it casts with and
/// the pass that draws it.
fn models(p: &Presentation) -> Vec<([f32; 4], u32, u32)> {
    let f = p.last_frame();
    f.batches
        .iter()
        .flat_map(|b| {
            let run = &f.instances[b.first as usize..(b.first + b.count) as usize];
            run.iter().map(move |i| (i.placement, b.caster, b.pass))
        })
        .collect()
}

fn shape(name: &str) -> f32 {
    SHAPE_NAMES
        .iter()
        .position(|s| *s == name)
        .unwrap_or_else(|| panic!("no {name} effect shape")) as f32
}

#[test]
fn a_high_air_unit_lies_in_shadow_on_the_ground_beneath_it_and_casts_none() {
    let p = framed();
    let models = models(&p);
    let near = |at: [f64; 2]| {
        let found = models
            .iter()
            .find(|(m, ..)| (m[0] as f64 - at[0]).abs() < 1.0 && (m[1] as f64 - at[1]).abs() < 1.0);
        *found.unwrap_or_else(|| panic!("a model drawn at {at:?}"))
    };
    let (jet, jet_caster, jet_pass) = near(JET_AT);
    let (heli, heli_caster, heli_pass) = near(HELI_AT);
    assert_eq!(
        jet_pass, HIGH_PASS,
        "the high-air jet draws in the high pass"
    );
    assert_eq!(
        heli_pass, VIEW_PASS,
        "the low-air helicopter draws with the world"
    );
    assert!(jet[2] > heli[2], "the jet flies above the helicopter");
    assert_eq!(jet_caster, NONE, "the high-air jet casts no sun shadow");
    assert_ne!(
        heli_caster, NONE,
        "the low-air helicopter casts its sun shadow"
    );

    let shadows: Vec<_> = p
        .effects()
        .iter()
        .filter(|e| e.misc[0] == shape("shadow"))
        .collect();
    assert_eq!(
        shadows.len(),
        1,
        "one ground shadow: the jet's, none for the helicopter"
    );
    let s = shadows[0];
    // On the ground (the map is flat at zero) straight beneath the jet, its
    // long axis along the jet's heading.
    for (got, want) in s.a[..3].iter().zip([jet[0], jet[1], 0.0]) {
        assert!((got - want).abs() < 0.5, "centre {:?} beneath {jet:?}", s.a);
    }
    let (sin, cos) = jet[3].sin_cos();
    assert!(
        (s.colour[0] - cos).abs() < 1e-4 && (s.colour[1] - sin).abs() < 1e-4,
        "turned with its heading {}: {:?}",
        jet[3],
        s.colour
    );
}

/// A fake style: the fixture's tuning is not the subject.
const STYLE: HighAirShadowStyle = HighAirShadowStyle {
    opacity: 0.6,
    fade_half_m: 300.0,
    blur_m: 0.5,
    blur_per_m: 0.02,
    ground_band_m: 0.6,
};
/// A 16 m long, 10 m span plan, its middle a metre behind its origin.
const PLAN: [f64; 4] = [-9.0, -5.0, 7.0, 5.0];

#[test]
fn the_higher_it_flies_the_fainter_and_softer_its_ground_shadow() {
    let flat = |_: f64, _: f64| 12.0;
    let at = |h: f64| high_air_ground_shadow([50.0, 80.0, 12.0 + h], 0.0, PLAN, flat, &STYLE);
    let (low, high) = (at(100.0), at(400.0));
    assert!(
        high.opacity < low.opacity,
        "{} then {}",
        low.opacity,
        high.opacity
    );
    assert!(
        high.blur_m > low.blur_m,
        "{} then {}",
        low.blur_m,
        high.blur_m
    );
    // Its size is its plan's, whatever its height.
    assert_eq!(low.half, [8.0, 5.0]);
    assert_eq!(high.half, low.half);
}

#[test]
fn its_ground_shadow_lies_beneath_its_plan_turned_with_its_heading_on_the_ground_s_slope() {
    // Ground rising 1 m in 10 to the north.
    let slope = |_: f64, y: f64| 0.1 * y;
    let s = high_air_ground_shadow([50.0, 80.0, 300.0], FRAC_PI_2, PLAN, slope, &STYLE);
    // Heading north, its plan's middle a metre behind it is a metre south.
    let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
    assert!(near(s.centre, [50.0, 79.0, 7.9]), "centre {:?}", s.centre);
    // Its nose points north, up the slope, along the ground.
    let rise = 0.1 / (1.0f64 + 0.01).sqrt();
    assert!(
        near(s.along, [0.0, 1.0 / 1.01f64.sqrt(), rise]),
        "along {:?}",
        s.along
    );
    let dot: f64 = s.along.iter().zip(s.normal).map(|(a, n)| a * n).sum();
    assert!(
        dot.abs() < 1e-9 && s.normal[2] > 0.99,
        "normal {:?}",
        s.normal
    );
}
