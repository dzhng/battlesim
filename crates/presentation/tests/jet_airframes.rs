//! Every fixed-wing jet in the roster takes the same exhaust effects from its
//! own data, through the path a battle draws it by: its nozzles glow, its
//! wingtips trail vapour and its wings raise a condensation sheet, all on its
//! drawn model; only a jet whose engine has an afterburner ever shows a
//! flame. The shipped presentation, roster and runtime art; synthetic
//! flights.
mod jets;
use jets::*;

use presentation::assets::Runtime;
use presentation::effects::exhaust::{DrawnJet, Emitter, JetExhaustStyle};
use presentation::effects::frame::EffectFrame;
use presentation::effects::tables::EffectPresentation;
use presentation::fielded::Fielding;
use presentation::units::drawn::DrawnModels;
use presentation::units::pose::{PoseFrame, VehiclePose};
use presentation::world::Appearance;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}

/// The shipped jet exhaust style.
fn shipped() -> JetExhaustStyle {
    EffectPresentation::from_value(&read("fixtures/presentation.json")["effects"])
        .unwrap()
        .jet_exhaust
        .expect("the shipped presentation draws jets' exhaust")
}

/// Every fixed-wing jet of the roster: its unit id and family.
fn roster_jets() -> Vec<(String, String)> {
    let dir = root().join("fixtures/units/roster");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let doc: Value =
            serde_json::from_slice(&std::fs::read(entry.unwrap().path()).unwrap()).unwrap();
        for (id, unit) in doc["units"].as_object().into_iter().flatten() {
            if unit["extends"] == "roster_profile_fixed_wing" {
                out.push((id.clone(), unit["family"].as_str().unwrap_or("").to_owned()));
            }
        }
    }
    out.sort();
    assert!(out.len() > 1, "the roster flies fixed-wing jets");
    out
}

/// Each roster jet drawn level at rest heading +X, standing at the origin:
/// its unit id, family, drawn jet and its model's plan box and height.
fn drawn_roster() -> Vec<(String, String, DrawnJet, [f64; 4], f64)> {
    let jets = roster_jets();
    let kinds: Vec<String> = jets.iter().map(|j| j.0.clone()).collect();
    let catalog = presentation::static_world::files::rules(&root(), "market-town")
        .unwrap()
        .catalog
        .view();
    let runtime = Runtime::open(&root().join("assets/runtime")).unwrap();
    let fielded = Fielding {
        presentation: &read("fixtures/presentation.json"),
        rules: &read("fixtures/rules.json"),
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    let units = Arc::new(fielded.units);
    let mut drawn = DrawnModels::new(units.clone(), Arc::new(fielded.tables), "blue");
    let pose = |(i, kind): (usize, &String)| VehiclePose {
        unit: i as f64,
        kind: kind.as_str().into(),
        side: "blue".into(),
        position: [0.0; 3],
        yaw: 0.0,
        airborne: true,
        downed: false,
        articulation: Default::default(),
        motion: None,
    };
    drawn.update(&PoseFrame {
        vehicles: kinds.iter().enumerate().map(pose).collect(),
        ..Default::default()
    });
    let mut out = Vec::new();
    drawn.jets(&shipped(), &mut out);
    assert_eq!(out.len(), jets.len(), "every roster jet is drawn running");
    jets.into_iter()
        .zip(out)
        .enumerate()
        .map(|(i, ((id, family), jet))| {
            let name = units.resolve(&id, "blue", i as f64, 0, None, None).unwrap();
            let Some(Appearance::Articulated { plan, bounds, .. }) = units.appearances.get(name)
            else {
                panic!("{id}: an articulated model");
            };
            let height = bounds.iter().map(|b| b.size).fold(0.0, f64::max);
            (id, family, jet, *plan, height)
        })
        .collect()
}

fn count(jet: &DrawnJet, kind: fn(&Emitter) -> bool) -> usize {
    jet.emitters.iter().filter(|e| kind(&e.0)).count()
}

#[test]
fn every_roster_jet_draws_glow_tip_vapour_and_a_wing_sheet_on_its_own_model() {
    let style = shipped();
    let rows = style.wing_sheet_rule.rows.len();
    for (id, _, jet, plan, height) in drawn_roster() {
        assert!(
            style.engines.contains_key(&jet.engine),
            "{id}: an engine look"
        );
        assert!(
            count(&jet, |e| matches!(e, Emitter::Nozzle { .. })) >= 1,
            "{id}: a nozzle"
        );
        assert_eq!(
            count(&jet, |e| *e == Emitter::Wingtip),
            2,
            "{id}: two wingtips"
        );
        assert!(
            count(&jet, |e| *e == Emitter::Sheet) >= 2 * rows,
            "{id}: each of the rule's rows of sheet points on each wing"
        );
        for (kind, p) in &jet.emitters {
            let inside = (plan[0] - 0.05..=plan[2] + 0.05).contains(&p[0])
                && (plan[1] - 0.05..=plan[3] + 0.05).contains(&p[1])
                && (0.0..=height).contains(&p[2]);
            assert!(
                inside,
                "{id}: {kind:?} at {p:?} outside its model {plan:?} ↑{height}"
            );
        }
        let tips: Vec<_> = jet
            .emitters
            .iter()
            .filter(|e| e.0 == Emitter::Wingtip)
            .collect();
        assert!(
            tips[0].1[1] > 0.0 && tips[1].1[1] < 0.0,
            "{id}: a tip on each side"
        );
    }
}

/// The last frame of `jet` flying straight along +X at `speed` for 1.5 s,
/// drawn as it is drawn at the origin, under the shipped style.
fn flown(jet: &DrawnJet, speed: f64) -> Vec<Vec<f32>> {
    let mut style = read("fixtures/presentation.json")["effects"]["jet_exhaust"].clone();
    style["airframes"] = serde_json::json!({});
    let mut f: EffectFrame = frame_with(style);
    let key = 2.0;
    let mut moved = jet.clone();
    moved.key = f64::to_bits(key);
    let cruise = jet.cruise_mps;
    fly_jets(
        &mut f,
        30.0,
        1.5,
        |t| vec![craft(key, "", [speed * cruise * t, 0.0, 900.0], 0.0)],
        |t| {
            let shift = |p: [f64; 3]| [p[0] + speed * cruise * t, p[1], p[2] + 900.0];
            let mut j = moved.clone();
            j.at = shift(j.at);
            j.emitters.iter_mut().for_each(|e| e.1 = shift(e.1));
            vec![j]
        },
    )
}

#[test]
fn only_a_jet_whose_engine_has_an_afterburner_burns_at_full_reheat() {
    let style = shipped();
    let full = style.reheat_share[1] + 0.1;
    let (mut burning, mut dry) = (Vec::new(), Vec::new());
    for (id, family, jet, ..) in drawn_roster() {
        let d = flown(&jet, full);
        assert!(glows(&d).0 > 0, "{id}: its nozzles glow");
        let burns = style.engines[&jet.engine].flame.is_some();
        assert_eq!(
            flames(&d) > 0,
            burns,
            "{id}: a flame only from an afterburner"
        );
        let at_cruise = flown(&jet, 0.95);
        assert_eq!(
            flames(&at_cruise),
            0,
            "{id}: no flame below its reheat point"
        );
        if burns { &mut burning } else { &mut dry }.push(family);
    }
    assert!(!burning.is_empty(), "the afterburning jets burn");
    // The A-10's TF34 and the Su-25's R-195 have no afterburner.
    for family in ["a_10_warthog", "su_25_frogfoot"] {
        assert!(
            dry.iter().any(|f| f == family),
            "{family} never burns: {dry:?}"
        );
    }
}

#[test]
fn the_shared_sheet_rule_lands_the_raptors_sheet_where_the_approved_look_put_it() {
    let style = shipped();
    let raptor = &style.airframes["us_f_22_raptor_f_22a"];
    let sheet = raptor.sheet(&style.wing_sheet_rule);
    // The approved look's hand-placed points, each side.
    let approved: Vec<[f64; 3]> = [
        [1.8, 1.8, 2.5],
        [0.2, 2.9, 2.4],
        [-1.4, 4.1, 2.3],
        [-0.8, 2.2, 2.45],
    ]
    .into_iter()
    .flat_map(|[x, y, z]| [[x, y, z], [x, -y, z]])
    .collect();
    let near = |p: &[f64; 3]| {
        sheet
            .iter()
            .map(|q| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt())
            .fold(f64::MAX, f64::min)
    };
    let distances: Vec<f64> = approved.iter().map(near).collect();
    let mean = distances.iter().sum::<f64>() / distances.len() as f64;
    // Fitted to within 0.22 m of each (0.18 m on average); the tolerance is the
    // sheet's own half width.
    assert!(
        distances.iter().all(|&d| d < 0.3) && mean < 0.25,
        "{distances:?}, mean {mean}"
    );
}

#[test]
fn a_bad_sheet_rule_or_airframe_is_refused() {
    let base = read("fixtures/presentation.json")["effects"].clone();
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut p = base.clone();
        edit(&mut p["jet_exhaust"]);
        EffectPresentation::from_value(&p).is_err()
    };
    assert!(!refused(&|_| {}), "the shipped style loads");
    assert!(refused(
        &|s| s["wing_sheet_rule"]["span"] = serde_json::json!([0.5, 0.2])
    ));
    assert!(refused(
        &|s| s["wing_sheet_rule"]["rows"] = serde_json::json!([])
    ));
    assert!(refused(
        &|s| s["wing_sheet_rule"]["rows"] = serde_json::json!([[0.1, 1.2]])
    ));
    assert!(refused(
        &|s| s["wing_sheet_rule"]["span"] = serde_json::json!([-0.5, 0.4])
    ));
    assert!(refused(&|s| s["sheet_from"] = serde_json::json!(1.0)));
    let first = |s: &mut Value| {
        let name = s["airframes"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        s["airframes"][name].take()
    };
    assert!(refused(&|s| {
        let a = first(s);
        let mut one = a.clone();
        one["wingtips"].as_array_mut().unwrap().pop();
        let name = s["airframes"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        s["airframes"][name] = one;
    }));
    assert!(refused(&|s| {
        let mut a = first(s);
        a["wings"][0]["stations"].as_array_mut().unwrap().reverse();
        let name = s["airframes"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        s["airframes"][name] = a;
    }));
}

#[test]
fn every_roster_jets_effects_mirror_across_its_middle() {
    let style = shipped();
    for (name, a) in &style.airframes {
        let mirrored = |p: &[f64; 3], q: &[f64; 3], tol: f64| {
            (p[0] - q[0]).abs() <= tol && (p[1] + q[1]).abs() <= tol && (p[2] - q[2]).abs() <= tol
        };
        let [l, r] = [a.wingtips[0].at, a.wingtips[1].at];
        assert!(mirrored(&l, &r, 0.1), "{name}: its tips {l:?} {r:?}");
        if let [n0, n1] = a.nozzles.as_slice() {
            assert!(
                mirrored(&n0.at, &n1.at, 0.1),
                "{name}: its nozzles {n0:?} {n1:?}"
            );
            assert!(
                (n0.radius_m - n1.radius_m).abs() <= 0.05,
                "{name}: its nozzles' size"
            );
        } else {
            assert_eq!(a.nozzles.len(), 1, "{name}: one nozzle or a pair");
            assert!(
                a.nozzles[0].at[1].abs() <= 0.05,
                "{name}: its one nozzle on its middle"
            );
        }
        // The sheet: each wing's points mirror the other's, within what the art
        // itself differs by where one wing's root meets the body.
        let sheet = a.sheet(&style.wing_sheet_rule);
        let (left, right) = sheet.split_at(sheet.len() / 2);
        for (p, q) in left.iter().zip(right) {
            assert!(mirrored(p, q, 0.25), "{name}: sheet points {p:?} {q:?}");
        }
    }
}

/// `jet` (drawn at rest at the origin heading +X) drawn where `c` flies.
fn placed(jet: &DrawnJet, c: &Craft) -> DrawnJet {
    let (s, k) = (c.yaw.sin(), c.yaw.cos());
    let put = |p: [f64; 3]| {
        [
            c.at[0] + p[0] * k - p[1] * s,
            c.at[1] + p[0] * s + p[1] * k,
            c.at[2] + p[2],
        ]
    };
    DrawnJet {
        key: f64::to_bits(c.key),
        at: put(jet.at),
        yaw: c.yaw,
        aft: [-k, -s, 0.0],
        emitters: jet.emitters.iter().map(|(e, p)| (*e, put(*p))).collect(),
        ..jet.clone()
    }
}

#[test]
fn every_roster_jet_vapours_only_in_a_hard_pull_and_then_at_its_tips_and_over_its_wings() {
    // The shipped look, its tips' and sheet's vapour told apart by colour.
    let mut style = read("fixtures/presentation.json")["effects"]["jet_exhaust"].clone();
    style["airframes"] = serde_json::json!({});
    style["wingtip_trail"]["albedo"] = serde_json::json!([TIP_ALBEDO, TIP_ALBEDO, TIP_ALBEDO]);
    style["wing_sheet"][0]["albedo"] =
        serde_json::json!([SHEET_ALBEDO, SHEET_ALBEDO, SHEET_ALBEDO]);
    for (id, _, jet, ..) in drawn_roster() {
        let cruise = jet.cruise_mps;
        let fly_at = |flight: &dyn Fn(f64) -> Vec<Craft>| {
            let drawn = |t: f64| flight(t).iter().map(|c| placed(&jet, c)).collect();
            fly_jets(&mut frame_with(style.clone()), 30.0, 2.0, flight, drawn)
        };
        let level = fly_at(&straight(2.0, "", cruise));
        assert!(
            vapour(&level).is_empty(),
            "{id}: level flight leaves its tips dry"
        );
        // The shipped sheet billows (`puffs`) rather than ribbons.
        let sheet =
            |d: &[Vec<f32>]| ribbons(d, SHEET_ALBEDO).len() + billows(d, SHEET_ALBEDO).len();
        assert_eq!(sheet(&level), 0, "{id}: and its wings bare");
        let hard = fly_at(&along(turning(cruise, |_| rate_for(cruise, 7.0))));
        assert!(
            !vapour(&hard).is_empty(),
            "{id}: a 7 g pull vapours at its tips"
        );
        assert!(sheet(&hard) > 0, "{id}: and over its wings");
    }
}
