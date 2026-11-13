//! The catalog view the browser reads (`fixtures/catalog.json`): current,
//! and each weapon row publishing what its contract derives.
use super::*;

/// Every weapon row the browser reads carries, under `derived`, what the
/// contract's methods derive from it, beside its authored fields untouched.
/// Generated over the shipped rows with `default` and `ammo` forced to each
/// combination, so both answers of `fires_regardless` are published.
#[test]
fn every_published_weapon_row_carries_its_contracts_derived_facts() {
    let mut regardless = std::collections::BTreeSet::new();
    for default in [false, true] {
        for unlimited in [false, true] {
            let mut game = fixtures::rules();
            for row in game["weapons"].as_object_mut().unwrap().values_mut() {
                let row = row.as_object_mut().unwrap();
                row.insert("default".into(), json!(default));
                row.remove("round_cost");
                if unlimited {
                    row.insert("ammo".into(), json!("unlimited"));
                } else {
                    row.insert("ammo".into(), json!(30));
                    row.insert("round_cost".into(), json!(1));
                }
            }
            let view = fixtures::admit(
                game.clone(),
                fixtures::catalog_documents(fixtures::CatalogSet::Game),
            )
            .unwrap_or_else(|e| panic!("default {default}, unlimited {unlimited}: {e}"));
            let rows = contract::weapons::resolve_weapons(game["weapons"].clone()).unwrap();
            for (id, def) in &rows {
                let mut published = view["weapons"][id].clone();
                let derived = published
                    .as_object_mut()
                    .and_then(|row| row.remove("derived"))
                    .unwrap_or_else(|| panic!("weapons.{id} publishes no derived facts"));
                assert_eq!(
                    derived,
                    serde_json::to_value(def.derived()).unwrap(),
                    "weapons.{id}"
                );
                assert_eq!(
                    published,
                    serde_json::to_value(def).unwrap(),
                    "weapons.{id}"
                );
                regardless.insert(def.fires_regardless());
            }
        }
    }
    assert_eq!(regardless.len(), 2, "both answers are published");
}

#[test]
fn the_browsers_catalog_view_is_current() {
    let path = fixtures::dir().join("catalog.json");
    let view = fixtures::catalog_view();
    if std::env::var_os("BLESS_CATALOG").is_some() {
        std::fs::write(&path, &view).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == view,
        "fixtures/catalog.json is stale: rerun with BLESS_CATALOG=1 (`cargo test -p sim --test sim catalog::`)"
    );
}
