//! Soldiers at a truck: the wounded are treated, the fallen are replaced by
//! new soldiers where their squad fights, and an eliminated squad never
//! returns.
use contract::ids::{Side, UnitId};
use contract::observation::ServiceStatus;
use serde_json::json;
use sim::battle::Battle;

use super::{battle, deploy_ticks, full_stock, holding, own, run, stock, truck};
use crate::common;

#[test]
fn casualties_are_replaced_by_new_soldiers_and_the_fallen_stay() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_rifle", 130.0, json!({ "casualties": 2 }))
        ]),
        4,
    );
    let area = own(&b, Side::Blue, 1).unwrap().area;
    let corpses: Vec<_> = b.observe(Side::Blue).corpses.clone();
    assert_eq!(corpses.len(), 2);
    let every = b.rules().service.soldier_replacement_s;
    run(&mut b, deploy_ticks() + (every * 30.0) as u64 * 2 + 4);
    let squad = own(&b, Side::Blue, 1).unwrap();
    assert_eq!(squad.members.len(), 8, "back to authored strength");
    assert_eq!(
        squad.area, area,
        "replacements do not enlarge the squad area"
    );
    // Replacements join on free ground, spaced from their squadmates.
    let spacing = common::game()["infantry_movement"]["spacing_m"]
        .as_f64()
        .unwrap();
    for (i, p) in squad.members.iter().enumerate() {
        for q in &squad.members[i + 1..] {
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            assert!(d >= spacing - 1e-9, "two soldiers {d:.2} m apart");
        }
    }
    assert_eq!(
        b.observe(Side::Blue).corpses,
        corpses,
        "the old records remain"
    );
    assert_eq!(squad.service, ServiceStatus::Full);
}

#[test]
fn a_garrisoned_squad_is_reinforced_inside_its_building() {
    // A rifle squad two men short holds the building beside a deployed truck.
    // Its replacements join it inside, each on a free facade slot of the
    // building, where the garrison's own soldiers stand.
    let map =
        json!({ "size": [800, 400], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35,
        "buildings": [common::building(0, "china-home-12x9-2f", [150.0, 200.0], 0.0)] })
        .to_string();
    let units = json!([
        truck(None),
        { "side": "blue", "kind": "test_rifle", "position": [150, 214], "engagement": "return_fire_only",
          "condition": { "casualties": 2 } },
    ]);
    let scripts = json!([{ "tick": 1, "side": "blue", "order": { "kind": "occupy_building", "units": [1], "building": 0, "gesture": 0 } }]);
    let mut b = Battle::new(&common::scenario_with(&map, units, json!([]), scripts), 4);
    let every = b.rules().service.soldier_replacement_s;
    let enter = common::game()["garrison"]["enter_exit_s"].as_f64().unwrap();
    run(
        &mut b,
        deploy_ticks().max((enter * 30.0) as u64 + 30) + (every * 30.0) as u64 * 2 + 30,
    );
    let squad = b.unit(UnitId(1)).unwrap();
    assert!(squad.garrisoned(), "the squad holds the building");
    assert_eq!(
        squad.members.iter().filter(|s| s.alive()).count(),
        8,
        "back to authored strength inside"
    );
    let g = squad.hold.as_ref().unwrap().building().unwrap();
    let mut seats = Vec::new();
    for (k, s) in squad.members.iter().enumerate().filter(|(_, s)| s.alive()) {
        let seat = b
            .windows()
            .seat(squad, k)
            .expect("every living soldier holds a slot");
        assert_eq!(s.position, seat.position, "each stands on his slot");
        seats.push(g.seats[k]);
    }
    seats.sort();
    seats.dedup();
    assert_eq!(seats.len(), 8, "no two share a slot");
    assert_eq!(own(&b, Side::Blue, 1).unwrap().service, ServiceStatus::Full);
}

#[test]
fn an_eliminated_squad_is_never_resurrected() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_at", 130.0, json!({ "casualties": 3 }))
        ]),
        5,
    );
    run(&mut b, deploy_ticks() + 30 * 20);
    assert!(own(&b, Side::Blue, 1).is_none());
    assert_eq!(stock(&b), full_stock());
}

#[test]
fn replacements_get_fresh_ids_unique_across_the_battle() {
    let mut b = battle(
        json!([
            truck(None),
            holding("test_rifle", 130.0, json!({ "casualties": 2 })),
            holding("test_recon", 150.0, json!({}))
        ]),
        15,
    );
    let first_ids = 8 + 4;
    run(&mut b, deploy_ticks() + 30 * 11);
    assert_eq!(own(&b, Side::Blue, 1).unwrap().members.len(), 8);
    let newest = b
        .unit(UnitId(1))
        .unwrap()
        .members
        .iter()
        .map(|s| s.id)
        .max();
    assert_eq!(
        newest,
        Some(first_ids as u32 + 2),
        "two new ids after every authored one"
    );
}

/// Wounded soldiers are patched up, the most wounded first, so the squad's
/// wounds even out: each treatment restores up to its share of health at its
/// price, never past the soldier's own.
#[test]
fn wounded_soldiers_are_treated_the_most_wounded_first() {
    let wounded = 55.0;
    let mut b = battle(
        json!([
            truck(None),
            holding("test_rifle", 130.0, json!({ "soldier_hp": wounded }))
        ]),
        16,
    );
    let full = stock(&b);
    let service = b.rules().service.clone();
    let max = common::rules().catalog.soldier("test_rifleman").hp;
    let period = (service.first_aid_s * b.rules().tick_hz as f64).round() as u64;
    let treatments = 5;
    run(&mut b, deploy_ticks() + period * treatments + 2);
    let hp = own(&b, Side::Blue, 1).unwrap().member_hp;
    let soldiers = hp.len() as u64;
    let healed: f64 = hp.iter().map(|h| h - wounded).sum();
    assert_eq!(healed, treatments as f64 * service.first_aid_hp);
    let spread =
        hp.iter().cloned().fold(f64::MIN, f64::max) - hp.iter().cloned().fold(f64::MAX, f64::min);
    assert!(
        spread <= service.first_aid_hp,
        "the most wounded is treated first: {hp:?}"
    );
    assert_eq!(
        stock(&b),
        full - treatments as u32 * service.stock_per_first_aid
    );
    // Every soldier back to his own health, and nothing more is paid for.
    let each = ((max - wounded) / service.first_aid_hp).ceil() as u64;
    run(&mut b, period * (soldiers * each - treatments) + 30 * 5);
    let squad = own(&b, Side::Blue, 1).unwrap();
    assert!(
        squad.member_hp.iter().all(|&h| h == max),
        "{:?}",
        squad.member_hp
    );
    assert_eq!(
        stock(&b),
        full - (soldiers * each) as u32 * service.stock_per_first_aid
    );
    assert_eq!(squad.service, ServiceStatus::Full);
}
