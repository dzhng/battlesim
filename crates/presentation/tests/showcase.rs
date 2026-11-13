//! A unit at rest on a stage of its own: posed by the battle's driver from a
//! unit standing still, with no publication behind it.
use std::path::PathBuf;

use presentation::assets::Runtime;
use presentation::effects::frame::SHAPE_NAMES;
use presentation::fielded::{Fielded, Fielding};
use presentation::presentation::lab::Asset;
use presentation::presentation::showcase::ShowcaseInput;
use presentation::presentation::{Presentation, Ui};
use presentation::records::{Counts, Instance, Joint};
use presentation::view::Camera;
use presentation::world::{Appearance, BattleInput, Detail, SoldierBody, WorldInput, Xray};
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The test units `kinds` as a battle fielding them resolves them, each side
/// for `factions`, with `edit` applied to the runtime catalog first.
fn fielded(kinds: &[&str], factions: Option<[&str; 2]>, edit: impl FnOnce(&mut Value)) -> Fielded {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_rules()).unwrap();
    let catalog = rules.catalog.view();
    let rules = serde_json::to_value(&rules).unwrap();
    let runtime = Runtime::open(&root().join("assets/runtime")).unwrap();
    let mut entries = runtime.catalog.clone();
    edit(&mut entries);
    let presentation: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/presentation.json")).unwrap(),
    )
    .unwrap();
    let kinds: Vec<String> = kinds.iter().map(|k| (*k).to_owned()).collect();
    Fielding {
        presentation: &presentation,
        rules: &rules,
        catalog: &catalog,
        runtime: &entries,
        kinds: &kinds,
        factions,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap()
}

fn plain(kind: &str) -> Fielded {
    fielded(&[kind], None, |_| {})
}

/// A hull standing at `at`, facing +X.
fn hull(kind: &str, at: [f64; 3]) -> ShowcaseInput {
    serde_json::from_value(json!({
        "kind": kind, "side": "blue", "at": at, "yaw": 0.0, "soldiers": [], "operators": []
    }))
    .unwrap()
}

/// `kind`'s squad of `slots` soldiers a metre and a half apart about the
/// origin, ids from 1, mount `operated` (if any) worked by soldier 1.
fn squad(
    kind: &str,
    side: &str,
    slots: usize,
    mounts: usize,
    operated: Option<usize>,
) -> ShowcaseInput {
    let soldiers: Vec<Value> = (0..slots)
        .map(|k| json!({"id": k + 1, "slot": k, "at": [1.5 * k as f64, 0.8 * (k % 2) as f64]}))
        .collect();
    let operators: Vec<Option<u32>> = (0..mounts)
        .map(|m| (Some(m) == operated).then_some(1))
        .collect();
    serde_json::from_value(json!({
        "kind": kind, "side": side, "at": [0.0, 0.0, 0.0], "yaw": 0.5,
        "soldiers": soldiers, "operators": operators
    }))
    .unwrap()
}

fn camera() -> Camera {
    Camera {
        eye: [-12.0, -16.0, 9.0],
        target: [1.5, 0.0, 1.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.9,
        near: 0.1,
        size: [1280, 720],
    }
}

/// What one drawn frame showed: each model's appearance and instance, the
/// joints their palettes index, and the frame's counts.
struct Shot {
    models: Vec<(String, Instance)>,
    palettes: Vec<Joint>,
    counts: Counts,
}

fn shown(p: &mut Presentation, time: f64) -> Shot {
    let f = p.frame(time, &camera(), &Ui::default(), 0).unwrap();
    let (palettes, counts) = (f.palettes.clone(), f.counts);
    let batches: Vec<_> = f
        .batches
        .iter()
        .map(|b| (b.appearance, b.first, b.count))
        .collect();
    let instances = f.instances.clone();
    let w = p.world();
    let models = batches
        .into_iter()
        .flat_map(|(row, first, count)| {
            let name = w.name(&w.appearances[row as usize]).to_owned();
            instances[first as usize..(first + count) as usize]
                .iter()
                .map(move |i| (name.clone(), *i))
        })
        .collect();
    Shot {
        models,
        palettes,
        counts,
    }
}

impl Shot {
    /// The model standing at `(x, y)`.
    fn at(&self, x: f64, y: f64) -> (String, Instance) {
        self.models
            .iter()
            .find(|(_, i)| {
                (f64::from(i.placement[0]) - x).abs() < 1e-3
                    && (f64::from(i.placement[1]) - y).abs() < 1e-3
            })
            .cloned()
            .unwrap_or_else(|| panic!("nothing stands at ({x}, {y})"))
    }

    /// The first joints of the palette of the model at `(x, y)`, as numbers.
    fn palette(&self, x: f64, y: f64) -> Vec<f32> {
        let first = self.at(x, y).1.data[0] as usize;
        self.palettes[first..first + 8]
            .iter()
            .flat_map(|j| j.x.iter().chain(&j.y).chain(&j.z).copied())
            .collect()
    }
}

#[test]
fn a_showcase_poses_without_any_publication() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_showcase(plain("test_tank"), hull("test_tank", [3.0, 2.0, 0.0]))
        .unwrap();
    let f = shown(&mut p, 0.0);
    let (name, tank) = f.at(3.0, 2.0);
    assert!(
        !name.is_empty() && tank.placement[2] == 0.0,
        "the tank stands on its spot"
    );
    assert_eq!(f.counts.vehicles, 1);
}

#[test]
fn showcased_soldiers_idle_each_at_their_own_tempo() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_showcase(plain("test_rifle"), squad("test_rifle", "blue", 3, 0, None))
        .unwrap();
    let spots = [(0.0, 0.0), (1.5, 0.8), (3.0, 0.0)];
    let poses =
        |f: &Shot| -> Vec<Vec<f32>> { spots.iter().map(|&(x, y)| f.palette(x, y)).collect() };
    let mut early = Vec::new();
    let mut late = Vec::new();
    for k in 0..=60 {
        let f = shown(&mut p, f64::from(k) / 30.0);
        match k {
            30 => early = poses(&f),
            60 => late = poses(&f),
            _ => {}
        }
    }
    // A watcher may stand still, weapon up; the rest idle.
    let idling: Vec<_> = (0..spots.len()).filter(|&s| early[s] != late[s]).collect();
    assert!(idling.len() > 1, "only {idling:?} of the squad idle");
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        assert_ne!(late[a], late[b], "soldiers {a} and {b} stand in one pose");
    }
}

#[test]
fn a_parked_rotorcraft_draws_its_blades_at_rest_with_no_rotor_disc() {
    let rotor = SHAPE_NAMES.iter().position(|s| *s == "rotor").unwrap() as f32;
    let fielded = plain("test_heli");
    let appearances = fielded.units.appearances.clone();
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_showcase(fielded, hull("test_heli", [0.0, 0.0, 0.0]))
        .unwrap();
    let mut blades = Vec::new();
    for k in 0..3 {
        let f = shown(&mut p, f64::from(k) / 30.0);
        let (name, heli) = f.at(0.0, 0.0);
        assert_eq!(heli.placement[2], 0.0, "it stands on the ground");
        assert!(
            !p.effects().iter().any(|e| e.misc[0] == rotor),
            "frame {k}: a parked rotorcraft draws no rotor disc"
        );
        let Some(Appearance::Articulated { rig, .. }) = appearances.get(&name) else {
            panic!("{name} is not articulated");
        };
        assert!(!rig.rotors.is_empty(), "{name} has rotors");
        let first = heli.data[0] as usize;
        let drawn: Vec<Joint> = rig
            .rotors
            .iter()
            .map(|r| f.palettes[first + r.node])
            .collect();
        for j in &drawn {
            // Each blade node keeps its full size: the blur would scale it to nothing.
            let [a, b, c] = [j.x, j.y, j.z].map(|r| [r[0], r[1], r[2]]);
            let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]);
            assert!(
                det.abs() > 1e-3,
                "frame {k}: a rotor's blades are drawn ({det})"
            );
        }
        blades.push(drawn);
    }
    assert!(
        blades.windows(2).all(|w| w[0] == w[1]),
        "the blades stand still between frames"
    );
}

#[test]
fn an_operator_at_rest_carries_his_weapons_carried_kit() {
    let fielded = plain("test_at");
    let draws = fielded.units.types["test_at"].draws["blue"].clone();
    let mounts = fielded.units.types["test_at"].mounts.len();
    let m = draws
        .operated
        .iter()
        .position(Option::is_some)
        .expect("the AT team operates a weapon");
    let carried = draws.operated[m].as_ref().unwrap().carried.clone();
    let slots = draws.slots.len();
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_showcase(fielded, squad("test_at", "blue", slots, mounts, Some(m)))
        .unwrap();
    let f = shown(&mut p, 0.0);
    let (operator, _) = f.at(0.0, 0.0);
    assert!(
        carried.contains(&operator),
        "{operator} is not one of the carried kits {carried:?}"
    );
    let (rifleman, _) = f.at(1.5, 0.8);
    assert!(
        !carried.contains(&rifleman),
        "a rifleman carries no launcher"
    );
}

#[test]
fn a_showcase_wears_the_look_fielding_resolved_for_its_faction() {
    // The test rifle wears the recon look for the eastern faction.
    let field = || {
        fielded(&["test_rifle"], Some(["us", "eastern"]), |runtime| {
            for look in ["test_rifle", "test_rifle_b", "test_rifle_c"] {
                runtime["appearances"][look]["factions"] = json!({"eastern": "test_recon"});
            }
        })
    };
    let worn = |side: &str| {
        let mut p = Presentation::new(None, 30.0).unwrap();
        p.set_showcase(field(), squad("test_rifle", side, 3, 0, None))
            .unwrap();
        shown(&mut p, 0.0).at(0.0, 0.0).0
    };
    let red = field().units.types["test_rifle"].draws["red"].slots[0].clone();
    let (us, eastern) = (worn("blue"), worn("red"));
    assert!(
        red.contains(&eastern),
        "{eastern} is not the red side's look {red:?}"
    );
    assert_ne!(us, eastern, "each faction wears its own look");
}

fn study() -> Asset {
    let head = serde_json::to_vec(&json!({
        "content": {"kind":"prop", "states":[{
            "name":"default", "bounds":{"min":[-1,-1,0], "max":[1,1,2]},
            "tiers":[{}, {}, {}, {}]
        }]}, "views":[]
    }))
    .unwrap();
    let mut bytes = Vec::new();
    for word in [0x5441_4742u32, 4, head.len() as u32, 0] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(head);
    Asset::from_bundle(&bytes, None).unwrap()
}

#[test]
fn a_shown_unit_s_rows_are_the_presentation_s_own_and_clearing_it_restores_the_lab_s() {
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.install_lab_assets(
        [("study".into(), study())].into(),
        Detail {
            lod_px: [80.0, 40.0, 20.0],
            impostor_px: 0.0,
            shadow_caster_px: 0.0,
        },
    );
    p.set_lab_models(
        serde_json::from_value(json!([{
            "appearance":"study", "x":-6, "y":0, "z":0, "yaw":0,
            "pose":{"kind":"static", "state":"default"}
        }]))
        .unwrap(),
    )
    .unwrap();
    let rows = |p: &Presentation| {
        let w = p.world();
        w.appearances
            .iter()
            .map(|r| w.name(r).to_owned())
            .collect::<Vec<_>>()
    };
    let before = rows(&p);
    let lab = shown(&mut p, 0.0).at(-6.0, 0.0);
    p.set_showcase(plain("test_tank"), hull("test_tank", [3.0, 0.0, 0.0]))
        .unwrap();
    let both = shown(&mut p, 0.0);
    let (tank, _) = both.at(3.0, 0.0);
    // The page loads the art of the canonical rows: the shown unit's are
    // among them, the lab's replaceable rows after.
    let shown_rows = rows(&p);
    let canonical = &shown_rows[..p.canonical_appearance_rows()];
    assert!(canonical.contains(&tank), "{tank} is not a canonical row");
    assert!(
        !canonical.contains(&lab.0),
        "the lab's rows stay replaceable"
    );
    assert_eq!(
        both.at(-6.0, 0.0).1.placement,
        lab.1.placement,
        "the lab model is drawn as it was"
    );
    p.clear_showcase();
    assert_eq!(rows(&p), before, "the shown unit's rows are gone");
    let after = shown(&mut p, 0.0);
    assert_eq!(after.models, vec![lab], "only the lab model is left");
    assert!(p.effects().is_empty());
}

#[test]
fn a_showcase_and_a_battle_never_share_a_presentation() {
    let battle = || {
        let f = plain("test_tank");
        let world = WorldInput {
            battle: Some(BattleInput {
                units: f.units,
                effects: f.effects,
                side: "blue".into(),
                tick_hz: 30.0,
                soldier: SoldierBody {
                    radius_m: 0.2,
                    height_m: 1.8,
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
        (world, f.tables)
    };
    let mut p = Presentation::new(None, 30.0).unwrap();
    let (world, tables) = battle();
    p.load_world(world, tables, None).unwrap();
    assert!(p
        .set_showcase(plain("test_tank"), hull("test_tank", [0.0; 3]))
        .is_err());
    let mut p = Presentation::new(None, 30.0).unwrap();
    p.set_showcase(plain("test_tank"), hull("test_tank", [0.0; 3]))
        .unwrap();
    let (world, tables) = battle();
    assert!(p.load_world(world, tables, None).is_err());
}
