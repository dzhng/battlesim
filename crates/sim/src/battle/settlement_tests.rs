//! A building collapse settles its kill reward through the battle.

use super::*;
use serde_json::json;

#[test]
fn hostile_collapse_uses_first_destructive_source_and_original_receipt() {
    let mut rules = crate::fixtures::with_map("sensors");
    let map = rules.as_object_mut().unwrap().remove("map").unwrap();
    for kind in ["test_tank", "test_rifle"] {
        crate::fixtures::patch_catalog(
            &mut rules,
            "units",
            kind,
            json!({"sensors":{"ground_m":1}}),
        );
    }
    crate::fixtures::patch_catalog(&mut rules, "units", "test_rifle", json!({"cost":900}));
    rules["bail_out"]["building"]["death"] = json!(1.0);
    let setup: ScenarioDefinition = serde_json::from_value(json!({
        "map":map,"rules":rules,"units":[
            {"side":"blue","kind":"test_tank","position":[50,50]},
            {"side":"red","kind":"test_rifle","position":[700,550]}
        ]
    }))
    .unwrap();
    let mut battle = Battle::new(&setup, 7);
    let building = battle.world.props().find(|p| p.body.garrison).unwrap().id;
    let center = battle.world.prop(building).unwrap().center;
    battle.units[1].position = center.with_z(0.0);
    for member in &mut battle.units[1].members {
        member.position = center.with_z(0.0);
    }
    battle.units[1].hold = Some(crate::hold::Hold {
        host: crate::hold::Host::Building(building),
        phase: crate::hold::Phase::Inside,
        place: crate::hold::Place::Building(garrison::Garrison {
            slots: Vec::new(),
            seats: vec![None; battle.units[1].members.len()],
            entry: center,
        }),
    });
    let players = vec![
        contract::players::Player {
            id: "blue-1".into(),
            side: Side::Blue,
            faction: Some(contract::catalog::Faction::Us),
            controller: contract::players::ControllerKind::Human,
            human_capable: true,
        },
        contract::players::Player {
            id: "red-1".into(),
            side: Side::Red,
            faction: Some(contract::catalog::Faction::Eastern),
            controller: contract::players::ControllerKind::PassiveAi,
            human_capable: false,
        },
    ];
    battle.units[0].owner = Some("blue-1".into());
    battle.units[1].owner = Some("red-1".into());
    battle.skirmish = Some(crate::skirmish::Skirmish::new(serde_json::from_value(json!({
    "rules":{"credits_per_minute":200,"starting_minutes":5,"preparation_s":60,"max_units":30,"max_players_per_side":10,"dispatch_interval_s":1, "capture_s": 20, "majority_s": 1800, "victory_score": 1000},
        "sites":{"entries":[{"side":"blue","center":[400,10],"yaw":0},{"side":"red","center":[400,590],"yaw":0}],
            "objectives":[{"id":"center","center":[400,300],"radius_m":50,"kind":"junction","counterpart":null},
                {"id":"south","center":[200,200],"radius_m":50,"kind":"field","counterpart":"north"},
                {"id":"north","center":[600,400],"radius_m":50,"kind":"field","counterpart":"south"}]}
    })).unwrap(), &players));
    battle.skirmish.as_mut().unwrap().entered.insert(
        UnitId(1),
        crate::skirmish::EnteredUnit {
            price: 200,
            entered_tick: 0,
            player: 1,
        },
    );
    battle.tick = 1;
    battle.observe_all();
    let before = battle.observe(Side::Blue).clone();
    let source = Some(damage::LethalSource {
        unit: UnitId(0),
        side: Side::Blue,
    });
    battle.consequences(damage::Outcome {
        destroyed: Vec::new(),
        attacked: Vec::new(),
        structural: vec![
            damage::StructuralHit {
                prop: building,
                amount: 1e12,
                toward: v2(1.0, 0.0),
                source,
            },
            damage::StructuralHit {
                prop: building,
                amount: 1e12,
                toward: v2(1.0, 0.0),
                source: None,
            },
        ],
    });
    assert!(!battle.units[1].alive());
    battle.observe_all();
    let after = battle.observe(Side::Blue);
    assert_eq!(after.skirmish.as_ref().unwrap().players[0].credits, 1050.0);
    assert_eq!(after.identified, before.identified);
    assert_eq!(after.contacts, before.contacts);
    assert_eq!(after.known_props, before.known_props);
    assert_eq!(after.corpses, before.corpses);
    assert_eq!(
        battle.skirmish.as_ref().unwrap().settlement.pools(),
        [100_000_000, 0]
    );
}
