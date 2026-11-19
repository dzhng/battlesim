//! Authored scenarios: a scenario document resolves under today's rules into
//! a skirmish with forces already placed, and every shipped scenario's forces
//! stand where its map lets them.

use contract::ids::Side;
use contract::maps::MapCategory;
use contract::scenario::Rules;
use contract::scenario_document::ScenarioDocument;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::encounter::legality::{apart, footprint, stands_on};
use sim::encounter::PreparedMap;
use sim::math::v2;

/// The test rules, with the test rifle sold to `us` and the test tank to
/// `eastern`, so a scenario can field them as roster cards.
fn rules_with(patch: impl FnOnce(&mut Value)) -> Rules {
    let mut fixture = sim::fixtures::test_rules();
    for (kind, faction) in [
        ("test_rifle", "us"),
        ("test_tank", "eastern"),
        ("test_apc", "us"),
    ] {
        sim::fixtures::patch_catalog(
            &mut fixture,
            "units",
            kind,
            json!({"roster":{"factions":[faction],"category":"inf","family_name":"Test","variant":"Test"}}),
        );
    }
    patch(&mut fixture);
    serde_json::from_value(fixture).unwrap()
}

/// A small scenario on the geometry test map: a human blue player, a
/// directed red one with a tank, and blue's opening credits covering a
/// rifle squad in its carrier.
fn document() -> Value {
    json!({
        "battle_seed": "7",
        "players": [
            {"id":"blue-1","side":"blue","faction":"us","controller":"human","human_capable":true},
            {"id":"red-1","side":"red","faction":"eastern","controller":"directed_ai"}
        ],
        "match_rules": {"preparation_s": 600},
        "sites": {
            "entries": [
                {"side":"blue","center":[40.0,40.0],"yaw":0.0},
                {"side":"red","center":[300.0,200.0],"yaw":3.0}
            ],
            "objectives": [
                {"id":"centre","center":[170.0,120.0],"radius_m":30.0,"kind":"junction","counterpart":null},
                {"id":"west","center":[60.0,160.0],"radius_m":30.0,"kind":"field","counterpart":"east"},
                {"id":"east","center":[280.0,60.0],"radius_m":30.0,"kind":"field","counterpart":"west"}
            ]
        },
        "forces": {
            "tank": {"side":"red","owner":"red-1","kind":"test_tank","position":[260.0,180.0],"yaw":3.0}
        },
        "opening_credits": {
            "blue-1": {"covering": [{"variant":"test_rifle","carrier":"test_apc"}]}
        },
        "zones": {"hilltop": {"center":[170.0,120.0],"radius_m":40.0}}
    })
}

fn resolve(
    document: Value,
    rules: Rules,
) -> Result<contract::scenario::ScenarioDefinition, String> {
    let map = sim::maps::load("geometry").unwrap().definition;
    let document: ScenarioDocument = serde_json::from_value(document).map_err(|e| e.to_string())?;
    document.on(map, rules, &sim::skirmish::starter_rules())
}

fn refusal(change: impl FnOnce(&mut Value)) -> String {
    let mut doc = document();
    change(&mut doc);
    resolve(doc, rules_with(|_| {})).expect_err("the scenario is refused")
}

#[test]
fn a_scenario_document_is_refused_naming_the_field_at_fault() {
    assert!(resolve(document(), rules_with(|_| {})).is_ok());
    /// A change to the document, and the field its refusal must name.
    type Case = (fn(&mut Value), &'static str);
    let cases: [Case; 6] = [
        (
            |d| d["forces"]["tank"]["owner"] = json!("nobody"),
            "forces.tank.owner",
        ),
        (
            |d| d["forces"]["tank"]["side"] = json!("blue"),
            "forces.tank.side",
        ),
        // A unit outside the owner's faction, or no roster card at all.
        (
            |d| d["forces"]["tank"]["kind"] = json!("test_rifle"),
            "forces.tank.kind",
        ),
        (
            |d| d["forces"]["tank"]["kind"] = json!("test_truck"),
            "forces.tank.kind",
        ),
        (
            |d| d["match_rules"]["income"] = json!(5),
            "match_rules.income",
        ),
        (
            |d| d["zones"]["hilltop"]["center"] = json!([-5.0, 0.0]),
            "zones.hilltop",
        ),
    ];
    for (change, field) in cases {
        let message = refusal(change);
        assert!(message.starts_with(field), "{field}: {message}");
    }
}

#[test]
fn covering_credits_follow_todays_prices() {
    let wallet = |rules: Rules| {
        resolve(document(), rules)
            .unwrap()
            .skirmish
            .unwrap()
            .opening_credits["blue-1"]
    };
    let cost = |rules: &Rules, kind: &str| {
        u64::from(rules.catalog.get(rules.catalog.index(kind).unwrap()).cost)
    };
    let shipped = rules_with(|_| {});
    let expected = cost(&shipped, "test_rifle") + cost(&shipped, "test_apc");
    let price = cost(&shipped, "test_apc");
    assert_eq!(
        wallet(shipped),
        expected,
        "the grant buys the purchase it covers"
    );
    let dearer = rules_with(|fixture| {
        sim::fixtures::patch_catalog(fixture, "units", "test_apc", json!({"cost": price + 100}));
    });
    assert_eq!(
        wallet(dearer),
        expected + 100,
        "a retuned price moves the grant with it"
    );
}

#[test]
fn a_placed_force_counts_as_its_owners_and_the_opening_credits_are_the_wallet() {
    let setup = resolve(document(), rules_with(|_| {})).unwrap();
    let granted = setup.skirmish.as_ref().unwrap().opening_credits["blue-1"];
    let mut battle = Battle::new(&setup, 7);
    battle.step();
    let player = |side: Side| battle.observe(side).skirmish.as_ref().unwrap().players[0].clone();
    assert_eq!(
        player(Side::Red).unit_count,
        1,
        "the placed tank fills a place under red's cap"
    );
    assert_eq!(player(Side::Blue).credits, granted as f64);
}

/// Every scenario on every `scenario` map: it resolves under the game's
/// rules, and each force stands on ground its map lets it, clear of the
/// others.
#[test]
fn every_shipped_scenario_resolves_and_places_its_forces_legally() {
    let catalogue = sim::maps::Catalogue::shipped();
    let rules: Rules = serde_json::from_value(sim::fixtures::rules()).unwrap();
    let mut checked = 0;
    for id in catalogue.ids() {
        if catalogue.category(&id).unwrap() != MapCategory::Scenario {
            continue;
        }
        for name in catalogue.scenarios(&id).unwrap() {
            let scenario = catalogue
                .scenario(&id, &name, rules.clone())
                .unwrap_or_else(|e| panic!("{e}"));
            let setup = &scenario.setup;
            let prepared = PreparedMap::new(&setup.map, &setup.rules);
            let mut placed = Vec::new();
            for (i, unit) in setup.units.iter().enumerate() {
                let kind = setup.rules.catalog.index(&unit.kind).unwrap();
                let t = setup.rules.catalog.get(kind);
                let at = v2(unit.position[0], unit.position[1]);
                if let Some(mobility) = sim::units::ground_mobility(t, &setup.rules) {
                    stands_on(
                        &prepared.world,
                        &prepared.grid,
                        &setup.rules,
                        t,
                        &mobility,
                        at,
                        unit.yaw,
                    )
                    .unwrap_or_else(|e| panic!("{id}/{name} units[{i}]: {}", e.describe()));
                }
                let here = footprint(t, &setup.rules, at, unit.yaw);
                assert!(
                    placed.iter().all(|other| apart(&here, other, 0.0)),
                    "{id}/{name} units[{i}] overlaps another force"
                );
                placed.push(here);
            }
            // It plays: its first ticks run.
            let mut battle = Battle::new(setup, scenario.battle_seed);
            for _ in 0..30 {
                battle.step();
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "the catalogue ships a scenario");
}
