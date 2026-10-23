//! What a replay is pinned to: the scenario it was played on and the rules
//! it was played by.

use super::*;

pub(super) fn scenario_digest(setup: &ScenarioDefinition) -> u64 {
    let map = serde_json::to_string(&setup.map).expect("map serializes");
    let units = serde_json::to_string(&setup.units).expect("units serialize");
    let events = serde_json::to_string(&setup.events).expect("events serialize");
    let scripts = serde_json::to_string(&setup.scripts).expect("scripts serialize");
    let authored_scripts =
        serde_json::to_string(&setup.authored_scripts).expect("authored scripts serialize");
    let victory = serde_json::to_string(&setup.victory).expect("victory serializes");
    // The opponent's policy and the encounter's rules change what a replay's
    // commands mean, so a replay is pinned to them too.
    let opponent = serde_json::to_string(&setup.opponent).expect("opponent serializes");
    let encounter = serde_json::to_string(&setup.encounter).expect("encounter serializes");
    let skirmish_identity = setup
        .skirmish
        .as_ref()
        .map(|_| {
            serde_json::to_string(&setup.players).expect("players serialize")
                + &serde_json::to_string(&setup.skirmish).expect("skirmish serializes")
        })
        .unwrap_or_default();
    digest::of_str(
        &(map
            + &units
            + &events
            + &scripts
            + &authored_scripts
            + &victory
            + &opponent
            + &encounter
            + &skirmish_identity),
    )
}

pub(super) fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}
