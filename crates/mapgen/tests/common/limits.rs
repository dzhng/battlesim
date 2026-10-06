//! The game's own compile limits (`fixtures/generated-battle.json`), so a
//! test asks for a map as the game does.

pub fn game_limits() -> mapgen::CompileLimits {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/generated-battle.json")).unwrap();
    serde_json::from_value(file["limits"].clone()).unwrap()
}
