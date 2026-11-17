//! The skirmish's rows in the publication: one per player of the observed
//! side, and each pending purchase with its owner and carrier.
use super::{decode, full_fog, names};
use crate::common;
use contract::ids::Side;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication;

#[test]
fn publication_layout_exposes_player_rows_instead_of_side_economy() {
    let map = json!({ "size": [800, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35, "props": [] });
    let battle = Battle::new(
        &common::scenario(
            &map.to_string(),
            json!([{ "side": "blue", "kind": "test_rifle", "position": [100, 300] }]),
            json!([]),
        ),
        1,
    );
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let header = names(&layout["header"]);
    assert!(header.contains(&"playersCount".to_owned()));
    assert!(!header.contains(&"readyBlue".to_owned()));
    assert!(!header.contains(&"credits".to_owned()));
    assert_eq!(
        names(
            &layout["groups"]
                .as_array()
                .unwrap()
                .iter()
                .find(|group| group["name"] == "players")
                .expect("players group")["fields"]
        ),
        vec![
            "index",
            "id",
            "controller",
            "ready",
            "credits",
            "incomePerMinute",
            "unitCount",
            "cap"
        ]
    );
}

#[test]
fn two_player_publication_keeps_player_rows_and_pending_owner_and_carrier() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/parity/publication/2v2-frame.json"
    ))
    .unwrap();
    let mut setup = crate::skirmish::setup();
    let mut blue_two = setup.players[0].clone();
    blue_two.id = "blue-2".into();
    let mut red_two = setup.players[1].clone();
    red_two.id = "red-2".into();
    setup.players.extend([blue_two, red_two]);
    setup.skirmish.as_mut().unwrap().sites.entries.extend([
        contract::encounter::EntrySite {
            side: Side::Blue,
            center: [200.0, 10.0],
            yaw: std::f64::consts::FRAC_PI_2,
        },
        contract::encounter::EntrySite {
            side: Side::Red,
            center: [200.0, 590.0],
            yaw: -std::f64::consts::FRAC_PI_2,
        },
    ]);
    let battle = Battle::new(&setup, 1);
    let mut frame = battle.observe(Side::Blue).clone();
    frame.skirmish.as_mut().unwrap().players[1]
        .pending
        .push(contract::skirmish::PendingPurchase {
            id: contract::skirmish::PurchaseId(7),
            kind: common::unit_kind("test_recon"),
            carrier: Some(common::unit_kind("test_apc")),
            destination: [100.0, 20.0],
            confirmed_tick: 4,
            blocked: false,
            arrival_tick: None,
            entry: 0,
        });
    let layout: Value = serde_json::from_str(&publication::layout_json(&battle)).unwrap();
    let patch = publication::GroundHeader {
        epoch: 1,
        side: Side::Blue,
        base: 0,
        revision: 1,
        full: true,
        count: 0,
    };
    let mut data = Vec::new();
    publication::pack_logical(
        &frame,
        &[],
        &patch,
        &full_fog(),
        std::iter::empty(),
        &mut data,
    )
    .unwrap();
    let groups = decode(&layout, &data);
    assert_eq!(groups["players"].len(), 2);
    assert_eq!(groups["players"][1].fields["id"], 2.0);
    assert_eq!(groups["pendingPurchases"][0].fields["owner"], 2.0);
    let carrier = groups["pendingPurchases"][0].fields["carrier"];
    assert_eq!(carrier, common::unit_kind("test_apc").0 as f32);
    assert_eq!(golden["players"][1]["pending"][0]["carrier"], "test_apc");
    assert_eq!(golden["players"][1]["pending"][0]["owner"], 2);
}
