//! Admit supplied mechanics without loading a map or changing fixture files.
//! Reads { "game": <game.json>, "catalog": [<authored documents>] } on stdin.
use std::io::{self, Read};

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    game: Value,
    catalog: Vec<Value>,
}

fn validate(text: &str) -> Result<Value, String> {
    let value = contract::catalog::parse_document(text).map_err(|e| e.to_string())?;
    let request: Request = serde_json::from_value(value).map_err(|e| e.to_string())?;
    sim::fixtures::admit(request.game, request.catalog)
}

fn main() {
    let result = (|| {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        let view = validate(&text)?;
        serde_json::to_string_pretty(&view).map_err(|e| e.to_string())
    })();
    match result {
        Ok(view) => println!("{view}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap()
    }

    #[test]
    fn rejects_launch_profiles_tighter_than_the_games_accuracy_ceiling() {
        // A row nothing extends, so the error names it and no other.
        let mut game = game();
        game["weapons"]["heavy_sniper"]["scatter_mrad"] = json!(0);
        let error = sim::fixtures::admit(
            game,
            sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game),
        )
        .err()
        .expect("invalid spread must fail");
        assert!(
            error.contains("weapons.heavy_sniper")
                && error.contains("Landing spread must be at least")
                && error.contains("entered 0 m"),
            "{error}"
        );
    }
    #[test]
    fn rejects_more_weapon_rows_than_the_publication_can_carry() {
        let mut game = game();
        for index in 0..sim::publication::MAX_WEAPON_ROWS {
            game["weapons"][format!("extra_{index}")] = json!({ "extends": "rifle" });
        }
        let error = sim::fixtures::admit(
            game,
            sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game),
        )
        .err()
        .expect("too many weapon rows must fail");
        assert!(
            error.contains("weapons") && error.contains("publication"),
            "{error}"
        );
    }

    #[test]
    fn rejects_more_ammunition_kinds_than_a_mount_can_publish() {
        let game = game();
        let mut documents = sim::fixtures::test_documents();
        let doc = documents
            .iter_mut()
            .find(|doc| doc["units"].get("test_tank").is_some())
            .unwrap();
        doc["units"]["test_tank"]["mounts"][0]["weapons"] = json!(["tank_ap", "tank_he", "rifle"]);
        let error = sim::fixtures::admit(game, documents)
            .err()
            .expect("too many ammunition kinds must fail");
        assert!(
            error.contains("units.test_tank") && error.contains("publication"),
            "{error}"
        );
    }

    #[test]
    fn rejects_finite_ammunition_without_a_supply_price() {
        let mut game = game();
        game["weapons"]["rifle"]["ammo"] = json!(60);
        let error = sim::fixtures::admit(
            game,
            sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game),
        )
        .err()
        .expect("finite ammunition needs a supply price");
        assert!(
            error.contains("service.round_costs") && error.contains("rifle"),
            "{error}"
        );
    }

    #[test]
    fn admitted_shipped_view_matches_the_generated_catalog() {
        let view = sim::fixtures::admit(
            game(),
            sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game),
        )
        .unwrap();
        let text = serde_json::to_string_pretty(&view).unwrap() + "\n";
        assert_eq!(text, include_str!("../../../fixtures/catalog.json"));
    }

    #[test]
    fn supplied_weapon_inheritance_is_in_the_returned_view() {
        let mut game = game();
        game["weapons"]["custom_rifle"] = json!({"extends": "rifle", "damage": 123});
        let view = sim::fixtures::admit(
            game,
            sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game),
        )
        .unwrap();
        assert_eq!(view["weapons"]["custom_rifle"]["damage"], json!(123.0));
        assert_eq!(
            view["weapons"]["custom_rifle"]["speed_mps"],
            view["weapons"]["rifle"]["speed_mps"]
        );
    }

    #[test]
    fn the_cli_rejects_duplicate_keys_before_they_can_be_lost() {
        let error =
            validate(r#"{"game":{"weapons":{"rifle":{"damage":1,"damage":2}}},"catalog":[]}"#)
                .err()
                .expect("duplicate keys must fail");
        assert!(
            error.contains("damage") && error.contains("twice"),
            "{error}"
        );
    }

    #[test]
    fn supplied_mounts_must_name_existing_weapon_rows() {
        let mut documents = sim::fixtures::test_documents();
        let doc = documents
            .iter_mut()
            .find(|doc| doc["units"].get("test_tank").is_some())
            .unwrap();
        doc["units"]["test_tank"]["mounts"][0]["weapons"] = json!(["missing_weapon"]);
        let error = sim::fixtures::admit(game(), documents)
            .err()
            .expect("unknown weapons must fail");
        assert!(
            error.contains("test_tank") && error.contains("missing_weapon"),
            "{error}"
        );
    }
}
