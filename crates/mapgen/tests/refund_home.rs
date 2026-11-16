//! A refunded supply truck drives home from where a player sent it, on the
//! generated town a player reported trucks stuck in: a large mixed map,
//! three players a side. Each case is a spot a truck once stalled leaving
//! or reaching: a garden fence panel it could only slide endwise into the
//! next, a tail in an alley's mouth, a corner of a parked car its turn
//! grazed, a waypoint under its own hull that it kept circling, and a fence
//! jammed behind it as it turned. Pinned to one generated map, a generator
//! change may move the spots; the cases then still pass.
#[path = "common/limits.rs"]
mod limits;
#[path = "common/parallel.rs"]
mod parallel;

use contract::command::{CommandEnvelope, Order};
use contract::generation::{GenerationProfile, GenerationRequest, MapSize, MapType};
use contract::ids::Side;
use contract::scenario::{Rules, ScenarioDefinition};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const SEED: u64 = 3390483735354143248;
const TRUCK: &str = "us_m977_hemtt_general_resupply";

fn command(seq: u64, side: Side, order: Order) -> CommandEnvelope {
    let player = if side == Side::Blue {
        "blue-1"
    } else {
        "red-1"
    };
    CommandEnvelope {
        side,
        player: Some(player.into()),
        seq,
        order,
        queued: false,
    }
}

/// Blue buys a truck down `lane` toward objective `objective`, `share` of
/// the way, refunds it wherever it is after the drive out (set up there, or
/// still on its way), and reports whether it reached base within four
/// minutes.
fn returns_home(rules: &Rules, (lane, objective, share): (usize, usize, f64)) -> bool {
    let players: Vec<contract::players::Player> = serde_json::from_value(serde_json::json!([
        {"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},
        {"id":"blue-2","side":"blue","faction":"us","controller":"passive_ai"},
        {"id":"blue-3","side":"blue","faction":"us","controller":"passive_ai"},
        {"id":"red-1","side":"red","faction":"eastern","controller":"human","human_capable":true},
        {"id":"red-2","side":"red","faction":"eastern","controller":"passive_ai"},
        {"id":"red-3","side":"red","faction":"eastern","controller":"passive_ai"}
    ]))
    .unwrap();
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let presets = mapgen::layout::PresetDefinitions::from_json(PRESETS).unwrap();
    let request = GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: SEED.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type: MapType::Mixed,
        size: MapSize::Large,
        entries_per_side: contract::players::entries_per_side(&players),
        profile: GenerationProfile::Skirmish,
        region: None,
        limits: limits::game_limits(),
    };
    let mapgen::CompileOutcome::Ok { result } = mapgen::generate_map(
        &serde_json::to_string(&request).unwrap(),
        PRESETS,
        TEMPLATES,
        &serde_json::to_string(rules).unwrap(),
    ) else {
        panic!("the reported map generates");
    };
    let prepared = sim::encounter::PreparedMap::new(&result.map, rules);
    let sites = sim::map_analysis::admit_skirmish(
        &prepared.queries(&result.map, &result.sites),
        rules,
        &players,
    )
    .unwrap()
    .sites
    .skirmish
    .unwrap();
    let entry = sites
        .entries
        .iter()
        .filter(|e| e.side == Side::Blue)
        .nth(lane)
        .unwrap()
        .center;
    let target = sites.objectives[objective].center;
    let spot = [
        entry[0] + (target[0] - entry[0]) * share,
        entry[1] + (target[1] - entry[1]) * share,
    ];
    let setup = ScenarioDefinition {
        skirmish: Some(sim::skirmish::setup_for(
            &players,
            sites,
            sim::skirmish::starter_rules(),
        )),
        players,
        map: result.map,
        rules: rules.clone(),
        units: vec![],
        events: vec![],
        scripts: vec![],
        authored_scripts: vec![],
        victory: None,
        opponent: None,
        encounter: None,
    };
    let mut battle = sim::battle::Battle::from_prepared(&setup, SEED, prepared);
    let purchase = Order::purchase(TRUCK, spot);
    assert!(battle
        .accept(command(1, Side::Blue, purchase))
        .error
        .is_none());
    battle.accept(command(2, Side::Blue, Order::Ready));
    battle.accept(command(1, Side::Red, Order::Ready));
    let hz = u64::from(rules.tick_hz);
    for _ in 0..200 * hz {
        battle.step();
    }
    let truck = battle.observe(Side::Blue).own[0].clone();
    let refund = Order::Refund {
        units: vec![truck.id],
    };
    assert!(battle
        .accept(command(3, Side::Blue, refund))
        .error
        .is_none());
    for _ in 0..240 * hz {
        if battle.observe(Side::Blue).own.is_empty() {
            return true;
        }
        battle.step();
    }
    false
}

#[test]
fn a_truck_refunded_in_town_drives_home() {
    let rules: Rules = serde_json::from_value(sim::fixtures::rules()).unwrap();
    // (lane, objective, share of the way): a fence panel jammed endwise, a
    // tail in an alley's mouth, a second fence, a parked car's corner, a
    // waypoint under the hull, and a fence jammed behind a turn.
    let cases = [
        (1, 1, 0.6),
        (0, 6, 0.6),
        (2, 6, 0.8),
        (0, 0, 0.8),
        (1, 6, 0.6),
        (0, 6, 0.8),
    ];
    let home = parallel::each(&cases, |&case| returns_home(&rules, case));
    let stuck: Vec<_> = cases.iter().zip(&home).filter(|(_, h)| !**h).collect();
    assert!(stuck.is_empty(), "trucks that never got home: {stuck:?}");
}
