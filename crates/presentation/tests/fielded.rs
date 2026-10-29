//! A battle's units and effects are resolved from the runtime contents
//! `Fielding::contents` names, and from no others: a page that fetches
//! exactly those has everything Rust reads.
use presentation::assets::Runtime;
use presentation::fielded::{self, Fielding};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn a_battle_reads_only_the_contents_it_names() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let game: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures/rules.json")).unwrap()).unwrap();
    let catalog = presentation::static_world::files::rules(&root, "market-town")
        .unwrap()
        .catalog
        .view();
    let runtime = Runtime::open(&root.join("assets/runtime")).unwrap();
    let kinds = fielded::encounter_kinds(&root, "market-town", "menu").unwrap();
    let fielding = Fielding {
        presentation: &serde_json::from_slice(
            &std::fs::read(root.join("fixtures/presentation.json")).unwrap(),
        )
        .unwrap(),
        weapons: &game["weapons"],
        rules: &game,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    };
    let named: BTreeSet<String> = fielding.contents().unwrap().into_iter().collect();
    let read = RefCell::new(BTreeSet::new());
    let sections = fielding
        .resolve(&|hash| {
            assert!(
                named.contains(hash),
                "content {hash} was read but not named"
            );
            read.borrow_mut().insert(hash.to_owned());
            runtime.read(hash, "bundle.bin")
        })
        .unwrap();
    // Every drawn appearance and wreck is read; a skeleton no body wears
    // may go unread.
    let skeletons: BTreeSet<String> = runtime.catalog["skeletons"]
        .as_object()
        .unwrap()
        .values()
        .map(|h| h.as_str().unwrap().to_owned())
        .collect();
    let unread: Vec<_> = named.difference(&read.borrow()).cloned().collect();
    assert!(
        unread.iter().all(|h| skeletons.contains(h)),
        "named but unread: {unread:?}"
    );
    let typed = sections.typed().unwrap();
    assert!(!typed.units.appearances.is_empty() && !typed.tables.is_empty());
}
