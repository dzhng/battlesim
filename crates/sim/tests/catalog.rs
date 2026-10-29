//! The unit catalog as the battle uses it: the view the browser reads stays
//! current, and every test unit type (`fixtures/units/test` and the
//! test-only M1 family) passes the same generated checks: it sets up, moves,
//! and fires each of its mounts. The faction roster is data on the same
//! rules: it is admitted where the view is written, and no test plays it;
//! only roster-wide data contracts (no squad runs dry) read its entries.
use contract::catalog::AltitudeLayer;
use contract::command::{CommandEnvelope, MoveDirection, Order, RoutePolicy, TargetRef};
use contract::ids::{Side, UnitId};
use contract::scenario::Rules;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

#[test]
fn disabled_model_manifest_covers_cards_without_admitting_units() {
    #[derive(serde::Deserialize)]
    struct Manifest {
        schema_version: u32,
        model_statuses: std::collections::BTreeMap<String, String>,
        entries: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        id: String,
        family: String,
        category: String,
        model_status: String,
        source_family: String,
        source_kind: String,
        disabled_reason: String,
        source_path: String,
    }

    let path = sim::fixtures::dir().join("units/model-manifest.json");
    let manifest: Manifest = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(manifest.schema_version, 1);
    let catalog = contract::catalog::resolve(&sim::fixtures::catalog_documents(
        sim::fixtures::CatalogSet::Game,
    ))
    .unwrap();
    let disabled: std::collections::BTreeMap<_, _> = catalog
        .cards()
        .filter(|card| card.disabled_reason.is_some())
        .map(|card| (card.id.to_owned(), card))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    let identity: Vec<String> = manifest
        .entries
        .iter()
        .map(|e| source_identity(&e.source_path))
        .collect();
    for (k, entry) in manifest.entries.iter().enumerate() {
        assert!(
            seen.insert(entry.id.clone()),
            "duplicate model manifest entry: {}",
            entry.id
        );
        let card = disabled
            .get(entry.id.as_str())
            .unwrap_or_else(|| panic!("model manifest entry is not a disabled card: {}", entry.id));
        assert_eq!(entry.family, card.family);
        assert_eq!(
            entry.category,
            format!("{:?}", card.roster.category).to_lowercase()
        );
        assert!(
            manifest.model_statuses.contains_key(&entry.model_status),
            "{}: model_status {:?} is not one of the manifest's model_statuses",
            entry.id,
            entry.model_status
        );
        if let Err(why) = disabled_source(&entry.source_path) {
            panic!("{}: {why}", entry.id);
        }
        // A rebuilt card has its own model: no other card's file, by path or content.
        if entry.model_status == "reference_built" {
            for (j, other) in manifest.entries.iter().enumerate().filter(|(j, _)| *j != k) {
                assert!(
                    other.source_path != entry.source_path && identity[j] != identity[k],
                    "{} shares its model with {}",
                    entry.id,
                    other.id
                );
            }
        }
        // Its reference library, which its receipt and review sheet name: the
        // folder its source is in, unless that is the shared `disabled/`.
        let folder = entry
            .source_path
            .trim_start_matches("assets/source/roster/")
            .trim_start_matches("infantry/")
            .split('/')
            .next()
            .unwrap_or_default();
        assert!(
            folder == "disabled" || folder == entry.source_family,
            "{}: source_family {:?} is not its source's folder {folder:?}",
            entry.id,
            entry.source_family
        );
        let library = format!("assets/references/{}/references.json", entry.source_family);
        assert!(
            sim::fixtures::dir()
                .parent()
                .unwrap()
                .join(&library)
                .is_file(),
            "{}: source_family {:?} has no {library}",
            entry.id,
            entry.source_family
        );
        assert!(!entry.source_kind.is_empty());
        assert_eq!(Some(entry.disabled_reason.as_str()), card.disabled_reason);
        assert!(
            catalog.index(&entry.id).is_none(),
            "disabled model became admitted: {}",
            entry.id
        );
    }
    assert_eq!(seen.len(), disabled.len());
    assert_eq!(seen, disabled.keys().cloned().collect());
}

#[test]
fn every_faction_has_a_sead_aircraft() {
    // A SEAD aircraft finds radars (a receiver) and fires at them (an
    // anti-radiation row): a roster guideline, not a rule.
    let rules: contract::scenario::Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let catalog = &rules.catalog;
    for faction in ["us", "europe", "eastern"] {
        assert!(
            catalog.indices().any(|i| {
                let t = catalog.get(i);
                t.faction == faction
                    && t.mobility.flight().is_some()
                    && t.capabilities.radar_receiver.is_some()
                    && catalog
                        .mounts(i)
                        .iter()
                        .flat_map(|m| &m.def.weapons)
                        .any(|w| rules.weapons[w].anti_radiation)
            }),
            "{faction} has no SEAD aircraft"
        );
    }
}

#[test]
fn every_faction_fields_a_ground_missile_jet_and_a_bomb_jet() {
    // Read across enabled and planned cards: a planned card's weapons are its
    // icons, so the contract holds while a row waits on the bench.
    let view: Value = serde_json::from_str(
        &std::fs::read_to_string(sim::fixtures::dir().join("catalog.json")).unwrap(),
    )
    .unwrap();
    let mut roles = std::collections::BTreeMap::<&str, (bool, bool)>::new();
    for card in view["cards"].as_array().unwrap() {
        let Some(roster) = card["roster"].as_object() else {
            continue;
        };
        if roster["category"] != "air" {
            continue;
        }
        let Some(factions) = roster["factions"].as_array() else {
            continue;
        };
        let enabled = card["disabled_reason"].is_null();
        let (ground, bomb) = if enabled {
            let unit = view["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|unit| unit["id"] == card["id"])
                .unwrap_or_else(|| panic!("missing enabled air unit {}", card["id"]));
            let weapons: Vec<&str> = unit["mounts"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|mount| mount["weapons"].as_array().into_iter().flatten())
                .filter_map(Value::as_str)
                .collect();
            (
                weapons.contains(&"air_ground_missile"),
                weapons
                    .iter()
                    .any(|w| *w == "air_bomb" || *w == "air_bomb_cluster"),
            )
        } else {
            let icons: Vec<&str> = card["planned_weapons"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|weapon| weapon["icon"].as_str())
                .collect();
            (icons.contains(&"atgm"), icons.contains(&"bomb"))
        };
        for faction in factions.iter().filter_map(Value::as_str) {
            let entry = roles.entry(faction).or_insert((false, false));
            entry.0 |= ground;
            entry.1 |= bomb;
        }
    }
    for faction in ["us", "europe", "eastern"] {
        assert_eq!(
            roles.get(faction),
            Some(&(true, true)),
            "{faction} lacks a ground-missile or bomb jet across enabled and planned loadouts"
        );
    }
}

#[test]
fn no_jet_carries_bombs_and_ground_missiles() {
    let view: Value = serde_json::from_str(
        &std::fs::read_to_string(sim::fixtures::dir().join("catalog.json")).unwrap(),
    )
    .unwrap();
    for card in view["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|card| card["roster"]["category"] == "air")
    {
        let weapons: (bool, bool) = if card["disabled_reason"].is_null() {
            let unit = view["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|unit| unit["id"] == card["id"])
                .unwrap();
            let ws: Vec<&str> = unit["mounts"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|m| m["weapons"].as_array().into_iter().flatten())
                .filter_map(Value::as_str)
                .collect();
            (
                ws.contains(&"air_ground_missile"),
                ws.iter()
                    .any(|w| *w == "air_bomb" || *w == "air_bomb_cluster"),
            )
        } else {
            let icons: Vec<&str> = card["planned_weapons"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|w| w["icon"].as_str())
                .collect();
            (icons.contains(&"atgm"), icons.contains(&"bomb"))
        };
        assert!(
            !(weapons.0 && weapons.1),
            "{} carries both ground missiles and bombs",
            card["id"]
        );
    }
}

/// D25: every unit the roster fields is a full row the battle plays, in every
/// category. The only cards still planned are drones, which wait for their own
/// spec (D37). A planned row names what it would become in its
/// `planned.profile`; the drone profiles are the one place drones are named.
#[test]
fn only_drones_are_planned() {
    const DRONE_PROFILES: [&str; 5] = [
        "armed-drone",
        "kamikaze-armor",
        "kamikaze-personnel",
        "scout-drone",
        "thermal-drone",
    ];
    let planned: Vec<String> = sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game)
        .iter()
        .filter_map(|doc| doc["units"].as_object())
        .flatten()
        .filter_map(|(id, unit)| {
            let profile = unit["planned"]["profile"].as_str()?;
            (!DRONE_PROFILES.contains(&profile)).then(|| format!("{id} ({profile})"))
        })
        .collect();
    assert!(
        planned.is_empty(),
        "non-drone rows still planned: {planned:?}"
    );
}

/// A disabled card's source model: present at its `source_path`, and a
/// binary glTF (version 2) whose JSON names meshes and a scene, or the Git
/// LFS pointer to one in a checkout that has not pulled it. `asset validate`
/// judges the model itself.
fn disabled_source(source_path: &str) -> Result<(), String> {
    let path = sim::fixtures::dir().parent().unwrap().join(source_path);
    let bytes = std::fs::read(&path).map_err(|e| format!("{source_path}: {e}"))?;
    if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        let text = String::from_utf8_lossy(&bytes);
        return if text.lines().any(|l| l.starts_with("oid sha256:")) && text.contains("\nsize ") {
            Ok(())
        } else {
            Err(format!("{source_path}: a malformed Git LFS pointer"))
        };
    }
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    };
    if bytes.get(0..4) != Some(b"glTF") || word(4) != Some(2) || word(16) != Some(0x4E4F534A) {
        return Err(format!(
            "{source_path}: not a binary glTF 2.0 with its JSON chunk first"
        ));
    }
    let length = word(12).unwrap() as usize;
    let json: Value = bytes
        .get(20..20 + length)
        .and_then(|chunk| serde_json::from_slice(chunk).ok())
        .ok_or_else(|| format!("{source_path}: its JSON chunk does not parse"))?;
    let named = |key: &str| json[key].as_array().is_some_and(|a| !a.is_empty());
    if named("meshes") && named("scenes") {
        Ok(())
    } else {
        Err(format!("{source_path}: draws no mesh in a scene"))
    }
}

/// What two source files compare by: an LFS pointer's oid (the content's
/// sha256), else a hash of the bytes; a pulled file and another's pointer are
/// not compared (pull both to compare them).
fn source_identity(source_path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let bytes = std::fs::read(sim::fixtures::dir().parent().unwrap().join(source_path)).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    match text.lines().find_map(|l| l.strip_prefix("oid sha256:")) {
        Some(oid) if bytes.starts_with(b"version https://git-lfs") => oid.to_owned(),
        _ => {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut h);
            format!("bytes {:x} {}", h.finish(), bytes.len())
        }
    }
}

#[test]
fn a_disabled_source_that_is_missing_or_not_a_model_is_refused() {
    assert!(disabled_source("assets/source/roster/disabled/no_such_card.glb").is_err());
    assert!(disabled_source("fixtures/units/model-manifest.json").is_err());
}

/// The test fixture with the M1 family's catalog document and the
/// weapon row its M1A1 swaps in (a row that `extends` the tank's sabot).
fn with_m1_family() -> Value {
    let mut fixture = sim::fixtures::test_game();
    let family: Value = serde_json::from_str(include_str!("fixtures/m1-family.json")).unwrap();
    fixture["catalog"].as_array_mut().unwrap().push(family);
    fixture["weapons"]["m829"] = json!({
        "extends": "tank_ap", "name": "M829", "description": "Test only.", "icon": "ap_shell",
        "penetration": 240
    });
    fixture["service"]["round_costs"]["m829"] = json!(6);
    fixture
}

/// The test and capture document sets are the game's set plus their own
/// folders, each document once, and resolve as one catalog: a lab never runs
/// without the game's units, and a document both sets name is not doubled.
#[test]
fn the_test_and_capture_sets_extend_the_games() {
    use sim::fixtures::{catalog_documents, CatalogSet};
    let game = catalog_documents(CatalogSet::Game);
    for set in [CatalogSet::Test, CatalogSet::Capture] {
        let documents = catalog_documents(set);
        for doc in &game {
            let copies = documents.iter().filter(|d| *d == doc).count();
            assert_eq!(copies, 1, "{set:?} holds a game document {copies} times");
        }
        if let Err(e) = contract::catalog::resolve(&documents) {
            panic!("the {set:?} set does not resolve: {e}");
        }
    }
    // The capture units extend the test units, so the capture set holds them too.
    let test = catalog_documents(CatalogSet::Test);
    let capture = catalog_documents(CatalogSet::Capture);
    for doc in sim::fixtures::test_documents() {
        assert!(
            test.contains(&doc),
            "the test set lacks a test unit document"
        );
        assert!(
            capture.contains(&doc),
            "the capture set lacks a test unit document"
        );
    }
}

/// The rules a native test reads carry no ground: a test that needs a map
/// names its test map, so no test stands on a battlefield by default.
#[test]
fn the_native_rules_fixture_carries_no_map() {
    assert!(sim::fixtures::game().get("map").is_none());
    assert!(sim::fixtures::test_game().get("map").is_none());
    let grounded = sim::fixtures::with_map("geometry");
    let map: contract::map::MapDefinition =
        serde_json::from_value(grounded["map"].clone()).expect("the test map, under `map`");
    let geometry = sim::maps::load("geometry").unwrap().definition;
    assert_eq!(map.size, geometry.size);
    assert_eq!(grounded["catalog"], sim::fixtures::test_game()["catalog"]);
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

/// What players get is the roster: every concrete unit type in the committed
/// catalog is a roster card, every soldier kind fills a slot of one, and no
/// test or menu unit ships.
#[test]
fn the_game_catalog_holds_only_what_roster_cards_reach() {
    let path = sim::fixtures::dir().join("catalog.json");
    let view: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cards: std::collections::BTreeSet<&str> = view["cards"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["disabled_reason"].is_null())
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    let mut stray = Vec::new();
    let mut reached = std::collections::BTreeSet::new();
    for unit in view["units"].as_array().unwrap() {
        let id = unit["id"].as_str().unwrap();
        if !cards.contains(id) {
            stray.push(format!("unit {id} is on no card"));
            continue;
        }
        for slot in unit["body"]["squad"]["slots"]
            .as_array()
            .into_iter()
            .flatten()
        {
            reached.insert(slot.as_str().unwrap());
        }
    }
    for id in view["soldiers"].as_object().unwrap().keys() {
        if !reached.contains(id.as_str()) {
            stray.push(format!("soldier kind {id} fills no card's slot"));
        }
    }
    for section in ["units", "soldiers", "props", "parts", "roles"] {
        for (id, _) in view["documents"][section].as_object().into_iter().flatten() {
            if id.starts_with("test_") || id.starts_with("capture_") {
                stray.push(format!("{section}.{id} is test or capture content"));
            }
        }
    }
    assert!(stray.is_empty(), "the game catalog ships: {stray:#?}");
}

#[test]
fn every_shipped_artillery_card_is_a_complete_physical_catalog_unit() {
    let view: Value = serde_json::from_str(
        &std::fs::read_to_string(sim::fixtures::dir().join("catalog.json")).unwrap(),
    )
    .unwrap();
    let ids = [
        "us_m109_paladin_m109a7",
        "us_m1129_stryker_mortar_120_mm_mortar_carrier",
        "us_m142_himars_gmlrs",
        "us_m142_himars_prsm",
        "us_m270_mlrs_m270a2",
        "europe_caesar_88",
        "europe_mars_ii_multiple_launch_rocket_system",
        "europe_pzh_2000_tracked_155_mm_howitzer",
        "eastern_2s19_msta_s_2s19m2",
        "eastern_9k515_tornado_s_heavy_rocket_artillery",
        "eastern_bm_21_grad_bm_21_rocket_artillery",
        "eastern_plz_05_chinese_tracked_155_mm_howitzer",
    ];
    let units = view["units"].as_array().unwrap();
    let cards = view["cards"].as_array().unwrap();
    for id in ids {
        let unit = units
            .iter()
            .find(|unit| unit["id"] == id)
            .unwrap_or_else(|| panic!("missing unit {id}"));
        let card = cards
            .iter()
            .find(|card| card["id"] == id)
            .unwrap_or_else(|| panic!("missing card {id}"));
        assert!(card["disabled_reason"].is_null(), "{id} remains disabled");
        assert_eq!(unit["roles"].as_array().unwrap(), &[json!("artillery")]);
        assert!(
            !unit["mounts"].as_array().unwrap().is_empty(),
            "{id} has no mount"
        );
        for mount in unit["mounts"].as_array().unwrap() {
            for weapon in mount["weapons"].as_array().unwrap() {
                let row = &view["weapons"][weapon.as_str().unwrap()];
                assert_eq!(
                    row["trajectory"], "indirect",
                    "{id} has a direct artillery weapon"
                );
                assert!(
                    row["targets"].get("ground").is_some(),
                    "{id} weapon lacks ground targeting"
                );
            }
        }
        assert!(
            unit["appearance"].as_str().is_some(),
            "{id} has no appearance"
        );
        assert!(
            unit["body"]["hull"]["wreck"].as_str().is_some(),
            "{id} has no wreck"
        );
        assert!(
            unit["sound"]["profile"].as_str().is_some(),
            "{id} has no sound profile"
        );
    }
}

/// No roster squad runs dry: every soldier who fills a card's slot carries
/// an unlimited squad gun, and any finite weapon is a specialist's extra
/// (`special`) that a comrade takes up when he falls. A sniper whose big
/// rifle is empty still has his rifle.
#[test]
fn every_roster_soldier_carries_an_unlimited_squad_gun() {
    let rules: Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let c = &rules.catalog;
    let unlimited = |w: &String| {
        matches!(
            rules.weapons[w].ammo,
            contract::weapons::AmmoCapacity::Unlimited(_)
        )
    };
    let mut dry = Vec::new();
    for t in c.indices() {
        for slot in c.get(t).slots().into_iter().flatten() {
            let kind = c.soldier(slot);
            for m in &kind.mounts {
                let finite = m.weapons.iter().any(|w| !unlimited(w));
                if m.squad && finite {
                    dry.push(format!("{slot}: its squad gun {:?} runs dry", m.id));
                }
                if !m.squad && !m.special {
                    dry.push(format!("{slot}: {:?} falls with him", m.id));
                }
            }
            if !kind
                .mounts
                .iter()
                .any(|m| m.squad && m.weapons.iter().all(unlimited))
            {
                dry.push(format!("{slot}: no unlimited squad gun"));
            }
        }
    }
    dry.sort();
    dry.dedup();
    assert!(dry.is_empty(), "{dry:#?}");
}

/// The roster is admitted without the test units: no roster kind
/// inherits from a test one, so editing a test unit changes no roster
/// squad.
#[test]
fn the_roster_resolves_without_the_test_units() {
    let game: Value = serde_json::from_str(
        &std::fs::read_to_string(sim::fixtures::dir().join("game.json")).unwrap(),
    )
    .unwrap();
    let documents = sim::fixtures::catalog_documents(sim::fixtures::CatalogSet::Game);
    if let Err(e) = sim::fixtures::admit(game, documents) {
        panic!("the roster leans on documents outside it: {e}");
    }
}

#[test]
fn the_m1_family_resolves_its_variants_from_one_base() {
    let rules: Rules = serde_json::from_value(with_m1_family()).unwrap();
    let c = &rules.catalog;
    let tank = c.by_id("test_tank").hull().unwrap();
    assert!(c.index("m1").is_none(), "the abstract base is not a type");
    // One differs in armour.
    let ip = c.by_id("m1ip").hull().unwrap();
    assert_eq!((ip.armor.front, ip.armor.side), (180.0, tank.armor.side));
    // One swaps a mount's weapon, keeping the mount's geometry.
    let (a1, base) = (c.by_id("m1a1"), c.by_id("test_tank"));
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
        "test_tank",
        json!({ "mounts": [{ "id": "HMG", "weapons": ["railgun"] }] }),
    );
    assert!(
        e.contains("units.test_tank: mount \"HMG\" names weapon row \"railgun\""),
        "{e}"
    );
    let e = load(
        "soldiers",
        "test_rifleman",
        json!({ "mounts": [{ "id": "rifles", "weapons": ["musket"] }] }),
    );
    // Every kind extending the rifleman inherits it: the first is named.
    assert!(
        e.contains("soldiers.") && e.contains(": mount \"rifles\" names weapon row \"musket\""),
        "{e}"
    );
}

/// A bomb is released only on a winged aircraft's pass, the one flight that
/// plans a release point: a bomb row on a helicopter or a ground vehicle
/// is refused as the rules load, naming the type.
#[test]
fn only_a_winged_aircraft_carries_a_bomb() {
    let load = |id: &str| {
        let mut fixture = common::game();
        fixture["weapons"]["test_bomb"] = json!({
            "name": "Test Bomb", "description": "A test row.", "icon": "bomb",
            "delivery": "unpowered", "stationary": false, "gravity_scale": 2,
            "aim_s": 0.1, "reload_s": 3, "ammo": 2, "penetration": 10,
            "partial_from": 0.08, "damage": 50,
            "near_miss_suppression": 0, "suppression_radius_m": 0,
            "blast_radius_m": 5, "structural_damage": 10,
            "targets": { "ground": { "classes": ["vehicle"], "range_m": 0, "scatter_mrad": 1 } }
        });
        fixture["service"]["round_costs"]["test_bomb"] = json!(1);
        sim::fixtures::patch_catalog(
            &mut fixture,
            "units",
            id,
            json!({ "mounts": [{ "id": "rack", "name": "Rack", "weapons": ["test_bomb"],
                "turret": false, "pivot_m": [0, 0, 1], "muzzle_m": [1, 0, 0] }] }),
        );
        serde_json::from_value::<Rules>(fixture).map(|_| ())
    };
    load("test_jet").expect("a jet carries bombs");
    for id in ["test_heli", "test_tank"] {
        let e = load(id).unwrap_err().to_string();
        assert!(
            e.contains(&format!("units.{id}")) && e.contains("test_bomb"),
            "{id}: {e}"
        );
    }
}

fn grenade_firing_profile(fixture: &mut Value) {
    common::json_ground_reach(&mut fixture["weapons"]["grenade"], 450.0);
    fixture["weapons"]["grenade"]["speed_mps"] = json!(100);
    fixture["weapons"]["grenade"]["gravity_scale"] = json!(0.09);
    fixture["weapons"]["grenade"]["targets"]["ground"]["scatter_mrad"] = json!(30);
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
            "test_rifleman", "test_rifleman", "test_rifleman", "test_rifleman",
            "test_rifleman", "test_rifleman", "test_rifleman", "test_grenadier"
        ] } } });
        patch(&mut fixture, "units", "test_rifle", slots);
        let launcher = json!({ "mounts": [{ "id": "grenade launcher", "special": special }] });
        patch(&mut fixture, "soldiers", "test_grenadier", launcher);
        patch(
            &mut fixture,
            "soldiers",
            "test_rifleman",
            json!({ "hp": 1.0e6 }),
        );
        let range = fixture["weapons"]["grenade"]["range_m"].as_f64().unwrap();
        let setup = serde_json::from_value(json!({
            "map": { "size": [700, 600], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
            "rules": fixture,
            "units": [
                { "side": "blue", "kind": "test_rifle", "position": [200, 300], "condition": { "casualties": 1 } },
                { "side": "red", "kind": "test_rifle", "position": [200.0 + range * 0.5, 300], "engagement": "return_fire_only" },
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
        { "side": "red", "kind": "test_rifle", "position": [200.0 + range * 0.5, 300.0 - range * 0.3], "engagement": "return_fire_only" },
        { "side": "red", "kind": "test_tank", "position": [200.0 + range * 0.5, 300.0 + range * 0.3], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
    ])
}

/// A battle on open ground: the type under test at [200, 300], and `others`.
fn battle(fixture: &Value, id: &str, others: Value) -> Battle {
    battle_at(fixture, id, others, [200.0, 300.0], 700.0, 600.0)
}

fn battle_at(
    fixture: &Value,
    id: &str,
    others: Value,
    at: [f64; 2],
    width: f64,
    depth: f64,
) -> Battle {
    let mut units = vec![json!({ "side": "blue", "kind": id, "position": at })];
    units.extend(others.as_array().unwrap().iter().cloned());
    let setup = serde_json::from_value(json!({
        "map": { "size": [width, depth], "fog_cell_m": 8, "height_grid_m": 4, "slope_cutoff_deg": 35 },
        "rules": fixture, "units": units, "events": [], "scripts": [],
    }))
    .unwrap();
    Battle::new(&setup, 1)
}

/// Every test unit type: it sets up, fires each mount within a minute at a
/// squad and a tank in reach, and on open ground drives or walks where it
/// is ordered.
#[test]
fn every_unit_type_sets_up_fires_each_mount_and_moves() {
    let mut fixture = with_m1_family();
    grenade_firing_profile(&mut fixture);
    sim::fixtures::patch_catalog(
        &mut fixture,
        "soldiers",
        "test_rifleman",
        json!({ "hp": 1.0e6 }),
    );
    sim::fixtures::patch_catalog(
        &mut fixture,
        "units",
        "test_tank",
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
        let never_stops = rules.catalog.get(t).mobility.flight()
            == Some(contract::catalog::FlightModel::FixedWing);
        let mut others = if never_stops {
            json!([
                { "side": "red", "kind": "test_rifle", "position": [1100, 995], "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_tank", "position": [1100, 1005], "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
                { "side": "red", "kind": "test_heli", "position": [1200, 1000], "engagement": "return_fire_only" },
            ])
        } else {
            targets(range.min(300.0))
        };
        // A weapon that reaches only aircraft gets a helicopter hovering in
        // reach; the rest never see one, so none takes its return fire.
        let air_only = rules.catalog.mounts(t).iter().any(|m| {
            m.def.weapons.iter().all(|w| {
                !rules.weapons[w]
                    .targets
                    .contains_key(&AltitudeLayer::Ground)
            })
        });
        if air_only {
            others.as_array_mut().unwrap().push(json!(
                { "side": "red", "kind": "test_heli", "position": [400, 300], "engagement": "return_fire_only" }
            ));
        }
        let mut b = if never_stops {
            battle_at(&fixture, id, others, [500.0, 1000.0], 3000.0, 2000.0)
        } else {
            battle(&fixture, id, others)
        };
        if never_stops {
            let ack = b.accept(CommandEnvelope {
                side: Side::Blue,
                player: None,
                seq: 1,
                order: Order::Attack {
                    units: vec![UnitId(0)],
                    target: TargetRef::Ground {
                        point: [1100.0, 1000.0, 0.0],
                    },
                },
                queued: false,
            });
            assert_eq!(ack.error, None, "{id}: {ack:?}");
        }
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

        // A jet flies to its goal on the open field it fought on: the small
        // field lies wholly inside the edge guard's margin.
        let (mut b, goal) = if never_stops {
            (
                battle_at(&fixture, id, json!([]), [500.0, 1000.0], 3000.0, 2000.0),
                sim::math::v2(1500.0, 1300.0),
            )
        } else {
            (battle(&fixture, id, json!([])), sim::math::v2(200.0, 500.0))
        };
        let ack = b.accept(CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            order: Order::Move {
                units: vec![UnitId(0)],
                gesture: 1,
                goal: [goal.x, goal.y],
                route: RoutePolicy::Shortest,
                direction: MoveDirection::Forward,
                facing: None,
            },
            queued: false,
        });
        assert_eq!(ack.error, None, "{id}: {ack:?}");
        // A jet never stops and cannot turn inside its turning circle, so it
        // has arrived once it comes within one turning radius of its goal.
        let turning_radius = match b.unit(UnitId(0)).unwrap().motion {
            sim::units::Motion::Air(flight)
                if flight.model == contract::catalog::FlightModel::FixedWing =>
            {
                Some(flight.cruise_mps / flight.turn_rad_s)
            }
            _ => None,
        };
        if let Some(radius) = turning_radius {
            let mut closest = f64::INFINITY;
            for _ in 0..90 * rules.tick_hz {
                b.step();
                closest = closest.min((b.unit(UnitId(0)).unwrap().position.xy() - goal).length());
            }
            if closest > radius + 10.0 {
                failures.push(format!(
                    "{id}: never came within its {radius:.0} m turning radius of its goal ({closest:.1} m)"
                ));
            }
        } else {
            for _ in 0..90 * rules.tick_hz {
                b.step();
            }
            let short = (b.unit(UnitId(0)).unwrap().position.xy() - goal).length();
            if short > 10.0 {
                failures.push(format!("{id}: stopped {short:.1} m short of its goal"));
            }
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

/// No hull is admitted past the absolute limits for its drive: what the map
/// generator and the route planner are proven against (tests drive hulls
/// built at the limits), so every unit within them is covered too.
#[test]
fn a_hull_past_its_drives_limits_is_refused_at_load() {
    for (unit, patch, limit) in [
        (
            "test_tank",
            json!({ "body": { "hull": { "half_extents_m": [3.5, 2.2, 1.2] } } }),
            "tracked half width",
        ),
        (
            "test_tank",
            json!({ "body": { "hull": { "half_extents_m": [4.9, 1.8, 1.2] } } }),
            "tracked half length",
        ),
        (
            "test_supply",
            json!({ "body": { "hull": { "half_extents_m": [3.0, 1.7, 1.8] } } }),
            "wheeled half width",
        ),
        (
            "test_supply",
            json!({ "body": { "hull": { "half_extents_m": [6.3, 1.4, 1.8] } } }),
            "wheeled half length",
        ),
        (
            "test_supply",
            json!({ "mobility": { "wheeled": { "turning_radius_m": 8.5 } } }),
            "wheeled turning radius",
        ),
    ] {
        let mut fixture = sim::fixtures::test_game();
        sim::fixtures::patch_catalog(&mut fixture, "units", unit, patch);
        let error = serde_json::from_value::<Rules>(fixture)
            .expect_err(limit)
            .to_string();
        assert!(
            error.contains(unit) && error.contains(limit),
            "{limit}: {error}"
        );
    }
    // The shipped roster stands within them.
    serde_json::from_value::<Rules>(common::game()).unwrap();
}
