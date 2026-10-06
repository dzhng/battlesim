//! The unit catalog as the battle uses it: the view the browser reads stays
//! current, and every resolved type (the shipped ones and the test-only M1
//! family) passes the same generated checks: it sets up, moves, and fires
//! each of its mounts. Type 300 gets these checks with no test of its own.
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

#[test]
fn player_roster_resolves_shared_identities_and_disabled_future_cards() {
    use contract::catalog::Faction;
    let documents = sim::fixtures::catalog_documents();
    let catalog = contract::catalog::resolve(&documents).unwrap();
    for faction in [Faction::Us, Faction::Europe, Faction::Eastern] {
        let cards: Vec<_> = catalog
            .cards()
            .filter(|card| card.roster.factions.contains(&faction))
            .collect();
        assert_eq!(cards.len(), 50, "{faction:?}");
        let mut families = std::collections::BTreeMap::<_, std::collections::BTreeSet<_>>::new();
        for card in cards {
            families
                .entry(card.roster.category)
                .or_default()
                .insert(card.family);
        }
        assert!(families.values().all(|families| families.len() <= 10));
    }
    let shared = catalog.card("f_35a").unwrap();
    assert_eq!(shared.roster.factions, [Faction::Us, Faction::Europe]);
    assert_eq!(shared.roster.family_name, "F-35 Lightning II");
    assert!(shared.disabled_reason.is_some());
    assert!(catalog.index("f_35a").is_none());
    // The same owner serializes every planned identity without admitting physics.
    let round_trip: contract::catalog::Catalog =
        serde_json::from_value(serde_json::to_value(&catalog).unwrap()).unwrap();
    assert_eq!(catalog, round_trip);
}

#[test]
fn faction_supply_trucks_start_with_two_hundred_fifty_stock() {
    let catalog = contract::catalog::resolve(&sim::fixtures::catalog_documents()).unwrap();
    for id in [
        "us_m977_hemtt_general_resupply",
        "europe_man_hx_general_resupply",
        "eastern_ural_4320_general_resupply",
    ] {
        assert_eq!(
            catalog.by_id(id).capabilities.supply.unwrap().stock,
            250,
            "{id}"
        );
    }
}

/// The shipped fixture with the M1 family's catalog document and the
/// weapon row its M1A1 swaps in (a row that `extends` the tank's sabot).
fn with_m1_family() -> Value {
    let mut fixture = common::game();
    let family: Value = serde_json::from_str(include_str!("fixtures/m1-family.json")).unwrap();
    fixture["catalog"].as_array_mut().unwrap().push(family);
    fixture["weapons"]["m829"] = json!({
        "extends": "tank_ap", "name": "M829", "description": "Test only.", "icon": "ap_shell",
        "penetration": 240
    });
    fixture["service"]["round_costs"]["m829"] = json!(6);
    fixture
}

#[test]
fn shipped_rules_admit_every_weapon_before_battle_startup() {
    let rules: Rules = serde_json::from_value(common::game()).unwrap();
    let config = sim::flight::FlightConfig::new(&rules.physics.flight, rules.tick_hz).unwrap();
    for (id, weapon) in &rules.weapons {
        assert!(
            config.profile(&weapon.ballistics).is_ok(),
            "weapon {id} cannot start a battle: {:?}",
            config.profile(&weapon.ballistics).err()
        );
    }
}

#[test]
fn the_browsers_catalog_view_is_current() {
    let path = sim::fixtures::dir().join("catalog.json");
    let view = sim::fixtures::catalog_view();
    if std::env::var_os("BLESS_CATALOG").is_some() {
        std::fs::write(&path, &view).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == view,
        "fixtures/catalog.json is stale: rerun with BLESS_CATALOG=1 (`cargo test -p sim --test sim catalog::`)"
    );
}

#[test]
fn the_m1_family_resolves_its_variants_from_one_base() {
    let rules: Rules = serde_json::from_value(with_m1_family()).unwrap();
    let c = &rules.catalog;
    let tank = c.by_id("tank").hull().unwrap();
    assert!(c.index("m1").is_none(), "the abstract base is not a type");
    // One differs in armour.
    let ip = c.by_id("m1ip").hull().unwrap();
    assert_eq!((ip.armor.front, ip.armor.side), (180.0, tank.armor.side));
    // One swaps a mount's weapon, keeping the mount's geometry.
    let (a1, base) = (c.by_id("m1a1"), c.by_id("tank"));
    assert_eq!(a1.mounts[0].weapons, ["m829", "tank_he"]);
    assert_eq!(
        (a1.mounts[0].pivot_m, a1.mounts[0].muzzle_m),
        (base.mounts[0].pivot_m, base.mounts[0].muzzle_m)
    );
    assert_eq!(rules.weapons["m829"].penetration, 240.0);
    assert_eq!(
        rules.weapons["m829"].ballistics.speed_mps,
        rules.weapons["tank_ap"].ballistics.speed_mps
    );
    // One has its own model, the M1A1's mounts and a part's armour.
    let a2 = c.by_id("m1a2");
    assert_eq!(a2.appearance.as_deref(), Some("m1a2"));
    assert_eq!(a2.mounts, a1.mounts);
    assert_eq!(a2.hull().unwrap().armor.side, 160.0);
    assert_eq!(a2.parts, ["era"]);
}

/// A mount naming a weapon row the rules lack fails as the rules load,
/// naming the type or soldier kind, not later in a battle's setup.
#[test]
fn a_mount_naming_no_weapon_row_fails_at_load() {
    let load = |section: &str, id: &str, patch: Value| {
        let mut fixture = common::game();
        sim::fixtures::patch_catalog(&mut fixture, section, id, patch);
        serde_json::from_value::<Rules>(fixture)
            .unwrap_err()
            .to_string()
    };
    let e = load(
        "units",
        "tank",
        json!({ "mounts": [{ "name": "HMG", "weapons": ["railgun"] }] }),
    );
    assert!(
        e.contains("units.tank: mount \"HMG\" names weapon row \"railgun\""),
        "{e}"
    );
    let e = load(
        "soldiers",
        "rifleman",
        json!({ "mounts": [{ "name": "rifles", "weapons": ["musket"] }] }),
    );
    // Every kind extending the rifleman inherits it: the first is named.
    assert!(
        e.contains("soldiers.") && e.contains(": mount \"rifles\" names weapon row \"musket\""),
        "{e}"
    );
}

/// Firing mechanics use fixed reach rather than the live grenade balance.
fn grenade_firing_profile(fixture: &mut Value) {
    fixture["weapons"]["grenade"]["range_m"] = json!(450);
    fixture["weapons"]["grenade"]["speed_mps"] = json!(100);
    fixture["weapons"]["grenade"]["gravity_scale"] = json!(0.09);
    fixture["weapons"]["grenade"]["scatter_mrad"] = json!(30);
}

/// A soldier's weapon falls with him, unless it is `special`: then the next
/// living soldier takes it up, and the squad keeps it while anyone remains.
#[test]
fn a_fallen_carriers_weapon_is_lost_unless_it_is_special() {
    for special in [true, false] {
        let mut fixture = common::game();
        grenade_firing_profile(&mut fixture);
        let patch = |f: &mut Value, section, id, p| sim::fixtures::patch_catalog(f, section, id, p);
        // The grenadier in the squad's last slot, so its one casualty is him.
        let slots = json!({ "body": { "squad": { "slots": [
            "rifleman", "rifleman", "rifleman", "rifleman",
            "rifleman", "rifleman", "rifleman", "grenadier"
        ] } } });
        patch(&mut fixture, "units", "rifle", slots);
        let launcher = json!({ "mounts": [{ "name": "grenade launcher", "special": special }] });
        patch(&mut fixture, "soldiers", "grenadier", launcher);
        patch(&mut fixture, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
        let range = fixture["weapons"]["grenade"]["range_m"].as_f64().unwrap();
        let setup = serde_json::from_value(json!({
            "map": { "size": [700, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": fixture,
            "units": [
                { "side": "blue", "kind": "rifle", "position": [200, 300], "condition": { "casualties": 1 } },
                { "side": "red", "kind": "rifle", "position": [200.0 + range * 0.5, 300], "engagement": "return_fire_only" },
            ],
            "events": [], "scripts": [],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 1);
        for _ in 0..60 * b.rules().tick_hz {
            b.step();
        }
        let launcher = &b.unit(UnitId(0)).unwrap().mounts[1];
        assert_eq!(
            launcher.shots > 0,
            special,
            "special {special}: {launcher:?}"
        );
    }
}

/// A red rifle squad and tank for the type under test to fire on: both
/// too tough to fall, and holding fire until fired on.
fn targets(range: f64) -> Value {
    json!([
        { "side": "red", "kind": "rifle", "position": [200.0 + range * 0.5, 300.0 - range * 0.3], "engagement": "return_fire_only" },
        { "side": "red", "kind": "tank", "position": [200.0 + range * 0.5, 300.0 + range * 0.3], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
    ])
}

/// A battle on open ground: the type under test at [200, 300], and `others`.
fn battle(fixture: &Value, id: &str, others: Value) -> Battle {
    let mut units = vec![json!({ "side": "blue", "kind": id, "position": [200, 300] })];
    units.extend(others.as_array().unwrap().iter().cloned());
    let setup = serde_json::from_value(json!({
        "map": { "size": [700, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Every resolved type: it sets up, fires each mount within a minute at a
/// squad and a tank in reach, and on open ground drives or walks where it
/// is ordered.
#[test]
fn every_unit_type_sets_up_fires_each_mount_and_moves() {
    let mut fixture = with_m1_family();
    grenade_firing_profile(&mut fixture);
    sim::fixtures::patch_catalog(&mut fixture, "soldiers", "rifleman", json!({ "hp": 1.0e6 }));
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "tank",
        json!({ "body": { "hull": { "hp": 1.0e6 } } }),
    );
    let rules: Rules = serde_json::from_value(fixture.clone()).unwrap();
    let mut failures = Vec::new();
    for t in rules.catalog.indices() {
        let id = rules.catalog.id(t);
        let range = rules
            .catalog
            .mounts(t)
            .iter()
            .flat_map(|m| &m.def.weapons)
            .map(|w| rules.weapons[w].ballistics.range_m)
            .fold(f64::INFINITY, f64::min);
        let mut b = battle(&fixture, id, targets(range.min(300.0)));
        for _ in 0..60 * rules.tick_hz {
            b.step();
        }
        for (m, mount) in b.unit(UnitId(0)).unwrap().mounts.iter().enumerate() {
            if mount.shots == 0 {
                let name = &rules.catalog.mounts(t)[m].def.name;
                failures.push(format!(
                    "{id}: mount {name} never fired ({:?})",
                    mount.reason
                ));
            }
        }

        let mut b = battle(&fixture, id, json!([]));
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            seq: 1,
            order: Order::Move {
                units: vec![UnitId(0)],
                gesture: 1,
                goal: [200.0, 500.0],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None, "{id}: {ack:?}");
        for _ in 0..90 * rules.tick_hz {
            b.step();
        }
        let short =
            (b.unit(UnitId(0)).unwrap().position.xy() - sim::math::v2(200.0, 500.0)).length();
        if short > 10.0 {
            failures.push(format!("{id}: stopped {short:.1} m short of its goal"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
#[test]
fn vehicle_wrecks_cannot_require_missing_building_facts() {
    let mut raw = crate::common::game();
    sim::fixtures::patch_catalog(
        &mut raw,
        "props",
        "light_wreck",
        serde_json::json!({"body":{"hp_scale":"building_floor_bands"}}),
    );
    let error = serde_json::from_value::<contract::scenario::Rules>(raw)
        .expect_err("vehicle wreck births have no placed aggregate");
    assert!(error.to_string().contains("wreck"), "{error}");
}
